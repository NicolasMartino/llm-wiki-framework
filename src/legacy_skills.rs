use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::Paths;

const LEGACY_SKILL_NAMES: &[&str] = &[
    "wiki",
    "wiki-init",
    "wiki-query",
    "wiki-ingest",
    "wiki-lint",
    "wiki-research",
];

pub fn is_legacy_global_skill_path(paths: &Paths, path: &Path) -> bool {
    path.starts_with(paths.home.join(".claude/skills"))
        || path.starts_with(paths.home.join(".codex/skills"))
}

pub fn existing_legacy_skill_dirs(paths: &Paths) -> Vec<PathBuf> {
    legacy_skill_dirs(paths)
        .into_iter()
        .filter(|path| path.exists())
        .collect()
}

pub fn legacy_skill_dirs(paths: &Paths) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for runtime_root in [
        paths.home.join(".claude/skills"),
        paths.home.join(".codex/skills"),
    ] {
        for skill in LEGACY_SKILL_NAMES {
            dirs.push(runtime_root.join(skill));
        }
    }
    dirs
}

pub fn remove_empty_legacy_parents(path: &Path, paths: &Paths) -> std::io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    let stop_roots = [
        paths.home.join(".claude/skills"),
        paths.home.join(".codex/skills"),
    ];
    let mut cursor = path.parent();
    while let Some(dir) = cursor {
        if stop_roots.iter().any(|root| dir == root) {
            break;
        }
        match fs::remove_dir(dir) {
            Ok(()) => removed.push(dir.to_path_buf()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(error) => return Err(error),
        }
        cursor = dir.parent();
    }
    Ok(removed)
}
