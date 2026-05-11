use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cli::{CliContext, ForgetArgs, OutputFormat, ProjectsArgs, RegisterArgs};
use crate::paths::Paths;
use crate::search::adapter::{BackendState, SearchBackend};
use crate::search::qmd_rs::QmdRsBackend;

const REGISTRY_VERSION: u32 = 1;
const BACKEND_NAME: &str = "qmd-rs";
const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectRegistry {
    pub version: u32,
    pub projects: Vec<RegisteredProject>,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            projects: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegisteredProject {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub wiki_path: PathBuf,
    pub registered_at: String,
    pub last_indexed_at: Option<String>,
    pub last_indexed_wiki_max_mtime: Option<String>,
    pub indexed_file_count: usize,
    pub backend: String,
    pub index_schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisterOutcome {
    Created(String),
    Updated(String),
    Unchanged(String),
}

pub fn register(args: &RegisterArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: register");
    context.diagnostic(format!(
        "requested path: {}",
        args.path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<current/update>".to_string())
    ));
    context.diagnostic(format!(
        "requested project id: {}",
        args.id.as_deref().unwrap_or("<auto>")
    ));
    context.diagnostic(format!(
        "requested update id: {}",
        args.update.as_deref().unwrap_or("<none>")
    ));
    let outcome = register_project_with_context(
        args.path.as_deref(),
        args.name.clone(),
        args.id.clone(),
        args.update.clone(),
        Some(context),
    )?;
    context.diagnostic(format!("registry outcome: {}", outcome_id(&outcome)));

    match outcome {
        RegisterOutcome::Created(id) => println!("Registered project: {id}"),
        RegisterOutcome::Updated(id) => println!("Updated project: {id}"),
        RegisterOutcome::Unchanged(id) => println!("Project already registered: {id}"),
    }
    Ok(())
}

pub fn register_project(
    path: Option<&Path>,
    name: Option<String>,
    id: Option<String>,
    update: Option<String>,
) -> Result<RegisterOutcome> {
    register_project_with_context(path, name, id, update, None)
}

pub fn register_project_with_context(
    path: Option<&Path>,
    name: Option<String>,
    id: Option<String>,
    update: Option<String>,
    context: Option<&CliContext>,
) -> Result<RegisterOutcome> {
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    if let Some(context) = context {
        context.diagnostic(format!("registry: {}", registry_path.display()));
    }
    let _lock = RegistryMutationLock::acquire(&registry_path)?;
    let mut registry = ProjectRegistry::read(&registry_path)?;
    let root = resolve_register_root(&registry, path, update.as_deref())?;
    if let Some(context) = context {
        context.diagnostic(format!("canonical root: {}", root.display()));
        context.diagnostic("validation: ok");
    }
    let outcome = registry.register(RegisterRequest {
        root,
        name,
        id,
        update,
    })?;
    maybe_sleep_for_test("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS");
    registry.write_atomic(&registry_path)?;
    Ok(outcome)
}

pub fn outcome_id(outcome: &RegisterOutcome) -> &str {
    match outcome {
        RegisterOutcome::Created(id)
        | RegisterOutcome::Updated(id)
        | RegisterOutcome::Unchanged(id) => id,
    }
}

pub fn forget(args: &ForgetArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: forget");
    context.diagnostic(format!("requested project id: {}", args.project_id));
    context.diagnostic(format!("delete cache: {}", args.delete_cache));
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    let _lock = RegistryMutationLock::acquire(&registry_path)?;
    let mut registry = ProjectRegistry::read(&registry_path)?;
    let removed = registry.remove(&args.project_id)?;
    context.diagnostic(format!("canonical root: {}", removed.root.display()));
    maybe_sleep_for_test("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS");
    registry.write_atomic(&registry_path)?;

    if args.delete_cache {
        let cache_dir = paths.project_index_dir(&removed.id);
        context.diagnostic(format!("cache dir: {}", cache_dir.display()));
        if cache_dir.exists() {
            let canonical_cache_home = fs::canonicalize(paths.cache_home()).with_context(|| {
                format!("canonicalize cache home {}", paths.cache_home().display())
            })?;
            let canonical_cache_dir = fs::canonicalize(&cache_dir)
                .with_context(|| format!("canonicalize search cache {}", cache_dir.display()))?;
            if !canonical_cache_dir.starts_with(&canonical_cache_home) {
                context.diagnostic("cache deletion: refused outside cache home");
                bail!(
                    "refusing to delete cache outside {}: {}",
                    canonical_cache_home.display(),
                    canonical_cache_dir.display()
                );
            }
            context.diagnostic("cache deletion: remove cache dir");
            fs::remove_dir_all(&cache_dir)
                .with_context(|| format!("remove search cache {}", cache_dir.display()))?;
        } else {
            context.diagnostic("cache deletion: cache dir missing");
        }
    } else {
        context.diagnostic("cache deletion: skipped");
    }

    println!("Forgot project: {}", removed.id);
    Ok(())
}

pub fn projects(args: &ProjectsArgs, context: &CliContext) -> Result<()> {
    context.diagnostic("command: projects");
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    context.diagnostic(format!("format: {}", output_format_label(args.format)));
    let registry = ProjectRegistry::read(&registry_path)?;
    context.diagnostic(format!("registered projects: {}", registry.projects.len()));
    let view = ProjectsView::from_registry(&registry, &paths)?;
    for project in &view.projects {
        context.diagnostic(format!(
            "project status: {} root={} index={} freshness={}",
            project.id, project.root_status, project.index_status, project.freshness
        ));
    }
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&view)?),
        OutputFormat::Text => print_projects_text(&view),
    }
    Ok(())
}

