//! The document types `llm-wiki-core` defines and the `project_guidelines.md`
//! that `llm-wiki init` renders say the same thing, section by section and
//! both ways: every type a section names is defined, and every definition the
//! section should name is named, with the same suffix, folder, index, fields
//! and statuses. The guidelines are read, not rendered from the definitions:
//! they stay a template people edit.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use llm_wiki_core::types::DocumentType;
use llm_wiki_core::types::llm_wiki::{CORE, FIELDS, ML};
use tempfile::TempDir;

mod support;

/// An initialised project's guidelines, and the `wiki/` folders init made.
struct Render {
    guidelines: String,
    wiki_folders: BTreeSet<String>,
}

fn init(blueprint: &str, packs: &[&str]) -> Render {
    let project = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("home");
    assert_cmd::Command::new(support::llm_wiki_bin())
        .env("HOME", home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            blueprint,
        ])
        .args(packs.iter().flat_map(|pack| ["--pack", *pack]))
        .assert()
        .success();
    let guidelines =
        fs::read_to_string(project.path().join("project_guidelines.md")).expect("guidelines");
    let wiki_folders = fs::read_dir(project.path().join("wiki"))
        .expect("wiki")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_dir())
        .map(|path| folder_name(&path))
        .collect();
    Render {
        guidelines,
        wiki_folders,
    }
}

fn folder_name(path: &Path) -> String {
    format!("wiki/{}", path.file_name().expect("name").to_string_lossy())
}

/// The lines under `heading` up to the next heading of its level or above.
fn section<'text>(text: &'text str, heading: &str) -> Option<Vec<&'text str>> {
    let level = heading.find(' ').expect("a heading");
    let mut lines = text.lines().skip_while(|line| *line != heading);
    lines.next()?;
    Some(
        lines
            .take_while(|line| {
                let hashes = line.chars().take_while(|c| *c == '#').count();
                hashes == 0 || hashes > level || !line[hashes..].starts_with(' ')
            })
            .collect(),
    )
}

/// The lines after the line `label`, up to the first blank line after a
/// line that is not, or the closing fence of a code block.
fn after<'text>(lines: &[&'text str], label: &str) -> Vec<&'text str> {
    let mut rest = lines.iter().skip_while(|line| **line != label).skip(1);
    let mut found = Vec::new();
    let mut in_fence = false;
    for line in rest.by_ref() {
        if line.starts_with("```") {
            if in_fence {
                break;
            }
            in_fence = true;
            continue;
        }
        if line.trim().is_empty() {
            if found.is_empty() || in_fence {
                continue;
            }
            break;
        }
        found.push(*line);
    }
    found
}

/// The rows of the first Markdown table in `lines`, header and rule left out,
/// each split into its trimmed cells.
fn table_rows(lines: &[&str]) -> Vec<Vec<String>> {
    lines
        .iter()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

/// A code span's text: `` `x` `` gives `x`.
fn unquote(text: &str) -> &str {
    text.trim().trim_matches('`')
}

/// The folders a code block's tree shows at `indent` under `wiki/`.
fn tree_folders(lines: &[&str], indent: usize) -> BTreeSet<String> {
    lines
        .iter()
        .filter(|line| {
            line.len() > indent
                && line[..indent].trim().is_empty()
                && !line[indent..].starts_with(' ')
        })
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|name| name.strip_suffix('/'))
        .map(|name| format!("wiki/{name}"))
        .collect()
}

fn names(types: &[&DocumentType], part: fn(&DocumentType) -> &str) -> BTreeSet<String> {
    types
        .iter()
        .map(|doc_type| part(doc_type).to_string())
        .collect()
}

/// Records what a section shows that no definition gives, and what a
/// definition gives that the section does not show.
fn compare(
    problems: &mut Vec<String>,
    what: &str,
    shown: BTreeSet<String>,
    defined: BTreeSet<String>,
) {
    for name in shown.difference(&defined) {
        problems.push(format!("{what}: shows {name}, which no definition gives"));
    }
    for name in defined.difference(&shown) {
        problems.push(format!(
            "{what}: does not show {name}, which a definition gives"
        ));
    }
}

