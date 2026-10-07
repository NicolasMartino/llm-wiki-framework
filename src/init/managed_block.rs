use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use sha2::{Digest, Sha256};

use super::compose::InitOutput;
use super::manifest::ManagedBlocks;

const START_MARKER: &str = "<!-- llm-wiki:managed:start -->";
const END_MARKER: &str = "<!-- llm-wiki:managed:end -->";
const NOTICE: &str = "<!-- llm-wiki init rewrites the text between these markers on every run; \
write this project's own text above or below them. -->";

const SAVED_BLOCKS_DIR: &str = ".llm_wiki/saved-blocks";
const MAX_LISTED_LINES: usize = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootFile {
    Guidelines,
    Agents,
    Claude,
}

impl RootFile {
    const ALL: [RootFile; 3] = [RootFile::Guidelines, RootFile::Agents, RootFile::Claude];

    fn composed_path(self) -> &'static str {
        match self {
            RootFile::Guidelines => "project_guidelines.md",
            RootFile::Agents => "AGENTS.md",
            RootFile::Claude => "CLAUDE.md",
        }
    }

    fn file_name(self, agents_file: &'static str) -> &'static str {
        match self {
            RootFile::Agents => agents_file,
            other => other.composed_path(),
        }
    }
}

/// Every composed path that a root schema file's block replaces; the scaffold
/// writes these only through [`RootSchemaWrites::apply`].
pub(super) fn is_root_schema_file(path: &str) -> bool {
    RootFile::ALL
        .iter()
        .any(|file| file.composed_path() == path)
}

/// The names in the project's folder as spelled on disk. `exists()` cannot
/// tell `AGENTS.md` from `AGENTS.MD` on a case-insensitive file system; the
/// listing can.
pub(super) fn folder_names(project: &Path) -> BTreeSet<String> {
    fs::read_dir(project)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// The project's AGENTS file: `AGENTS.MD` when that spelling is the one on
/// disk, `AGENTS.md` otherwise.
pub(super) fn agents_file_name(names: &BTreeSet<String>) -> &'static str {
    if !names.contains("AGENTS.md") && names.contains("AGENTS.MD") {
        "AGENTS.MD"
    } else {
        "AGENTS.md"
    }
}

/// CLAUDE.md's one line, naming the AGENTS file the project has.
fn claude_redirect(agents_file: &str) -> String {
    format!("See @{agents_file}.\n")
}

/// What init will write into the root schema files, worked out before
/// anything is written so that a refusal leaves the project as it was.
pub(super) struct RootSchemaWrites {
    files: Vec<PlannedFile>,
    hashes: ManagedBlocks,
}

struct PlannedFile {
    name: &'static str,
    /// Set when this file is a link to an earlier root schema file, whose
    /// block is the one written; writing this one too would put its block
    /// over the other's through the link.
    linked_to: Option<&'static str>,
    contents: String,
    changed: bool,
    edited_block: Option<EditedBlock>,
    kept_unmarked: bool,
}

struct EditedBlock {
    text: String,
    lines_not_kept: Vec<String>,
}

