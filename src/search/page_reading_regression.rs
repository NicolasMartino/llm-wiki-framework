//! Search reads every page as it did before the page reader moved into
//! `llm-wiki-core`. The `frozen_pages` snapshot was taken with the reader
//! search used then, over the frozen corpus in `tests/fixtures/page-reader/`
//! (see its README), and the reader search uses now must still match it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use llm_wiki_core::page::Page;
use proptest::prelude::*;

use crate::search::index_text::mask_search_ignored_spans;
use crate::search::metadata::parse_wiki_metadata;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/page-reader")
}

/// Every Markdown file under `dir`, sorted, so the snapshot keeps one order.
fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// What search reads of a page: masked as both search callers mask it, then
/// its title and its fields.
fn read_page(text: &str) -> (Option<String>, BTreeMap<String, String>) {
    let metadata = parse_wiki_metadata(&mask_search_ignored_spans(text));
    (metadata.title, metadata.fields)
}

#[test]
fn frozen_pages_read_as_before() {
    let root = corpus_root();
    let mut pages = String::new();
    for path in markdown_files(&root) {
        let name = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let (title, fields) = read_page(&fs::read_to_string(&path).unwrap());
        writeln!(pages, "{name}").unwrap();
        writeln!(pages, "  title: {title:?}").unwrap();
        for (key, value) in fields {
            writeln!(pages, "  {key:?}: {value:?}").unwrap();
        }
    }
    insta::assert_snapshot!("frozen_pages", pages);
}

/// The old reader and the shared crate's wiki view, on the same masked text.
fn both_readers(text: &str) -> [(Option<String>, BTreeMap<String, String>); 2] {
    let masked = mask_search_ignored_spans(text);
    let old = parse_wiki_metadata(&masked);
    let new = Page::read(&masked).wiki_view();
    [
        (old.title, old.fields),
        (new.title().map(ToString::to_string), new.fields().clone()),
    ]
}

#[test]
fn both_readers_agree_on_the_live_wiki_and_every_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = markdown_files(&root.join("wiki"));
    pages.extend(markdown_files(&root.join("tests/fixtures")));
    for path in &pages {
        let [old, new] = both_readers(&fs::read_to_string(path).unwrap());
        assert_eq!(old, new, "{}", path.display());
    }
    eprintln!("both readers agree on {} pages", pages.len());
}

const FORMS: &[&str] = &[
    "---",
    "  ---",
    "# Title",
    "   #  Spaced Title  ",
    "#",
    "#NoSpace",
    "## Section",
    "- Status: Active",
    "Status: Draft",
    "**Status:** Bold",
    "- **Status**: Bold bullet",
    "Status:**bold value",
    "- Document Class: Plan",
    "Category:",
    "- : empty key",
    "**: bold empty key",
    "tags:",
    "  - item",
    "  continued",
    "\tcontinued by a tab",
    " one space",
    "   ",
    "",
    "Prose with a ratio of 4:1.",
    "Prose without a colon.",
    "<!-- llm-wiki-search-ignore-start -->",
    "<!-- llm-wiki-search-ignore-end -->",
    "\u{e9}\u{20ac}\u{1f600} Wide: chars",
    "<!-- llm-wiki-search-ignore-start -->\u{e9}<!-- llm-wiki-search-ignore-end -->Key: value",
];

fn page_line() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => proptest::sample::select(FORMS).prop_map(ToString::to_string),
        1 => "(- |\\*\\*)?[A-Za-z ]{0,8}(\\*\\*)?:(\\*\\*)? ?[a-z: ]{0,8}",
        1 => "( {0,3}|\t)[a-z#:-]{0,6}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn both_readers_agree_on_generated_pages(
        lines in proptest::collection::vec(page_line(), 0..16),
        crlf in any::<bool>(),
    ) {
        let text = lines.join(if crlf { "\r\n" } else { "\n" });
        let [old, new] = both_readers(&text);
        prop_assert_eq!(old, new, "{:?}", text);
    }
}
