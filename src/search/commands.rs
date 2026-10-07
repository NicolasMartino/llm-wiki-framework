use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::thread::sleep;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::backup_policy::exclude_rebuildable_from_time_machine;
use crate::cli::{
    CliContext, IndexAllArgs, IndexArgs, OutputFormat, SearchAllArgs, SearchArgs, SearchModeArg,
};
use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::registry::{self, ProjectRegistry, RegisteredProject};
use crate::search::adapter::{
    BackendAccessError, BackendState, BackendStatus, Freshness, Score, SearchBackend,
    SearchFilters, SearchMode, SearchResult,
};
use crate::search::gguf_runtime::{self, GgufRuntimeError, GgufRuntimeReport};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::{LexicalSearch, QmdRsBackend};
use crate::search::sanitize::sanitize_fts_query;
use crate::search::semantic::{
    QueryEmbedder, SemanticIndexMetadata, SemanticSearchContext, SemanticVectorIndex,
    embed_query_with_runtime_report, select_thresholds_for_index,
};
use crate::search_models::{
    AcceptedLicenses, ModelArtifactRecord, ModelArtifacts, SearchThresholdStore, SearchThresholds,
    model_by_id,
};
use crate::search_profile::{ProjectSearchConfig, SearchConfig, SearchProfile};

const HIGH_CONFIDENCE_SEMANTIC_PREFIX_BOOST: f64 = 0.05;

pub fn index(args: &IndexArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: index");
    context.diagnostic(format!(
        "requested project id: {}",
        args.project.as_deref().unwrap_or("<current directory>")
    ));
    context.diagnostic(format!("force: {}", args.force));
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    let registry = ProjectRegistry::read(&registry_path)?;
    let project = select_project(&registry, args.project.as_deref())?;
    context.diagnostic(format!("selected project: {}", project.id));
    index_registered_project(&paths, &project, args.force, context)
}

pub fn index_all(args: &IndexAllArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: index-all");
    context.diagnostic(format!("force: {}", args.force));
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    let registry = ProjectRegistry::read(&registry_path)?;
    context.diagnostic(format!("registered projects: {}", registry.projects.len()));
    if registry.projects.is_empty() {
        println!("No registered projects.");
        return Ok(());
    }

    let projects = registry.projects.clone();
    let total_projects = projects.len();
    let mut indexed_projects = 0usize;
    let mut failures = Vec::new();
    for project in projects {
        context.diagnostic(format!("index-all project: {}", project.id));
        if !project.root.exists() {
            context.diagnostic(format!("per-project outcome {}: root missing", project.id));
            failures.push(format!("{}: root missing", project.id));
            continue;
        }
        if let Err(error) = index_registered_project(&paths, &project, args.force, context) {
            context.diagnostic(format!(
                "per-project outcome {}: failed: {error}",
                project.id
            ));
            failures.push(format!("{}: {error}", project.id));
        } else {
            context.diagnostic(format!("per-project outcome {}: indexed", project.id));
            indexed_projects += 1;
        }
    }

    println!("Indexed {indexed_projects} of {total_projects} projects.");

    if !failures.is_empty() {
        bail!("index-all failed:\n{}", failures.join("\n"));
    }
    Ok(())
}

fn index_registered_project(
    paths: &Paths,
    project: &RegisteredProject,
    force: bool,
    context: &CliContext,
) -> Result<()> {
    let store_path = paths.qmd_rs_store_path(&project.id);
    let project_index_dir = paths.project_index_dir(&project.id);
    let lock_path = project_index_dir.join("qmd-rs.lock");
    context.diagnostic(format!("project id: {}", project.id));
    context.diagnostic(format!("wiki root: {}", project.wiki_root().display()));
    context.diagnostic(format!("store path: {}", store_path.display()));
    context.diagnostic(format!(
        "metadata path: {}",
        store_path.with_extension("llm-wiki.json").display()
    ));
    context.diagnostic(format!("lock path: {}", lock_path.display()));
    fs::create_dir_all(&project_index_dir)
        .with_context(|| format!("create search index dir {}", project_index_dir.display()))?;
    exclude_rebuildable_from_time_machine(&paths.managed_index_root(), context);
    let _lock = ProjectIndexLock::acquire(&project_index_dir)?;
    maybe_sleep_for_test("LLM_WIKI_TEST_INDEX_SLEEP_MS");
    let temp_build = TempIndexBuild::new(&project_index_dir)?;
    context.diagnostic(format!("temp store: {}", temp_build.store_path.display()));

    let backend = QmdRsBackend::new();
    let status = backend.rebuild_or_recover(
        &project.id,
        &project.wiki_root(),
        &temp_build.store_path,
        force,
    )?;
    if !matches!(status.state, BackendState::Ready) {
        bail!(
            "index failed for project {}: {}",
            project.id,
            status
                .message
                .unwrap_or_else(|| format!("{:?}", status.state))
        );
    }

    context.diagnostic(format!("indexed files: {}", status.indexed_files));
    context.diagnostic(format!(
        "promotion: {} -> {}",
        temp_build.store_path.display(),
        store_path.display()
    ));
    promote_qmd_rs_store(&temp_build.store_path, &store_path)?;
    context.diagnostic("promotion: complete");
    temp_build.cleanup()?;
    context.diagnostic("temp store cleanup: complete");
    // Record success against the just-promoted, on-disk lexical store *before* the
    // semantic metadata step. If the semantic build below fails, the error still
    // surfaces to the caller, but the registry must reflect the lexical index that
    // is now live on disk rather than disagreeing with observable state.
    registry::record_index_success(&project.id, status.indexed_files, &project.wiki_root())?;
    context.diagnostic("registry metadata: recorded index success");
    update_semantic_index_metadata(paths, project, context)?;
    println!(
        "Indexed project: {} ({} files)",
        project.id, status.indexed_files
    );
    Ok(())
}

fn update_semantic_index_metadata(
    paths: &Paths,
    project: &RegisteredProject,
    context: &CliContext,
) -> Result<()> {
    let metadata_path = paths.semantic_index_metadata(&project.id);
    let vector_path = paths.semantic_vector_index(&project.id);
    let Some(profile) = project_search_profile(paths, project)? else {
        context.diagnostic("semantic index metadata: skipped, search profile missing");
        return Ok(());
    };
    if !profile.llm_search_enabled {
        context.diagnostic("semantic index metadata: skipped, LLM search disabled");
        if metadata_path.exists() {
            fs::remove_file(&metadata_path).with_context(|| {
                format!(
                    "remove disabled semantic index metadata {}",
                    metadata_path.display()
                )
            })?;
        }
        if vector_path.exists() {
            fs::remove_file(&vector_path).with_context(|| {
                format!(
                    "remove disabled semantic vector index {}",
                    vector_path.display()
                )
            })?;
        }
        return Ok(());
    }
    let Some(artifacts) = ModelArtifacts::read(&paths.model_artifacts())? else {
        bail!(
            "semantic indexing requires model artifact records; run `llm-wiki install --configure-search`"
        );
    };
    let Some(accepted_licenses) = AcceptedLicenses::read(&paths.accepted_licenses())? else {
        bail!(
            "semantic indexing requires accepted model license records; run `llm-wiki install --configure-search`"
        );
    };
    let Some(embedding_model_id) = profile.embedding_model.as_deref() else {
        bail!("semantic indexing requires an embedding model in the search profile");
    };
    let Some(embedding_model) = model_by_id(embedding_model_id) else {
        bail!("semantic indexing requires a known embedding model: {embedding_model_id}");
    };
    if !accepted_licenses.accepts_model(embedding_model) {
        bail!(
            "semantic indexing requires accepted license records for {}; run `llm-wiki install --configure-search`",
            embedding_model.id
        );
    }
    let metadata =
        SemanticIndexMetadata::build(&project.id, &project.wiki_root(), &profile, &artifacts)?;
    context.diagnostic(format!(
        "semantic index metadata: chunks={}, path={}",
        metadata.chunks.len(),
        metadata_path.display()
    ));
    let threshold_store = SearchThresholdStore::read(&paths.search_thresholds())?;
    let Some(embedding_artifact) = embedding_artifact_for_profile(&artifacts, &profile) else {
        bail!("semantic vector indexing requires a verified embedding model artifact");
    };
    let Some(_thresholds) = resolve_thresholds_for_index(
        threshold_store.as_ref(),
        &project.id,
        &metadata,
        embedding_artifact,
    ) else {
        context.diagnostic("semantic vector index: skipped, no thresholds match index inputs");
        // No vectors will be built; publish metadata alone so doctor/status still
        // report chunk counts. Readers that require both treat this as incomplete.
        metadata.write_atomic(&metadata_path)?;
        return Ok(());
    };
    let vector_index =
        SemanticVectorIndex::build(&metadata, &project.wiki_root(), embedding_artifact)?;
    context.diagnostic(format!(
        "semantic vector index: vectors={}, path={}",
        vector_index.vectors.len(),
        vector_path.display()
    ));
    // Publish the vectors before the metadata: metadata is the commit point that
    // readers key off. The slow rebuild above left both files at their previous
    // (consistent) versions; these two renames leave only a sub-millisecond
    // window of (old metadata, new vectors), which the reader's bounded retry
    // absorbs — far better than publishing metadata first and exposing a
    // mismatched pair for the entire rebuild duration.
    vector_index.write_atomic(&vector_path)?;
    metadata.write_atomic(&metadata_path)?;
    Ok(())
}

#[derive(Debug)]
struct ProjectIndexLock {
    _file: File,
}

impl ProjectIndexLock {
    /// Do not add a retry loop on `try_lock` failure. The OS lock is keyed on
    /// the inode; if the lockfile is unlinked between attempts, a retry can
    /// lock a dead inode and stop conflicting with a fresh acquisition. Any
    /// future retry must reopen the file after verifying it still names the
    /// live inode.
    fn acquire(project_index_dir: &Path) -> Result<Self> {
        let path = project_index_dir.join("qmd-rs.lock");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("project index lock cannot be opened: {}", path.display()))?;
        file.try_lock().with_context(|| {
            format!(
                "project index is already locked: {}. Another llm-wiki index process is likely running; wait for it to finish and retry. Do not delete the lock file: the OS lock is keyed on the inode, so unlinking it can let a concurrent process acquire a stale lock.",
                path.display()
            )
        })?;
        file.set_len(0)
            .with_context(|| format!("truncate project index lock {}", path.display()))?;
        writeln!(file, "pid={}", process::id())
            .with_context(|| format!("write project index lock {}", path.display()))?;
        Ok(Self { _file: file })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreFileRole {
    Sqlite,
    Metadata,
    Wal,
    Shm,
}

impl StoreFileRole {
    const ALL: [Self; 4] = [Self::Sqlite, Self::Metadata, Self::Wal, Self::Shm];
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RelatedStorePaths {
    sqlite: PathBuf,
    metadata: PathBuf,
    wal: PathBuf,
    shm: PathBuf,
}

impl RelatedStorePaths {
    fn new(store_path: &Path) -> Self {
        Self {
            sqlite: store_path.to_path_buf(),
            metadata: store_path.with_extension("llm-wiki.json"),
            wal: store_path.with_extension("sqlite-wal"),
            shm: store_path.with_extension("sqlite-shm"),
        }
    }

    fn path(&self, role: StoreFileRole) -> &Path {
        match role {
            StoreFileRole::Sqlite => &self.sqlite,
            StoreFileRole::Metadata => &self.metadata,
            StoreFileRole::Wal => &self.wal,
            StoreFileRole::Shm => &self.shm,
        }
    }
}

struct TempIndexBuild {
    dir: PathBuf,
    store_path: PathBuf,
}

impl TempIndexBuild {
    fn new(project_index_dir: &Path) -> Result<Self> {
        let suffix = unique_suffix();
        let dir = project_index_dir.join(format!(".qmd-rs-build-{suffix}"));
        fs::create_dir_all(&dir)
            .with_context(|| format!("create temp search index dir {}", dir.display()))?;
        let store_path = dir.join("qmd-rs.sqlite");
        Ok(Self { dir, store_path })
    }