fn output_format_label(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Text => "text",
        OutputFormat::Json => "json",
    }
}

/// Output is `\t`-separated; the first row is a header. The field set and
/// order are part of the public CLI contract.
fn print_projects_text(view: &ProjectsView) {
    if view.projects.is_empty() {
        println!("No registered projects.");
        return;
    }

    println!("id\tname\troot\troot_status\tindex_status\tfreshness\tbackend\tcache_size");
    for project in &view.projects {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{} bytes",
            project.id,
            project.name,
            project.root,
            project.root_status,
            project.index_status,
            project.freshness,
            project.backend,
            project.cache_size_bytes
        );
    }
}

#[derive(Clone, Debug, Serialize)]
struct ProjectsView {
    version: u32,
    projects: Vec<ProjectStatusView>,
}

impl ProjectsView {
    fn from_registry(registry: &ProjectRegistry, paths: &Paths) -> Result<Self> {
        let projects = registry
            .projects
            .iter()
            .map(|project| ProjectStatusView::from_project(project, paths))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            version: registry.version,
            projects,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct ProjectStatusView {
    id: String,
    name: String,
    root: String,
    wiki_path: String,
    root_status: &'static str,
    index_status: &'static str,
    freshness: &'static str,
    backend: String,
    index_schema_version: u32,
    cache_size_bytes: u64,
    indexed_file_count: usize,
    last_indexed_at: Option<String>,
    last_indexed_wiki_max_mtime: Option<String>,
    status_message: Option<String>,
}

impl ProjectStatusView {
    fn from_project(project: &RegisteredProject, paths: &Paths) -> Result<Self> {
        let store_path = paths.qmd_rs_store_path(&project.id);
        let root_exists = project.root.exists();
        let (index_status, freshness, status_message) = if !store_path.exists() {
            (
                "index-missing",
                "missing",
                (!root_exists).then(|| "project root is missing".to_string()),
            )
        } else if !root_exists {
            (
                "index-present",
                "unknown",
                Some("project root is missing".to_string()),
            )
        } else {
            backend_status_labels(&store_path, &project.wiki_root())
        };

        Ok(Self {
            id: project.id.clone(),
            name: project.name.clone(),
            root: project.root.to_string_lossy().to_string(),
            wiki_path: project.wiki_path.to_string_lossy().to_string(),
            root_status: if root_exists {
                "root-ok"
            } else {
                "root-missing"
            },
            index_status,
            freshness,
            backend: project.backend.clone(),
            index_schema_version: project.index_schema_version,
            cache_size_bytes: dir_size(&paths.project_index_dir(&project.id))?,
            indexed_file_count: project.indexed_file_count,
            last_indexed_at: project.last_indexed_at.clone(),
            last_indexed_wiki_max_mtime: project.last_indexed_wiki_max_mtime.clone(),
            status_message,
        })
    }
}

fn backend_status_labels(
    store_path: &Path,
    wiki_root: &Path,
) -> (&'static str, &'static str, Option<String>) {
    let backend = QmdRsBackend::new();
    match backend.status(store_path, wiki_root) {
        Ok(status) => {
            let (index_status, freshness) = match status.state {
                BackendState::Ready => ("index-present", "fresh"),
                BackendState::Stale => ("index-present", "stale"),
                BackendState::Missing => ("index-missing", "missing"),
                BackendState::Corrupt | BackendState::SchemaMismatch => {
                    ("index-unusable", "unknown")
                }
            };
            (index_status, freshness, status.message)
        }
        Err(error) => ("index-unknown", "unknown", Some(error.to_string())),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RegisterRequest {
    root: PathBuf,
    name: Option<String>,
    id: Option<String>,
    update: Option<String>,
}

impl ProjectRegistry {
    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read project registry {}", path.display()))?;
        let registry: Self = serde_json::from_str(&raw)
            .with_context(|| format!("parse project registry {}", path.display()))?;
        if registry.version != REGISTRY_VERSION {
            bail!(
                "unsupported project registry version {} in {}",
                registry.version,
                path.display()
            );
        }
        for project in &registry.projects {
            validate_project_id(&project.id)
                .with_context(|| format!("invalid project id in {}", path.display()))?;
            validate_registered_root(&project.root)
                .with_context(|| format!("invalid project root in {}", path.display()))?;
            validate_registered_wiki_path(&project.wiki_path)
                .with_context(|| format!("invalid wiki path in {}", path.display()))?;
        }
        Ok(registry)
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create registry dir {}", parent.display()))?;
        }
        let tmp = unique_temp_path(path, "json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("write registry temp {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| {
            format!(
                "replace project registry {} with {}",
                path.display(),
                tmp.display()
            )
        })?;
        Ok(())
    }

    pub fn project_by_id(&self, project_id: &str) -> Option<&RegisteredProject> {
        self.projects
            .iter()
            .find(|project| project.id == project_id)
    }

    pub fn project_by_root(&self, root: &Path) -> Option<&RegisteredProject> {
        self.project_index_by_root(root)
            .map(|index| &self.projects[index])
    }

    pub fn record_index_success(
        &mut self,
        project_id: &str,
        indexed_files: usize,
        wiki_root: &Path,
    ) -> Result<()> {
        let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.id == project_id)
        else {
            bail!("project id {project_id} is not registered");
        };
        project.last_indexed_at = Some(Utc::now().to_rfc3339());
        project.last_indexed_wiki_max_mtime = max_wiki_modified(wiki_root)?;
        project.indexed_file_count = indexed_files;
        project.backend = BACKEND_NAME.to_string();
        project.index_schema_version = INDEX_SCHEMA_VERSION;
        Ok(())
    }

    fn register(&mut self, request: RegisterRequest) -> Result<RegisterOutcome> {
        if let Some(update_id) = request.update.clone() {
            validate_project_id(&update_id)?;
            return self.update_existing(&update_id, request);
        }

        if let Some(existing_index) = self.project_index_by_root(&request.root) {
            let existing = &mut self.projects[existing_index];
            if let Some(id) = &request.id
                && id != &existing.id
            {
                validate_project_id(id)?;
                bail!(
                    "project root {} is already registered as {}",
                    request.root.display(),
                    existing.id
                );
            }
            let mut changed = false;
            if let Some(name) = request.name
                && existing.name != name
            {
                existing.name = name;
                changed = true;
            }
            return Ok(if changed {
                RegisterOutcome::Updated(existing.id.clone())
            } else {
                RegisterOutcome::Unchanged(existing.id.clone())
            });
        }

        let requested_id = request.id.clone().unwrap_or_else(|| {
            let base = base_project_id(&request);
            debug_assert!(validate_project_id(&base).is_ok());
            self.available_id(&base)
        });
        validate_project_id(&requested_id)?;
        if self
            .projects
            .iter()
            .any(|project| project.id == requested_id)
        {
            bail!("project id {requested_id} is already registered");
        }
        let name = request.name.unwrap_or_else(|| display_name(&request.root));
        self.projects.push(RegisteredProject {
            id: requested_id.clone(),
            name,
            root: request.root,
            wiki_path: PathBuf::from("wiki"),
            registered_at: Utc::now().to_rfc3339(),
            last_indexed_at: None,
            last_indexed_wiki_max_mtime: None,
            indexed_file_count: 0,
            backend: BACKEND_NAME.to_string(),
            index_schema_version: INDEX_SCHEMA_VERSION,
        });
        self.projects.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(RegisterOutcome::Created(requested_id))
    }

    fn update_existing(
        &mut self,
        update_id: &str,
        request: RegisterRequest,
    ) -> Result<RegisterOutcome> {
        let Some(index) = self
            .projects
            .iter()
            .position(|project| project.id == update_id)
        else {
            bail!("project id {update_id} is not registered");
        };
        if let Some(conflict) = self.project_index_by_root(&request.root)
            && conflict != index
        {
            bail!(
                "project root {} is already registered as {}",
                request.root.display(),
                self.projects[conflict].id
            );
        }

        let project = &mut self.projects[index];
        project.root = request.root;
        if let Some(name) = request.name {
            project.name = name;
        }
        Ok(RegisterOutcome::Updated(update_id.to_string()))
    }

    fn remove(&mut self, project_id: &str) -> Result<RegisteredProject> {
        let Some(index) = self
            .projects
            .iter()
            .position(|project| project.id == project_id)
        else {
            bail!("project id {project_id} is not registered");
        };
        Ok(self.projects.remove(index))
    }

    fn project_index_by_root(&self, root: &Path) -> Option<usize> {
        self.projects
            .iter()
            .position(|project| project.root == root)
    }

    fn available_id(&self, base: &str) -> String {
        if !self.projects.iter().any(|project| project.id == base) {
            return base.to_string();
        }
        for index in 2.. {
            let candidate = format!("{base}-{index}");
            debug_assert!(validate_project_id(&candidate).is_ok());
            if !self.projects.iter().any(|project| project.id == candidate) {
                return candidate;
            }
        }
        unreachable!("unbounded id suffix loop")
    }
}

impl RegisteredProject {
    pub fn wiki_root(&self) -> PathBuf {
        self.root.join(&self.wiki_path)
    }
}

fn validate_project_root(path: &Path) -> Result<PathBuf> {
    let root = fs::canonicalize(path)
        .with_context(|| format!("canonicalize project root {}", path.display()))?;
    let wiki = root.join("wiki");
    if !wiki.join("index.md").is_file() {
        bail!("project root {} is missing wiki/index.md", root.display());
    }
    if !wiki.join("log.md").is_file() {
        bail!("project root {} is missing wiki/log.md", root.display());
    }
    if ![
        "project_guidelines.md",
        "CLAUDE.md",
        "AGENTS.md",
        "AGENTS.MD",
    ]
    .iter()
    .any(|name| root.join(name).is_file())
    {
        bail!(
            "project root {} is missing an orientation file: project_guidelines.md, CLAUDE.md, AGENTS.md, or AGENTS.MD",
            root.display()
        );
    }
    Ok(root)
}

fn validate_registered_root(root: &Path) -> Result<()> {
    if !root.is_absolute() {
        bail!("registered root {} must be absolute", root.display());
    }
    if root.exists() {
        let canonical = fs::canonicalize(root)
            .with_context(|| format!("canonicalize registered root {}", root.display()))?;
        if canonical != root {
            bail!(
                "registered root {} is not canonical; re-register the project to repair the registry",
                root.display()
            );
        }
    }
    Ok(())
}

fn validate_registered_wiki_path(path: &Path) -> Result<()> {
    let mut components = path.components();
    let Some(Component::Normal(_)) = components.next() else {
        bail!(
            "wiki path {} must be a single relative component",
            path.display()
        );
    };
    if components.next().is_some() {
        bail!(
            "wiki path {} must be a single relative component",
            path.display()
        );
    }
    Ok(())
}

fn validate_project_id(id: &str) -> Result<()> {
    if id.is_empty() || id.trim() != id || id == "." || id == ".." {
        bail!("invalid project id {id:?}: use letters, numbers, '-' or '_'");
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("invalid project id {id:?}: use letters, numbers, '-' or '_'");
    }
    Ok(())
}

fn resolve_register_root(
    registry: &ProjectRegistry,
    path: Option<&Path>,
    update: Option<&str>,
) -> Result<PathBuf> {
    match (path, update) {
        (Some(path), _) => validate_project_root(path),
        (None, Some(project_id)) => {
            let project = registry
                .project_by_id(project_id)
                .with_context(|| format!("project id {project_id} is not registered"))?;
            validate_project_root(&project.root)
        }
        (None, None) => bail!("register requires <path> unless --update is set"),
    }
}

fn unique_temp_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("projects");
    path.with_file_name(format!(
        "{file_name}.{suffix}.{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ))
}

fn registry_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("projects.json");
    path.with_file_name(format!("{file_name}.lock"))
}

fn maybe_sleep_for_test(var: &str) {
    if let Ok(value) = std::env::var(var)
        && let Ok(ms) = value.parse::<u64>()
        && ms > 0
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

struct RegistryMutationLock {
    path: PathBuf,
    _file: File,
}

impl RegistryMutationLock {
    fn acquire(registry_path: &Path) -> Result<Self> {
        let path = registry_lock_path(registry_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create registry lock dir {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("open registry lock {}", path.display()))?;
        file.lock()
            .with_context(|| format!("lock registry mutations {}", path.display()))?;
        file.set_len(0)
            .with_context(|| format!("truncate registry lock {}", path.display()))?;
        writeln!(file, "pid={}", process::id())
            .with_context(|| format!("write registry lock {}", path.display()))?;
        Ok(Self { path, _file: file })
    }
}

impl Drop for RegistryMutationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn record_index_success(
    project_id: &str,
    indexed_files: usize,
    wiki_root: &Path,
) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry_path = paths.project_registry();
    let _lock = RegistryMutationLock::acquire(&registry_path)?;
    let mut registry = ProjectRegistry::read(&registry_path)?;
    registry.record_index_success(project_id, indexed_files, wiki_root)?;
    maybe_sleep_for_test("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS");
    registry.write_atomic(&registry_path)
}

fn base_project_id(request: &RegisterRequest) -> String {
    request
        .name
        .as_deref()
        .map(slugify)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            request
                .root
                .file_name()
                .and_then(|value| value.to_str())
                .map(slugify)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "project".to_string())
        })
}

