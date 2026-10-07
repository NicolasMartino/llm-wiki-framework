//! Where a repository sets the branch its deadline files land on: the one
//! key of `poman.toml` at its root, master when the file or the key is
//! absent.

use std::fs;
use std::io;
use std::path::Path;

use toml::de::{DeTable, DeValue};

use crate::finding::Finding;

#[cfg(test)]
mod tests;

/// The file, at the repository root.
pub const FILE: &str = "poman.toml";

/// Its one key.
pub const KEY: &str = "landing-branch";

/// The branch deadline files land on when the repository sets none.
pub const DEFAULT_BRANCH: &str = "master";

/// The branch deadline files land on in the repository at `root`.
///
/// # Errors
///
/// Returns each finding that makes `poman.toml` unreadable or wrong: a file
/// that cannot be read, is not UTF-8 or is not TOML, a key poman does not
/// know, or a value that is not a branch name, each at its line.
pub fn landing_branch(root: &Path) -> Result<String, Vec<Finding>> {
    let bytes = match fs::read(root.join(FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(DEFAULT_BRANCH.to_owned());
        }
        Err(error) => {
            return Err(vec![Finding::error(
                FILE,
                1,
                format!("cannot be read: {error}"),
            )]);
        }
    };
    let text =
        String::from_utf8(bytes).map_err(|_| vec![Finding::error(FILE, 1, "is not UTF-8")])?;
    let table = DeTable::parse(&text).map_err(|error| {
        let line = error.span().map_or(1, |span| line_of(&text, span.start));
        vec![Finding::error(
            FILE,
            line,
            format!("is not TOML: {}", error.message().trim_end()),
        )]
    })?;
    let mut branch = DEFAULT_BRANCH.to_owned();
    let mut findings = Vec::new();
    for (key, value) in table.get_ref() {
        if key.get_ref() != KEY {
            findings.push(Finding::error(
                FILE,
                line_of(&text, key.span().start),
                format!(
                    "`{}` is not a key poman knows; its one key is `{KEY}`",
                    key.get_ref()
                ),
            ));
        } else if let DeValue::String(name) = value.get_ref()
            && is_branch_name(name)
        {
            branch = name.to_string();
        } else {
            findings.push(Finding::error(
                FILE,
                line_of(&text, value.span().start),
                format!("`{KEY}` must be a branch name in quotes, as `{KEY} = \"develop\"`"),
            ));
        }
    }
    if findings.is_empty() {
        Ok(branch)
    } else {
        Err(findings)
    }
}

/// The line, counted from 1, that holds the byte at `offset`.
fn line_of(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset)
        .filter(|&byte| byte == b'\n')
        .count()
        + 1
}

/// Whether `name` is a name Git takes for a branch: not empty, no part
/// starting with `.` or ending in `.lock`, no `..`, `@{` or `//`, no space,
/// control character or any of `~^:?*[\`, and no `/` or `-` to start it.
#[must_use]
pub fn is_branch_name(name: &str) -> bool {
    !name.is_empty()
        && name != "@"
        && !name.starts_with(['-', '/'])
        && !name.ends_with(['/', '.'])
        && !name.contains("..")
        && !name.contains("@{")
        && !name.contains("//")
        && name
            .chars()
            .all(|character| !character.is_ascii_control() && !" ~^:?*[\\".contains(character))
        && name
            .split('/')
            .all(|part| !part.starts_with('.') && part.strip_suffix(".lock").is_none())
}