    fn cleanup(self) -> Result<()> {
        if self.dir.exists() {
            fs::remove_dir_all(&self.dir)
                .with_context(|| format!("remove temp search index dir {}", self.dir.display()))?;
        }
        Ok(())
    }
}

impl Drop for TempIndexBuild {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", process::id())
}

fn promote_qmd_rs_store(temp_store: &Path, live_store: &Path) -> Result<()> {
    promote_qmd_rs_store_inner(temp_store, live_store, None)
}

/// Promotes a fully-built qmd-rs store into place.
///
/// Assumptions:
/// - All paths live on the same filesystem, so `rename` stays atomic.
/// - The caller already holds the per-project index lock.
/// - Readers can briefly observe `Missing` between backup-rename and
///   final-rename, and are expected to retry once.
fn promote_qmd_rs_store_inner(
    temp_store: &Path,
    live_store: &Path,
    fail_after_promotes: Option<usize>,
) -> Result<()> {
    if !temp_store.exists() {
        bail!(
            "temporary qmd-rs store was not created: {}",
            temp_store.display()
        );
    }

    let suffix = unique_suffix();
    let temp_files = RelatedStorePaths::new(temp_store);
    let live_files = RelatedStorePaths::new(live_store);
    let mut backups: Vec<(PathBuf, PathBuf)> = Vec::new();
    for role in StoreFileRole::ALL {
        let live = live_files.path(role).to_path_buf();
        if live.exists() {
            let backup = backup_path(&live, &suffix);
            if let Err(error) = fs::rename(&live, &backup).with_context(|| {
                format!(
                    "move existing qmd-rs store file {} to {}",
                    live.display(),
                    backup.display()
                )
            }) {
                // Symmetric with the promote-phase rollback below: a failure part
                // way through the backup loop must restore the already-renamed
                // backup files to their live positions so the live store is not
                // left dismembered.
                for (live, backup) in backups.iter().rev() {
                    if backup.exists() {
                        let _ = fs::rename(backup, live);
                    }
                }
                return Err(error);
            }
            backups.push((live, backup));
        }
    }

    let mut promoted = Vec::new();
    let promote_result = (|| -> Result<()> {
        maybe_write_test_marker("LLM_WIKI_TEST_PROMOTE_MARKER");
        maybe_sleep_for_test("LLM_WIKI_TEST_PROMOTE_PRE_COMMIT_SLEEP_MS");
        for role in StoreFileRole::ALL {
            let temp = temp_files.path(role);
            let live = live_files.path(role);
            if temp.exists() {
                fs::rename(temp, live).with_context(|| {
                    format!(
                        "promote qmd-rs store file {} to {}",
                        temp.display(),
                        live.display()
                    )
                })?;
                promoted.push(live.to_path_buf());
                if fail_after_promotes.is_some_and(|limit| promoted.len() >= limit) {
                    bail!("simulated qmd-rs store promotion failure");
                }
            }
        }
        Ok(())
    })();

    if let Err(error) = promote_result {
        maybe_remove_test_marker("LLM_WIKI_TEST_PROMOTE_MARKER");
        for live in promoted.iter().rev() {
            if live.exists() {
                let _ = fs::remove_file(live);
            }
        }
        for (live, backup) in backups.iter().rev() {
            if live.exists() {
                let _ = fs::remove_file(live);
            }
            if backup.exists() {
                let _ = fs::rename(backup, live);
            }
        }
        return Err(error);
    }
    maybe_remove_test_marker("LLM_WIKI_TEST_PROMOTE_MARKER");

    for (_, backup) in backups {
        if backup.exists() {
            fs::remove_file(&backup)
                .with_context(|| format!("remove qmd-rs backup {}", backup.display()))?;
        }
    }
    Ok(())
}

fn backup_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("qmd-rs-store");
    path.with_file_name(format!("{file_name}.backup-{suffix}"))
}

pub fn search(args: &SearchArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: search");
    context.diagnostic(format!("query: {}", args.query));
    context.diagnostic(format!("requested mode: {}", args.mode.label()));
    context.diagnostic(format!(
        "allow lexical fallback: {}",
        args.allow_lexical_fallback
    ));
    context.diagnostic(format!("rerank: {}", args.rerank));
    context.diagnostic(format!("fts query: {}", sanitize_fts_query(&args.query)));
    context.diagnostic(format!(
        "filters: class={}, status={}, limit={}",
        filter_label(args.document_class.as_deref()),
        filter_label(args.status.as_deref()),
        args.limit
    ));
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let (project, selection_source) =
        select_project_with_source(&registry, args.project.as_deref())?;
    ensure_project_root_exists(&project)?;
    let resolution =
        match resolve_project_mode(&paths, &project, args.mode, args.allow_lexical_fallback)? {
            ModeResolutionOutcome::Ready(resolution) => resolution,
            ModeResolutionOutcome::NotReady { failure, profile } => {
                return handle_readiness_failure(
                    &args.query,
                    Some((&project.id, &project.name)),
                    args.format,
                    args.mode,
                    args.rerank,
                    failure,
                    profile.as_ref(),
                );
            }
        };
    context.diagnostic(format!(
        "selected mode: {}",
        resolution.selected_mode.label()
    ));
    context.diagnostic(format!("mode reason: {}", resolution.reason));
    if let Some(reason) = &resolution.fallback_reason {
        context.diagnostic(format!("fallback reason: {reason}"));
    }
    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let (store_path, store_location) = qmd_store_path_for_search(&paths, &project.id);
    let wiki_root = project.wiki_root();
    context.diagnostic(format!("project selection: {}", selection_source.label()));
    context.diagnostic(format!(
        "selected project: {} ({})",
        project.id,
        project.root.display()
    ));
    context.diagnostic(format!("project name: {}", project.name));
    context.diagnostic(format!("wiki root: {}", wiki_root.display()));
    context.diagnostic(format!("index store: {}", store_path.display()));
    context.diagnostic(format!("index store location: {}", store_location.label()));
    context.diagnostic(format!(
        "backend mode: {}",
        resolution.selected_mode.label()
    ));
    let backend = QmdRsBackend::new();
    let search_input = ProjectSearchInput {
        paths: &paths,
        backend: &backend,
        project: &project,
        store_path: &store_path,
        wiki_root: &wiki_root,
        query: &args.query,
        filters: &filters,
        limit: args.limit,
    };
    let search =
        match perform_resolved_project_search(&search_input, &resolution, args.rerank, context) {
            Ok(search) => search,
            Err(error) => {
                if let Some(failure) = error.downcast_ref::<BackendReadinessFailure>() {
                    context.diagnostic(format!("backend readiness failure: {}", failure.reason));
                    context.diagnostic(format!(
                        "index status: {}, open_mode={}, freshness={}, indexed_files={}",
                        backend_state_label(&failure.status.state),
                        failure.status.open_mode.label(),
                        freshness_label(freshness_for_status(&failure.status)),
                        failure.status.indexed_files
                    ));
                    if let Some(message) = &failure.status.message {
                        context.diagnostic(format!("index message: {message}"));
                    }
                    if args.format == OutputFormat::Json {
                        let mut mode_metadata =
                            SearchModeJson::from_resolution(&resolution, args.rerank);
                        mode_metadata.readiness_reason = Some(failure.reason.clone());
                        print_search_json(
                            &args.query,
                            Some((&project.id, &project.name)),
                            None,
                            Some(&failure.status),
                            &[],
                            &[],
                            &[],
                            mode_metadata,
                            SearchJsonOptions::from_search_args(args),
                        );
                    }
                } else if let Some(failure) = error.downcast_ref::<GgufRuntimeError>() {
                    context.diagnostic(format!(
                        "GGUF runtime failure: role={}, stage={}, kind={}",
                        failure.role().label(),
                        failure.stage().label(),
                        failure.kind().label()
                    ));
                    if args.format == OutputFormat::Json {
                        let mut mode_metadata =
                            SearchModeJson::from_resolution(&resolution, args.rerank);
                        mode_metadata.apply_runtime_failure(failure);
                        let status = backend.status(&project.id, &store_path, &wiki_root).ok();
                        print_search_json(
                            &args.query,
                            Some((&project.id, &project.name)),
                            None,
                            status.as_ref(),
                            &[],
                            &[],
                            &[],
                            mode_metadata,
                            SearchJsonOptions::from_search_args(args),
                        );
                    }
                } else if args.format == OutputFormat::Json {
                    // Catch-all so every error kind (license bail, missing reranker
                    // artifact, query-expansion bail, stale-race bail, ...) still
                    // emits a parseable JSON error envelope on stdout. Without this,
                    // `--format json` consumers would receive zero bytes on these
                    // failures. The error text is surfaced via `readiness_reason`.
                    context.diagnostic(format!("search error: {error}"));
                    let mut mode_metadata =
                        SearchModeJson::from_resolution(&resolution, args.rerank);
                    mode_metadata.readiness_reason = Some(error.to_string());
                    let status = backend.status(&project.id, &store_path, &wiki_root).ok();
                    print_search_json(
                        &args.query,
                        Some((&project.id, &project.name)),
                        None,
                        status.as_ref(),
                        &[],
                        &[],
                        &[],
                        mode_metadata,
                        SearchJsonOptions::from_search_args(args),
                    );
                }
                return Err(error);
            }
        };
    let mut search = search;
    search.warnings.extend(phrase_fallback_warning(
        &project.id,
        search.phrase_fallback_pages,
        FallbackPlacement::Last,
    ));
    context.diagnostic(format!(
        "index status: {}, open_mode={}, freshness={}, indexed_files={}",
        backend_state_label(&search.status.state),
        search.status.open_mode.label(),
        freshness_label(freshness_for_status(&search.status)),
        search.status.indexed_files
    ));
    if let Some(message) = &search.status.message {
        context.diagnostic(format!("index message: {message}"));
    }
    let no_result = explain_no_results_for_mode(
        &backend,
        &resolution,
        NoResultContext {
            project_id: &project.id,
            store_path: &store_path,
            wiki_root: &wiki_root,
            query: &args.query,
            filters: &filters,
            limit: args.limit,
            status: &search.status,
            results: &search.results,
        },
    );
    context.diagnostic(format!("results: {}", search.results.len()));
    if let Some(explanation) = &no_result {
        context.diagnostic(format!("no-result: {explanation}"));
    }

    let mut results = search.results;
    for result in &mut results {
        result.project_id = project.id.clone();
        result.project_name = Some(project.name.clone());
    }
    let warning = search
        .warnings
        .first()
        .map(|warning| warning.message.as_str());
    let mut mode_metadata = SearchModeJson::from_resolution(&resolution, args.rerank);
    mode_metadata.zero_result_reason = no_result;
    mode_metadata.thresholds_source = search
        .thresholds_source
        .map(|source| source.label().to_string());
    if let Some(report) = &search.runtime_report {
        mode_metadata.apply_runtime_report(report);
    }

    match args.format {
        OutputFormat::Text => print_search_text(&search.warnings, &results),
        OutputFormat::Json => print_search_json(
            &args.query,
            Some((&project.id, &project.name)),
            warning,
            Some(&search.status),
            &search.warnings,
            &[],
            &results,
            mode_metadata,
            SearchJsonOptions::from_search_args(args),
        ),
    }
    Ok(())
}

pub fn search_all(args: &SearchAllArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: search-all");
    context.diagnostic(format!("query: {}", args.query));
    context.diagnostic(format!("requested mode: {}", args.mode.label()));
    context.diagnostic(format!(
        "allow lexical fallback: {}",
        args.allow_lexical_fallback
    ));
    context.diagnostic(format!("rerank: {}", args.rerank));
    context.diagnostic(format!("fts query: {}", sanitize_fts_query(&args.query)));
    context.diagnostic(format!(
        "filters: class={}, status={}, limit={}",
        filter_label(args.document_class.as_deref()),
        filter_label(args.status.as_deref()),
        args.limit
    ));
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let projects = select_projects(&registry, &args.include, &args.exclude)?;
    ensure_base_install(&paths)?;
    context.diagnostic(format!(
        "project selection: {}",
        search_all_selection_label(&args.include, &args.exclude)
    ));
    context.diagnostic(format!(
        "selected projects: {}",
        projects
            .iter()
            .map(|project| project.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let backend = QmdRsBackend::new();
    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let mut warnings = Vec::new();
    let mut fused: BTreeMap<(String, String), FusedResult> = BTreeMap::new();
    let mut fallback_keys: BTreeSet<(String, String)> = BTreeSet::new();
    let mut project_reports = Vec::new();

    for project in &projects {
        if let Some(report) =
            stale_project_search_report(project, args.mode, args.rerank, &mut warnings, context)
        {
            project_reports.push(report);
            continue;
        }
        let (store_path, store_location) = qmd_store_path_for_search(&paths, &project.id);
        let wiki_root = project.wiki_root();
        context.diagnostic(format!(
            "project {}: name={} root={}",
            project.id,
            project.name,
            project.root.display()
        ));
        context.diagnostic(format!("wiki root {}: {}", project.id, wiki_root.display()));
        context.diagnostic(format!(
            "index store {}: {}",
            project.id,
            store_path.display()
        ));
        context.diagnostic(format!(
            "index store location {}: {}",
            project.id,
            store_location.label()
        ));
        let per_project_limit = if args.limit == 0 {
            0
        } else {
            args.limit.max(20)
        };
        let search_input = ProjectSearchInput {
            paths: &paths,
            backend: &backend,
            project,
            store_path: &store_path,
            wiki_root: &wiki_root,
            query: &args.query,
            filters: &filters,
            limit: per_project_limit,
        };
        let resolution =
            match resolve_project_mode(&paths, project, args.mode, args.allow_lexical_fallback)? {
                ModeResolutionOutcome::Ready(resolution) => resolution,
                ModeResolutionOutcome::NotReady { failure, profile } => {
                    context.diagnostic(format!(
                        "per-project readiness {}: {}",
                        project.id, failure.reason
                    ));
                    warnings.push(SearchWarning {
                        project_id: project.id.clone(),
                        message: format!("project skipped: {}", failure.guidance),
                    });
                    project_reports.push(ProjectSearchReport {
                        project_id: project.id.clone(),
                        requested_mode: args.mode.label().to_string(),
                        selected_mode: None,
                        mode_selection_reason: None,
                        fallback_reason: None,
                        readiness_reason: Some(failure.reason),
                        rerank_requested: args.rerank,
                        rerank_applied: false,
                        rerank_reason: args.rerank.then(|| "search_not_ready".to_string()),
                        runtime_backend_requested: None,
                        runtime_backend_used: None,
                        runtime_backend_fallback: None,
                        runtime_failure_stage: None,
                        runtime_error_kind: None,
                        backend_status: None,
                        result_count: 0,
                        no_result: None,
                        profile,
                        thresholds_source: None,
                    });
                    continue;
                }
            };
        context.diagnostic(format!(
            "selected mode {}: {}",
            project.id,
            resolution.selected_mode.label()
        ));
        context.diagnostic(format!("mode reason {}: {}", project.id, resolution.reason));
        if let Some(reason) = &resolution.fallback_reason {
            context.diagnostic(format!("fallback reason {}: {reason}", project.id));
        }
        let search =
            match perform_resolved_project_search(&search_input, &resolution, args.rerank, context)
            {
                Ok(search) => search,
                Err(error) => {
                    if let Some(failure) = error.downcast_ref::<BackendReadinessFailure>() {
                        context.diagnostic(format!(
                            "per-project backend readiness {}: {}",
                            project.id, failure.reason
                        ));
                        warnings.push(SearchWarning {
                            project_id: project.id.clone(),
                            message: format!("project skipped: {}", failure.guidance),
                        });
                        project_reports.push(ProjectSearchReport {
                            project_id: project.id.clone(),
                            requested_mode: resolution.requested_mode.label().to_string(),
                            selected_mode: Some(resolution.selected_mode.label().to_string()),
                            mode_selection_reason: Some(resolution.reason.clone()),
                            fallback_reason: resolution.fallback_reason.clone(),
                            readiness_reason: Some(failure.reason.clone()),
                            rerank_requested: args.rerank,
                            rerank_applied: false,
                            rerank_reason: args.rerank.then(|| "search_not_ready".to_string()),
                            runtime_backend_requested: None,
                            runtime_backend_used: None,
                            runtime_backend_fallback: None,
                            runtime_failure_stage: None,
                            runtime_error_kind: None,
                            backend_status: Some(failure.status.clone()),
                            result_count: 0,
                            no_result: None,
                            profile: resolution.profile.clone(),
                            thresholds_source: None,
                        });
                        continue;
                    }
                    if let Some(failure) = error.downcast_ref::<GgufRuntimeError>() {
                        context.diagnostic(format!(
                            "per-project GGUF runtime failure {}: role={}, stage={}, kind={}",
                            project.id,
                            failure.role().label(),
                            failure.stage().label(),
                            failure.kind().label()
                        ));
                        warnings.push(SearchWarning {
                            project_id: project.id.clone(),
                            message: format!(
                                "project skipped: {}",
                                runtime_failure_guidance(failure)
                            ),
                        });
                        project_reports.push(ProjectSearchReport {
                            project_id: project.id.clone(),
                            requested_mode: resolution.requested_mode.label().to_string(),
                            selected_mode: Some(resolution.selected_mode.label().to_string()),
                            mode_selection_reason: Some(resolution.reason.clone()),
                            fallback_reason: resolution.fallback_reason.clone(),
                            readiness_reason: Some(failure.kind().readiness_reason().to_string()),
                            rerank_requested: args.rerank,
                            rerank_applied: false,
                            rerank_reason: args.rerank.then(|| "search_not_ready".to_string()),
                            runtime_backend_requested: Some(
                                gguf_runtime::requested_backend().label().to_string(),
                            ),
                            runtime_backend_used: None,
                            runtime_backend_fallback: Some(false),
                            runtime_failure_stage: Some(failure.stage().label().to_string()),
                            runtime_error_kind: Some(failure.kind().label().to_string()),
                            backend_status: backend
                                .status(&project.id, &store_path, &wiki_root)
                                .ok(),
                            result_count: 0,
                            no_result: None,
                            profile: resolution.profile.clone(),
                            thresholds_source: None,
                        });
                        continue;
                    }
                    // Catch-all: skip ANY other failing project (mirroring how
                    // `index-all` aggregates per-project failures) so one bad
                    // project can never take down the whole multi-project search.
                    context.diagnostic(format!("per-project failure {}: {error}", project.id));
                    warnings.push(SearchWarning {
                        project_id: project.id.clone(),
                        message: format!("project skipped: {error}"),
                    });
                    project_reports.push(ProjectSearchReport {
                        project_id: project.id.clone(),
                        requested_mode: resolution.requested_mode.label().to_string(),
                        selected_mode: Some(resolution.selected_mode.label().to_string()),
                        mode_selection_reason: Some(resolution.reason.clone()),
                        fallback_reason: resolution.fallback_reason.clone(),
                        readiness_reason: Some(error.to_string()),
                        rerank_requested: args.rerank,
                        rerank_applied: false,
                        rerank_reason: args.rerank.then(|| "search_not_ready".to_string()),
                        runtime_backend_requested: None,
                        runtime_backend_used: None,
                        runtime_backend_fallback: None,
                        runtime_failure_stage: None,
                        runtime_error_kind: None,
                        backend_status: None,
                        result_count: 0,
                        no_result: None,
                        profile: resolution.profile.clone(),
                        thresholds_source: None,
                    });
                    continue;
                }
            };
        context.diagnostic(format!(
            "index status {}: {}, open_mode={}, freshness={}, indexed_files={}",
            project.id,
            backend_state_label(&search.status.state),
            search.status.open_mode.label(),
            freshness_label(freshness_for_status(&search.status)),
            search.status.indexed_files
        ));
        if let Some(message) = &search.status.message {
            context.diagnostic(format!("index message {}: {message}", project.id));
        }
        // Compute unconditionally (not gated on `--verbose`) so the per-project
        // `no_result` JSON field's presence matches the single-project `search`
        // path (see the unconditional computation above) instead of appearing only
        // under `--verbose`.
        let no_result = explain_no_results_for_mode(
            &backend,
            &resolution,
            NoResultContext {
                project_id: &project.id,
                store_path: &store_path,
                wiki_root: &wiki_root,
                query: &args.query,
                filters: &filters,
                limit: per_project_limit,
                status: &search.status,
                results: &search.results,
            },
        );
        let (rerank_applied, rerank_reason) = rerank_status(
            args.rerank,
            resolution.selected_mode,
            resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.reranker_model.as_deref()),
        );
        project_reports.push(ProjectSearchReport {
            project_id: project.id.clone(),
            requested_mode: resolution.requested_mode.label().to_string(),
            selected_mode: Some(resolution.selected_mode.label().to_string()),
            mode_selection_reason: Some(resolution.reason.clone()),
            fallback_reason: resolution.fallback_reason.clone(),
            readiness_reason: None,
            rerank_requested: args.rerank,
            rerank_applied,
            rerank_reason,
            runtime_backend_requested: search
                .runtime_report
                .as_ref()
                .map(|report| report.requested_backend().label().to_string()),
            runtime_backend_used: search
                .runtime_report
                .as_ref()
                .map(|report| report.used_backend().label().to_string()),
            runtime_backend_fallback: search
                .runtime_report
                .as_ref()
                .map(GgufRuntimeReport::fallback),
            runtime_failure_stage: None,
            runtime_error_kind: None,
            backend_status: Some(search.status.clone()),
            result_count: search.results.len(),
            no_result,
            profile: resolution.profile.clone(),
            thresholds_source: search.thresholds_source,
        });
        warnings.extend(search.warnings);
        let mut results = search.results;
        let all_words_pages = results.len() - search.phrase_fallback_pages;
        for result in &results[all_words_pages..] {
            fallback_keys.insert((
                project.id.clone(),
                result.path.to_string_lossy().to_string(),
            ));
        }
        for (rank, result) in results.iter_mut().enumerate() {
            result.project_id = project.id.clone();
            result.project_name = Some(project.name.clone());
            let rrf = 1.0 / (60.0 + rank as f64 + 1.0);
            let key = (
                result.project_id.clone(),
                result.path.to_string_lossy().to_string(),
            );
            fused
                .entry(key)
                .and_modify(|entry| entry.score += rrf)
                .or_insert_with(|| FusedResult {
                    result: result.clone(),
                    score: rrf,
                    lexical_rank: None,
                    lexical_score: None,
                    semantic_rank: None,
                    semantic_score: None,
                });
        }
    }

    let mut results = fused.into_values().collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.result.project_id.cmp(&right.result.project_id))
            .then_with(|| left.result.path.cmp(&right.result.path))
    });
    let results = results
        .into_iter()
        .take(args.limit)
        .map(|mut fused| {
            fused.result.score = Score(fused.score);
            fused.result
        })
        .collect::<Vec<_>>();
    warnings.extend(shown_phrase_fallback_warnings(&results, &fallback_keys));
    for report in &project_reports {
        context.diagnostic(format!(
            "per-project results {}: {}",
            report.project_id, report.result_count
        ));
        if let Some(explanation) = &report.no_result {
            context.diagnostic(format!("no-result {}: {explanation}", report.project_id));
        }
    }
    context.diagnostic(format!("fused results: {}", results.len()));
    if results.is_empty() {
        context.diagnostic("no-result: no selected project returned results");
    }

