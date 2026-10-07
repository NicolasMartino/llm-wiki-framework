//! `poman check`: every deadline file under the repository's `wiki/` held to
//! its type, every `Blocked by` path to a deadline file, and `poman.toml` to
//! its one key.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use llm_wiki_core::names::{SLUG_RULE, is_slug, near_miss};
use llm_wiki_core::types::WikiFilename;
use llm_wiki_core::types::poman::{ALL, DEADLINE};

use crate::deadline::{BLOCKED_BY, Blockers, check_text};
use crate::finding::{Finding, Severity};
use crate::graph::{Graph, loops};
use crate::landing::landing_branch;
use crate::repo::{WIKI, relative};

#[cfg(test)]
mod tests;

/// What a check found.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    /// How many deadline files it read.
    pub files_checked: usize,
    /// Whether the repository has a `wiki/` folder.
    pub has_wiki: bool,
    /// Every finding, sorted by path, then line.
    pub findings: Vec<Finding>,
}

impl Report {
    /// How many findings are of `severity`.
    #[must_use]
    pub fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    }
}

/// One deadline file as the walk found it.
struct Read {
    path: String,
    blockers: Option<Blockers>,
}

/// Checks the repository at `root`.
#[must_use]
pub fn check(root: &Path) -> Report {
    let mut report = Report::default();
    if let Err(findings) = landing_branch(root) {
        report.findings.extend(findings);
    }
    let wiki = root.join(WIKI);
    report.has_wiki = wiki.is_dir();
    let mut read = Vec::new();
    if report.has_wiki {
        let canonical_root = fs::canonicalize(root).ok();
        walk(
            root,
            canonical_root.as_deref(),
            &wiki,
            &mut read,
            &mut report.findings,
        );
    }
    report.files_checked = read.len();
    check_references(root, &read, &mut report.findings);
    report
        .findings
        .sort_by(|left, right| (&left.path, left.line).cmp(&(&right.path, right.line)));
    report
}

/// The suffix every deadline file's name ends in.
fn suffix() -> String {
    format!(".{}", DEADLINE.suffix)
}

fn walk(
    root: &Path,
    canonical_root: Option<&Path>,
    folder: &Path,
    read: &mut Vec<Read>,
    findings: &mut Vec<Finding>,
) {
    let path = relative(root, folder);
    let mut entries: Vec<_> = match fs::read_dir(folder) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect(),
        Err(error) => {
            findings.push(Finding::error(
                &path,
                1,
                format!("this folder cannot be read: {error}"),
            ));
            return;
        }
    };
    entries.sort();
    for entry in entries {
        let path = relative(root, &entry);
        let is_link = fs::symlink_metadata(&entry).is_ok_and(|meta| meta.file_type().is_symlink());
        if is_link {
            follow_link(canonical_root, &entry, &path, read, findings);
        } else if entry.is_dir() {
            walk(root, canonical_root, &entry, read, findings);
        } else {
            file(&entry, &path, read, findings);
        }
    }
}

/// A symbolic link: a file read only when it stays in the repository, a
/// folder never walked, and a warning for one skipped whose name says it is
/// a deadline file.
fn follow_link(
    canonical_root: Option<&Path>,
    entry: &Path,
    path: &str,
    read: &mut Vec<Read>,
    findings: &mut Vec<Finding>,
) {
    let target = fs::canonicalize(entry).ok();
    let inside = target.as_deref().filter(|target| {
        canonical_root.is_some_and(|root| target.starts_with(root)) && target.is_file()
    });
    if inside.is_some() {
        file(entry, path, read, findings);
    } else if path.ends_with(&suffix()) {
        let reason = match target {
            Some(target) if target.is_dir() => "links to a folder",
            Some(_) => "links outside the repository",
            None => "is a link to nothing",
        };
        findings.push(Finding::warning(
            path,
            1,
            format!("{reason}; poman does not follow it, so it is not checked"),
        ));
    }
}

fn file(entry: &Path, path: &str, read: &mut Vec<Read>, findings: &mut Vec<Finding>) {
    let (folder, name) = path.rsplit_once('/').unwrap_or_default();
    if name.ends_with(&suffix()) {
        read.push(deadline_file(entry, path, folder, name, findings));
        return;
    }
    let known = ALL
        .iter()
        .filter_map(|doc_type| doc_type.suffix.strip_suffix(".md"));
    let close = name
        .strip_suffix(".md")
        .and_then(|stem| stem.rsplit_once('.'))
        .and_then(|(_, written)| near_miss(written, known).map(|close| (written, close)));
    if let Some((written, close)) = close {
        findings.push(Finding::warning(
            path,
            1,
            format!("the type `{written}` is close to `{close}`; this file is not read as a {close} file"),
        ));
    } else if folder == DEADLINE.folder
        && Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        findings.push(Finding::warning(
            path,
            1,
            format!(
                "this Markdown file in {}/ does not end in {}, so it is not read",
                DEADLINE.folder,
                suffix()
            ),
        ));
    }
}

