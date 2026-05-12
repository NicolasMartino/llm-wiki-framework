use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::thread::sleep;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::cli::{
    CliContext, IndexAllArgs, IndexArgs, OutputFormat, SearchAllArgs, SearchArgs, SearchModeArg,
};
use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::registry::{self, ProjectRegistry, RegisteredProject};
use crate::search::adapter::{
    BackendState, BackendStatus, Freshness, Score, SearchBackend, SearchFilters, SearchMode,
    SearchResult,
};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;
use crate::search::sanitize::sanitize_fts_query;
use crate::search::semantic::{
    SemanticIndexMetadata, SemanticSearchContext, SemanticVectorIndex, embed_query,
    select_thresholds_for_index,
};
use crate::search_models::{
    ModelArtifactRecord, ModelArtifacts, SearchThresholdStore, SearchThresholds, model_by_id,
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
    update_semantic_index_metadata(paths, project, context)?;
    registry::record_index_success(&project.id, status.indexed_files, &project.wiki_root())?;
    context.diagnostic("registry metadata: recorded index success");
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
    let metadata =
        SemanticIndexMetadata::build(&project.id, &project.wiki_root(), &profile, &artifacts)?;
    context.diagnostic(format!(
        "semantic index metadata: chunks={}, path={}",
        metadata.chunks.len(),
        metadata_path.display()
    ));
    metadata.write_atomic(&metadata_path)?;
    let Some(threshold_store) = SearchThresholdStore::read(&paths.search_thresholds())? else {
        context.diagnostic("semantic vector index: skipped, thresholds unconfigured");
        return Ok(());
    };
    let Some(embedding_artifact) = embedding_artifact_for_profile(&artifacts, &profile) else {
        bail!("semantic vector indexing requires a verified embedding model artifact");
    };
    let Some(_thresholds) =
        select_thresholds_for_index(&threshold_store, &project.id, &metadata, embedding_artifact)
    else {
        context
            .diagnostic("semantic vector index: skipped, no scoped thresholds match index inputs");
        return Ok(());
    };
    let vector_index =
        SemanticVectorIndex::build(&metadata, &project.wiki_root(), embedding_artifact)?;
    context.diagnostic(format!(
        "semantic vector index: vectors={}, path={}",
        vector_index.vectors.len(),
        vector_path.display()
    ));
    vector_index.write_atomic(&vector_path)?;
    Ok(())
}