    match args.format {
        OutputFormat::Text => print_search_text(&warnings, &results),
        OutputFormat::Json => {
            let mut mode_metadata =
                SearchModeJson::from_search_all_reports(args.mode, args.rerank, &project_reports);
            if results.is_empty() {
                mode_metadata.zero_result_reason =
                    Some("no selected project returned results".to_string());
            }
            print_search_json(
                &args.query,
                None,
                None,
                None,
                &warnings,
                &project_reports,
                &results,
                mode_metadata,
                SearchJsonOptions::from_search_all_args(args),
            )
        }
    }
    Ok(())
}

fn select_project(
    registry: &ProjectRegistry,
    requested_id: Option<&str>,
) -> Result<RegisteredProject> {
    select_project_with_source(registry, requested_id).map(|(project, _)| project)
}

fn select_project_with_source(
    registry: &ProjectRegistry,
    requested_id: Option<&str>,
) -> Result<(RegisteredProject, ProjectSelectionSource)> {
    if let Some(project_id) = requested_id {
        return Ok((
            registry
                .project_by_id(project_id)
                .cloned()
                .with_context(|| format!("project id {project_id} is not registered"))?,
            ProjectSelectionSource::ExplicitProject,
        ));
    }

    let Some(discovered) = discover_from_cwd()? else {
        bail!("not inside a wiki project; pass --project <id>");
    };
    registry
        .project_by_root(&discovered.project_root)
        .cloned()
        .with_context(|| {
            format!(
                "current project {} is not registered; run `llm-wiki register {}`",
                discovered.project_root.display(),
                discovered.project_root.display()
            )
        })
        .map(|project| (project, ProjectSelectionSource::CurrentDirectory))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectSelectionSource {
    ExplicitProject,
    CurrentDirectory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StoreLocation {
    Managed,
    LegacyCache,
    MissingManaged,
}

impl StoreLocation {
    fn label(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::LegacyCache => "legacy-cache",
            Self::MissingManaged => "managed-missing",
        }
    }
}

fn qmd_store_path_for_search(paths: &Paths, project_id: &str) -> (PathBuf, StoreLocation) {
    let managed = paths.qmd_rs_store_path(project_id);
    if managed.exists() {
        return (managed, StoreLocation::Managed);
    }
    let legacy = paths.legacy_qmd_rs_store_path(project_id);
    if legacy.exists() {
        return (legacy, StoreLocation::LegacyCache);
    }
    (managed, StoreLocation::MissingManaged)
}

enum ModeResolutionOutcome {
    Ready(ModeResolution),
    NotReady {
        failure: ReadinessFailure,
        profile: Option<SearchProfile>,
    },
}

fn resolve_project_mode(
    paths: &Paths,
    project: &RegisteredProject,
    requested_mode: SearchModeArg,
    allow_lexical_fallback: bool,
) -> Result<ModeResolutionOutcome> {
    ensure_base_install(paths)?;
    let profile = project_search_profile(paths, project)?;
    resolve_mode(
        paths,
        Some(project),
        requested_mode,
        allow_lexical_fallback,
        profile,
    )
}

fn ensure_base_install(paths: &Paths) -> Result<()> {
    if Manifest::read(&paths.manifest())?.is_none() {
        bail!(
            "llm-wiki install is required before search; run `llm-wiki install` or `llm-wiki install --configure-search`"
        );
    }
    Ok(())
}

fn project_search_profile(
    paths: &Paths,
    project: &RegisteredProject,
) -> Result<Option<SearchProfile>> {
    let project_search_config = project.root.join(".llm_wiki/search.toml");
    if let Some(config) = ProjectSearchConfig::read(&project_search_config)? {
        return Ok(Some(config.project));
    }
    Ok(SearchConfig::read(&paths.search_config())?.map(|config| config.project_default))
}

pub(crate) struct ProjectSearchReadiness {
    pub lexical_ready: bool,
    pub semantic_ready: bool,
    pub hybrid_ready: bool,
    pub readiness_reason: Option<String>,
}

pub(crate) fn project_search_readiness(
    paths: &Paths,
    project: &RegisteredProject,
    status: &BackendStatus,
) -> Result<ProjectSearchReadiness> {
    let lexical_ready = matches!(status.state, BackendState::Ready | BackendState::Stale);
    if !lexical_ready {
        return Ok(ProjectSearchReadiness {
            lexical_ready,
            semantic_ready: false,
            hybrid_ready: false,
            readiness_reason: Some(backend_state_json_label(&status.state).to_string()),
        });
    }

    let profile = project_search_profile(paths, project)?;
    let semantic_reason = llm_mode_readiness_reason(
        paths,
        project,
        profile.as_ref(),
        RuntimeSearchMode::Semantic,
    )?;
    let hybrid_reason =
        llm_mode_readiness_reason(paths, project, profile.as_ref(), RuntimeSearchMode::Hybrid)?;

    let needs_runtime_validation = semantic_reason.is_none() || hybrid_reason.is_none();
    let runtime_reason = if needs_runtime_validation {
        profile.as_ref().and_then(|profile| {
            load_semantic_runtime_state(paths, project, &project.wiki_root(), profile)
                .err()
                .map(|error| semantic_runtime_readiness_reason(&error))
        })
    } else {
        None
    };

    let semantic_ready = semantic_reason.is_none() && runtime_reason.is_none();
    let hybrid_ready = hybrid_reason.is_none() && runtime_reason.is_none();
    let readiness_reason = semantic_reason.or(hybrid_reason).or(runtime_reason);

    Ok(ProjectSearchReadiness {
        lexical_ready,
        semantic_ready,
        hybrid_ready,
        readiness_reason,
    })
}

fn llm_mode_readiness_reason(
    paths: &Paths,
    project: &RegisteredProject,
    profile: Option<&SearchProfile>,
    selected_mode: RuntimeSearchMode,
) -> Result<Option<String>> {
    let Some(profile) = profile else {
        return Ok(Some("install_profile_missing".to_string()));
    };
    if !profile.llm_search_enabled {
        return Ok(Some(
            profile
                .reason
                .clone()
                .unwrap_or_else(|| "llm_search_disabled".to_string()),
        ));
    }
    Ok(
        readiness_failure(paths, profile, selected_mode, Some(project))?
            .map(|failure| failure.reason),
    )
}

fn semantic_runtime_readiness_reason(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("semantic vector index missing") {
        "semantic_index_missing".to_string()
    } else if message.contains("semantic index stale")
        || message.contains("semantic vector index stale")
    {
        "semantic_index_stale".to_string()
    } else {
        "semantic_runtime_unavailable".to_string()
    }
}

fn resolve_mode(
    paths: &Paths,
    project: Option<&RegisteredProject>,
    requested_mode: SearchModeArg,
    allow_lexical_fallback: bool,
    profile: Option<SearchProfile>,
) -> Result<ModeResolutionOutcome> {
    if requested_mode == SearchModeArg::Lexical {
        return Ok(ModeResolutionOutcome::Ready(ModeResolution {
            requested_mode,
            selected_mode: RuntimeSearchMode::Lexical,
            reason: "explicit_lexical".to_string(),
            fallback_reason: None,
            profile,
        }));
    }

    if requested_mode == SearchModeArg::Auto {
        let Some(profile) = profile else {
            return Ok(ModeResolutionOutcome::Ready(ModeResolution {
                requested_mode,
                selected_mode: RuntimeSearchMode::Lexical,
                reason: "install_profile_missing".to_string(),
                fallback_reason: None,
                profile: None,
            }));
        };
        if !profile.llm_search_enabled {
            let reason = profile
                .reason
                .clone()
                .unwrap_or_else(|| "llm_search_disabled".to_string());
            return Ok(ModeResolutionOutcome::Ready(ModeResolution {
                requested_mode,
                selected_mode: RuntimeSearchMode::Lexical,
                reason,
                fallback_reason: None,
                profile: Some(profile),
            }));
        }
        if let Some(failure) =
            readiness_failure(paths, &profile, RuntimeSearchMode::Hybrid, project)?
        {
            // `auto` is a best-effort mode: when semantic/hybrid is not ready
            // (e.g. uncalibrated `thresholds_unconfigured` on a fresh project),
            // degrade to lexical and report the readiness reason instead of
            // hard-failing. Explicit `semantic`/`hybrid` still surface the
            // failure via `explicit_readiness_outcome`.
            return Ok(ModeResolutionOutcome::Ready(ModeResolution {
                requested_mode,
                selected_mode: RuntimeSearchMode::Lexical,
                reason: "auto_lexical_fallback".to_string(),
                fallback_reason: Some(failure.reason),
                profile: Some(profile),
            }));
        }
        return Ok(ModeResolutionOutcome::Ready(ModeResolution {
            requested_mode,
            selected_mode: RuntimeSearchMode::Hybrid,
            reason: "enabled_profile".to_string(),
            fallback_reason: None,
            profile: Some(profile),
        }));
    }

    let selected_mode = match requested_mode {
        SearchModeArg::Semantic => RuntimeSearchMode::Semantic,
        SearchModeArg::Hybrid => RuntimeSearchMode::Hybrid,
        SearchModeArg::Auto | SearchModeArg::Lexical => unreachable!("handled above"),
    };
    let Some(profile) = profile else {
        return explicit_readiness_outcome(
            requested_mode,
            selected_mode,
            allow_lexical_fallback,
            None,
            readiness("install_profile_missing"),
        );
    };
    if !profile.llm_search_enabled {
        let reason = profile
            .reason
            .clone()
            .unwrap_or_else(|| "llm_search_disabled".to_string());
        return explicit_readiness_outcome(
            requested_mode,
            selected_mode,
            allow_lexical_fallback,
            Some(profile),
            readiness(&reason),
        );
    }
    if let Some(failure) = readiness_failure(paths, &profile, selected_mode, project)? {
        return explicit_readiness_outcome(
            requested_mode,
            selected_mode,
            allow_lexical_fallback,
            Some(profile),
            failure,
        );
    }
    Ok(ModeResolutionOutcome::Ready(ModeResolution {
        requested_mode,
        selected_mode,
        reason: "explicit_mode".to_string(),
        fallback_reason: None,
        profile: Some(profile),
    }))
}

fn explicit_readiness_outcome(
    requested_mode: SearchModeArg,
    selected_mode: RuntimeSearchMode,
    allow_lexical_fallback: bool,
    profile: Option<SearchProfile>,
    failure: ReadinessFailure,
) -> Result<ModeResolutionOutcome> {
    if allow_lexical_fallback {
        return Ok(ModeResolutionOutcome::Ready(ModeResolution {
            requested_mode,
            selected_mode: RuntimeSearchMode::Lexical,
            reason: "lexical_fallback".to_string(),
            fallback_reason: Some(failure.reason),
            profile,
        }));
    }
    let _ = selected_mode;
    Ok(ModeResolutionOutcome::NotReady { failure, profile })
}

fn readiness_failure(
    paths: &Paths,
    profile: &SearchProfile,
    selected_mode: RuntimeSearchMode,
    project: Option<&RegisteredProject>,
) -> Result<Option<ReadinessFailure>> {
    let Some(model_ids) = required_model_ids(profile, selected_mode)? else {
        return Ok(Some(readiness("model_missing")));
    };
    let Some(artifacts) = ModelArtifacts::read(&paths.model_artifacts())? else {
        return Ok(Some(readiness("model_missing")));
    };
    let Some(accepted_licenses) = AcceptedLicenses::read(&paths.accepted_licenses())? else {
        return Ok(Some(readiness("license_not_accepted")));
    };
    for model_id in model_ids {
        let Some(model) = model_by_id(&model_id) else {
            return Ok(Some(readiness("model_missing")));
        };
        if !accepted_licenses.accepts_model(model) {
            return Ok(Some(readiness("license_not_accepted")));
        }
        let Some(record) = artifacts.artifacts.iter().find(|artifact| {
            artifact.model_id == model_id
                && artifact.expected_sha256 == model.expected_sha256
                && artifact.observed_sha256 == model.expected_sha256
        }) else {
            return Ok(Some(readiness("model_missing")));
        };
        if !record.path.exists() {
            return Ok(Some(readiness("model_missing")));
        }
    }
    let threshold_store = SearchThresholdStore::read(&paths.search_thresholds())?;
    let Some(project) = project else {
        return Ok(None);
    };
    let Some(embedding_artifact) = embedding_artifact_for_profile(&artifacts, profile) else {
        return Ok(Some(readiness("model_missing")));
    };
    let metadata_path = paths.semantic_index_metadata(&project.id);
    let Some(metadata) = SemanticIndexMetadata::read(&metadata_path)? else {
        return Ok(Some(readiness("semantic_index_missing")));
    };
    let Some(thresholds) = resolve_thresholds_for_index(
        threshold_store.as_ref(),
        &project.id,
        &metadata,
        embedding_artifact,
    ) else {
        return Ok(Some(readiness("thresholds_incompatible")));
    };
    if !metadata.is_fresh(&project.wiki_root())? {
        return Ok(Some(readiness("semantic_index_stale")));
    }
    let vector_path = paths.semantic_vector_index(&project.id);
    let Some(vector_index) = SemanticVectorIndex::read(&vector_path)? else {
        return Ok(Some(readiness("semantic_index_missing")));
    };
    if !vector_index.is_compatible(&metadata, &thresholds.thresholds) {
        return Ok(Some(readiness("semantic_index_stale")));
    }
    Ok(None)
}

fn required_model_ids(
    profile: &SearchProfile,
    selected_mode: RuntimeSearchMode,
) -> Result<Option<Vec<String>>> {
    let Some(embedding_model) = profile.embedding_model.clone() else {
        return Ok(None);
    };
    let mut model_ids = vec![embedding_model];
    if selected_mode == RuntimeSearchMode::Hybrid {
        let Some(query_expansion_model) = profile.query_expansion_model.clone() else {
            return Ok(None);
        };
        model_ids.push(query_expansion_model);
    }
    // The reranker is an optional post-retrieval enhancement, not a search
    // readiness gate: when it is unconfigured (or unmaterialized) the search
    // still succeeds and `rerank_status` reports `reranker_model_unconfigured`.
    // Requiring it here would turn `rerank: true` into a hard `model_missing`
    // failure even when hybrid search itself is fully ready.
    Ok(Some(model_ids))
}

fn readiness(reason: &str) -> ReadinessFailure {
    let guidance = match reason {
        "install_profile_missing" => {
            "run `llm-wiki install --configure-search` to configure LLM search".to_string()
        }
        "llm_search_disabled" => {
            "run `llm-wiki install --configure-search` to enable LLM search".to_string()
        }
        "model_missing" => {
            "run `llm-wiki install --configure-search` to materialize and verify search models"
                .to_string()
        }
        "license_not_accepted" => {
            "run `llm-wiki install --configure-search` to review and accept model licenses"
                .to_string()
        }
        "thresholds_unconfigured" => {
            "run `llm-wiki eval calibrate --record` to calibrate and record semantic/hybrid \
             thresholds, then retry (or use mode `auto`/`lexical`, which do not require calibration)"
                .to_string()
        }
        "thresholds_incompatible" => {
            "run `llm-wiki eval calibrate --record` to record calibrated thresholds for the \
             current semantic index inputs"
                .to_string()
        }
        "semantic_index_missing" => {
            "run `llm-wiki index` to build semantic search state".to_string()
        }
        "semantic_index_stale" => {
            "run `llm-wiki index --force` to rebuild semantic search state".to_string()
        }
        other => format!("resolve search readiness issue `{other}`"),
    };
    ReadinessFailure {
        reason: reason.to_string(),
        guidance,
    }
}

fn runtime_failure_guidance(error: &GgufRuntimeError) -> String {
    format!(
        "GGUF runtime {} failed during {}; runtime backend failures are not model install failures",
        error.role().label(),
        error.stage().label()
    )
}

fn handle_readiness_failure(
    query: &str,
    project: Option<(&str, &str)>,
    format: OutputFormat,
    requested_mode: SearchModeArg,
    rerank_requested: bool,
    failure: ReadinessFailure,
    profile: Option<&SearchProfile>,
) -> Result<()> {
    if format == OutputFormat::Json {
        print_search_json(
            query,
            project,
            None,
            None,
            &[],
            &[],
            &[],
            SearchModeJson::readiness_failure(
                requested_mode,
                &failure.reason,
                profile,
                rerank_requested,
            ),
            SearchJsonOptions::full(),
        );
    }
    bail!(
        "search readiness failure: {} ({})",
        failure.reason,
        failure.guidance
    )
}

impl ProjectSelectionSource {
    fn label(self) -> &'static str {
        match self {
            Self::ExplicitProject => "--project",
            Self::CurrentDirectory => "current directory",
        }
    }
}

