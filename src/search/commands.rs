use std::collections::BTreeMap;
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

use crate::cli::{CliContext, IndexAllArgs, IndexArgs, OutputFormat, SearchAllArgs, SearchArgs};
use crate::paths::Paths;
use crate::registry::{self, ProjectRegistry, RegisteredProject};
use crate::search::adapter::{
    BackendState, BackendStatus, Freshness, Score, SearchBackend, SearchFilters, SearchResult,
};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;
use crate::search::sanitize::sanitize_fts_query;

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
    registry::record_index_success(&project.id, status.indexed_files, &project.wiki_root())?;
    context.diagnostic("registry metadata: recorded index success");
    println!(
        "Indexed project: {} ({} files)",
        project.id, status.indexed_files
    );
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
    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let store_path = paths.qmd_rs_store_path(&project.id);
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
    context.diagnostic("backend: qmd-rs fts");
    let backend = QmdRsBackend::new();
    let search = perform_project_search(
        &backend,
        &project,
        &store_path,
        &wiki_root,
        &args.query,
        &filters,
        args.limit,
    )?;
    context.diagnostic(format!(
        "index status: {}, freshness={}, indexed_files={}",
        backend_state_label(&search.status.state),
        freshness_label(freshness_for_status(&search.status)),
        search.status.indexed_files
    ));
    if let Some(message) = &search.status.message {
        context.diagnostic(format!("index message: {message}"));
    }
    let no_result = context.verbose.then(|| {
        explain_no_results(NoResultContext {
            backend: &backend,
            store_path: &store_path,
            wiki_root: &wiki_root,
            query: &args.query,
            filters: &filters,
            limit: args.limit,
            status: &search.status,
            results: &search.results,
        })
    });
    context.diagnostic(format!("results: {}", search.results.len()));
    if let Some(Some(explanation)) = no_result {
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

    match args.format {
        OutputFormat::Text => print_search_text(&search.warnings, &results),
        OutputFormat::Json => print_search_json(
            &args.query,
            Some((&project.id, &project.name)),
            warning,
            &[],
            &results,
        ),
    }
    Ok(())
}

pub fn search_all(args: &SearchAllArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: search-all");
    context.diagnostic(format!("query: {}", args.query));
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
        let store_path = paths.qmd_rs_store_path(&project.id);
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
        let per_project_limit = if args.limit == 0 {
            0
        } else {
            args.limit.max(20)
        };
        let search = perform_project_search(
            &backend,
            project,
            &store_path,
            &wiki_root,
            &args.query,
            &filters,
            per_project_limit,
        )?;
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
            explain_no_results(NoResultContext {
                backend: &backend,
                store_path: &store_path,
                wiki_root: &wiki_root,
                query: &args.query,
                filters: &filters,
                limit: per_project_limit,
                status: &search.status,
                results: &search.results,
            })
        });
        project_reports.push(ProjectSearchReport {
            project_id: project.id.clone(),
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
        OutputFormat::Json => print_search_json(&args.query, None, None, &warnings, &results),
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectSearchReport {
    project_id: String,
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
}

#[derive(Clone, Debug, Serialize)]
struct SearchWarningJson {
    project_id: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
struct SearchEnvelopeJson {
    query: String,
    project_id: Option<String>,
    project_name: Option<String>,
    warning: Option<String>,
    warnings: Vec<SearchWarningJson>,
    results: Vec<SearchResultJson>,
}

/// Public JSON contract from the D9 project-registry/search output plan.
fn print_search_json(
    query: &str,
    project: Option<(&str, &str)>,
    warning: Option<&str>,
    warnings: &[SearchWarning],
    results: &[SearchResult],
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
        })
        .collect::<Vec<_>>();
    let warnings = warnings
        .iter()
        .map(|warning| SearchWarningJson {
            project_id: warning.project_id.clone(),
            message: warning.message.clone(),
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&SearchEnvelopeJson {
            query: query.to_string(),
            project_id: project.map(|(id, _)| id.to_string()),
            project_name: project.map(|(_, name)| name.to_string()),
            warning: warning.map(ToString::to_string),
            warnings,
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

fn freshness_for_status(status: &BackendStatus) -> Freshness {
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
    backend: &'a QmdRsBackend,
    store_path: &'a Path,
    wiki_root: &'a Path,
    query: &'a str,
    filters: &'a SearchFilters,
    limit: usize,
    status: &'a BackendStatus,
    results: &'a [SearchResult],
}

fn explain_no_results(context: NoResultContext<'_>) -> Option<String> {
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
        let unfiltered_count = context
            .backend
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
    use std::path::Path;

    use super::{ProjectIndexLock, RelatedStorePaths, StoreFileRole, promote_qmd_rs_store_inner};

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
}
