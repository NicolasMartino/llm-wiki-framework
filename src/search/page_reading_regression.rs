//! Search reads every page as it did before the page reader moved into
//! `llm-wiki-core`. The `frozen_pages` snapshot was taken with the reader
//! search used then, `parse_wiki_metadata`, over the frozen corpus in
//! `tests/fixtures/page-reader/` (see its README), and the reader search uses
//! now must still match it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use llm_wiki_core::page::Page;

use crate::search::index_text::mask_search_ignored_spans;

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
    let metadata = Page::read(&mask_search_ignored_spans(text)).wiki_view();
    (
        metadata.title().map(ToString::to_string),
        metadata.fields().clone(),
    )
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