fn ensure_project_root_exists(project: &RegisteredProject) -> Result<()> {
    if !project.root.exists() {
        bail!(
            "project root missing for {}; run `llm-wiki projects` or `llm-wiki forget {}`",
            project.id,
            project.id
        );
    }
    Ok(())
}

fn stale_project_search_report(
    project: &RegisteredProject,
    requested_mode: SearchModeArg,
    rerank_requested: bool,
    warnings: &mut Vec<SearchWarning>,
    context: &CliContext,
) -> Option<ProjectSearchReport> {
    let wiki_root = project.wiki_root();
    let (reason, detail) = if !project.root.is_dir() {
        (
            "project_root_missing",
            format!(
                "project root missing for {}; run `llm-wiki projects` or `llm-wiki forget {}`",
                project.id, project.id
            ),
        )
    } else if !wiki_root.is_dir() {
        (
            "wiki_root_missing",
            format!(
                "wiki root missing for {}; run `llm-wiki projects` or `llm-wiki forget {}`",
                project.id, project.id
            ),
        )
    } else {
        return None;
    };

    context.diagnostic(format!("per-project readiness {}: {}", project.id, reason));
    warnings.push(SearchWarning {
        project_id: project.id.clone(),
        message: format!("project skipped: {detail}"),
    });

    Some(ProjectSearchReport {
        project_id: project.id.clone(),
        requested_mode: requested_mode.label().to_string(),
        selected_mode: None,
        mode_selection_reason: None,
        fallback_reason: None,
        readiness_reason: Some(reason.to_string()),
        rerank_requested,
        rerank_applied: false,
        rerank_reason: rerank_requested.then(|| "search_not_ready".to_string()),
        runtime_backend_requested: None,
        runtime_backend_used: None,
        runtime_backend_fallback: None,
        runtime_failure_stage: None,
        runtime_error_kind: None,
        backend_status: None,
        result_count: 0,
        no_result: None,
        profile: None,
        thresholds_source: None,
    })
}

fn select_projects(
    registry: &ProjectRegistry,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<RegisteredProject>> {
    for project_id in exclude {
        if registry.project_by_id(project_id).is_none() {
            bail!("project id {project_id} is not registered");
        }
    }

    // Dedupe `--include` (preserving first-seen order) so a repeated project id
    // (e.g. `--include a --include a`) is selected once and does not double its
    // RRF contribution during fusion.
    let mut seen_include = BTreeSet::new();
    let include: Vec<String> = include
        .iter()
        .filter(|project_id| seen_include.insert((*project_id).clone()))
        .cloned()
        .collect();

    let mut projects = if include.is_empty() {
        registry.projects.clone()
    } else {
        include
            .iter()
            .map(|project_id| {
                registry
                    .project_by_id(project_id)
                    .cloned()
                    .with_context(|| format!("project id {project_id} is not registered"))
            })
            .collect::<Result<Vec<_>>>()?
    };
    projects.retain(|project| !exclude.iter().any(|id| id == &project.id));
    if projects.is_empty() {
        bail!("no registered projects selected");
    }
    projects.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(projects)
}

#[derive(Clone, Debug)]
struct FusedResult {
    result: SearchResult,
    score: f64,
    lexical_rank: Option<usize>,
    lexical_score: Option<f64>,
    semantic_rank: Option<usize>,
    semantic_score: Option<f64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HybridBranch {
    Lexical,
    Semantic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThresholdsSource {
    Recorded,
    Default,
}

impl ThresholdsSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Default => "default",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectSearchReport {
    project_id: String,
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    rerank_requested: bool,
    rerank_applied: bool,
    rerank_reason: Option<String>,
    runtime_backend_requested: Option<String>,
    runtime_backend_used: Option<String>,
    runtime_backend_fallback: Option<bool>,
    runtime_failure_stage: Option<String>,
    runtime_error_kind: Option<String>,
    backend_status: Option<BackendStatus>,
    result_count: usize,
    no_result: Option<String>,
    profile: Option<SearchProfile>,
    thresholds_source: Option<ThresholdsSource>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct SearchWarning {
    project_id: String,
    message: String,
}

const DEFAULT_COMPACT_SEARCH_PAGE_SIZE: usize = 3;

#[derive(Clone, Copy, Debug)]
struct SearchJsonOptions {
    compact: bool,
    page_size: Option<usize>,
    offset: usize,
}

impl SearchJsonOptions {
    const fn full() -> Self {
        Self {
            compact: false,
            page_size: None,
            offset: 0,
        }
    }

    fn from_search_args(args: &SearchArgs) -> Self {
        Self {
            compact: args.compact,
            page_size: args.page_size,
            offset: args.offset,
        }
    }

    fn from_search_all_args(args: &SearchAllArgs) -> Self {
        Self {
            compact: args.compact,
            page_size: args.page_size,
            offset: args.offset,
        }
    }

    fn effective_page_size(self) -> usize {
        self.page_size
            .filter(|page_size| *page_size > 0)
            .unwrap_or(DEFAULT_COMPACT_SEARCH_PAGE_SIZE)
    }
}

#[derive(Debug)]
struct SearchExecution {
    results: Vec<SearchResult>,
    warnings: Vec<SearchWarning>,
    status: BackendStatus,
    runtime_report: Option<GgufRuntimeReport>,
    thresholds_source: Option<ThresholdsSource>,
    /// How many of the last `results` the phrase fallback added.
    phrase_fallback_pages: usize,
}

enum SearchAttempt {
    Success(SearchExecution),
    Missing(BackendStatus),
    PermissionDenied(BackendStatus),
    ForceReindex(BackendStatus),
    RetryableUnavailable(BackendStatus),
}

#[derive(Debug)]
struct BackendReadinessFailure {
    reason: String,
    guidance: String,
    status: BackendStatus,
}

impl BackendReadinessFailure {
    fn new(reason: &str, guidance: String, status: BackendStatus) -> Self {
        Self {
            reason: reason.to_string(),
            guidance,
            status,
        }
    }
}

impl fmt::Display for BackendReadinessFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "search readiness failure: {} ({})",
            self.reason, self.guidance
        )
    }
}

impl std::error::Error for BackendReadinessFailure {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModeResolution {
    requested_mode: SearchModeArg,
    selected_mode: RuntimeSearchMode,
    reason: String,
    fallback_reason: Option<String>,
    profile: Option<SearchProfile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeSearchMode {
    Lexical,
    Semantic,
    Hybrid,
}

impl RuntimeSearchMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Lexical => "lexical",
            Self::Semantic => "semantic",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReadinessFailure {
    reason: String,
    guidance: String,
}

#[derive(Clone, Debug, Serialize)]
struct SearchModeJson {
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    zero_result_reason: Option<String>,
    profile: Option<String>,
    embedding_model: Option<String>,
    thresholds_source: Option<String>,
    query_expansion_model: Option<String>,
    reranker_model: Option<String>,
    rerank_requested: bool,
    rerank_applied: bool,
    rerank_reason: Option<String>,
    runtime_backend_requested: Option<String>,
    runtime_backend_used: Option<String>,
    runtime_backend_fallback: Option<bool>,
    runtime_failure_stage: Option<String>,
    runtime_error_kind: Option<String>,
}

/// Report whether reranking actually ran and, if not, why. Reranking only
/// applies in hybrid mode with a configured reranker model; otherwise the
/// request is reported as not-applied with an explicit reason so callers never
/// have to infer it from a null `reranker_model`.
fn rerank_status(
    requested: bool,
    selected_mode: RuntimeSearchMode,
    reranker_model: Option<&str>,
) -> (bool, Option<String>) {
    if !requested {
        return (false, None);
    }
    if selected_mode != RuntimeSearchMode::Hybrid {
        return (
            false,
            Some(format!(
                "rerank_requires_hybrid_mode (selected `{}`)",
                selected_mode.label()
            )),
        );
    }
    match reranker_model {
        Some(_) => (true, None),
        None => (false, Some("reranker_model_unconfigured".to_string())),
    }
}

impl SearchModeJson {
    fn from_resolution(resolution: &ModeResolution, rerank_requested: bool) -> Self {
        let reranker_model = resolution
            .profile
            .as_ref()
            .and_then(|profile| profile.reranker_model.clone());
        let (rerank_applied, rerank_reason) = rerank_status(
            rerank_requested,
            resolution.selected_mode,
            reranker_model.as_deref(),
        );
        Self {
            requested_mode: resolution.requested_mode.label().to_string(),
            selected_mode: Some(resolution.selected_mode.label().to_string()),
            mode_selection_reason: Some(resolution.reason.clone()),
            fallback_reason: resolution.fallback_reason.clone(),
            readiness_reason: None,
            zero_result_reason: None,
            profile: resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.profile.clone()),
            embedding_model: resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.embedding_model.clone()),
            thresholds_source: None,
            query_expansion_model: resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.query_expansion_model.clone()),
            reranker_model,
            rerank_requested,
            rerank_applied,
            rerank_reason,
            runtime_backend_requested: None,
            runtime_backend_used: None,
            runtime_backend_fallback: None,
            runtime_failure_stage: None,
            runtime_error_kind: None,
        }
    }

    fn from_search_all_reports(
        requested_mode: SearchModeArg,
        rerank_requested: bool,
        reports: &[ProjectSearchReport],
    ) -> Self {
        let selected_mode = common_report_value(reports.iter().filter_map(|report| {
            report
                .readiness_reason
                .is_none()
                .then_some(report.selected_mode.as_deref())
                .flatten()
        }));
        let mode_selection_reason = common_report_value(reports.iter().filter_map(|report| {
            report
                .readiness_reason
                .is_none()
                .then_some(report.mode_selection_reason.as_deref())
                .flatten()
        }))
        .or_else(|| {
            reports
                .iter()
                .any(|report| report.readiness_reason.is_some())
                .then(|| "per_project".to_string())
        });
        let fallback_reason = common_report_value(reports.iter().filter_map(|report| {
            report
                .readiness_reason
                .is_none()
                .then_some(report.fallback_reason.as_deref())
                .flatten()
        }));
        let profile = common_report_value(reports.iter().filter_map(|report| {
            report
                .profile
                .as_ref()
                .and_then(|profile| profile.profile.as_deref())
        }));
        let embedding_model = common_report_value(reports.iter().filter_map(|report| {
            report
                .profile
                .as_ref()
                .and_then(|profile| profile.embedding_model.as_deref())
        }));
        let thresholds_source = common_report_value(
            reports
                .iter()
                .filter_map(|report| report.thresholds_source.map(ThresholdsSource::label)),
        );
        let query_expansion_model = common_report_value(reports.iter().filter_map(|report| {
            report
                .profile
                .as_ref()
                .and_then(|profile| profile.query_expansion_model.as_deref())
        }));
        let reranker_model = common_report_value(reports.iter().filter_map(|report| {
            report
                .profile
                .as_ref()
                .and_then(|profile| profile.reranker_model.as_deref())
        }));
        let readiness_reason = reports
            .iter()
            .all(|report| report.readiness_reason.is_some())
            .then(|| {
                common_report_value(
                    reports
                        .iter()
                        .filter_map(|report| report.readiness_reason.as_deref()),
                )
                .unwrap_or_else(|| "per_project_readiness_failure".to_string())
            });
        let all_runtime_failures = !reports.is_empty()
            && reports
                .iter()
                .all(|report| report.runtime_error_kind.is_some());
        let runtime_backend_requested = common_report_value(
            reports
                .iter()
                .filter_map(|report| report.runtime_backend_requested.as_deref()),
        );
        let runtime_backend_used = common_report_value(
            reports
                .iter()
                .filter_map(|report| report.runtime_backend_used.as_deref()),
        );
        let runtime_backend_fallback = common_report_bool(
            reports
                .iter()
                .filter_map(|report| report.runtime_backend_fallback),
        );
        let runtime_failure_stage = all_runtime_failures
            .then(|| {
                common_report_value(
                    reports
                        .iter()
                        .filter_map(|report| report.runtime_failure_stage.as_deref()),
                )
            })
            .flatten();
        let runtime_error_kind = all_runtime_failures
            .then(|| {
                common_report_value(
                    reports
                        .iter()
                        .filter_map(|report| report.runtime_error_kind.as_deref()),
                )
            })
            .flatten();
        // Aggregate rerank status from the per-project reports rather than from
        // the collapsed `selected_mode` (which goes `None` whenever projects
        // disagree, and would otherwise mislabel a mixed run as not-ready). The
        // per-project reports carry the authoritative status; the top level is
        // only a summary that admits when it is partial.
        let (rerank_applied, rerank_reason) = if !rerank_requested {
            (false, None)
        } else if reports.is_empty() {
            (false, Some("no_projects".to_string()))
        } else if reports.iter().all(|report| report.rerank_applied) {
            (true, None)
        } else if reports.iter().any(|report| report.rerank_applied) {
            // Some projects reranked and some did not: the summary cannot be a
            // single boolean, so point callers at the per-project rows.
            (false, Some("partial_across_projects".to_string()))
        } else {
            // No project reranked: surface the shared reason when every project
            // agrees, otherwise mark it partial.
            let reason = common_report_value(
                reports
                    .iter()
                    .filter_map(|report| report.rerank_reason.as_deref()),
            )
            .unwrap_or_else(|| "partial_across_projects".to_string());
            (false, Some(reason))
        };
        Self {
            requested_mode: requested_mode.label().to_string(),
            selected_mode,
            mode_selection_reason,
            fallback_reason,
            readiness_reason,
            zero_result_reason: None,
            profile,
            embedding_model,
            thresholds_source,
            query_expansion_model,
            reranker_model,
            rerank_requested,
            rerank_applied,
            rerank_reason,
            runtime_backend_requested,
            runtime_backend_used,
            runtime_backend_fallback,
            runtime_failure_stage,
            runtime_error_kind,
        }
    }

    fn readiness_failure(
        requested_mode: SearchModeArg,
        reason: &str,
        profile: Option<&SearchProfile>,
        rerank_requested: bool,
    ) -> Self {
        Self {
            requested_mode: requested_mode.label().to_string(),
            selected_mode: None,
            mode_selection_reason: None,
            fallback_reason: None,
            readiness_reason: Some(reason.to_string()),
            zero_result_reason: None,
            profile: profile.and_then(|profile| profile.profile.clone()),
            embedding_model: profile.and_then(|profile| profile.embedding_model.clone()),
            thresholds_source: None,
            query_expansion_model: profile
                .and_then(|profile| profile.query_expansion_model.clone()),
            reranker_model: profile.and_then(|profile| profile.reranker_model.clone()),
            rerank_requested,
            rerank_applied: false,
            rerank_reason: rerank_requested.then(|| "search_not_ready".to_string()),
            runtime_backend_requested: None,
            runtime_backend_used: None,
            runtime_backend_fallback: None,
            runtime_failure_stage: None,
            runtime_error_kind: None,
        }
    }

    fn apply_runtime_failure(&mut self, error: &GgufRuntimeError) {
        self.readiness_reason = Some(error.kind().readiness_reason().to_string());
        self.runtime_backend_requested =
            Some(gguf_runtime::requested_backend().label().to_string());
        self.runtime_backend_used = None;
        self.runtime_backend_fallback = Some(false);
        self.runtime_failure_stage = Some(error.stage().label().to_string());
        self.runtime_error_kind = Some(error.kind().label().to_string());
    }

    fn apply_runtime_report(&mut self, report: &GgufRuntimeReport) {
        self.runtime_backend_requested = Some(report.requested_backend().label().to_string());
        self.runtime_backend_used = Some(report.used_backend().label().to_string());
        self.runtime_backend_fallback = Some(report.fallback());
    }
}