fn display_name(root: &Path) -> String {
    root.file_name()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "Project".to_string())
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug.to_string()
    }
}

fn dir_size(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0;
    for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}

fn max_wiki_modified(wiki_root: &Path) -> Result<Option<String>> {
    let mut max_modified = None;
    collect_max_modified(wiki_root, &mut max_modified)?;
    Ok(max_modified.map(|time| DateTime::<Utc>::from(time).to_rfc3339()))
}

fn collect_max_modified(
    path: &Path,
    max_modified: &mut Option<std::time::SystemTime>,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_max_modified(&path, max_modified)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            let modified = entry.metadata()?.modified().unwrap_or(UNIX_EPOCH);
            if max_modified.is_none_or(|current| modified > current) {
                *max_modified = Some(modified);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProjectRegistry, RegisterOutcome, RegisterRequest};
    use proptest::prelude::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn generated_ids_are_slugged_and_collision_safe() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root_a = fixture_project(temp.path(), "My Project");
        let root_b = fixture_project(temp.path(), "Other Project");
        let mut registry = ProjectRegistry::default();

        assert_eq!(
            registry
                .register(RegisterRequest {
                    root: root_a,
                    name: Some("My Project!".to_string()),
                    id: None,
                    update: None,
                })
                .expect("register"),
            RegisterOutcome::Created("my-project".to_string())
        );
        assert_eq!(
            registry
                .register(RegisterRequest {
                    root: root_b,
                    name: Some("My Project!".to_string()),
                    id: None,
                    update: None,
                })
                .expect("register"),
            RegisterOutcome::Created("my-project-2".to_string())
        );
    }

    #[test]
    fn same_root_registration_is_idempotent() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = fixture_project(temp.path(), "same");
        let mut registry = ProjectRegistry::default();

        registry
            .register(RegisterRequest {
                root: root.clone(),
                name: Some("Same".to_string()),
                id: None,
                update: None,
            })
            .expect("first");
        assert_eq!(
            registry
                .register(RegisterRequest {
                    root,
                    name: Some("Same Renamed".to_string()),
                    id: None,
                    update: None,
                })
                .expect("second"),
            RegisterOutcome::Updated("same".to_string())
        );
        assert_eq!(registry.projects.len(), 1);
        assert_eq!(registry.projects[0].name, "Same Renamed");
    }

    #[test]
    fn same_root_with_different_explicit_id_is_rejected() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = fixture_project(temp.path(), "same");
        let mut registry = ProjectRegistry::default();

        registry
            .register(RegisterRequest {
                root: root.clone(),
                name: None,
                id: Some("first".to_string()),
                update: None,
            })
            .expect("first");
        let error = registry
            .register(RegisterRequest {
                root,
                name: None,
                id: Some("second".to_string()),
                update: None,
            })
            .expect_err("duplicate root");

        assert!(error.to_string().contains("already registered"));
    }

    #[test]
    fn registry_round_trips_from_disk() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let root = fixture_project(temp.path(), "disk");
        let path = temp.path().join("data/projects.json");
        let mut registry = ProjectRegistry::default();
        registry
            .register(RegisterRequest {
                root,
                name: None,
                id: None,
                update: None,
            })
            .expect("register");

        registry.write_atomic(&path).expect("write");
        let read = ProjectRegistry::read(&path).expect("read");

        assert_eq!(read.projects.len(), 1);
        assert_eq!(read.projects[0].id, "disk");
    }

    #[test]
    fn registry_read_rejects_unsafe_persisted_ids() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let path = temp.path().join("projects.json");
        fs::write(
            &path,
            serde_json::json!({
                "version": 1,
                "projects": [{
                    "id": "../models",
                    "name": "Bad",
                    "root": "/tmp/bad",
                    "wiki_path": "wiki",
                    "registered_at": "2026-05-07T00:00:00Z",
                    "last_indexed_at": null,
                    "last_indexed_wiki_max_mtime": null,
                    "indexed_file_count": 0,
                    "backend": "qmd-rs",
                    "index_schema_version": 1
                }]
            })
            .to_string(),
        )
        .expect("write");

        let error = ProjectRegistry::read(&path).expect_err("unsafe id");

        assert!(error.to_string().contains("invalid project id"));
    }

    #[test]
    fn registry_read_rejects_non_absolute_roots() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let path = temp.path().join("projects.json");
        fs::write(
            &path,
            serde_json::json!({
                "version": 1,
                "projects": [{
                    "id": "fixture",
                    "name": "Fixture",
                    "root": "relative/root",
                    "wiki_path": "wiki",
                    "registered_at": "2026-05-07T00:00:00Z",
                    "last_indexed_at": null,
                    "last_indexed_wiki_max_mtime": null,
                    "indexed_file_count": 0,
                    "backend": "qmd-rs",
                    "index_schema_version": 1
                }]
            })
            .to_string(),
        )
        .expect("write");

        let error = ProjectRegistry::read(&path).expect_err("relative root");

        assert!(error.to_string().contains("invalid project root"));
    }

    #[test]
    fn registry_read_rejects_noncanonical_existing_roots() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let project = fixture_project(temp.path(), "fixture");
        let path = temp.path().join("projects.json");
        let noncanonical = project.join("..").join("fixture");
        fs::write(
            &path,
            serde_json::json!({
                "version": 1,
                "projects": [{
                    "id": "fixture",
                    "name": "Fixture",
                    "root": noncanonical,
                    "wiki_path": "wiki",
                    "registered_at": "2026-05-07T00:00:00Z",
                    "last_indexed_at": null,
                    "last_indexed_wiki_max_mtime": null,
                    "indexed_file_count": 0,
                    "backend": "qmd-rs",
                    "index_schema_version": 1
                }]
            })
            .to_string(),
        )
        .expect("write");

        let error = ProjectRegistry::read(&path).expect_err("noncanonical root");

        assert!(error.to_string().contains("invalid project root"));
    }

    #[test]
    fn registry_read_rejects_unsafe_wiki_path() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let project = fixture_project(temp.path(), "fixture");
        let path = temp.path().join("projects.json");
        fs::write(
            &path,
            serde_json::json!({
                "version": 1,
                "projects": [{
                    "id": "fixture",
                    "name": "Fixture",
                    "root": project,
                    "wiki_path": "../wiki",
                    "registered_at": "2026-05-07T00:00:00Z",
                    "last_indexed_at": null,
                    "last_indexed_wiki_max_mtime": null,
                    "indexed_file_count": 0,
                    "backend": "qmd-rs",
                    "index_schema_version": 1
                }]
            })
            .to_string(),
        )
        .expect("write");

        let error = ProjectRegistry::read(&path).expect_err("unsafe wiki path");

        assert!(error.to_string().contains("invalid wiki path"));
    }

    proptest! {
        #[test]
        fn validate_project_id_rejects_anything_outside_safe_charset(value in "\\PC*") {
            let contains_forbidden = value.contains('/')
                || value.contains('\\')
                || value.contains("..")
                || value.trim() != value
                || value.is_empty()
                || value == "."
                || !value
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');

            prop_assume!(contains_forbidden);
            prop_assert!(super::validate_project_id(&value).is_err());
        }
    }

    fn fixture_project(root: &Path, name: &str) -> PathBuf {
        let project = root.join(name);
        fs::create_dir_all(project.join("wiki")).expect("wiki");
        fs::write(project.join("wiki/index.md"), "# Index").expect("index");
        fs::write(project.join("wiki/log.md"), "# Log").expect("log");
        fs::write(project.join("AGENTS.md"), "# Agents").expect("agents");
        project.canonicalize().expect("canonical")
    }
}
