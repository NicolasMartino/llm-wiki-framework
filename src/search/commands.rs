use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde_json::json;

use crate::cli::{IndexAllArgs, IndexArgs, OutputFormat, SearchAllArgs, SearchArgs};
use crate::paths::Paths;
use crate::registry::{ProjectRegistry, RegisteredProject};
use crate::search::adapter::{
    BackendState, Freshness, Score, SearchBackend, SearchFilters, SearchResult,
};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;

pub fn index(args: &IndexArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    let mut registry = ProjectRegistry::read(&registry_path)?;
    let project = select_project(&registry, args.project.as_deref())?;
    index_registered_project(&paths, &registry_path, &mut registry, &project, args.force)
}

pub fn index_all(args: &IndexAllArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    let mut registry = ProjectRegistry::read(&registry_path)?;
    if registry.projects.is_empty() {
        println!("No registered projects.");
        return Ok(());
    }

    let projects = registry.projects.clone();
    let total_projects = projects.len();
    let mut indexed_projects = 0usize;
    let mut failures = Vec::new();
    for project in projects {
        if !project.root.exists() {
            failures.push(format!("{}: root missing", project.id));
            continue;
        }
        if let Err(error) =
            index_registered_project(&paths, &registry_path, &mut registry, &project, args.force)
        {
            failures.push(format!("{}: {error}", project.id));
        } else {
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
    registry_path: &std::path::Path,
    registry: &mut ProjectRegistry,
    project: &RegisteredProject,
    force: bool,
) -> Result<()> {
    let store_path = paths.qmd_rs_store_path(&project.id);
    let project_index_dir = paths.project_index_dir(&project.id);
    fs::create_dir_all(&project_index_dir)
        .with_context(|| format!("create search index dir {}", project_index_dir.display()))?;
    let _lock = ProjectIndexLock::acquire(&project_index_dir)?;
    let temp_build = TempIndexBuild::new(&project_index_dir)?;

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

    promote_qmd_rs_store(&temp_build.store_path, &store_path)?;
    temp_build.cleanup()?;
    registry.record_index_success(&project.id, status.indexed_files, &project.wiki_root())?;
    registry.write_atomic(registry_path)?;
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
    let mut backups = Vec::new();
    for live in related_store_paths(live_store) {
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
        for (temp, live) in related_store_paths(temp_store)
            .into_iter()
            .zip(related_store_paths(live_store))
        {
            if temp.exists() {
                fs::rename(&temp, &live).with_context(|| {
                    format!(
                        "promote qmd-rs store file {} to {}",
                        temp.display(),
                        live.display()
                    )
                })?;
                promoted.push(live.clone());
                if fail_after_promotes.is_some_and(|limit| promoted.len() >= limit) {
                    bail!("simulated qmd-rs store promotion failure");
                }
            }
        }
        Ok(())
    })();

    if let Err(error) = promote_result {
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

    for (_, backup) in backups {
        if backup.exists() {
            fs::remove_file(&backup)
                .with_context(|| format!("remove qmd-rs backup {}", backup.display()))?;
        }
    }
    Ok(())
}

fn related_store_paths(store_path: &Path) -> Vec<PathBuf> {
    vec![
        store_path.to_path_buf(),
        store_path.with_extension("llm-wiki.json"),
        store_path.with_extension("sqlite-wal"),
        store_path.with_extension("sqlite-shm"),
    ]
}

fn backup_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("qmd-rs-store");
    path.with_file_name(format!("{file_name}.backup-{suffix}"))
}

pub fn search(args: &SearchArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let project = select_project(&registry, args.project.as_deref())?;
    let backend = QmdRsBackend::new();
    let store_path = paths.qmd_rs_store_path(&project.id);
    let wiki_root = project.wiki_root();
    let status = backend.status(&store_path, &wiki_root)?;

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
        BackendState::Ready | BackendState::Stale => {}
    }

    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let mut results =
        backend.search_project(&store_path, &wiki_root, &args.query, &filters, args.limit)?;
    for result in &mut results {
        result.project_id = project.id.clone();
        result.project_name = Some(project.name.clone());
    }
    let warning = if matches!(status.state, BackendState::Stale) {
        Some(format!(
            "search index stale for project {}; run `llm-wiki index --project {}`",
            project.id, project.id
        ))
    } else {
        None
    };

    match args.format {
        OutputFormat::Text => print_search_text(warning.as_deref(), &results),
        OutputFormat::Json => print_search_json(
            &args.query,
            Some((&project.id, &project.name)),
            warning.as_deref(),
            &results,
        ),
    }
    Ok(())
}

pub fn search_all(args: &SearchAllArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let projects = select_projects(&registry, &args.include, &args.exclude)?;
    let backend = QmdRsBackend::new();
    let filters = SearchFilters {
        document_class: args.document_class.clone(),
        status: args.status.clone(),
    };
    let mut warnings = Vec::new();
    let mut fused: BTreeMap<(String, String), FusedResult> = BTreeMap::new();

    for project in &projects {
        let store_path = paths.qmd_rs_store_path(&project.id);
        let wiki_root = project.wiki_root();
        let status = backend.status(&store_path, &wiki_root)?;
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
            BackendState::Ready | BackendState::Stale => {}
        }
        if matches!(status.state, BackendState::Stale) {
            warnings.push(format!(
                "search index stale for project {}; run `llm-wiki index --project {}`",
                project.id, project.id
            ));
        }

        let mut results =
            backend.search_project(&store_path, &wiki_root, &args.query, &filters, 20)?;
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
    let warning = if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    };

    match args.format {
        OutputFormat::Text => print_search_text(warning.as_deref(), &results),
        OutputFormat::Json => print_search_json(&args.query, None, warning.as_deref(), &results),
    }
    Ok(())
}

fn select_project(
    registry: &ProjectRegistry,
    requested_id: Option<&str>,
) -> Result<RegisteredProject> {
    if let Some(project_id) = requested_id {
        return registry
            .project_by_id(project_id)
            .cloned()
            .with_context(|| format!("project id {project_id} is not registered"));
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

fn print_search_text(warning: Option<&str>, results: &[SearchResult]) {
    if let Some(warning) = warning {
        println!("Warning: {warning}");
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

fn print_search_json(
    query: &str,
    project: Option<(&str, &str)>,
    warning: Option<&str>,
    results: &[SearchResult],
) {
    let results = results
        .iter()
        .map(|result| {
            json!({
                "project_id": result.project_id,
                "project_name": result.project_name,
                "path": result.path.to_string_lossy(),
                "title": result.title,
                "document_class": result.document_class,
                "status": result.status,
                "score": result.score.0,
                "snippet": result.snippet,
                "backend": result.backend,
                "mode": result.mode.to_string(),
                "freshness": freshness_label(result.freshness),
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "query": query,
            "project_id": project.map(|(id, _)| id),
            "project_name": project.map(|(_, name)| name),
            "warning": warning,
            "results": results,
        }))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{ProjectIndexLock, promote_qmd_rs_store_inner};

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
}