fn common_report_value<'a>(mut values: impl Iterator<Item = &'a str>) -> Option<String> {
    let first = values.next()?;
    if values.all(|value| value == first) {
        Some(first.to_string())
    } else {
        Some("mixed".to_string())
    }
}

fn common_report_bool(mut values: impl Iterator<Item = bool>) -> Option<bool> {
    let first = values.next()?;
    values.all(|value| value == first).then_some(first)
}

fn print_search_text(warnings: &[SearchWarning], results: &[SearchResult]) {
    match warnings {
        [] => {}
        [warning] => println!("Warning: {}", warning.message),
        many => {
            for warning in many.iter().take(3) {
                println!("Warning: {}", warning.message);
            }
            if many.len() > 3 {
                println!("Warning: ...{} more warnings", many.len() - 3);
            }
        }
    }
    if results.is_empty() {
        println!("No results.");
        return;
    }
    for (index, result) in results.iter().enumerate() {
        println!(
            "{}. [{}] {} ({}) score={:.3} freshness={}",
            index + 1,
            result.project_id,
            result.title,
            result.path.display(),
            result.score.0,
            freshness_label(result.freshness)
        );
        if let Some(snippet) = &result.snippet {
            println!("   {snippet}");
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct SearchResultJson {
    project_id: String,
    project_name: Option<String>,
    path: String,
    title: String,
    document_class: Option<String>,
    status: Option<String>,
    score: f64,
    snippet: Option<String>,
    backend: String,
    mode: String,
    freshness: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    lexical_rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lexical_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_score: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
struct SearchWarningJson {
    project_id: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
struct CompactSearchResultJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    path: String,
    title: String,
    #[serde(rename = "class", skip_serializing_if = "Option::is_none")]
    document_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    score: f64,
    mode: String,
}

#[derive(Clone, Debug, Serialize)]
struct CompactProjectSearchReportJson {
    project_id: String,
    result_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    readiness_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zero_result_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ProjectSearchReportJson {
    project_id: String,
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    rerank_requested: bool,
    rerank_applied: bool,
    rerank_reason: Option<String>,
    runtime_backend_requested: Option<String>,
    runtime_backend_used: Option<String>,
    runtime_backend_fallback: Option<bool>,
    runtime_failure_stage: Option<String>,
    runtime_error_kind: Option<String>,
    backend_status: Option<BackendStatusJson>,
    result_count: usize,
    zero_result_reason: Option<String>,
    thresholds_source: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct BackendStatusJson {
    state: &'static str,
    open_mode: &'static str,
    store_path: String,
    indexed_files: usize,
    freshness: &'static str,
    message: Option<String>,
}

impl BackendStatusJson {
    fn from_status(status: &BackendStatus) -> Self {
        Self {
            state: backend_state_json_label(&status.state),
            open_mode: status.open_mode.label(),
            store_path: status.store_path.to_string_lossy().to_string(),
            indexed_files: status.indexed_files,
            freshness: freshness_label(freshness_for_status(status)),
            message: status.message.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct SearchEnvelopeJson {
    query: String,
    project_id: Option<String>,
    project_name: Option<String>,
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    zero_result_reason: Option<String>,
    profile: Option<String>,
    embedding_model: Option<String>,
    thresholds_source: Option<String>,
    query_expansion_model: Option<String>,
    reranker_model: Option<String>,
    rerank_requested: bool,
    rerank_applied: bool,
    rerank_reason: Option<String>,
    runtime_backend_requested: Option<String>,
    runtime_backend_used: Option<String>,
    runtime_backend_fallback: Option<bool>,
    runtime_failure_stage: Option<String>,
    runtime_error_kind: Option<String>,
    warning: Option<String>,
    warnings: Vec<SearchWarningJson>,
    backend_status: Option<BackendStatusJson>,
    projects: Vec<ProjectSearchReportJson>,
    results: Vec<SearchResultJson>,
}

#[derive(Clone, Debug, Serialize)]
struct CompactSearchEnvelopeJson {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    requested_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zero_result_reason: Option<String>,
    result_count: usize,
    offset: usize,
    page_size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<SearchWarningJson>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    projects: Vec<CompactProjectSearchReportJson>,
    results: Vec<CompactSearchResultJson>,
}

/// Public JSON contract from the D9 project-registry/search output plan.
// Pre-existing aggregate JSON printer; the wide argument list mirrors the
// public search-JSON contract. Silenced to keep the workspace clippy gate green
// under newer clippy without reshaping an unrelated public surface.
#[allow(clippy::too_many_arguments)]
fn print_search_json(
    query: &str,
    project: Option<(&str, &str)>,
    warning: Option<&str>,
    backend_status: Option<&BackendStatus>,
    warnings: &[SearchWarning],
    project_reports: &[ProjectSearchReport],
    results: &[SearchResult],
    mode: SearchModeJson,
    options: SearchJsonOptions,
) {
    if options.compact {
        print_compact_search_json(
            query,
            project,
            warning,
            warnings,
            project_reports,
            results,
            mode,
            options,
        );
        return;
    }

    let results = results
        .iter()
        .map(|result| SearchResultJson {
            project_id: result.project_id.clone(),
            project_name: result.project_name.clone(),
            path: result.path.to_string_lossy().to_string(),
            title: result.title.clone(),
            document_class: result.document_class.clone(),
            status: result.status.clone(),
            score: result.score.0,
            snippet: result.snippet.clone(),
            backend: result.backend.clone(),
            mode: result.mode.to_string(),
            freshness: freshness_label(result.freshness),
            lexical_rank: result.lexical_rank,
            lexical_score: result.lexical_score.map(|score| score.0),
            semantic_rank: result.semantic_rank,
            semantic_score: result.semantic_score.map(|score| score.0),
        })
        .collect::<Vec<_>>();
    let warnings = warnings
        .iter()
        .map(|warning| SearchWarningJson {
            project_id: warning.project_id.clone(),
            message: warning.message.clone(),
        })
        .collect::<Vec<_>>();
    let projects = project_reports
        .iter()
        .map(|report| ProjectSearchReportJson {
            project_id: report.project_id.clone(),
            requested_mode: report.requested_mode.clone(),
            selected_mode: report.selected_mode.clone(),
            mode_selection_reason: report.mode_selection_reason.clone(),
            fallback_reason: report.fallback_reason.clone(),
            readiness_reason: report.readiness_reason.clone(),
            rerank_requested: report.rerank_requested,
            rerank_applied: report.rerank_applied,
            rerank_reason: report.rerank_reason.clone(),
            runtime_backend_requested: report.runtime_backend_requested.clone(),
            runtime_backend_used: report.runtime_backend_used.clone(),
            runtime_backend_fallback: report.runtime_backend_fallback,
            runtime_failure_stage: report.runtime_failure_stage.clone(),
            runtime_error_kind: report.runtime_error_kind.clone(),
            backend_status: report
                .backend_status
                .as_ref()
                .map(BackendStatusJson::from_status),
            result_count: report.result_count,
            zero_result_reason: report.no_result.clone(),
            thresholds_source: report
                .thresholds_source
                .map(|source| source.label().to_string()),
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&SearchEnvelopeJson {
            query: query.to_string(),
            project_id: project.map(|(id, _)| id.to_string()),
            project_name: project.map(|(_, name)| name.to_string()),
            requested_mode: mode.requested_mode,
            selected_mode: mode.selected_mode,
            mode_selection_reason: mode.mode_selection_reason,
            fallback_reason: mode.fallback_reason,
            readiness_reason: mode.readiness_reason,
            zero_result_reason: mode.zero_result_reason,
            profile: mode.profile,
            embedding_model: mode.embedding_model,
            thresholds_source: mode.thresholds_source,
            query_expansion_model: mode.query_expansion_model,
            reranker_model: mode.reranker_model,
            rerank_requested: mode.rerank_requested,
            rerank_applied: mode.rerank_applied,
            rerank_reason: mode.rerank_reason,
            runtime_backend_requested: mode.runtime_backend_requested,
            runtime_backend_used: mode.runtime_backend_used,
            runtime_backend_fallback: mode.runtime_backend_fallback,
            runtime_failure_stage: mode.runtime_failure_stage,
            runtime_error_kind: mode.runtime_error_kind,
            warning: warning.map(ToString::to_string),
            warnings,
            backend_status: backend_status.map(BackendStatusJson::from_status),
            projects,
            results,
        })
        .expect("serialize search json")
    );
}

#[allow(clippy::too_many_arguments)]
fn print_compact_search_json(
    query: &str,
    project: Option<(&str, &str)>,
    warning: Option<&str>,
    warnings: &[SearchWarning],
    project_reports: &[ProjectSearchReport],
    results: &[SearchResult],
    mode: SearchModeJson,
    options: SearchJsonOptions,
) {
    let page_size = options.effective_page_size();
    let offset = options.offset.min(results.len());
    let page_end = offset.saturating_add(page_size).min(results.len());
    let next_offset = (page_end < results.len()).then_some(page_end);
    let results_page = results[offset..page_end]
        .iter()
        .map(|result| CompactSearchResultJson {
            project_id: (!result.project_id.is_empty()).then(|| result.project_id.clone()),
            project_name: result.project_name.clone(),
            path: result.path.to_string_lossy().to_string(),
            title: result.title.clone(),
            document_class: result.document_class.clone(),
            status: result.status.clone(),
            score: result.score.0,
            mode: result.mode.to_string(),
        })
        .collect::<Vec<_>>();
    let warnings = warnings
        .iter()
        .map(|warning| SearchWarningJson {
            project_id: warning.project_id.clone(),
            message: warning.message.clone(),
        })
        .collect::<Vec<_>>();
    let projects = project_reports
        .iter()
        .map(|report| CompactProjectSearchReportJson {
            project_id: report.project_id.clone(),
            result_count: report.result_count,
            readiness_reason: report.readiness_reason.clone(),
            zero_result_reason: report.no_result.clone(),
        })
        .collect::<Vec<_>>();
    let (project_id, project_name) =
        project.map_or((None, None), |(id, name)| (Some(id), Some(name)));

    println!(
        "{}",
        serde_json::to_string_pretty(&CompactSearchEnvelopeJson {
            query: query.to_string(),
            project_id: project_id.map(str::to_string),
            project_name: project_name.map(str::to_string),
            requested_mode: mode.requested_mode,
            selected_mode: mode.selected_mode,
            zero_result_reason: mode.zero_result_reason,
            result_count: results.len(),
            offset,
            page_size,
            next_offset,
            warning: warning.map(str::to_string),
            warnings,
            projects,
            results: results_page,
        })
        .expect("serialize compact search json")
    );
}

fn freshness_label(freshness: Freshness) -> &'static str {
    match freshness {
        Freshness::Fresh => "fresh",
        Freshness::Stale => "stale",
        Freshness::Unknown => "unknown",
    }
}

pub(crate) fn freshness_for_status(status: &BackendStatus) -> Freshness {
    match status.state {
        BackendState::Ready => Freshness::Fresh,
        BackendState::Stale => Freshness::Stale,
        BackendState::Missing
        | BackendState::Transient
        | BackendState::PermissionDenied
        | BackendState::Corrupt
        | BackendState::SchemaMismatch => Freshness::Unknown,
    }
}

fn backend_state_label(state: &BackendState) -> &'static str {
    match state {
        BackendState::Ready => "ready",
        BackendState::Missing => "missing",
        BackendState::Stale => "stale",
        BackendState::Transient => "transient",
        BackendState::PermissionDenied => "permission-denied",
        BackendState::Corrupt => "corrupt",
        BackendState::SchemaMismatch => "schema-mismatch",
    }
}

fn backend_state_json_label(state: &BackendState) -> &'static str {
    match state {
        BackendState::Ready => "ready",
        BackendState::Missing => "missing",
        BackendState::Stale => "stale",
        BackendState::Transient => "transient",
        BackendState::PermissionDenied => "permission_denied",
        BackendState::Corrupt => "corrupt",
        BackendState::SchemaMismatch => "schema_mismatch",
    }
}

fn filter_label(value: Option<&str>) -> &str {
    value.unwrap_or("<none>")
}

fn search_all_selection_label(include: &[String], exclude: &[String]) -> &'static str {
    match (include.is_empty(), exclude.is_empty()) {
        (true, true) => "all registered projects",
        (false, true) => "include filters",
        (true, false) => "exclude filters",
        (false, false) => "include/exclude filters",
    }
}

struct NoResultContext<'a> {
    project_id: &'a str,
    store_path: &'a Path,
    wiki_root: &'a Path,
    query: &'a str,
    filters: &'a SearchFilters,
    limit: usize,
    status: &'a BackendStatus,
    results: &'a [SearchResult],
}

fn explain_no_results_for_mode(
    backend: &QmdRsBackend,
    resolution: &ModeResolution,
    context: NoResultContext<'_>,
) -> Option<String> {
    if resolution.selected_mode == RuntimeSearchMode::Lexical {
        return explain_lexical_no_results(backend, context);
    }
    explain_llm_no_results(resolution.selected_mode, context)
}

fn explain_llm_no_results(
    selected_mode: RuntimeSearchMode,
    context: NoResultContext<'_>,
) -> Option<String> {
    if !context.results.is_empty() {
        return None;
    }
    if context.limit == 0 {
        return Some("limit was 0".to_string());
    }
    if filters_active(context.filters) {
        return Some(format!(
            "{} relevance thresholds or filters excluded all matched hits",
            selected_mode.label()
        ));
    }
    if matches!(context.status.state, BackendState::Stale) {
        return Some(format!(
            "stale-but-searchable {} index produced zero results",
            selected_mode.label()
        ));
    }
    Some(format!(
        "{} retrieval returned zero hits after relevance thresholds",
        selected_mode.label()
    ))
}

fn explain_lexical_no_results(
    backend: &QmdRsBackend,
    context: NoResultContext<'_>,
) -> Option<String> {
    if !context.results.is_empty() {
        return None;
    }

    if context.limit == 0 {
        return Some("limit was 0".to_string());
    }

    if sanitize_fts_query(context.query).is_empty() {
        return Some("zero terms after FTS sanitization".to_string());
    }

    if context.status.indexed_files == 0 {
        let state = match context.status.state {
            BackendState::Ready => "ready",
            BackendState::Stale => "stale",
            BackendState::Missing
            | BackendState::Transient
            | BackendState::PermissionDenied
            | BackendState::Corrupt
            | BackendState::SchemaMismatch => "unusable",
        };
        return Some(format!("{state} index has zero indexed files"));
    }

    if filters_active(context.filters) {
        let unfiltered_count = backend
            .search_project(
                context.project_id,
                context.store_path,
                context.wiki_root,
                context.query,
                &SearchFilters::default(),
                context.limit.max(1),
            )
            .map(|results| results.len())
            .unwrap_or(0);
        if unfiltered_count > 0 {
            return Some("filters excluded all matched hits".to_string());
        }
    }

    if matches!(context.status.state, BackendState::Stale) {
        return Some("stale-but-searchable index produced zero results".to_string());
    }

    Some("backend returned zero hits before filters".to_string())
}

fn filters_active(filters: &SearchFilters) -> bool {
    filters.document_class.is_some() || filters.status.is_some()
}

struct ProjectSearchInput<'a> {
    paths: &'a Paths,
    backend: &'a QmdRsBackend,
    project: &'a RegisteredProject,
    store_path: &'a Path,
    wiki_root: &'a Path,
    query: &'a str,
    filters: &'a SearchFilters,
    limit: usize,
}

fn perform_resolved_project_search(
    input: &ProjectSearchInput<'_>,
    resolution: &ModeResolution,
    rerank_requested: bool,
    context: &CliContext,
) -> Result<SearchExecution> {
    match resolution.selected_mode {
        RuntimeSearchMode::Lexical => perform_project_search(
            input.backend,
            input.project,
            input.store_path,
            input.wiki_root,
            input.query,
            input.filters,
            input.limit,
            LexicalQuery::PhraseFallback,
        ),
        RuntimeSearchMode::Semantic => perform_semantic_project_search(input, resolution),
        RuntimeSearchMode::Hybrid => {
            perform_hybrid_project_search(input, resolution, rerank_requested, context)
        }
    }
}

struct SemanticRuntimeState {
    metadata: SemanticIndexMetadata,
    vectors: SemanticVectorIndex,
    thresholds: SearchThresholds,
    thresholds_source: ThresholdsSource,
    embedding_artifact: ModelArtifactRecord,
    artifacts: ModelArtifacts,
    accepted_licenses: AcceptedLicenses,
}

struct ResolvedSearchThresholds {
    thresholds: SearchThresholds,
    source: ThresholdsSource,
}

fn perform_semantic_project_search(
    input: &ProjectSearchInput<'_>,
    resolution: &ModeResolution,
) -> Result<SearchExecution> {
    let profile = resolution
        .profile
        .as_ref()
        .context("semantic search selected without an LLM search profile")?;
    let status = semantic_base_status(
        input.backend,
        input.project,
        input.store_path,
        input.wiki_root,
    )?;
    let state = load_semantic_runtime_state(input.paths, input.project, input.wiki_root, profile)?;
    ensure_profile_model_license(&state.accepted_licenses, profile.embedding_model.as_deref())?;
    let query_embedding = embed_query_with_runtime_report(
        input.query,
        &state.embedding_artifact,
        state.metadata.embedding_dimensions,
    )?;
    let results = state.vectors.search(
        &state.metadata,
        SemanticSearchContext {
            project_id: &input.project.id,
            project_name: Some(&input.project.name),
            wiki_root: input.wiki_root,
            query_embedding: &query_embedding.embedding,
            filters: input.filters,
            limit: input.limit,
            floor: state.thresholds.semantic_similarity_floor,
            freshness: freshness_for_status(&status),
            mode: SearchMode::Semantic,
        },
    )?;
    Ok(SearchExecution {
        results,
        warnings: stale_warning(input.project, &status).into_iter().collect(),
        status,
        runtime_report: query_embedding.runtime_report,
        thresholds_source: Some(state.thresholds_source),
        phrase_fallback_pages: 0,
    })
}

fn perform_hybrid_project_search(
    input: &ProjectSearchInput<'_>,
    resolution: &ModeResolution,
    rerank_requested: bool,
    context: &CliContext,
) -> Result<SearchExecution> {
    let profile = resolution
        .profile
        .as_ref()
        .context("hybrid search selected without an LLM search profile")?;
    let state = load_semantic_runtime_state(input.paths, input.project, input.wiki_root, profile)?;
    ensure_profile_model_license(&state.accepted_licenses, profile.embedding_model.as_deref())?;
    ensure_profile_model_license(
        &state.accepted_licenses,
        profile.query_expansion_model.as_deref(),
    )?;
    let expanded = expand_hybrid_queries(input.query, profile, &state.artifacts)?;
    let mut runtime_report = expanded.runtime_report.clone();
    context.diagnostic(format!(
        "hybrid query expansion: lexical={}, semantic={}",
        expanded.lexical.len(),
        expanded.semantic.len()
    ));

    let per_branch_limit = if input.limit == 0 {
        0
    } else {
        input.limit.max(20)
    };
    let mut warnings = Vec::new();
    let mut status = None;
    let mut lexical_results = Vec::new();
    for lexical_query in &expanded.lexical {
        let search = hybrid_lexical_branch_search(input, lexical_query, per_branch_limit)?;
        if status.is_none() {
            status = Some(search.status.clone());
        }
        warnings.extend(search.warnings);
        lexical_results.extend(search.results);
    }
    dedupe_warnings(&mut warnings);
    let status = match status {
        Some(status) => status,
        None => semantic_base_status(
            input.backend,
            input.project,
            input.store_path,
            input.wiki_root,
        )?,
    };
    let lexical_results = dedupe_by_path_preserving_rank(lexical_results);

    let mut semantic_results = Vec::new();
    // Load the embedding engine once and reuse it across every expanded sub-query
    // instead of reconstructing the ~333 MB model per expansion.
    let mut query_embedder = QueryEmbedder::new(
        &state.embedding_artifact,
        state.metadata.embedding_dimensions,
    )?;
    for semantic_query in &expanded.semantic {
        let query_embedding = query_embedder.embed(semantic_query)?;
        merge_runtime_report(&mut runtime_report, query_embedding.runtime_report);
        semantic_results.extend(state.vectors.search(
            &state.metadata,
            SemanticSearchContext {
                project_id: &input.project.id,
                project_name: Some(&input.project.name),
                wiki_root: input.wiki_root,
                query_embedding: &query_embedding.embedding,
                filters: input.filters,
                limit: per_branch_limit,
                floor: state.thresholds.hybrid_pre_fusion_semantic_floor,
                freshness: freshness_for_status(&status),
                mode: SearchMode::Semantic,
            },
        )?);
    }
    let semantic_results = dedupe_by_path_preserving_rank(semantic_results);
    let mut results = if state.thresholds_source == ThresholdsSource::Default {
        let results = fuse_hybrid_results(
            input.query,
            &state.thresholds,
            lexical_results.clone(),
            semantic_results.clone(),
            input.limit,
        );
        if results.is_empty() {
            let relaxed_thresholds = relaxed_default_hybrid_thresholds(&state.thresholds);
            fuse_hybrid_results(
                input.query,
                &relaxed_thresholds,
                lexical_results,
                semantic_results,
                input.limit,
            )
        } else {
            results
        }
    } else {
        fuse_hybrid_results(
            input.query,
            &state.thresholds,
            lexical_results,
            semantic_results,
            input.limit,
        )
    };
    let rerank_output = maybe_rerank_results(
        input.query,
        results,
        RerankInputs {
            profile,
            artifacts: &state.artifacts,
            accepted_licenses: &state.accepted_licenses,
            thresholds: &state.thresholds,
            wiki_root: input.wiki_root,
            requested: rerank_requested,
        },
    )?;
    results = rerank_output.results;
    merge_runtime_report(&mut runtime_report, rerank_output.runtime_report);

    Ok(SearchExecution {
        results,
        warnings,
        status,
        runtime_report,
        thresholds_source: Some(state.thresholds_source),
        phrase_fallback_pages: 0,
    })
}

fn merge_runtime_report(
    target: &mut Option<GgufRuntimeReport>,
    incoming: Option<GgufRuntimeReport>,
) {
    let Some(incoming) = incoming else {
        return;
    };
    *target = Some(match target.take() {
        Some(existing) => existing.merged(&incoming),
        None => incoming,
    });
}

fn semantic_base_status(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
) -> Result<BackendStatus> {
    let status = backend.status(&project.id, store_path, wiki_root)?;
    match status.state {
        BackendState::Missing => Err(BackendReadinessFailure::new(
            "index_missing",
            format!(
                "run `llm-wiki index --project {}` to create the search index",
                project.id
            ),
            status,
        )
        .into()),
        BackendState::Transient => Err(BackendReadinessFailure::new(
            "transient",
            format!(
                "search index publication is in progress for project {}; retry the command",
                project.id
            ),
            status,
        )
        .into()),
        BackendState::PermissionDenied => Err(BackendReadinessFailure::new(
            "permission_denied",
            cache_access_guidance(project),
            status,
        )
        .into()),
        BackendState::Corrupt | BackendState::SchemaMismatch => Err(BackendReadinessFailure::new(
            "index_unusable",
            format!(
                "run `llm-wiki index --project {} --force` to rebuild the search index",
                project.id
            ),
            status,
        )
        .into()),
        BackendState::Ready | BackendState::Stale => Ok(status),
    }
}

fn load_semantic_runtime_state(
    paths: &Paths,
    project: &RegisteredProject,
    wiki_root: &Path,
    profile: &SearchProfile,
) -> Result<SemanticRuntimeState> {
    let artifacts = ModelArtifacts::read(&paths.model_artifacts())?
        .context("semantic search requires verified model artifact records")?;
    let accepted_licenses = AcceptedLicenses::read(&paths.accepted_licenses())?
        .context("semantic search requires accepted model license records")?;
    let threshold_store = SearchThresholdStore::read(&paths.search_thresholds())?;
    let embedding_artifact = embedding_artifact_for_profile(&artifacts, profile)
        .cloned()
        .context("semantic search requires a verified embedding model artifact")?;
    let metadata_path = paths.semantic_index_metadata(&project.id);
    let vector_path = paths.semantic_vector_index(&project.id);

    // The metadata and vector files are published as two independent atomic
    // writes, so a concurrent `index` run can briefly expose a mismatched pair.
    // Re-read both a few times before declaring the index stale; a genuinely
    // stale index (or a real config mismatch) still fails on the final attempt.
    const MAX_PUBLISH_RACE_RETRIES: usize = 4;
    let (metadata, vectors, resolved_thresholds) = {
        let mut attempt = 0;
        loop {
            let metadata = SemanticIndexMetadata::read(&metadata_path)?
                .with_context(|| format!("semantic index missing: {}", metadata_path.display()))?;
            let resolved_thresholds = resolve_thresholds_for_index(
                threshold_store.as_ref(),
                &project.id,
                &metadata,
                &embedding_artifact,
            )
            .context(
                "semantic thresholds do not match the current index inputs or project scope",
            )?;
            if !metadata.is_fresh(wiki_root)? {
                bail!(
                    "semantic index stale for project {}; run `llm-wiki index --project {} --force`",
                    project.id,
                    project.id
                );
            }
            let vectors = SemanticVectorIndex::read(&vector_path)?.with_context(|| {
                format!("semantic vector index missing: {}", vector_path.display())
            })?;
            if vectors.is_compatible(&metadata, &resolved_thresholds.thresholds) {
                break (metadata, vectors, resolved_thresholds);
            }
            attempt += 1;
            if attempt > MAX_PUBLISH_RACE_RETRIES {
                bail!(
                    "semantic vector index stale for project {}; run `llm-wiki index --project {} --force`",
                    project.id,
                    project.id
                );
            }
            // Back off before re-reading so the retry lands after a racing indexer
            // finishes its vector→metadata renames rather than re-observing the
            // same momentarily-mismatched pair.
            std::thread::sleep(std::time::Duration::from_millis(20 * attempt as u64));
        }
    };

    Ok(SemanticRuntimeState {
        metadata,
        vectors,
        thresholds: resolved_thresholds.thresholds,
        thresholds_source: resolved_thresholds.source,
        embedding_artifact,
        artifacts,
        accepted_licenses,
    })
}

fn embedding_artifact_for_profile<'a>(
    artifacts: &'a ModelArtifacts,
    profile: &SearchProfile,
) -> Option<&'a ModelArtifactRecord> {
    profile
        .embedding_model
        .as_deref()
        .and_then(|model_id| artifact_for_model(artifacts, model_id))
}

fn artifact_for_model<'a>(
    artifacts: &'a ModelArtifacts,
    model_id: &str,
) -> Option<&'a ModelArtifactRecord> {
    artifacts
        .artifacts
        .iter()
        .find(|artifact| artifact.model_id == model_id)
}