impl RootSchemaWrites {
    /// `previous` is what init renders from the answers recorded before this
    /// run; the migration of unmarked files and blocks with no recorded hash
    /// compare against it.
    pub(super) fn plan(
        project: &Path,
        names: &BTreeSet<String>,
        output: &InitOutput,
        previous: Option<&InitOutput>,
        recorded: &ManagedBlocks,
    ) -> Result<Self> {
        let agents_file = agents_file_name(names);
        let mut files = Vec::new();
        let mut hashes = ManagedBlocks::default();
        let mut written: Vec<(PathBuf, &'static str)> = Vec::new();
        for root in RootFile::ALL {
            let name = root.file_name(agents_file);
            let canonical = fs::canonicalize(project.join(name)).ok();
            let linked_to = canonical.as_ref().and_then(|canonical| {
                written
                    .iter()
                    .find(|(path, _)| path == canonical)
                    .map(|(_, other)| *other)
            });
            if let Some(other) = linked_to {
                files.push(PlannedFile::linked(name, other));
                continue;
            }
            if let Some(canonical) = canonical {
                written.push((canonical, name));
            }
            let body = render(root, output, agents_file)?;
            let previous_body = previous
                .map(|previous| render(root, previous, agents_file))
                .transpose()?;
            let existing = read_existing(&project.join(name))?;
            let planned = plan_file(
                root,
                name,
                &body,
                previous_body.as_deref(),
                recorded_hash(recorded, root),
                existing.as_deref(),
            )?;
            *recorded_hash_mut(&mut hashes, root) = Some(hash(&block_inner(&body)));
            files.push(planned);
        }
        Ok(Self { files, hashes })
    }

    pub(super) fn hashes(&self) -> &ManagedBlocks {
        &self.hashes
    }

    pub(super) fn apply(&self, project: &Path) -> Result<()> {
        for file in &self.files {
            if let Some(other) = file.linked_to {
                println!(
                    "{} links to {other}; init wrote the llm-wiki block into {other} only.",
                    file.name
                );
                continue;
            }
            let target = project.join(file.name);
            if let Some(edited) = &file.edited_block {
                let copy = save_edited_block(project, file.name, &edited.text)?;
                warn_edited_block(file.name, &copy, project, &edited.lines_not_kept);
            }
            if file.changed {
                fs::write(&target, &file.contents)
                    .with_context(|| format!("failed to write {}", target.display()))?;
            }
            if file.kept_unmarked {
                eprintln!(
                    "Warning: {} had no llm-wiki block; init wrote its block at the top and \
                     kept the file's earlier text below it, unchanged. That text may repeat what \
                     the block now says; trimming it is the project's call.",
                    file.name
                );
            }
        }
        Ok(())
    }
}

impl PlannedFile {
    fn linked(name: &'static str, other: &'static str) -> Self {
        Self {
            name,
            linked_to: Some(other),
            contents: String::new(),
            changed: false,
            edited_block: None,
            kept_unmarked: false,
        }
    }
}

fn render(root: RootFile, output: &InitOutput, agents_file: &str) -> Result<String> {
    if root == RootFile::Claude {
        return Ok(claude_redirect(agents_file));
    }
    output
        .files
        .iter()
        .find(|file| file.path == root.composed_path())
        .map(|file| file.contents.clone())
        .with_context(|| format!("init composed no {}", root.composed_path()))
}

fn read_existing(path: &Path) -> Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .with_context(|| format!("refusing to run init: {} is not UTF-8 text", path.display())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn recorded_hash(recorded: &ManagedBlocks, root: RootFile) -> Option<&str> {
    match root {
        RootFile::Guidelines => recorded.guidelines.as_deref(),
        RootFile::Agents => recorded.agents.as_deref(),
        RootFile::Claude => recorded.claude.as_deref(),
    }
}

fn recorded_hash_mut(recorded: &mut ManagedBlocks, root: RootFile) -> &mut Option<String> {
    match root {
        RootFile::Guidelines => &mut recorded.guidelines,
        RootFile::Agents => &mut recorded.agents,
        RootFile::Claude => &mut recorded.claude,
    }
}

fn plan_file(
    root: RootFile,
    name: &'static str,
    body: &str,
    previous_body: Option<&str>,
    recorded_hash: Option<&str>,
    existing: Option<&str>,
) -> Result<PlannedFile> {
    let inner = block_inner(body);
    // An empty file has no text of the project's to keep.
    let existing = existing.filter(|text| !text.trim_matches(is_blank).is_empty());
    let Some(existing) = existing else {
        return Ok(PlannedFile {
            name,
            linked_to: None,
            contents: wrap(&inner, "\n"),
            changed: true,
            edited_block: None,
            kept_unmarked: false,
        });
    };

    let Some(bounds) = find_block(existing).map_err(|error| refusal(name, &error))? else {
        let unchanged_render =
            previous_body.is_some_and(|previous| same_render(root, existing, previous));
        let contents = if unchanged_render {
            wrap(&inner, "\n")
        } else {
            format!(
                "{}\n## Kept From Before The llm-wiki Block ({})\n\n{existing}",
                wrap(&inner, "\n"),
                Utc::now().date_naive()
            )
        };
        return Ok(PlannedFile {
            name,
            linked_to: None,
            changed: contents != existing,
            contents,
            edited_block: None,
            kept_unmarked: !unchanged_render,
        });
    };

    let old_inner = &existing[bounds.inner_start..bounds.inner_end];
    let edited = match recorded_hash {
        Some(recorded) => hash(old_inner) != recorded,
        None => !previous_body
            .is_some_and(|previous| same_render(root, old_inner, &block_inner(previous))),
    };
    let edited_block = (edited && normalize(old_inner) != normalize(&inner)).then(|| EditedBlock {
        text: old_inner.to_string(),
        lines_not_kept: lines_not_kept(old_inner, &inner),
    });
    let contents = format!(
        "{}{}{}",
        &existing[..bounds.start],
        wrap(&inner, bounds.end_terminator(existing)),
        &existing[bounds.end..]
    );
    Ok(PlannedFile {
        name,
        linked_to: None,
        changed: contents != existing,
        contents,
        edited_block,
        kept_unmarked: false,
    })
}

fn block_inner(body: &str) -> String {
    let mut inner = format!("{NOTICE}\n\n{body}");
    if !inner.ends_with('\n') {
        inner.push('\n');
    }
    inner
}

fn wrap(inner: &str, end_terminator: &str) -> String {
    format!("{START_MARKER}\n{inner}{END_MARKER}{end_terminator}")
}

/// Line endings are not an edit: an editor that turns a file to CRLF leaves
/// the block as init wrote it.
fn hash(inner: &str) -> String {
    format!("{:x}", Sha256::digest(normalize(inner).as_bytes()))
}

fn normalize(text: &str) -> String {
    text.trim_start_matches(BOM).replace("\r\n", "\n")
}

const BOM: char = '\u{feff}';

fn is_blank(c: char) -> bool {
    c.is_whitespace() || c == BOM
}

/// The guidelines carry the date of their render, which differs between any
/// two runs on different days; it is left out on both sides.
fn same_render(root: RootFile, existing: &str, render: &str) -> bool {
    let strip = |text: &str| {
        let text = normalize(text);
        if root != RootFile::Guidelines {
            return text;
        }
        let mut dropped = false;
        text.split_inclusive('\n')
            .filter(|line| {
                if !dropped && line.starts_with("- Date:") {
                    dropped = true;
                    false
                } else {
                    true
                }
            })
            .collect()
    };
    strip(existing) == strip(render)
}

fn lines_not_kept(old_inner: &str, new_inner: &str) -> Vec<String> {
    let new_lines: BTreeSet<&str> = new_inner.lines().map(str::trim_end).collect();
    let mut seen = BTreeSet::new();
    old_inner
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty() && !new_lines.contains(line))
        .filter(|line| seen.insert(*line))
        .map(ToOwned::to_owned)
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BlockBounds {
    start: usize,
    inner_start: usize,
    inner_end: usize,
    end: usize,
}

impl BlockBounds {
    fn end_terminator<'a>(&self, contents: &'a str) -> &'a str {
        let end_line = &contents[self.inner_end..self.end];
        let content = end_line.trim_end_matches(['\n', '\r']);
        &end_line[content.len()..]
    }
}