/// Each place the rendered guidelines disagree with the definitions.
fn disagreements(render: &Render, with_ml: bool) -> Vec<String> {
    let text = &render.guidelines;
    let mut problems = Vec::new();
    let core: Vec<&DocumentType> = CORE.iter().collect();
    let all: Vec<&DocumentType> = CORE.iter().chain(ML).collect();
    let rendered: &[&DocumentType] = if with_ml { &all } else { &core };
    let ml: Vec<&DocumentType> = ML.iter().collect();

    let suffixes = |lines: &[&str]| -> BTreeSet<String> {
        table_rows(lines)
            .iter()
            .map(|row| unquote(&row[0]).trim_start_matches("*.").to_string())
            .collect()
    };
    let core_table = section(text, "### Core Document Types (all projects)").unwrap_or_default();
    compare(
        &mut problems,
        "Core Document Types",
        suffixes(&core_table),
        names(&core, |doc_type| doc_type.suffix),
    );
    match section(text, "### ML/AI Document Types (ML/AI projects only)") {
        Some(ml_table) if with_ml => compare(
            &mut problems,
            "ML/AI Document Types",
            suffixes(&ml_table),
            names(&ml, |doc_type| doc_type.suffix),
        ),
        Some(_) => problems.push("ML/AI Document Types: shown without the ML pack".into()),
        None if with_ml => problems.push("ML/AI Document Types: missing with the ML pack".into()),
        None => {}
    }

    let roles = section(text, "## Document Roles").unwrap_or_default();
    compare(
        &mut problems,
        "Short version",
        roles
            .iter()
            .skip_while(|line| **line != "Short version:")
            .filter_map(|line| line.strip_prefix("- "))
            .filter_map(|line| line.split(" - ").next())
            .map(ToString::to_string)
            .collect(),
        names(rendered, |doc_type| doc_type.name),
    );

    // The folders that are not a type's here: init's own `archive/`, and with
    // the ML pack its `model-cards/`, whose Model Card type stays in
    // `src/init/packs.rs`. The folders init made must be these too, so the
    // ML pack's folders there are held to the same list.
    let mut folders = names(rendered, |doc_type| doc_type.folder);
    folders.insert("wiki/archive".to_string());
    if with_ml {
        folders.insert("wiki/model-cards".to_string());
    }
    compare(
        &mut problems,
        "the wiki folders init made",
        render.wiki_folders.clone(),
        folders.clone(),
    );
    let wiki_tree = section(text, "## Wiki Folder Structure").unwrap_or_default();
    compare(
        &mut problems,
        "Wiki Folder Structure",
        tree_folders(&wiki_tree, 2),
        folders.clone(),
    );
    let repository = section(text, "## Full Repository Structure").unwrap_or_default();
    let under_wiki: Vec<&str> = repository
        .iter()
        .skip_while(|line| !line.starts_with("  wiki/"))
        .skip(1)
        .take_while(|line| line.starts_with("    ") || line.trim().is_empty())
        .copied()
        .collect();
    compare(
        &mut problems,
        "Full Repository Structure",
        tree_folders(&under_wiki, 4),
        folders,
    );

    // The template writes these whatever the packs: all nine types.
    let naming = section(text, "## Naming And Metadata").unwrap_or_default();
    let patterns: BTreeSet<String> = after(&naming, "Filename patterns:")
        .iter()
        .map(|line| {
            let (indexed, suffix) = match line.strip_prefix("[index]-[slug].") {
                Some(suffix) => (true, suffix),
                None => (false, line.trim_start_matches("[slug].")),
            };
            format!("{suffix} indexed={indexed}")
        })
        .collect();
    compare(
        &mut problems,
        "Filename patterns",
        patterns,
        all.iter()
            .map(|doc_type| format!("{} indexed={}", doc_type.suffix, doc_type.indexed))
            .collect(),
    );

    let metadata = after(&naming, "Metadata block for all wiki documents:");
    let classes = metadata
        .iter()
        .find_map(|line| line.strip_prefix("- Document Class: ["))
        .and_then(|line| line.strip_suffix(']'))
        .unwrap_or_default();
    compare(
        &mut problems,
        "Document Class list",
        classes.split(" / ").map(ToString::to_string).collect(),
        names(&all, |doc_type| doc_type.name),
    );
    let required: Vec<String> = metadata
        .iter()
        .filter_map(|line| line.strip_prefix("- "))
        .filter_map(|line| line.split_once(':'))
        .map(|(key, _)| key.to_string())
        .collect();
    let optional: Vec<String> = after(&naming, "Optional fields:")
        .iter()
        .filter_map(|line| line.split_once(". ").map(|(_, entry)| entry))
        .map(|entry| entry.split(" (").next().unwrap_or(entry))
        .flat_map(|entry| entry.split(" / "))
        .map(|key| unquote(key).to_string())
        .collect();
    let fields = |required_or_not: bool| -> Vec<String> {
        FIELDS
            .iter()
            .filter(|field| field.required == required_or_not)
            .map(|field| field.key.to_string())
            .collect()
    };
    if required != fields(true) {
        problems.push(format!(
            "metadata block: shows {required:?}, the definitions give {:?}",
            fields(true)
        ));
    }
    if optional != fields(false) {
        problems.push(format!(
            "optional fields: shows {optional:?}, the definitions give {:?}",
            fields(false)
        ));
    }
    for doc_type in &all {
        if doc_type.fields != FIELDS {
            problems.push(format!(
                "{}: its fields are not the guidelines'",
                doc_type.name
            ));
        }
    }

    let statuses: BTreeSet<String> =
        table_rows(&section(text, "## Status Vocabulary").unwrap_or_default())
            .iter()
            .map(|row| {
                let statuses: Vec<&str> = row[1].split(", ").map(unquote).collect();
                format!("{}: {}", row[0], statuses.join(", "))
            })
            .collect();
    compare(
        &mut problems,
        "Status Vocabulary",
        statuses,
        all.iter()
            .map(|doc_type| format!("{}: {}", doc_type.plural, doc_type.statuses.join(", ")))
            .collect(),
    );
    problems
}

#[test]
fn the_guidelines_without_the_ml_pack_agree_with_the_definitions() {
    let render = init("generic", &[]);
    assert_eq!(disagreements(&render, false), Vec::<String>::new());
}

#[test]
fn the_guidelines_with_the_ml_pack_agree_with_the_definitions() {
    let render = init("custom", &["ml"]);
    assert_eq!(disagreements(&render, true), Vec::<String>::new());
}