fn resolve_thresholds_for_index(
    threshold_store: Option<&SearchThresholdStore>,
    project_id: &str,
    metadata: &SemanticIndexMetadata,
    embedding_artifact: &ModelArtifactRecord,
) -> Option<ResolvedSearchThresholds> {
    if let Some(thresholds) = threshold_store.and_then(|store| {
        select_thresholds_for_index(store, project_id, metadata, embedding_artifact)
    }) {
        return Some(ResolvedSearchThresholds {
            thresholds: thresholds.clone(),
            source: ThresholdsSource::Recorded,
        });
    }

    SearchThresholds::default_for_model(
        metadata
            .profile
            .clone()
            .unwrap_or_else(|| crate::search_models::DEFAULT_PROFILE_ID.to_string()),
        metadata.embedding_model.clone(),
        embedding_artifact.observed_sha256.clone(),
        metadata.embedding_dimensions,
        metadata.chunking_strategy.clone(),
    )
    .map(|thresholds| ResolvedSearchThresholds {
        thresholds,
        source: ThresholdsSource::Default,
    })
}

fn ensure_profile_model_license(
    accepted_licenses: &AcceptedLicenses,
    model_id: Option<&str>,
) -> Result<()> {
    let Some(model_id) = model_id else {
        bail!("selected search profile is missing a required model");
    };
    let Some(model) = model_by_id(model_id) else {
        bail!("selected search profile references unknown model {model_id}");
    };
    if !accepted_licenses.accepts_model(model) {
        bail!(
            "license not accepted for model {model_id}; run `llm-wiki install --configure-search`"
        );
    }
    Ok(())
}

pub(crate) struct HybridQuerySet {
    pub(crate) lexical: Vec<String>,
    pub(crate) semantic: Vec<String>,
    pub(crate) runtime_report: Option<GgufRuntimeReport>,
}

pub(crate) fn expand_hybrid_queries(
    query: &str,
    profile: &SearchProfile,
    artifacts: &ModelArtifacts,
) -> Result<HybridQuerySet> {
    if env::var("LLM_WIKI_TEST_QUERY_EXPANSION")
        .ok()
        .is_some_and(|value| value == "deterministic")
    {
        return Ok(HybridQuerySet {
            lexical: vec![query.to_string()],
            semantic: vec![query.to_string()],
            runtime_report: None,
        });
    }

    let Some(model_id) = profile.query_expansion_model.as_deref() else {
        bail!("hybrid search requires a query expansion model");
    };
    let artifact = artifact_for_model(artifacts, model_id)
        .context("hybrid search requires a verified query expansion model artifact")?;
    let mut engine = gguf_runtime::generation_engine(&artifact.path)?;
    let expanded = gguf_runtime::expand_query(&mut engine, query)?;
    let mut lexical = vec![query.to_string()];
    let mut semantic = vec![query.to_string()];
    for item in expanded.value {
        match item.query_type {
            qmd::QueryType::Lex => lexical.push(item.text),
            qmd::QueryType::Vec | qmd::QueryType::Hyde => semantic.push(item.text),
        }
    }
    dedupe_strings_preserving_order(&mut lexical);
    dedupe_strings_preserving_order(&mut semantic);
    Ok(HybridQuerySet {
        lexical,
        semantic,
        runtime_report: Some(expanded.report),
    })
}

fn dedupe_strings_preserving_order(values: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

pub(crate) fn dedupe_by_path_preserving_rank(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for result in results {
        let key = result.path.to_string_lossy().to_string();
        if seen.insert(key) {
            deduped.push(result);
        }
    }
    deduped
}

fn dedupe_warnings(warnings: &mut Vec<SearchWarning>) {
    let mut seen = BTreeSet::new();
    warnings.retain(|warning| seen.insert((warning.project_id.clone(), warning.message.clone())));
}

pub(crate) fn fuse_hybrid_results(
    query: &str,
    thresholds: &SearchThresholds,
    lexical_results: Vec<SearchResult>,
    semantic_results: Vec<SearchResult>,
    limit: usize,
) -> Vec<SearchResult> {
    let mut fused: BTreeMap<String, FusedResult> = BTreeMap::new();
    add_ranked_results(&mut fused, HybridBranch::Lexical, lexical_results.clone());
    add_ranked_results(&mut fused, HybridBranch::Semantic, semantic_results);
    let mut results = fused.into_values().collect::<Vec<_>>();
    apply_hybrid_anchor_boost(query, &mut results);
    if !query_has_exact_identifier(query) {
        boost_semantic_confidence_prefix(&mut results, thresholds, 1);
    }
    results.retain(|result| hybrid_candidate_survives_final_gate(query, thresholds, result));
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.result.project_id.cmp(&right.result.project_id))
            .then_with(|| left.result.path.cmp(&right.result.path))
    });
    // Build the FULL transformed vec (no premature truncation): the lexical-guard
    // below must be able to restore a lexical top-3 doc that fused below `limit`,
    // which is impossible if the tail is dropped before pinning.
    let mut results = results
        .into_iter()
        .map(|mut fused| {
            fused.result.score = Score(fused.score);
            fused.result.backend = "qmd-rs-hybrid".to_string();
            fused.result.mode = SearchMode::Hybrid;
            fused.result.lexical_rank = fused.lexical_rank;
            fused.result.lexical_score = fused.lexical_score.map(Score);
            fused.result.semantic_rank = fused.semantic_rank;
            fused.result.semantic_score = fused.semantic_score.map(Score);
            fused.result
        })
        .collect::<Vec<_>>();
    if query_has_exact_identifier(query)
        && thresholds.lexical_exact_identifier_guard == "preserve_lexical_top_3"
    {
        // `pin_lexical_prefix` truncates to `limit` internally after pinning.
        pin_lexical_prefix(&mut results, &lexical_results, 3, limit);
    } else {
        results.truncate(limit);
    }
    results
}

fn relaxed_default_hybrid_thresholds(thresholds: &SearchThresholds) -> SearchThresholds {
    // Default thresholds start with the calibrated floors but can degrade for fresh projects
    // that would otherwise return no hybrid results before operators record project thresholds.
    let mut relaxed = thresholds.clone();
    relaxed.hybrid_final_semantic_floor = 0.0;
    relaxed.hybrid_semantic_only_floor = 0.0;
    relaxed.hybrid_strong_lexical_score_floor = 0.0;
    relaxed
}

fn boost_semantic_confidence_prefix(
    results: &mut [FusedResult],
    thresholds: &SearchThresholds,
    count: usize,
) {
    if count == 0 {
        return;
    }
    let floor = thresholds.hybrid_semantic_only_floor.max(0.50);
    for result in results {
        if result.semantic_rank.is_some_and(|rank| rank < count)
            && result.semantic_score.is_some_and(|score| score >= floor)
        {
            result.score += HIGH_CONFIDENCE_SEMANTIC_PREFIX_BOOST;
        }
    }
}

