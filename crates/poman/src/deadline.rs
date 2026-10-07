//! One deadline file's text held to the deadline type: its title, its field
//! block, and each value's form. References between files are checked over
//! the whole tree, in [`crate::check`].

use std::collections::BTreeMap;

use llm_wiki_core::names::near_miss;
use llm_wiki_core::page::{Block, Field, Form, Page};
use llm_wiki_core::types::format::paths;
use llm_wiki_core::types::poman::DEADLINE;

use crate::finding::Finding;

/// The key whose value lists the files a deadline waits on.
pub const BLOCKED_BY: &str = "Blocked by";

/// What one file's `Blocked by` field lists, and its line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Blockers {
    /// The field's line.
    pub line: usize,
    /// The paths it lists, in its order.
    pub paths: Vec<String>,
}

/// Holds the deadline file at `path`, whose text is `text`, to the type: the
/// findings, and its blockers when its `Blocked by` field is well written.
#[must_use]
pub fn check_text(path: &str, text: &str) -> (Vec<Finding>, Option<Blockers>) {
    let page = Page::read(text);
    let block = page.bullet_block();
    if block.byte_order_mark() {
        let finding = Finding::error(
            path,
            1,
            "starts with a byte-order mark; save it without one",
        );
        return (vec![finding], None);
    }
    let Some(title) = page.title() else {
        let finding = Finding::error(path, 1, "has no title; its first line must be `# <title>`");
        return (vec![finding], None);
    };
    let mut findings = Vec::new();
    for field in block.elsewhere() {
        findings.push(Finding::error(
            path,
            field.line(),
            format!(
                "`{}` is written {}; poman reads a field only as a `- Key: Value` line in the block under the title",
                field.key(),
                written(field)
            ),
        ));
    }

    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut blockers = None;
    for field in block.fields() {
        check_field(path, field, &mut seen, &mut blockers, &mut findings);
    }

    let block_missing = block.fields().is_empty()
        && !block
            .elsewhere()
            .iter()
            .any(|field| field.block() == Block::AfterTitle);
    let stray = text
        .lines()
        .enumerate()
        .skip(title.line())
        .find(|(_, line)| !line.trim().is_empty());
    if let Some((index, _)) = stray.filter(|_| block_missing) {
        findings.push(Finding::error(
            path,
            index + 1,
            "this line sits between the title and the field block, so no field below it is read; the `- Key: Value` lines must come right under the title, blank lines apart",
        ));
    } else {
        for field in DEADLINE.fields.iter().filter(|field| field.required) {
            let written_elsewhere = block
                .elsewhere()
                .iter()
                .any(|other| other.key() == field.key);
            if !seen.contains_key(field.key) && !written_elsewhere {
                findings.push(Finding::error(
                    path,
                    title.line(),
                    format!("`{}` is missing", field.key),
                ));
            }
        }
    }
    (findings, blockers)
}

fn check_field<'page>(
    path: &str,
    field: &'page Field,
    seen: &mut BTreeMap<&'page str, usize>,
    blockers: &mut Option<Blockers>,
    findings: &mut Vec<Finding>,
) {
    let key = field.key();
    let value = field.value();
    let Some(definition) = DEADLINE.field(key) else {
        let keys = DEADLINE.fields.iter().map(|field| field.key);
        findings.push(near_miss(key, keys).map_or_else(
            || {
                Finding::error(
                    path,
                    field.line(),
                    format!("`{key}` is not a field of a deadline file; its fields are {}", field_list()),
                )
            },
            |close| {
                Finding::warning(
                    path,
                    field.line(),
                    format!("`{key}` is not a field; it is close to `{close}`, and its value is not read as `{close}`"),
                )
            },
        ));
        return;
    };
    if let Some(first) = seen.get(key) {
        findings.push(Finding::error(
            path,
            field.line(),
            format!("`{key}` is written twice; it is first at line {first}"),
        ));
        return;
    }
    seen.insert(key, field.line());
    if !definition.format.accepts(value) {
        findings.push(Finding::error(
            path,
            field.line(),
            format!(
                "`{key}` is {}, not {}",
                shown(value),
                definition.format.expected()
            ),
        ));
    } else if key == BLOCKED_BY {
        *blockers = paths(value).map(|listed| Blockers {
            line: field.line(),
            paths: listed.into_iter().map(str::to_owned).collect(),
        });
    }
}

/// A value as a message shows it.
fn shown(value: &str) -> String {
    if value.is_empty() {
        "empty".to_owned()
    } else {
        format!("`{value}`")
    }
}

/// Where or how a field outside the bullet block was written.
const fn written(field: &Field) -> &'static str {
    match (field.block(), field.form()) {
        (Block::FrontMatter, _) => "in front matter",
        (Block::BeforeTitle, _) => "before the title",
        (_, Form::Bold) => "in bold",
        (_, Form::Asterisk) => "as a `* ` list item",
        _ => "with no `- ` in front",
    }
}

/// The type's keys, as a sentence lists them.
fn field_list() -> String {
    let count = DEADLINE.fields.len();
    let parts: Vec<String> = DEADLINE
        .fields
        .iter()
        .enumerate()
        .map(|(position, field)| {
            let before = match position {
                0 => "",
                _ if position + 1 == count => " and ",
                _ => ", ",
            };
            format!("{before}`{}`", field.key)
        })
        .collect();
    parts.concat()
}
