//! Where poman works: the repository root, found from where it runs.

use std::path::{Path, PathBuf};

/// The folder deadline files and the wiki live in, from the root.
pub const WIKI: &str = "wiki";

/// The nearest folder at or above `dir` holding `.git`: a folder, or a file
/// in a worktree.
#[must_use]
pub fn root_from(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|folder| folder.join(".git").exists())
        .map(Path::to_path_buf)
}

/// `path` from `root`, its parts joined by `/`, a part that is not UTF-8
/// written with replacement characters.
#[must_use]
pub fn relative(root: &Path, path: &Path) -> String {
    let parts: Vec<String> = path
        .strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.join("/")
}