#[derive(Debug)]
struct ProjectIndexLock {
    path: PathBuf,
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
                "project index is already locked: {}. If no llm-wiki index process is running, remove this stale lock file and retry.",
                path.display()
            )
        })?;
        file.set_len(0)
            .with_context(|| format!("truncate project index lock {}", path.display()))?;
        writeln!(file, "pid={}", process::id())
            .with_context(|| format!("write project index lock {}", path.display()))?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for ProjectIndexLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
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
    let mut backups = Vec::new();
    for role in StoreFileRole::ALL {
        let live = live_files.path(role).to_path_buf();
        if live.exists() {
            let backup = backup_path(&live, &suffix);
            fs::rename(&live, &backup).with_context(|| {
                format!(
                    "move existing qmd-rs store file {} to {}",
                    live.display(),
                    backup.display()
                )
            })?;
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
    let resolution = match resolve_project_mode(
        &paths,
        &project,
        args.mode,
        args.allow_lexical_fallback,
        args.rerank,
    )? {
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
    let search = perform_resolved_project_search(&search_input, &resolution, args.rerank, context)?;
    context.diagnostic(format!(
        "index status: {}, freshness={}, indexed_files={}",
        backend_state_label(&search.status.state),
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

    match args.format {
        OutputFormat::Text => print_search_text(&search.warnings, &results),
        OutputFormat::Json => print_search_json(
            &args.query,
            Some((&project.id, &project.name)),
            warning,
            &[],
            &[],
            &results,
            mode_metadata,
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
    let resolution =
        match resolve_global_mode(&paths, args.mode, args.allow_lexical_fallback, args.rerank)? {
            ModeResolutionOutcome::Ready(resolution) => resolution,
            ModeResolutionOutcome::NotReady { failure, profile } => {
                return handle_readiness_failure(
                    &args.query,
                    None,
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
    let mut project_reports = Vec::new();

    for project in &projects {
        ensure_project_root_exists(project)?;
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
        let search =
            perform_resolved_project_search(&search_input, &resolution, args.rerank, context)?;
        context.diagnostic(format!(
            "index status {}: {}, freshness={}, indexed_files={}",
            project.id,
            backend_state_label(&search.status.state),
            freshness_label(freshness_for_status(&search.status)),
            search.status.indexed_files
        ));
        if let Some(message) = &search.status.message {
            context.diagnostic(format!("index message {}: {message}", project.id));
        }
        let no_result = context.verbose.then(|| {
            explain_no_results_for_mode(
                &backend,
                &resolution,
                NoResultContext {
                    store_path: &store_path,
                    wiki_root: &wiki_root,
                    query: &args.query,
                    filters: &filters,
                    limit: per_project_limit,
                    status: &search.status,
                    results: &search.results,
                },
            )
        });
        project_reports.push(ProjectSearchReport {
            project_id: project.id.clone(),
            requested_mode: resolution.requested_mode.label().to_string(),
            selected_mode: Some(resolution.selected_mode.label().to_string()),
            mode_selection_reason: Some(resolution.reason.clone()),
            fallback_reason: resolution.fallback_reason.clone(),
            readiness_reason: None,
            result_count: search.results.len(),
            no_result: no_result.flatten(),
        });
        warnings.extend(search.warnings);
        let mut results = search.results;
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
            let mut mode_metadata = SearchModeJson::from_resolution(&resolution, args.rerank);
            if results.is_empty() {
                mode_metadata.zero_result_reason =
                    Some("no selected project returned results".to_string());
            }
            print_search_json(
                &args.query,
                None,
                None,
                &warnings,
                &project_reports,
                &results,
                mode_metadata,
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
    rerank: bool,
) -> Result<ModeResolutionOutcome> {
    ensure_base_install(paths)?;
    let profile = project_search_profile(paths, project)?;
    resolve_mode(
        paths,
        Some(project),
        requested_mode,
        allow_lexical_fallback,
        rerank,
        profile,
    )
}

fn resolve_global_mode(
    paths: &Paths,
    requested_mode: SearchModeArg,
    allow_lexical_fallback: bool,
    rerank: bool,
) -> Result<ModeResolutionOutcome> {
    ensure_base_install(paths)?;
    let profile = SearchConfig::read(&paths.search_config())?.map(|config| config.global_search);
    resolve_mode(
        paths,
        None,
        requested_mode,
        allow_lexical_fallback,
        rerank,
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

fn resolve_mode(
    paths: &Paths,
    project: Option<&RegisteredProject>,
    requested_mode: SearchModeArg,
    allow_lexical_fallback: bool,
    rerank: bool,
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
            readiness_failure(paths, &profile, RuntimeSearchMode::Hybrid, rerank, project)?
        {
            return Ok(ModeResolutionOutcome::NotReady {
                failure,
                profile: Some(profile),
            });
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
    if let Some(failure) = readiness_failure(paths, &profile, selected_mode, rerank, project)? {
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
    rerank: bool,
    project: Option<&RegisteredProject>,
) -> Result<Option<ReadinessFailure>> {
    let Some(model_ids) = required_model_ids(profile, selected_mode, rerank)? else {
        return Ok(Some(readiness("model_missing")));
    };
    let Some(artifacts) = ModelArtifacts::read(&paths.model_artifacts())? else {
        return Ok(Some(readiness("model_missing")));
    };
    for model_id in model_ids {
        let Some(model) = model_by_id(&model_id) else {
            return Ok(Some(readiness("model_missing")));
        };
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
    let Some(threshold_store) = SearchThresholdStore::read(&paths.search_thresholds())? else {
        return Ok(Some(readiness("thresholds_unconfigured")));
    };
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
    let Some(thresholds) =
        select_thresholds_for_index(&threshold_store, &project.id, &metadata, embedding_artifact)
    else {
        return Ok(Some(readiness("thresholds_incompatible")));
    };
    if !metadata.is_fresh(&project.wiki_root())? {
        return Ok(Some(readiness("semantic_index_stale")));
    }
    let vector_path = paths.semantic_vector_index(&project.id);
    let Some(vector_index) = SemanticVectorIndex::read(&vector_path)? else {
        return Ok(Some(readiness("semantic_index_missing")));
    };
    if !vector_index.is_compatible(&metadata, &thresholds) {
        return Ok(Some(readiness("semantic_index_stale")));
    }
    Ok(None)
}

fn required_model_ids(
    profile: &SearchProfile,
    selected_mode: RuntimeSearchMode,
    rerank: bool,
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
    if rerank && selected_mode == RuntimeSearchMode::Hybrid {
        let Some(reranker_model) = profile.reranker_model.clone() else {
            return Ok(None);
        };
        model_ids.push(reranker_model);
    }
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
        "thresholds_unconfigured" => {
            "record calibrated semantic/hybrid thresholds before running LLM search".to_string()
        }
        "thresholds_incompatible" => {
            "record calibrated thresholds for the current semantic index inputs".to_string()
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
            &[],
            &[],
            &[],
            SearchModeJson::readiness_failure(
                requested_mode,
                &failure.reason,
                profile,
                rerank_requested,
            ),
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectSearchReport {
    project_id: String,
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    result_count: usize,
    no_result: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct SearchWarning {
    project_id: String,
    message: String,
}

#[derive(Debug)]
struct SearchExecution {
    results: Vec<SearchResult>,
    warnings: Vec<SearchWarning>,
    status: BackendStatus,
}

enum SearchAttempt {
    Success(SearchExecution),
    Missing,
    ForceReindex,
    RetryableUnavailable,
}

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
    query_expansion_model: Option<String>,
    reranker_model: Option<String>,
    rerank_requested: bool,
}

impl SearchModeJson {
    fn from_resolution(resolution: &ModeResolution, rerank_requested: bool) -> Self {
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
            query_expansion_model: resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.query_expansion_model.clone()),
            reranker_model: resolution
                .profile
                .as_ref()
                .and_then(|profile| profile.reranker_model.clone()),
            rerank_requested,
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
            query_expansion_model: profile
                .and_then(|profile| profile.query_expansion_model.clone()),
            reranker_model: profile.and_then(|profile| profile.reranker_model.clone()),
            rerank_requested,
        }
    }
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
                println!("Warning: ...{} more stale projects", many.len() - 3);
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
struct ProjectSearchReportJson {
    project_id: String,
    requested_mode: String,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    result_count: usize,
    zero_result_reason: Option<String>,
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
    query_expansion_model: Option<String>,
    reranker_model: Option<String>,
    rerank_requested: bool,
    warning: Option<String>,
    warnings: Vec<SearchWarningJson>,
    projects: Vec<ProjectSearchReportJson>,
    results: Vec<SearchResultJson>,
}

/// Public JSON contract from the D9 project-registry/search output plan.
fn print_search_json(
    query: &str,
    project: Option<(&str, &str)>,
    warning: Option<&str>,
    warnings: &[SearchWarning],
    project_reports: &[ProjectSearchReport],
    results: &[SearchResult],
    mode: SearchModeJson,
) {
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
            result_count: report.result_count,
            zero_result_reason: report.no_result.clone(),
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
            query_expansion_model: mode.query_expansion_model,
            reranker_model: mode.reranker_model,
            rerank_requested: mode.rerank_requested,
            warning: warning.map(ToString::to_string),
            warnings,
            projects,
            results,
        })
        .expect("serialize search json")
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
        BackendState::Missing | BackendState::Corrupt | BackendState::SchemaMismatch => {
            Freshness::Unknown
        }
    }
}

fn backend_state_label(state: &BackendState) -> &'static str {
    match state {
        BackendState::Ready => "ready",
        BackendState::Missing => "missing",
        BackendState::Stale => "stale",
        BackendState::Corrupt => "corrupt",
        BackendState::SchemaMismatch => "schema-mismatch",
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
            BackendState::Missing | BackendState::Corrupt | BackendState::SchemaMismatch => {
                "unusable"
            }
        };
        return Some(format!("{state} index has zero indexed files"));
    }

    if filters_active(context.filters) {
        let unfiltered_count = backend
            .search_project(
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
    embedding_artifact: ModelArtifactRecord,
    artifacts: ModelArtifacts,
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
    let query_embedding = embed_query(
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
            query_embedding: &query_embedding,
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
    let expanded = expand_hybrid_queries(input.query, profile, &state.artifacts)?;
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
        let search = perform_project_search(
            input.backend,
            input.project,
            input.store_path,
            input.wiki_root,
            lexical_query,
            input.filters,
            per_branch_limit,
        )?;
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
    for semantic_query in &expanded.semantic {
        let query_embedding = embed_query(
            semantic_query,
            &state.embedding_artifact,
            state.metadata.embedding_dimensions,
        )?;
        semantic_results.extend(state.vectors.search(
            &state.metadata,
            SemanticSearchContext {
                project_id: &input.project.id,
                project_name: Some(&input.project.name),
                wiki_root: input.wiki_root,
                query_embedding: &query_embedding,
                filters: input.filters,
                limit: per_branch_limit,
                floor: state.thresholds.hybrid_pre_fusion_semantic_floor,
                freshness: freshness_for_status(&status),
                mode: SearchMode::Semantic,
            },
        )?);
    }
    let semantic_results = dedupe_by_path_preserving_rank(semantic_results);
    let mut results = fuse_hybrid_results(
        input.query,
        &state.thresholds,
        lexical_results,
        semantic_results,
        input.limit,
    );
    results = maybe_rerank_results(
        input.query,
        results,
        profile,
        &state.artifacts,
        &state.thresholds,
        input.wiki_root,
        rerank_requested,
    )?;

    Ok(SearchExecution {
        results,
        warnings,
        status,
    })
}

fn semantic_base_status(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
) -> Result<BackendStatus> {
    let status = backend.status(store_path, wiki_root)?;
    match status.state {
        BackendState::Missing => bail!(
            "search index missing for project {}; run `llm-wiki index --project {}`",
            project.id,
            project.id
        ),
        BackendState::Corrupt | BackendState::SchemaMismatch => bail!(
            "search index unusable for project {}; run `llm-wiki index --project {} --force`",
            project.id,
            project.id
        ),
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
    let threshold_store = SearchThresholdStore::read(&paths.search_thresholds())?
        .context("semantic search requires calibrated thresholds")?;
    let embedding_artifact = embedding_artifact_for_profile(&artifacts, profile)
        .cloned()
        .context("semantic search requires a verified embedding model artifact")?;
    let metadata_path = paths.semantic_index_metadata(&project.id);
    let metadata = SemanticIndexMetadata::read(&metadata_path)?
        .with_context(|| format!("semantic index missing: {}", metadata_path.display()))?;
    let thresholds = select_thresholds_for_index(
        &threshold_store,
        &project.id,
        &metadata,
        &embedding_artifact,
    )
    .cloned()
    .context("semantic thresholds do not match the current index inputs or project scope")?;
    if !metadata.is_fresh(wiki_root)? {
        bail!(
            "semantic index stale for project {}; run `llm-wiki index --project {} --force`",
            project.id,
            project.id
        );
    }
    let vector_path = paths.semantic_vector_index(&project.id);
    let vectors = SemanticVectorIndex::read(&vector_path)?
        .with_context(|| format!("semantic vector index missing: {}", vector_path.display()))?;
    if !vectors.is_compatible(&metadata, &thresholds) {
        bail!(
            "semantic vector index stale for project {}; run `llm-wiki index --project {} --force`",
            project.id,
            project.id
        );
    }

    Ok(SemanticRuntimeState {
        metadata,
        vectors,
        thresholds,
        embedding_artifact,
        artifacts,
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

pub(crate) struct HybridQuerySet {
    pub(crate) lexical: Vec<String>,
    pub(crate) semantic: Vec<String>,
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
        });
    }

    let Some(model_id) = profile.query_expansion_model.as_deref() else {
        bail!("hybrid search requires a query expansion model");
    };
    let artifact = artifact_for_model(artifacts, model_id)
        .context("hybrid search requires a verified query expansion model artifact")?;
    let engine = qmd::GenerationEngine::new(&artifact.path)?;
    let expanded = engine.expand_query(query, true)?;
    let mut lexical = vec![query.to_string()];
    let mut semantic = vec![query.to_string()];
    for item in expanded {
        match item.query_type {
            qmd::QueryType::Lex => lexical.push(item.text),
            qmd::QueryType::Vec | qmd::QueryType::Hyde => semantic.push(item.text),
        }
    }
    dedupe_strings_preserving_order(&mut lexical);
    dedupe_strings_preserving_order(&mut semantic);
    Ok(HybridQuerySet { lexical, semantic })
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
    let mut results = results
        .into_iter()
        .take(limit)
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
        pin_lexical_prefix(&mut results, &lexical_results, 3, limit);
    }
    results
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

pub(crate) fn maybe_rerank_results(
    query: &str,
    results: Vec<SearchResult>,
    profile: &SearchProfile,
    artifacts: &ModelArtifacts,
    thresholds: &SearchThresholds,
    wiki_root: &Path,
    rerank_requested: bool,
) -> Result<Vec<SearchResult>> {
    if !rerank_requested {
        return Ok(results);
    }
    let Some(model_id) = profile.reranker_model.as_deref() else {
        bail!("rerank requested but selected profile has no reranker model");
    };
    let artifact = artifact_for_model(artifacts, model_id)
        .context("rerank requested but reranker artifact is missing")?;
    if env::var("LLM_WIKI_TEST_RERANK")
        .ok()
        .is_some_and(|value| value == "deterministic")
    {
        return Ok(deterministic_rerank_results(results));
    }
    let project_root = wiki_root.parent().unwrap_or(wiki_root);
    let documents = results
        .iter()
        .map(|result| {
            let text = fs::read_to_string(project_root.join(&result.path))
                .with_context(|| format!("read rerank input {}", result.path.display()))?;
            Ok(qmd::RerankDocument {
                file: result.path.to_string_lossy().to_string(),
                text,
                title: Some(result.title.clone()),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut engine = qmd::RerankEngine::new(&artifact.path)?;
    let reranked = engine.rerank(query, &documents)?;
    let mut results_by_rank = Vec::new();
    for item in reranked.results {
        if f64::from(item.score) < thresholds.reranker_probability_floor {
            continue;
        }
        let Some(original) = results.get(item.index) else {
            continue;
        };
        let mut result = original.clone();
        result.score = Score(f64::from(item.score));
        result.backend = "qmd-rs-rerank".to_string();
        results_by_rank.push(result);
    }
    Ok(results_by_rank)
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

fn perform_project_search(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
    query: &str,
    filters: &SearchFilters,
    limit: usize,
) -> Result<SearchExecution> {
    for attempt in 0..2 {
        match search_attempt(
            backend, project, store_path, wiki_root, query, filters, limit,
        )? {
            SearchAttempt::Success(search) => return Ok(search),
            SearchAttempt::Missing if attempt == 0 => retry_search_delay(),
            SearchAttempt::RetryableUnavailable if attempt == 0 => retry_search_delay(),
            SearchAttempt::Missing => {
                bail!(
                    "search index missing for project {}; run `llm-wiki index --project {}`",
                    project.id,
                    project.id
                );
            }
            SearchAttempt::ForceReindex => {
                bail!(
                    "search index unusable for project {}; run `llm-wiki index --project {} --force`",
                    project.id,
                    project.id
                );
            }
            SearchAttempt::RetryableUnavailable => {
                bail!(
                    "search index unavailable for project {}; run `llm-wiki index --project {} --force`",
                    project.id,
                    project.id
                );
            }
        }
    }
    unreachable!("search retry loop returns or bails")
}

fn search_attempt(
    backend: &QmdRsBackend,
    project: &RegisteredProject,
    store_path: &Path,
    wiki_root: &Path,
    query: &str,
    filters: &SearchFilters,
    limit: usize,
) -> Result<SearchAttempt> {
    let status = backend.status(store_path, wiki_root)?;
    match status.state {
        BackendState::Missing => return Ok(SearchAttempt::Missing),
        BackendState::Corrupt | BackendState::SchemaMismatch => {
            return Ok(SearchAttempt::ForceReindex);
        }
        BackendState::Ready | BackendState::Stale => {}
    }
    maybe_sleep_for_test("LLM_WIKI_TEST_SEARCH_AFTER_STATUS_SLEEP_MS");
    let results = match backend.search_project(store_path, wiki_root, query, filters, limit) {
        Ok(results) => results,
        Err(error) if is_retryable_search_open_error(&error) => {
            return Ok(SearchAttempt::RetryableUnavailable);
        }
        Err(error) => return Err(error),
    };
    Ok(SearchAttempt::Success(SearchExecution {
        results,
        warnings: stale_warning(project, &status).into_iter().collect(),
        status,
    }))
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

fn retry_search_delay() {
    sleep(Duration::from_millis(50));
}

fn is_retryable_search_open_error(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("open qmd-rs store") || message.contains("qmd-rs store could not be opened")
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
        hybrid_candidate_survives_final_gate, promote_qmd_rs_store_inner, query_anchor_terms,
    };
    use crate::search::adapter::{Freshness, Score, SearchMode, SearchResult};
    use crate::search_models::SearchThresholds;

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
    fn project_index_lock_ignores_stale_lock_file() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        fs::write(temp.path().join("qmd-rs.lock"), "pid=999999").expect("stale lock");

        let lock = ProjectIndexLock::acquire(temp.path()).expect("lock with stale file");

        assert!(temp.path().join("qmd-rs.lock").exists());
        drop(lock);
        assert!(!temp.path().join("qmd-rs.lock").exists());
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
}
