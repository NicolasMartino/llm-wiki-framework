use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::cli::{ForgetArgs, OutputFormat, ProjectsArgs, RegisterArgs};
use crate::paths::Paths;

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

pub fn register(args: &RegisterArgs) -> Result<()> {
    let outcome = register_project(
        &args.path,
        args.name.clone(),
        args.id.clone(),
        args.update.clone(),
    )?;

    match outcome {
        RegisterOutcome::Created(id) => println!("Registered project: {id}"),
        RegisterOutcome::Updated(id) => println!("Updated project: {id}"),
        RegisterOutcome::Unchanged(id) => println!("Project already registered: {id}"),
    }
    Ok(())
}

pub fn register_project(
    path: &Path,
    name: Option<String>,
    id: Option<String>,
    update: Option<String>,
) -> Result<RegisterOutcome> {
    let paths = Paths::from_env()?;
    let mut registry = ProjectRegistry::read(&paths.project_registry())?;
    let root = validate_project_root(path)?;
    let outcome = registry.register(RegisterRequest {
        root,
        name,
        id,
        update,
    })?;
    registry.write_atomic(&paths.project_registry())?;
    Ok(outcome)
}

pub fn outcome_id(outcome: &RegisterOutcome) -> &str {
    match outcome {
        RegisterOutcome::Created(id)
        | RegisterOutcome::Updated(id)
        | RegisterOutcome::Unchanged(id) => id,
    }
}

pub fn forget(args: &ForgetArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let mut registry = ProjectRegistry::read(&paths.project_registry())?;
    let removed = registry.remove(&args.project_id)?;
    registry.write_atomic(&paths.project_registry())?;

    if args.delete_cache {
        let cache_dir = paths.project_index_dir(&removed.id);
        if cache_dir.exists() {
            fs::remove_dir_all(&cache_dir)
                .with_context(|| format!("remove search cache {}", cache_dir.display()))?;
        }
    }

    println!("Forgot project: {}", removed.id);
    Ok(())
}

pub fn projects(args: &ProjectsArgs) -> Result<()> {
    let paths = Paths::from_env()?;
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&registry)?),
        OutputFormat::Text => print_projects_text(&registry, &paths)?,
    }
    Ok(())
}

fn print_projects_text(registry: &ProjectRegistry, paths: &Paths) -> Result<()> {
    if registry.projects.is_empty() {
        println!("No registered projects.");
        return Ok(());
    }

    for project in &registry.projects {
        let root_status = if project.root.exists() {
            "root-ok"
        } else {
            "root-missing"
        };
        let index_status = if paths.qmd_rs_store_path(&project.id).exists() {
            "index-present"
        } else {
            "index-missing"
        };
        let cache_size = dir_size(&paths.project_index_dir(&project.id))?;
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{} bytes",
            project.id,
            project.name,
            project.root.display(),
            root_status,
            index_status,
            project.backend,
            cache_size
        );
    }

    Ok(())
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
        Ok(registry)
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create registry dir {}", parent.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
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

    fn register(&mut self, request: RegisterRequest) -> Result<RegisterOutcome> {
        if let Some(update_id) = request.update.clone() {
            return self.update_existing(&update_id, request);
        }

        if let Some(existing_index) = self.project_index_by_root(&request.root) {
            let existing = &mut self.projects[existing_index];
            if let Some(id) = &request.id
                && id != &existing.id
            {
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

        let requested_id = request
            .id
            .clone()
            .unwrap_or_else(|| self.available_id(&base_project_id(&request)));
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
            if !self.projects.iter().any(|project| project.id == candidate) {
                return candidate;
            }
        }
        unreachable!("unbounded id suffix loop")
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
    if !["project_guidelines.md", "CLAUDE.md", "AGENTS.md"]
        .iter()
        .any(|name| root.join(name).is_file())
    {
        bail!(
            "project root {} is missing an orientation file: project_guidelines.md, CLAUDE.md, or AGENTS.md",
            root.display()
        );
    }
    Ok(root)
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

#[cfg(test)]
mod tests {
    use super::{ProjectRegistry, RegisterOutcome, RegisterRequest};
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

    fn fixture_project(root: &Path, name: &str) -> PathBuf {
        let project = root.join(name);
        fs::create_dir_all(project.join("wiki")).expect("wiki");
        fs::write(project.join("wiki/index.md"), "# Index").expect("index");
        fs::write(project.join("wiki/log.md"), "# Log").expect("log");
        fs::write(project.join("AGENTS.md"), "# Agents").expect("agents");
        project.canonicalize().expect("canonical")
    }
}