#[derive(Debug, Eq, PartialEq)]
struct MarkerError {
    line: usize,
    problem: String,
}

fn refusal(name: &str, error: &MarkerError) -> anyhow::Error {
    anyhow::anyhow!(
        "refusing to run init: {name} line {}: {}. A file holds one block, from a \
         `{START_MARKER}` line to a `{END_MARKER}` line; put back the missing marker or \
         remove the stray one, then run init again. Nothing was written.",
        error.line,
        error.problem
    )
}

fn find_block(contents: &str) -> Result<Option<BlockBounds>, MarkerError> {
    let mut open: Option<(usize, usize, usize)> = None;
    let mut found: Option<(BlockBounds, usize, usize)> = None;
    let mut offset = 0;
    for (index, line) in contents.split_inclusive('\n').enumerate() {
        let number = index + 1;
        // A byte-order mark before a marker is part of the text above it.
        let lead = line.len() - line.trim_start_matches(BOM).len();
        let marker = line[lead..].trim();
        if marker == START_MARKER {
            if let Some((_, _, begin_line)) = open {
                return Err(MarkerError {
                    line: number,
                    problem: format!(
                        "a second begin marker inside the block that begins at line {begin_line}"
                    ),
                });
            }
            if let Some((_, begin_line, end_line)) = found {
                return Err(MarkerError {
                    line: number,
                    problem: format!(
                        "a second block; the file already has one at lines {begin_line} to {end_line}"
                    ),
                });
            }
            open = Some((offset + lead, offset + line.len(), number));
        } else if marker == END_MARKER {
            let Some((start, inner_start, begin_line)) = open.take() else {
                let problem = match found {
                    Some((_, _, end_line)) => {
                        format!("a second end marker; the block already ends at line {end_line}")
                    }
                    None => "an end marker before any begin marker".to_string(),
                };
                return Err(MarkerError {
                    line: number,
                    problem,
                });
            };
            let bounds = BlockBounds {
                start,
                inner_start,
                inner_end: offset,
                end: offset + line.len(),
            };
            found = Some((bounds, begin_line, number));
        }
        offset += line.len();
    }
    if let Some((_, _, begin_line)) = open {
        return Err(MarkerError {
            line: begin_line,
            problem: "a begin marker with no end marker after it".to_string(),
        });
    }
    Ok(found.map(|(bounds, _, _)| bounds))
}