fn add_ranked_results(
    fused: &mut BTreeMap<String, FusedResult>,
    branch: HybridBranch,
    results: Vec<SearchResult>,
) {
    for (rank, result) in results.into_iter().enumerate() {
        let rrf = 1.0 / (60.0 + rank as f64 + 1.0);
        let branch_weight = match branch {
            HybridBranch::Lexical => 1.0,
            HybridBranch::Semantic => 1.25,
        };
        let key = result.path.to_string_lossy().to_string();
        fused
            .entry(key)
            .and_modify(|entry| {
                entry.score += branch_weight * rrf;
                record_branch_evidence(entry, branch, rank, result.score.0);
                if result.score.0 > entry.result.score.0 {
                    entry.result = result.clone();
                }
            })
            .or_insert_with(|| {
                let score = result.score.0;
                let mut fused = FusedResult {
                    result,
                    score: branch_weight * rrf,
                    lexical_rank: None,
                    lexical_score: None,
                    semantic_rank: None,
                    semantic_score: None,
                };
                record_branch_evidence(&mut fused, branch, rank, score);
                fused
            });
    }
}

fn record_branch_evidence(fused: &mut FusedResult, branch: HybridBranch, rank: usize, score: f64) {
    match branch {
        HybridBranch::Lexical => {
            fused.lexical_rank = Some(fused.lexical_rank.map_or(rank, |current| current.min(rank)));
            fused.lexical_score = Some(
                fused
                    .lexical_score
                    .map_or(score, |current| current.max(score)),
            );
        }
        HybridBranch::Semantic => {
            fused.semantic_rank = Some(
                fused
                    .semantic_rank
                    .map_or(rank, |current| current.min(rank)),
            );
            fused.semantic_score = Some(
                fused
                    .semantic_score
                    .map_or(score, |current| current.max(score)),
            );
        }
    }
}

fn hybrid_candidate_survives_final_gate(
    query: &str,
    thresholds: &SearchThresholds,
    fused: &FusedResult,
) -> bool {
    if query_has_exact_identifier(query)
        && thresholds.lexical_exact_identifier_guard == "preserve_lexical_top_3"
        && fused.lexical_rank.is_some_and(|rank| rank < 3)
    {
        return true;
    }

    if fused
        .lexical_score
        .is_some_and(|score| score >= thresholds.hybrid_strong_lexical_score_floor)
    {
        return true;
    }

    if fused
        .semantic_score
        .is_some_and(|score| score >= thresholds.hybrid_semantic_only_floor)
    {
        return true;
    }

    let semantic_passes_final_floor = fused
        .semantic_score
        .is_some_and(|score| score >= thresholds.hybrid_final_semantic_floor);
    if !semantic_passes_final_floor {
        return false;
    }

    fused.lexical_rank.is_some() || candidate_anchor_match_count(query, &fused.result) > 0
}

fn apply_hybrid_anchor_boost(query: &str, results: &mut [FusedResult]) {
    for result in results {
        let matched = candidate_anchor_match_count(query, &result.result);
        if matched > 0 {
            result.score += matched as f64 * 0.004;
        }
    }
}

fn candidate_anchor_match_count(query: &str, result: &SearchResult) -> usize {
    let anchors = query_anchor_terms(query);
    if anchors.is_empty() {
        return 0;
    }
    let haystack = format!(
        "{} {} {}",
        result.path.to_string_lossy().to_ascii_lowercase(),
        result.title.to_ascii_lowercase(),
        result
            .snippet
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    );
    anchors
        .iter()
        .filter(|anchor| haystack.contains(anchor.as_str()))
        .count()
}

pub(crate) fn query_anchor_terms(query: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|term| term.to_ascii_lowercase())
        .filter(|term| term.len() >= 4)
        .filter(|term| seen.insert(term.clone()))
        .collect()
}

fn query_has_exact_identifier(query: &str) -> bool {
    query.split_whitespace().any(|term| {
        term.contains("--")
            || term.contains('/')
            || term.contains('.')
            || term.contains('_')
            || term.chars().any(|ch| ch.is_ascii_digit())
    })
}

fn pin_lexical_prefix(
    results: &mut Vec<SearchResult>,
    lexical_results: &[SearchResult],
    count: usize,
    limit: usize,
) {
    if results.is_empty() {
        return;
    }
    let mut pinned = Vec::new();
    for lexical in lexical_results.iter().take(count) {
        if let Some(index) = results.iter().position(|result| {
            result.project_id == lexical.project_id && result.path == lexical.path
        }) {
            pinned.push(results.remove(index));
        }
    }
    pinned.append(results);
    pinned.truncate(limit);
    *results = pinned;
}

pub(crate) struct RerankInputs<'a> {
    pub(crate) profile: &'a SearchProfile,
    pub(crate) artifacts: &'a ModelArtifacts,
    pub(crate) accepted_licenses: &'a AcceptedLicenses,
    pub(crate) thresholds: &'a SearchThresholds,
    pub(crate) wiki_root: &'a Path,
    pub(crate) requested: bool,
}

pub(crate) struct RerankOutput {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) runtime_report: Option<GgufRuntimeReport>,
}

pub(crate) fn maybe_rerank_results(
    query: &str,
    results: Vec<SearchResult>,
    inputs: RerankInputs<'_>,
) -> Result<RerankOutput> {
    if !inputs.requested {
        return Ok(RerankOutput {
            results,
            runtime_report: None,
        });
    }
    let Some(model_id) = inputs.profile.reranker_model.as_deref() else {
        // Reranking is an optional enhancement. When the selected profile has no
        // reranker configured, degrade to the un-reranked results instead of
        // failing the whole search; `rerank_status` reports the request as
        // not-applied with `reranker_model_unconfigured`.
        return Ok(RerankOutput {
            results,
            runtime_report: None,
        });
    };
    ensure_profile_model_license(inputs.accepted_licenses, Some(model_id))?;
    let artifact = artifact_for_model(inputs.artifacts, model_id)
        .context("rerank requested but reranker artifact is missing")?;
    if env::var("LLM_WIKI_TEST_RERANK")
        .ok()
        .is_some_and(|value| value == "deterministic")
    {
        return Ok(RerankOutput {
            results: deterministic_rerank_results(results),
            runtime_report: None,
        });
    }
    let project_root = inputs.wiki_root.parent().unwrap_or(inputs.wiki_root);
    // Reranking is an optional enhancement, so a single unreadable result (e.g. a
    // file deleted between the freshness check and rerank) must not fail the whole
    // search. Skip such results with a warning and rerank the rest. `doc_results`
    // is kept in lockstep with `documents` so the reranker's `item.index` still
    // maps back to the correct original result.
    let mut documents = Vec::with_capacity(results.len());
    let mut doc_results = Vec::with_capacity(results.len());
    for result in &results {
        match fs::read_to_string(project_root.join(&result.path)) {
            Ok(text) => {
                documents.push(qmd::RerankDocument {
                    file: result.path.to_string_lossy().to_string(),
                    text,
                    title: Some(result.title.clone()),
                });
                doc_results.push(result.clone());
            }
            Err(error) => {
                eprintln!(
                    "warning: skipping unreadable rerank input {}: {error}",
                    result.path.display()
                );
            }
        }
    }
    let mut engine = gguf_runtime::rerank_engine(&artifact.path)?;
    let reranked = gguf_runtime::rerank(&mut engine, query, &documents)?;
    let mut results_by_rank = Vec::new();
    for item in reranked.value.results {
        if f64::from(item.score) < inputs.thresholds.reranker_probability_floor {
            continue;
        }
        let Some(original) = doc_results.get(item.index) else {
            continue;
        };
        let mut result = original.clone();
        result.score = Score(f64::from(item.score));
        result.backend = "qmd-rs-rerank".to_string();
        results_by_rank.push(result);
    }
    Ok(RerankOutput {
        results: results_by_rank,
        runtime_report: Some(reranked.report),
    })
}

fn deterministic_rerank_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let total = results.len().max(1);
    results
        .into_iter()
        .rev()
        .enumerate()
        .map(|(index, mut result)| {
            let score = 1.0 - (index as f64 / (total as f64 + 1.0));
            result.score = Score(score);
            result.backend = "qmd-rs-rerank-deterministic".to_string();
            result
        })
        .collect()
}

/// Which query a lexical search runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LexicalQuery {
    AllWords,
    /// The all-words query, then the phrase fallback when it finds too few
    /// pages. Hybrid's lexical branch never takes it: its partial matches
    /// would enter fusion with lexical ranks they never earned (the owner's
    /// choice on issue #25).
    PhraseFallback,
}

/// Hybrid's lexical branch: the all-words query alone.
fn hybrid_lexical_branch_search(
    input: &ProjectSearchInput<'_>,
    lexical_query: &str,
    limit: usize,
) -> Result<SearchExecution> {
    perform_project_search(
        input.backend,
        input.project,
        input.store_path,
        input.wiki_root,
        lexical_query,
        input.filters,
        limit,
        LexicalQuery::AllWords,
    )
}

#[allow(clippy::too_many_arguments)]
fn perform_project_search(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
    query: &str,
    filters: &SearchFilters,
    limit: usize,
    lexical_query: LexicalQuery,
) -> Result<SearchExecution> {
    for attempt in 0..2 {
        match search_attempt(
            backend,
            project,
            store_path,
            wiki_root,
            query,
            filters,
            limit,
            lexical_query,
        )? {
            SearchAttempt::Success(search) => return Ok(search),
            SearchAttempt::Missing(_) if attempt == 0 => retry_search_delay(),
            SearchAttempt::RetryableUnavailable(_) if attempt == 0 => retry_search_delay(),
            SearchAttempt::Missing(status) => {
                return Err(BackendReadinessFailure::new(
                    "index_missing",
                    format!(
                        "run `llm-wiki index --project {}` to create the search index",
                        project.id
                    ),
                    status,
                )
                .into());
            }
            SearchAttempt::PermissionDenied(status) => {
                return Err(BackendReadinessFailure::new(
                    "permission_denied",
                    cache_access_guidance(project),
                    status,
                )
                .into());
            }
            SearchAttempt::ForceReindex(status) => {
                return Err(BackendReadinessFailure::new(
                    "index_unusable",
                    format!(
                        "run `llm-wiki index --project {} --force` to rebuild the search index",
                        project.id
                    ),
                    status,
                )
                .into());
            }
            SearchAttempt::RetryableUnavailable(status) => {
                return Err(BackendReadinessFailure::new(
                    "transient",
                    format!(
                        "search index publication is in progress for project {}; retry the command",
                        project.id
                    ),
                    status,
                )
                .into());
            }
        }
    }
    unreachable!("search retry loop returns or bails")
}

#[allow(clippy::too_many_arguments)]
fn search_attempt(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
    query: &str,
    filters: &SearchFilters,
    limit: usize,
    lexical_query: LexicalQuery,
) -> Result<SearchAttempt> {
    let status = backend.status(&project.id, store_path, wiki_root)?;
    match status.state {
        BackendState::Missing => return Ok(SearchAttempt::Missing(status)),
        BackendState::Transient => return Ok(SearchAttempt::RetryableUnavailable(status)),
        BackendState::PermissionDenied => return Ok(SearchAttempt::PermissionDenied(status)),
        BackendState::Corrupt | BackendState::SchemaMismatch => {
            return Ok(SearchAttempt::ForceReindex(status));
        }
        BackendState::Ready | BackendState::Stale => {}
    }
    maybe_sleep_for_test("LLM_WIKI_TEST_SEARCH_AFTER_STATUS_SLEEP_MS");
    let search = match lexical_query {
        LexicalQuery::AllWords => backend
            .search_project(&project.id, store_path, wiki_root, query, filters, limit)
            .map(|results| LexicalSearch {
                results,
                fallback_pages: 0,
            }),
        LexicalQuery::PhraseFallback => backend.search_project_with_phrase_fallback(
            &project.id,
            store_path,
            wiki_root,
            query,
            filters,
            limit,
        ),
    };
    let search = match search {
        Ok(search) => search,
        Err(error) => {
            if let Some(status) = backend_access_failure(&error) {
                return Ok(match status.state {
                    BackendState::Transient | BackendState::Missing => {
                        SearchAttempt::RetryableUnavailable(status)
                    }
                    BackendState::PermissionDenied => SearchAttempt::PermissionDenied(status),
                    BackendState::Corrupt | BackendState::SchemaMismatch => {
                        SearchAttempt::ForceReindex(status)
                    }
                    BackendState::Ready | BackendState::Stale => return Err(error),
                });
            }
            if is_retryable_search_open_error(&error) {
                return Ok(SearchAttempt::RetryableUnavailable(status));
            }
            return Err(error);
        }
    };
    Ok(SearchAttempt::Success(SearchExecution {
        results: search.results,
        warnings: stale_warning(project, &status).into_iter().collect(),
        status,
        runtime_report: None,
        thresholds_source: None,
        phrase_fallback_pages: search.fallback_pages,
    }))
}

fn cache_access_guidance(project: &RegisteredProject) -> String {
    format!(
        "grant read access to the managed search cache for project {} and retry",
        project.id
    )
}

fn stale_warning(project: &RegisteredProject, status: &BackendStatus) -> Option<SearchWarning> {
    matches!(status.state, BackendState::Stale).then(|| SearchWarning {
        project_id: project.id.clone(),
        message: format!(
            "search index stale for project {}; run `llm-wiki index --project {}`",
            project.id, project.id
        ),
    })
}

/// A line per project whose phrase-fallback pages made it into the fused,
/// truncated reply; pages the limit cut off are not counted.
fn shown_phrase_fallback_warnings(
    results: &[SearchResult],
    fallback_keys: &BTreeSet<(String, String)>,
) -> Vec<SearchWarning> {
    let mut shown: BTreeMap<&str, usize> = BTreeMap::new();
    for result in results {
        let key = (
            result.project_id.clone(),
            result.path.to_string_lossy().to_string(),
        );
        if fallback_keys.contains(&key) {
            *shown.entry(result.project_id.as_str()).or_default() += 1;
        }
    }
    shown
        .into_iter()
        .filter_map(|(project_id, pages)| {
            phrase_fallback_warning(project_id, pages, FallbackPlacement::Fused)
        })
        .collect()
}

