use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::{CliContext, ReadArgs};
use crate::paths::Paths;
use crate::registry::ProjectRegistry;

const MAX_INLINE_CONTENT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Deserialize)]
pub struct WikiReadRequest {
    pub path: PathBuf,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiReadResponse {
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub project_root: String,
    pub wiki_root: String,
    pub raw_root: String,
    pub path: String,
    // `scope`/`bytes` are compat aliases for `tree`/`byte_len`: they duplicate the
    // same values on purpose as part of the response payload contract. Do not remove.
    pub tree: String,
    pub scope: String,
    pub byte_len: usize,
    pub bytes: usize,
    pub sha256: String,
    pub encoding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_omitted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<WikiReadEntry>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiReadEntry {
    pub path: String,
    pub tree: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

pub fn run_cli(args: &ReadArgs, _context: &CliContext) -> Result<()> {
    let response = read(WikiReadRequest {
        path: args.path.clone(),
        project: args.project.clone(),
    })?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

pub fn read(request: WikiReadRequest) -> Result<WikiReadResponse> {
    let project = resolve_project(request.project.as_deref())?;
    let target = resolve_requested_path(&project.root, &request.path)?;

    let wiki_root = fs::canonicalize(project.root.join("wiki")).with_context(|| {
        format!(
            "canonicalize wiki root {}",
            project.root.join("wiki").display()
        )
    })?;
    let raw_root = project.root.join("raw");
    let raw_root_for_compare = if raw_root.exists() {
        fs::canonicalize(&raw_root)
            .with_context(|| format!("canonicalize raw root {}", raw_root.display()))?
    } else {
        raw_root.clone()
    };

    let scope = if target.starts_with(&wiki_root) {
        "wiki"
    } else if raw_root.exists() && target.starts_with(&raw_root_for_compare) {
        "raw"
    } else {
        bail!(
            "{} is outside the project wiki/ or raw/ trees",
            request.path.display()
        );
    };

    let relative = target
        .strip_prefix(&project.root)
        .with_context(|| format!("relativize {}", target.display()))?;

    if target.is_dir() {
        let entries = directory_entries(&project.root, &target, scope)?;
        return Ok(WikiReadResponse {
            project_id: project.id,
            project_name: project.name,
            project_root: project.root.to_string_lossy().to_string(),
            wiki_root: wiki_root.to_string_lossy().to_string(),
            raw_root: raw_root_for_compare.to_string_lossy().to_string(),
            path: slash_path(relative),
            tree: scope.to_string(),
            scope: scope.to_string(),
            byte_len: 0,
            bytes: 0,
            sha256: sha256_hex(&[]),
            encoding: "directory".to_string(),
            content: None,
            content_omitted: Some("directory listing".to_string()),
            entries: Some(entries),
        });
    }

    if !target.is_file() {
        bail!("{} is not a readable file", request.path.display());
    }

    let metadata = fs::metadata(&target).with_context(|| format!("stat {}", target.display()))?;
    // `relative` is already computed above (shared by the directory and file
    // paths); no need to recompute it here.

    // Files over the inline cap (raw/ holds PDFs/transcripts that can be GBs) are
    // hashed by streaming so we never read+clone the whole thing just to omit it.
    let (byte_len, sha256, encoding, content, content_omitted) =
        if metadata.len() > MAX_INLINE_CONTENT_BYTES as u64 {
            let (len, hash, is_utf8) = scan_file_streaming(&target)?;
            let (encoding, omitted) = if is_utf8 {
                (
                    "utf-8".to_string(),
                    format!("content exceeds {MAX_INLINE_CONTENT_BYTES} byte inline cap"),
                )
            } else {
                ("binary".to_string(), "binary".to_string())
            };
            (len, hash, encoding, None, Some(omitted))
        } else {
            let bytes = fs::read(&target).with_context(|| format!("read {}", target.display()))?;
            let len = bytes.len();
            let hash = sha256_hex(&bytes);
            let (encoding, content, content_omitted) = inline_content(bytes);
            (len, hash, encoding, content, content_omitted)
        };

    Ok(WikiReadResponse {
        project_id: project.id,
        project_name: project.name,
        project_root: project.root.to_string_lossy().to_string(),
        wiki_root: wiki_root.to_string_lossy().to_string(),
        raw_root: raw_root_for_compare.to_string_lossy().to_string(),
        path: slash_path(relative),
        tree: scope.to_string(),
        scope: scope.to_string(),
        byte_len,
        bytes: byte_len,
        sha256,
        encoding,
        content,
        content_omitted,
        entries: None,
    })
}

pub fn discover_current_project_root() -> Result<PathBuf> {
    let root = discover_project_root(&env::current_dir().context("resolve current directory")?)?;
    fs::canonicalize(&root).with_context(|| format!("canonicalize project root {}", root.display()))
}

#[derive(Debug, Clone)]
struct ResolvedProject {
    id: Option<String>,
    name: Option<String>,
    root: PathBuf,
}

fn resolve_project(project_id: Option<&str>) -> Result<ResolvedProject> {
    if let Some(project_id) = project_id {
        let paths = Paths::from_env()?;
        let registry = ProjectRegistry::read(&paths.project_registry())?;
        let project = registry
            .project_by_id(project_id)
            .with_context(|| format!("project id {project_id} is not registered"))?;
        return Ok(ResolvedProject {
            id: Some(project.id.clone()),
            name: Some(project.name.clone()),
            root: fs::canonicalize(&project.root)
                .with_context(|| format!("canonicalize project root {}", project.root.display()))?,
        });
    }

    let root = discover_current_project_root()?;

    let paths = Paths::from_env()?;
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let registered = registry.project_by_root(&root);

    Ok(ResolvedProject {
        id: registered.map(|project| project.id.clone()),
        name: registered.map(|project| project.name.clone()),
        root,
    })
}

fn discover_project_root(start: &Path) -> Result<PathBuf> {
    for candidate in start.ancestors() {
        if candidate.join("wiki/index.md").is_file() {
            return Ok(candidate.to_path_buf());
        }
    }
    bail!(
        "could not find wiki/index.md from {} or any parent directory",
        start.display()
    )
}

fn resolve_requested_path(project_root: &Path, requested: &Path) -> Result<PathBuf> {
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        project_root.join(requested)
    };
    // Reject paths that lexically escape the project root *before* touching the
    // filesystem, so a `..`-escaping relative path gets the same clear
    // outside-tree rejection as an absolute outside path instead of a generic
    // "not found" (which is what a non-existent escaped target would otherwise
    // produce). See the 2026-06-29 field test, Issue 8.
    if path_escapes_base(project_root, &candidate) {
        bail!(
            "{} is outside the project wiki/ or raw/ trees",
            requested.display()
        );
    }
    match fs::canonicalize(&candidate) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            bail!("{} not found", requested.display())
        }
        Err(error) => Err(error).with_context(|| format!("canonicalize {}", candidate.display())),
    }
}

/// Lexically normalize `candidate` (resolving `.` and `..` without touching the
/// filesystem) and report whether the result falls outside `base`. `base` is the
/// already-canonicalized project root. This is a cheap escape guard; the existing
/// canonicalize + `starts_with(wiki_root|raw_root)` scope check in `read` remains
/// the authority for symlink-based escapes of paths that actually exist.
fn path_escapes_base(base: &Path, candidate: &Path) -> bool {
    use std::path::Component;
    let mut normalized: Vec<Component> = Vec::new();
    for component in candidate.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.last() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                // A `..` that cannot pop a normal segment walks above the root.
                _ => normalized.push(component),
            },
            other => normalized.push(other),
        }
    }
    let normalized: PathBuf = normalized.iter().collect();
    !normalized.starts_with(base)
}