/// Saved copies are named by the time of the run and never overwritten: a
/// counter is added when the name is taken.
fn save_edited_block(project: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let dir = project.join(SAVED_BLOCKS_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let stem = Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let stamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
    for counter in 1u32.. {
        let file_name = if counter == 1 {
            format!("{stem}-{stamp}.md")
        } else {
            format!("{stem}-{stamp}-{counter}.md")
        };
        let path = dir.join(file_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .with_context(|| format!("failed to write {}", path.display()))?;
                return Ok(path);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("failed to create {}", path.display()));
            }
        }
    }
    bail!("no free name for a saved copy of {name}'s block")
}

fn warn_edited_block(name: &str, copy: &Path, project: &Path, lines_not_kept: &[String]) {
    let copy = copy.strip_prefix(project).unwrap_or(copy);
    eprintln!(
        "Warning: the llm-wiki block in {name} was edited since init wrote it; init replaced \
         it and saved the edited block to {}. Text that belongs to the project goes above or \
         below the block's markers.",
        copy.display()
    );
    if lines_not_kept.is_empty() {
        return;
    }
    eprintln!("Lines of the edited block that the new block does not hold:");
    for line in lines_not_kept.iter().take(MAX_LISTED_LINES) {
        eprintln!("  {line}");
    }
    if lines_not_kept.len() > MAX_LISTED_LINES {
        eprintln!(
            "  ... and {} more, in the saved copy",
            lines_not_kept.len() - MAX_LISTED_LINES
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(inner: &str) -> String {
        format!("{START_MARKER}\n{inner}{END_MARKER}\n")
    }

    #[test]
    fn a_file_without_markers_has_no_block() {
        assert_eq!(find_block("# Title\n\ntext\n"), Ok(None));
    }

    #[test]
    fn the_block_bounds_cover_the_marker_lines() {
        let contents = format!("above\r\n{}below", block("inside\n"));
        let bounds = find_block(&contents).unwrap().unwrap();

        assert_eq!(&contents[..bounds.start], "above\r\n");
        assert_eq!(&contents[bounds.inner_start..bounds.inner_end], "inside\n");
        assert_eq!(&contents[bounds.end..], "below");
        assert_eq!(bounds.end_terminator(&contents), "\n");
    }

    #[test]
    fn an_end_marker_without_a_final_newline_keeps_none() {
        let contents = format!("{START_MARKER}\ninside\n{END_MARKER}");
        let bounds = find_block(&contents).unwrap().unwrap();

        assert_eq!(bounds.end, contents.len());
        assert_eq!(bounds.end_terminator(&contents), "");
    }

    #[test]
    fn broken_markers_name_their_line() {
        let missing_end = format!("one\n{START_MARKER}\ntwo\n");
        assert_eq!(find_block(&missing_end).unwrap_err().line, 2);

        let end_first = format!("{END_MARKER}\n{START_MARKER}\n");
        assert_eq!(find_block(&end_first).unwrap_err().line, 1);

        let two_blocks = format!("{}x\n{}", block("a\n"), block("b\n"));
        let error = find_block(&two_blocks).unwrap_err();
        assert_eq!(error.line, 5);
        assert!(error.problem.contains("a second block"));

        let nested = format!("{START_MARKER}\n{START_MARKER}\n{END_MARKER}\n");
        assert_eq!(find_block(&nested).unwrap_err().line, 2);
    }

    #[test]
    fn a_byte_order_mark_before_the_begin_marker_stays_above_the_block() {
        let contents = format!("{BOM}{}", block("inside\n"));
        let bounds = find_block(&contents).unwrap().unwrap();

        assert_eq!(&contents[..bounds.start], "\u{feff}");
        assert_eq!(&contents[bounds.inner_start..bounds.inner_end], "inside\n");
    }

    #[test]
    fn a_marker_quoted_inside_a_line_is_not_a_marker() {
        let contents = format!("Init's block starts at `{START_MARKER}`.\n");
        assert_eq!(find_block(&contents), Ok(None));
    }

    #[test]
    fn the_guidelines_date_line_is_left_out_of_the_comparison() {
        let old = "# G\n\n- Status: Active\n- Date: 2025-01-01\n\nbody\n";
        let new = "# G\n\n- Status: Active\n- Date: 2026-10-07\n\nbody\n";

        assert!(same_render(RootFile::Guidelines, old, new));
        assert!(!same_render(RootFile::Agents, old, new));
        assert!(!same_render(
            RootFile::Guidelines,
            old,
            "# G\n\n- Status: Draft\n- Date: 2026-10-07\n\nbody\n"
        ));
    }

    #[test]
    fn crlf_line_endings_do_not_change_the_hash() {
        assert_eq!(hash("a\r\nb\r\n"), hash("a\nb\n"));
    }

    #[test]
    fn lines_not_kept_lists_each_new_line_once() {
        let old = "kept\nadded by hand\n\nadded by hand\n";
        assert_eq!(lines_not_kept(old, "kept\n"), vec!["added by hand"]);
    }
}