/// One project's line when the phrase fallback added pages to the reply. In a
/// single-project reply they are its last results; in search-all, fusion
/// places them among other projects' results.
fn phrase_fallback_warning(
    project_id: &str,
    shown_pages: usize,
    placement: FallbackPlacement,
) -> Option<SearchWarning> {
    let which = match placement {
        FallbackPlacement::Last => format!("the last {shown_pages} result(s)"),
        FallbackPlacement::Fused => format!("{shown_pages} result(s) from this project"),
    };
    (shown_pages > 0).then(|| SearchWarning {
        project_id: project_id.to_string(),
        message: format!(
            "too few pages hold every word of the query; {which} hold only some of its names and are scored by a separate phrase search"
        ),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FallbackPlacement {
    Last,
    Fused,
}

fn retry_search_delay() {
    sleep(Duration::from_millis(50));
}

fn is_retryable_search_open_error(error: &anyhow::Error) -> bool {
    if let Some(status) = backend_access_failure(error) {
        return matches!(
            status.state,
            BackendState::Transient | BackendState::Missing
        );
    }
    let message = error.to_string();
    message.contains("open qmd-rs store") || message.contains("qmd-rs store could not be opened")
}

fn backend_access_failure(error: &anyhow::Error) -> Option<BackendStatus> {
    error
        .downcast_ref::<BackendAccessError>()
        .map(|error| error.status().clone())
}

fn maybe_sleep_for_test(var: &str) {
    if let Ok(value) = env::var(var)
        && let Ok(ms) = value.parse::<u64>()
        && ms > 0
    {
        sleep(Duration::from_millis(ms));
    }
}

fn maybe_write_test_marker(var: &str) {
    if let Ok(path) = env::var(var) {
        let marker = PathBuf::from(path);
        if let Some(parent) = marker.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(marker, b"ready");
    }
}

fn maybe_remove_test_marker(var: &str) {
    if let Ok(path) = env::var(var) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{
        FusedResult, ProjectIndexLock, RelatedStorePaths, StoreFileRole, fuse_hybrid_results,
        hybrid_candidate_survives_final_gate, is_retryable_search_open_error,
        promote_qmd_rs_store_inner, query_anchor_terms,
    };
    use crate::search::adapter::{
        BackendAccessError, BackendOpenMode, BackendState, BackendStatus, Freshness, Score,
        SearchMode, SearchResult,
    };
    use crate::search_models::SearchThresholds;

    #[test]
    fn thresholds_unconfigured_guidance_names_calibration_command() {
        use super::readiness;

        let failure = readiness("thresholds_unconfigured");
        assert_eq!(failure.reason, "thresholds_unconfigured");
        assert!(
            failure.guidance.contains("eval calibrate --record"),
            "guidance must name the calibration command, got: {}",
            failure.guidance
        );

        // The stricter variant points at the same command.
        assert!(
            readiness("thresholds_incompatible")
                .guidance
                .contains("eval calibrate --record")
        );
    }

    fn rerank_report(
        project_id: &str,
        rerank_applied: bool,
        rerank_reason: Option<&str>,
    ) -> super::ProjectSearchReport {
        super::ProjectSearchReport {
            project_id: project_id.to_string(),
            requested_mode: "auto".to_string(),
            selected_mode: Some("hybrid".to_string()),
            mode_selection_reason: Some("enabled_profile".to_string()),
            fallback_reason: None,
            readiness_reason: None,
            rerank_requested: true,
            rerank_applied,
            rerank_reason: rerank_reason.map(str::to_string),
            runtime_backend_requested: None,
            runtime_backend_used: None,
            runtime_backend_fallback: None,
            runtime_failure_stage: None,
            runtime_error_kind: None,
            backend_status: None,
            result_count: 1,
            no_result: None,
            profile: None,
            thresholds_source: None,
        }
    }

    #[test]
    fn search_all_rerank_status_summarizes_without_mislabeling() {
        use super::SearchModeJson;
        use crate::cli::SearchModeArg;

        // All projects reranked -> top level true.
        let all = [
            rerank_report("a", true, None),
            rerank_report("b", true, None),
        ];
        let json = SearchModeJson::from_search_all_reports(SearchModeArg::Auto, true, &all);
        assert!(json.rerank_applied);
        assert_eq!(json.rerank_reason, None);

        // Mixed outcomes -> not a single boolean; explicitly partial, never
        // borrowing readiness vocabulary like `search_not_ready`.
        let mixed = [
            rerank_report("a", true, None),
            rerank_report("b", false, Some("reranker_model_unconfigured")),
        ];
        let json = SearchModeJson::from_search_all_reports(SearchModeArg::Auto, true, &mixed);
        assert!(!json.rerank_applied);
        assert_eq!(
            json.rerank_reason.as_deref(),
            Some("partial_across_projects")
        );

        // None reranked for the same reason -> surface the shared reason.
        let none = [
            rerank_report("a", false, Some("reranker_model_unconfigured")),
            rerank_report("b", false, Some("reranker_model_unconfigured")),
        ];
        let json = SearchModeJson::from_search_all_reports(SearchModeArg::Auto, true, &none);
        assert!(!json.rerank_applied);
        assert_eq!(
            json.rerank_reason.as_deref(),
            Some("reranker_model_unconfigured")
        );

        // Not requested -> no reason at all.
        let json = SearchModeJson::from_search_all_reports(SearchModeArg::Auto, false, &all);
        assert!(!json.rerank_applied);
        assert_eq!(json.rerank_reason, None);
    }

    #[test]
    fn reranker_is_not_a_search_readiness_requirement() {
        use super::{RuntimeSearchMode, required_model_ids};
        use crate::search_profile::SearchProfile;

        let profile = SearchProfile {
            llm_search_enabled: true,
            configured_at: String::new(),
            configured_by_version: String::new(),
            reason: None,
            profile: Some("balanced".to_string()),
            embedding_model: Some("embed-model".to_string()),
            query_expansion_model: Some("query-expansion-model".to_string()),
            // The shipped `balanced` profile has no reranker: rerank must remain
            // an optional enhancement, never a `model_missing` readiness gate.
            reranker_model: None,
            source: None,
            source_install_id: None,
        };

        // Hybrid readiness depends only on embedding + query-expansion models;
        // the absent reranker must not collapse the requirement set to `None`
        // (which `readiness_failure` maps to a hard `model_missing`).
        let hybrid = required_model_ids(&profile, RuntimeSearchMode::Hybrid)
            .expect("required models")
            .expect("hybrid models present");
        assert_eq!(
            hybrid,
            vec![
                "embed-model".to_string(),
                "query-expansion-model".to_string()
            ]
        );

        // Even a profile that *names* a reranker must not add it to the readiness
        // gate; reranker availability is reported by `rerank_status` instead.
        let with_reranker = SearchProfile {
            reranker_model: Some("reranker-model".to_string()),
            ..profile
        };
        let hybrid = required_model_ids(&with_reranker, RuntimeSearchMode::Hybrid)
            .expect("required models")
            .expect("hybrid models present");
        assert!(
            !hybrid.iter().any(|id| id == "reranker-model"),
            "reranker must never gate search readiness"
        );
    }

    #[test]
    fn rerank_status_reports_application_and_reason() {
        use super::{RuntimeSearchMode, rerank_status};

        // Not requested: not applied, no reason.
        assert_eq!(
            rerank_status(false, RuntimeSearchMode::Hybrid, Some("reranker")),
            (false, None)
        );

        // Requested in hybrid with a configured model: applied, no reason.
        assert_eq!(
            rerank_status(true, RuntimeSearchMode::Hybrid, Some("reranker")),
            (true, None)
        );

        // Requested in hybrid without a model: not applied, explicit reason
        // (Issue 0.4 — never a silent no-op).
        let (applied, reason) = rerank_status(true, RuntimeSearchMode::Hybrid, None);
        assert!(!applied);
        assert_eq!(reason.as_deref(), Some("reranker_model_unconfigured"));

        // Requested outside hybrid: not applied, names the requirement.
        let (applied, reason) = rerank_status(true, RuntimeSearchMode::Lexical, Some("reranker"));
        assert!(!applied);
        assert!(
            reason
                .as_deref()
                .is_some_and(|r| r.contains("rerank_requires_hybrid_mode"))
        );
    }

    #[test]
    fn auto_mode_degrades_to_lexical_when_not_ready() {
        use super::{ModeResolutionOutcome, RuntimeSearchMode, resolve_mode};
        use crate::cli::SearchModeArg;
        use crate::paths::Paths;
        use crate::search_profile::SearchProfile;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let paths = Paths {
            home: temp.path().to_path_buf(),
            cache_home: temp.path().join("cache"),
            data_home: temp.path().join("data"),
            managed_home: temp.path().join("managed"),
        };
        // Enabled profile, but the temp managed home has no model artifacts or
        // thresholds, so readiness fails. `auto` must degrade, not error.
        let profile = SearchProfile {
            llm_search_enabled: true,
            configured_at: String::new(),
            configured_by_version: String::new(),
            reason: None,
            profile: Some("balanced".to_string()),
            embedding_model: Some("embed-model".to_string()),
            query_expansion_model: Some("query-expansion-model".to_string()),
            reranker_model: None,
            source: None,
            source_install_id: None,
        };

        let outcome = resolve_mode(&paths, None, SearchModeArg::Auto, false, Some(profile))
            .expect("resolve auto mode");

        match outcome {
            ModeResolutionOutcome::Ready(resolution) => {
                assert_eq!(resolution.selected_mode, RuntimeSearchMode::Lexical);
                assert_eq!(resolution.reason, "auto_lexical_fallback");
                assert!(
                    resolution.fallback_reason.is_some(),
                    "degraded auto must record why semantic/hybrid was skipped"
                );
            }
            ModeResolutionOutcome::NotReady { failure, .. } => {
                panic!(
                    "auto must degrade to lexical, got NotReady: {}",
                    failure.reason
                )
            }
        }
    }

    #[test]
    fn project_index_lock_is_exclusive() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let first = ProjectIndexLock::acquire(temp.path()).expect("first lock");

        let error = ProjectIndexLock::acquire(temp.path()).expect_err("second lock");

        assert!(error.to_string().contains("qmd-rs.lock"));
        drop(first);
        ProjectIndexLock::acquire(temp.path()).expect("lock after drop");
    }

    #[test]
    fn project_index_lock_preserves_stale_lock_file() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        fs::write(temp.path().join("qmd-rs.lock"), "pid=999999").expect("stale lock");

        let lock = ProjectIndexLock::acquire(temp.path()).expect("lock with stale file");

        assert!(temp.path().join("qmd-rs.lock").exists());
        drop(lock);
        assert!(temp.path().join("qmd-rs.lock").exists());
    }

    #[test]
    fn failed_store_promotion_restores_all_old_live_files() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let live = temp.path().join("live/qmd-rs.sqlite");
        let staging = temp.path().join("staging/qmd-rs.sqlite");
        fs::create_dir_all(live.parent().expect("live parent")).expect("live dir");
        fs::create_dir_all(staging.parent().expect("staging parent")).expect("staging dir");
        fs::write(&live, "old sqlite").expect("old sqlite");
        fs::write(live.with_extension("llm-wiki.json"), "old metadata").expect("old metadata");
        fs::write(&staging, "new sqlite").expect("new sqlite");
        fs::write(staging.with_extension("llm-wiki.json"), "new metadata").expect("new metadata");

        let error =
            promote_qmd_rs_store_inner(&staging, &live, Some(1)).expect_err("simulated failure");

        assert!(error.to_string().contains("simulated"));
        assert_eq!(
            fs::read_to_string(&live).expect("live sqlite"),
            "old sqlite"
        );
        assert_eq!(
            fs::read_to_string(live.with_extension("llm-wiki.json")).expect("live metadata"),
            "old metadata"
        );
    }

    #[test]
    fn promote_pairs_temp_and_live_files_by_role() {
        let temp = RelatedStorePaths::new(Path::new("/tmp/temp/qmd-rs.sqlite"));
        let live = RelatedStorePaths::new(Path::new("/tmp/live/qmd-rs.sqlite"));

        assert_eq!(
            temp.path(StoreFileRole::Sqlite).file_name(),
            live.path(StoreFileRole::Sqlite).file_name()
        );
        assert_eq!(
            temp.path(StoreFileRole::Metadata).extension(),
            live.path(StoreFileRole::Metadata).extension()
        );
        assert_eq!(
            temp.path(StoreFileRole::Wal).extension(),
            live.path(StoreFileRole::Wal).extension()
        );
        assert_eq!(
            temp.path(StoreFileRole::Shm).extension(),
            live.path(StoreFileRole::Shm).extension()
        );
    }

    #[test]
    fn hybrid_final_gate_rejects_weak_lexical_only_candidates() {
        let thresholds = test_thresholds();
        let mut candidate = fused_result("wiki/evals/search.eval.md");
        candidate.lexical_rank = Some(0);
        candidate.lexical_score = Some(6.0);

        assert!(!hybrid_candidate_survives_final_gate(
            "GPU shader compiler roadmap",
            &thresholds,
            &candidate
        ));

        candidate.lexical_score = Some(12.0);
        assert!(hybrid_candidate_survives_final_gate(
            "what are the most cutting edge battery technologies",
            &thresholds,
            &candidate
        ));
    }

    #[test]
    fn hybrid_final_gate_keeps_high_confidence_semantic_candidates() {
        let thresholds = test_thresholds();
        let mut candidate = fused_result("wiki/specs/wiki-init-skill.spec.md");
        candidate.semantic_rank = Some(0);
        candidate.semantic_score = Some(0.62);

        assert!(hybrid_candidate_survives_final_gate(
            "which files prove the wiki init skill works",
            &thresholds,
            &candidate
        ));
    }

    #[test]
    fn hybrid_boosts_high_confidence_semantic_top_result() {
        let thresholds = test_thresholds();
        let lexical_results = vec![
            search_result("wiki/log.md", 6.0),
            search_result("wiki/plans/llm-wiki-binary.plan.md", 5.5),
        ];
        let semantic_results = vec![
            search_result("wiki/specs/wiki-init-skill.spec.md", 0.64),
            search_result("wiki/log.md", 0.60),
            search_result("wiki/plans/llm-wiki-binary.plan.md", 0.58),
        ];

        let results = fuse_hybrid_results(
            "which files prove the wiki init skill works",
            &thresholds,
            lexical_results,
            semantic_results,
            5,
        );

        assert_eq!(
            results.first().map(|result| result.path.as_path()),
            Some(Path::new("wiki/specs/wiki-init-skill.spec.md"))
        );
    }

    #[test]
    fn query_anchor_terms_keep_meaningful_path_terms() {
        assert_eq!(
            query_anchor_terms("which files prove the wiki init skill works"),
            vec!["which", "files", "prove", "wiki", "init", "skill", "works"]
        );
    }

    #[test]
    fn retry_predicate_accepts_typed_transient_backend_access() {
        let error: anyhow::Error =
            BackendAccessError::new(backend_status(BackendState::Transient)).into();

        assert!(is_retryable_search_open_error(&error));
    }

    #[test]
    fn retry_predicate_rejects_typed_permission_denied_backend_access() {
        let error: anyhow::Error =
            BackendAccessError::new(backend_status(BackendState::PermissionDenied)).into();

        assert!(!is_retryable_search_open_error(&error));
    }

    fn fused_result(path: &str) -> FusedResult {
        FusedResult {
            result: search_result(path, 0.0),
            score: 0.0,
            lexical_rank: None,
            lexical_score: None,
            semantic_rank: None,
            semantic_score: None,
        }
    }

    fn search_result(path: &str, score: f64) -> SearchResult {
        SearchResult {
            project_id: "fixture".to_string(),
            project_name: None,
            path: PathBuf::from(path),
            title: path.replace(['/', '.', '-'], " "),
            document_class: None,
            status: None,
            score: Score(score),
            snippet: None,
            match_span: None,
            backend: "test".to_string(),
            mode: SearchMode::Hybrid,
            freshness: Freshness::Fresh,
            lexical_rank: None,
            lexical_score: None,
            semantic_rank: None,
            semantic_score: None,
        }
    }

    fn backend_status(state: BackendState) -> BackendStatus {
        BackendStatus {
            store_path: PathBuf::from("/tmp/qmd-rs.sqlite"),
            state,
            open_mode: BackendOpenMode::ReadOnlyImmutable,
            indexed_files: 0,
            stale: false,
            message: Some("test backend access".to_string()),
        }
    }

    fn test_thresholds() -> SearchThresholds {
        SearchThresholds {
            schema_version: 1,
            updated_at: "2026-05-11T00:00:00Z".to_string(),
            project_id: None,
            profile: "balanced".to_string(),
            semantic_similarity_floor: 0.35,
            hybrid_pre_fusion_semantic_floor: 0.35,
            hybrid_final_semantic_floor: 0.39,
            hybrid_semantic_only_floor: 0.50,
            hybrid_strong_lexical_score_floor: 10.0,
            reranker_probability_floor: 0.50,
            lexical_exact_identifier_guard: "preserve_lexical_top_3".to_string(),
            qmd_rs_version: "0.3.2".to_string(),
            adapter_schema_version: 1,
            chunking_strategy: "qmd-rs-character-v1:3200:480".to_string(),
            embedding_model: "embeddinggemma-300m-q8_0".to_string(),
            embedding_artifact_sha256: "sha".to_string(),
            embedding_dimensions: 768,
        }
    }

    #[test]
    fn the_phrase_fallback_runs_in_lexical_search_and_not_in_hybrids_lexical_branch() {
        use super::{
            LexicalQuery, ProjectSearchInput, hybrid_lexical_branch_search, perform_project_search,
        };
        use crate::paths::Paths;
        use crate::registry::RegisteredProject;
        use crate::search::adapter::{IndexOptions, SearchBackend, SearchFilters};
        use crate::search::qmd_rs::QmdRsBackend;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/search-eval");
        let wiki = root.join("wiki");
        let temp = tempfile::TempDir::new().expect("tempdir");
        let store = temp.path().join("qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        let project = RegisteredProject {
            id: "fixture".to_string(),
            name: "fixture".to_string(),
            root,
            wiki_path: PathBuf::from("wiki"),
            registered_at: String::new(),
            last_indexed_at: None,
            last_indexed_wiki_max_mtime: None,
            indexed_file_count: 0,
            backend: "qmd-rs".to_string(),
            index_schema_version: 1,
        };
        let paths = Paths {
            home: temp.path().to_path_buf(),
            cache_home: temp.path().to_path_buf(),
            data_home: temp.path().to_path_buf(),
            managed_home: temp.path().to_path_buf(),
        };
        let query = "headroom-wrap-command headroom-passthrough-launcher";
        let filters = SearchFilters::default();
        let input = ProjectSearchInput {
            paths: &paths,
            backend: &backend,
            project: &project,
            store_path: &store,
            wiki_root: &wiki,
            query,
            filters: &filters,
            limit: 20,
        };
        let all_words = backend
            .search_project("fixture", &store, &wiki, query, &filters, 20)
            .expect("all words");
        assert!(all_words.len() < 20, "the query needs the fallback to run");
        let found = |results: &[SearchResult]| {
            results
                .iter()
                .map(|result| result.path.clone())
                .collect::<Vec<_>>()
        };

        let hybrid_branch = hybrid_lexical_branch_search(&input, query, 20).expect("hybrid");
        assert_eq!(found(&hybrid_branch.results), found(&all_words));
        assert_eq!(hybrid_branch.phrase_fallback_pages, 0);

        let lexical = perform_project_search(
            &backend,
            &project,
            &store,
            &wiki,
            query,
            &filters,
            20,
            LexicalQuery::PhraseFallback,
        )
        .expect("lexical");
        assert!(lexical.results.len() > all_words.len());
        assert_eq!(
            found(&lexical.results[..all_words.len()]),
            found(&all_words)
        );
        assert_eq!(
            lexical.phrase_fallback_pages,
            lexical.results.len() - all_words.len()
        );
    }
}
