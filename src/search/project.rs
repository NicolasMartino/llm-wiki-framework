use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredProject {
    pub project_root: PathBuf,
    pub wiki_root: PathBuf,
    pub project_key: String,
}

pub fn discover_from_cwd() -> Result<Option<DiscoveredProject>> {
    discover_from(env::current_dir().context("read current directory")?)
}

pub fn discover_from(start: impl AsRef<Path>) -> Result<Option<DiscoveredProject>> {
    let mut current = fs::canonicalize(start.as_ref()).with_context(|| {
        format!(
            "canonicalize project discovery start {}",
            start.as_ref().display()
        )
    })?;

    loop {
        let wiki_root = current.join("wiki");
        if wiki_root.join("index.md").is_file() && wiki_root.join("log.md").is_file() {
            let project_key = project_key_for_wiki_root(&wiki_root)?;
            return Ok(Some(DiscoveredProject {
                project_root: current,
                wiki_root,
                project_key,
            }));
        }

        if !current.pop() {
            return Ok(None);
        }
    }
}

pub fn project_key_for_wiki_root(wiki_root: &Path) -> Result<String> {
    let canonical = fs::canonicalize(wiki_root)
        .with_context(|| format!("canonicalize wiki root {}", wiki_root.display()))?;
    let slug = canonical
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .map(slugify)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "project".to_string());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    Ok(format!("{slug}-{}", &hash[..10]))
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
    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::{discover_from, project_key_for_wiki_root};
    use std::fs;

    #[test]
    fn discovers_project_by_walking_upward() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let nested = temp.path().join("a/b/c");
        fs::create_dir_all(temp.path().join("wiki")).expect("wiki");
        fs::create_dir_all(&nested).expect("nested");
        fs::write(temp.path().join("wiki/index.md"), "# Index").expect("index");
        fs::write(temp.path().join("wiki/log.md"), "# Log").expect("log");

        let project = discover_from(&nested).expect("discovery").expect("project");

        assert_eq!(
            project.project_root,
            temp.path().canonicalize().expect("root")
        );
        assert_eq!(
            project.wiki_root,
            temp.path().join("wiki").canonicalize().expect("wiki")
        );
        assert!(project.project_key.starts_with("project-") || project.project_key.contains('-'));
    }

    #[test]
    fn reports_none_outside_wiki_project() {
        let temp = tempfile::TempDir::new().expect("tempdir");

        assert_eq!(discover_from(temp.path()).expect("discovery"), None);
    }

    #[test]
    fn project_key_is_stable_for_same_wiki_root() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("wiki");

        assert_eq!(
            project_key_for_wiki_root(&wiki).expect("left"),
            project_key_for_wiki_root(&wiki).expect("right")
        );
    }
}