fn directory_entries(
    project_root: &Path,
    target: &Path,
    scope: &str,
) -> Result<Vec<WikiReadEntry>> {
    let mut entries = Vec::new();
    for entry in
        fs::read_dir(target).with_context(|| format!("read directory {}", target.display()))?
    {
        // A single unreadable entry must not abort the whole listing.
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        // Use symlink_metadata so a symlink is identified without following it; a
        // broken symlink (or a vanished entry) then surfaces as an Err we skip
        // rather than a bare `?` that aborts the listing.
        let Ok(link_metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let metadata = if link_metadata.file_type().is_symlink() {
            // Only list a symlink whose canonical target stays inside the project
            // scope (consistent with the read-side rejection); skip broken or
            // escaping links instead of leaking outside paths.
            match fs::canonicalize(&path) {
                Ok(resolved) if resolved.starts_with(project_root) => match fs::metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                },
                _ => continue,
            }
        } else {
            link_metadata
        };
        let kind = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };
        let relative = path
            .strip_prefix(project_root)
            .with_context(|| format!("relativeize {}", path.display()))?;
        entries.push(WikiReadEntry {
            path: slash_path(relative),
            tree: scope.to_string(),
            kind: kind.to_string(),
            bytes: metadata.is_file().then_some(metadata.len()),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Stream a file to compute its byte length, SHA-256, and UTF-8 validity without
/// holding the whole file in memory. UTF-8 is validated incrementally, carrying a
/// short (≤3 byte) tail across chunk boundaries for sequences split between reads.
fn scan_file_streaming(path: &Path) -> Result<(usize, String, bool)> {
    use std::io::Read;

    let mut file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut carry: Vec<u8> = Vec::new();
    let mut total: usize = 0;
    let mut valid_utf8 = true;

    loop {
        let read = file
            .read(&mut buf)
            .with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        total = total.saturating_add(read);

        if valid_utf8 {
            carry.extend_from_slice(&buf[..read]);
            match std::str::from_utf8(&carry) {
                Ok(_) => carry.clear(),
                Err(error) => match error.error_len() {
                    // Genuine invalid byte: this file is not UTF-8.
                    Some(_) => {
                        valid_utf8 = false;
                        carry.clear();
                    }
                    // Incomplete trailing sequence: keep the tail for the next chunk.
                    None => {
                        let valid_up_to = error.valid_up_to();
                        carry.drain(..valid_up_to);
                    }
                },
            }
        }
    }

    // A leftover incomplete sequence at EOF means the file is not valid UTF-8.
    if valid_utf8 && !carry.is_empty() {
        valid_utf8 = false;
    }

    let digest = hasher.finalize();
    let hash = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok((total, hash, valid_utf8))
}

fn inline_content(bytes: Vec<u8>) -> (String, Option<String>, Option<String>) {
    match String::from_utf8(bytes) {
        Ok(content) if content.len() <= MAX_INLINE_CONTENT_BYTES => {
            ("utf-8".to_string(), Some(content), None)
        }
        Ok(_) => (
            "utf-8".to_string(),
            None,
            Some(format!(
                "content exceeds {MAX_INLINE_CONTENT_BYTES} byte inline cap"
            )),
        ),
        Err(_) => ("binary".to_string(), None, Some("binary".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::path_escapes_base;
    use std::path::Path;

    #[test]
    fn relative_parent_escape_is_detected() {
        let base = Path::new("/home/user/project");
        // `../../etc/hosts` joined onto the project root walks above it.
        let candidate = base.join("../../etc/hosts");
        assert!(
            path_escapes_base(base, &candidate),
            "a `..`-escaping relative path must be flagged as outside the project"
        );
    }

    #[test]
    fn absolute_outside_path_is_detected() {
        let base = Path::new("/home/user/project");
        assert!(path_escapes_base(base, Path::new("/etc/hosts")));
    }

    #[test]
    fn in_tree_paths_are_allowed() {
        let base = Path::new("/home/user/project");
        assert!(!path_escapes_base(base, &base.join("wiki/index.md")));
        // `..` that stays within the tree is fine.
        assert!(!path_escapes_base(
            base,
            &base.join("wiki/../raw/manifest.md")
        ));
        assert!(!path_escapes_base(base, base));
    }
}