fn deadline_file(
    entry: &Path,
    path: &str,
    folder: &str,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Read {
    if folder != DEADLINE.folder {
        findings.push(Finding::error(
            path,
            1,
            format!("a deadline file must sit directly in {}/", DEADLINE.folder),
        ));
    }
    match WikiFilename::parse_unindexed(name) {
        Err(error) => findings.push(Finding::error(path, 1, error.to_string())),
        Ok(filename) if !is_slug(filename.slug()) => findings.push(Finding::error(
            path,
            1,
            format!("the slug `{}` is not {SLUG_RULE}", filename.slug()),
        )),
        Ok(_) => {}
    }
    let text = match fs::read(entry) {
        Ok(bytes) => String::from_utf8(bytes).ok(),
        Err(error) => {
            findings.push(Finding::error(path, 1, format!("cannot be read: {error}")));
            return Read {
                path: path.to_owned(),
                blockers: None,
            };
        }
    };
    let Some(text) = text else {
        findings.push(Finding::error(path, 1, "is not UTF-8"));
        return Read {
            path: path.to_owned(),
            blockers: None,
        };
    };
    let (found, blockers) = check_text(path, &text);
    findings.extend(found);
    Read {
        path: path.to_owned(),
        blockers,
    }
}

/// What a `Blocked by` path names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Blocker {
    /// A deadline file read in its folder.
    Deadline,
    /// Nothing, by that exact name.
    Missing,
    /// Something that is not a deadline file read in its folder.
    NotDeadline,
}

/// What `path` names, decided from the names folders list and never from a
/// lookup: a file system that ignores letter case finds
/// `wiki/deadlines/B.deadline.md` when the folder holds `b.deadline.md`, and
/// the answer must not depend on the volume. `deadlines` are the deadline
/// files read in their folder.
#[must_use]
pub fn blocker(root: &Path, deadlines: &BTreeSet<&str>, path: &str) -> Blocker {
    if deadlines.contains(path) {
        Blocker::Deadline
    } else if exists_exactly(root, path) {
        Blocker::NotDeadline
    } else {
        Blocker::Missing
    }
}

/// Whether each part of `path`, from the repository root, is a name its
/// folder lists, letter case included.
fn exists_exactly(root: &Path, path: &str) -> bool {
    let mut folder = root.to_path_buf();
    for part in path
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
    {
        let listed = part != ".."
            && fs::read_dir(&folder).is_ok_and(|entries| {
                entries
                    .filter_map(Result::ok)
                    .any(|entry| entry.file_name() == part)
            });
        if !listed {
            return false;
        }
        folder.push(part);
    }
    true
}

fn in_deadline_folder(path: &str) -> bool {
    path.rsplit_once('/')
        .is_some_and(|(folder, _)| folder == DEADLINE.folder)
}

/// The deadline files in their folder, read as `poman check` reads them, and
/// the `Blocked by` paths of each.
#[must_use]
pub fn deadline_folder(root: &Path) -> Graph {
    let mut read = Vec::new();
    let canonical_root = fs::canonicalize(root).ok();
    walk(
        root,
        canonical_root.as_deref(),
        &root.join(DEADLINE.folder),
        &mut read,
        &mut Vec::new(),
    );
    read.into_iter()
        .filter(|file| in_deadline_folder(&file.path))
        .map(|file| {
            (
                file.path,
                file.blockers.map(|found| found.paths).unwrap_or_default(),
            )
        })
        .collect()
}

fn check_references(root: &Path, read: &[Read], findings: &mut Vec<Finding>) {
    let mut graph = Graph::new();
    let mut lines: BTreeMap<&str, usize> = BTreeMap::new();
    let deadlines: BTreeSet<&str> = read
        .iter()
        .map(|file| file.path.as_str())
        .filter(|path| in_deadline_folder(path))
        .collect();
    for file in read {
        let Some(blockers) = &file.blockers else {
            continue;
        };
        lines.insert(&file.path, blockers.line);
        let mut listed = BTreeSet::new();
        let mut edges = Vec::new();
        for blocker in &blockers.paths {
            let problem = if !listed.insert(blocker) {
                Some(format!("`{BLOCKED_BY}` lists `{blocker}` twice"))
            } else if *blocker == file.path {
                Some(format!("`{BLOCKED_BY}` names this file itself"))
            } else {
                match self::blocker(root, &deadlines, blocker) {
                    Blocker::Deadline => {
                        edges.push(blocker.clone());
                        None
                    }
                    Blocker::Missing => Some(format!(
                        "`{BLOCKED_BY}` names `{blocker}`, which does not exist"
                    )),
                    Blocker::NotDeadline => Some(format!(
                        "`{BLOCKED_BY}` names `{blocker}`, which is not a deadline file in {}/",
                        DEADLINE.folder
                    )),
                }
            };
            if let Some(problem) = problem {
                findings.push(Finding::error(&file.path, blockers.line, problem));
            }
        }
        graph.insert(file.path.clone(), edges);
    }
    for group in loops(&graph) {
        let members: Vec<String> = group
            .iter()
            .map(|member| {
                format!(
                    "{member}:{}",
                    lines.get(member.as_str()).copied().unwrap_or(1)
                )
            })
            .collect();
        if let Some(first) = group.first() {
            findings.push(Finding::error(
                first,
                lines.get(first.as_str()).copied().unwrap_or(1),
                format!(
                    "`{BLOCKED_BY}` forms a loop: {} wait on one another",
                    members.join(", ")
                ),
            ));
        }
    }
}
