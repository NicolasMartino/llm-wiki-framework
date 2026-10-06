//! Property tests: splitting a filename and writing it back gives the same
//! name, and only Markdown names are split; a page's fields come back as they
//! were written, and the two views of a page agree with its fields.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use llm_wiki_core::page::{Block, Form, Page};
use llm_wiki_core::types::{FilenameError, WikiFilename};
use proptest::collection::vec;
use proptest::prelude::{Strategy, prop_assert, prop_assert_eq};
use proptest::test_runner::{TestError, TestRunner};

#[test]
fn a_split_filename_writes_back_unchanged() -> Result<(), TestError<String>> {
    let filenames = "([0-9]{1,3}-)?[a-z0-9-]{0,23}[a-z0-9]\\.[a-z]{1,12}\\.md";
    TestRunner::default().run(&filenames, |filename| {
        let name = WikiFilename::parse(&filename);
        prop_assert!(name.is_ok(), "{filename}: {name:?}");
        if let Ok(name) = name {
            prop_assert_eq!(name.to_string(), filename);
            prop_assert_eq!(WikiFilename::parse(&name.to_string()), Ok(name));
        }
        Ok(())
    })
}

#[test]
fn a_name_not_ending_in_md_is_refused() -> Result<(), TestError<String>> {
    let names =
        "[a-z0-9.-]{0,24}".prop_filter("not Markdown", |name| name.strip_suffix(".md").is_none());
    TestRunner::default().run(&names, |name| {
        prop_assert_eq!(WikiFilename::parse(&name), Err(FilenameError::NotMarkdown));
        Ok(())
    })
}

/// A generated page's title and its fields' keys and values.
type WrittenPage = (String, Vec<(String, String)>);

#[test]
fn a_written_bullet_block_reads_back() -> Result<(), TestError<WrittenPage>> {
    let title = "[A-Za-z][A-Za-z0-9 ]{0,20}[A-Za-z0-9]";
    let field = (
        "[A-Z][A-Za-z ]{0,12}[a-z]",
        "([A-Za-z0-9`./][A-Za-z0-9`./ -]{0,20})?[A-Za-z0-9]",
    );
    TestRunner::default().run(&(title, vec(field, 0..8)), |(title, fields)| {
        let mut text = format!("# {title}\n\n");
        for (key, value) in &fields {
            let _ = writeln!(text, "- {key}: {value}");
        }
        text.push_str("\n## Body\n\n- Not: a field\n");
        let page = Page::read(&text);
        prop_assert_eq!(
            page.title().map(|title| (title.text(), title.line())),
            Some((title.as_str(), 1))
        );
        let block = page.bullet_block();
        let read: Vec<(&str, &str, usize)> = block
            .fields()
            .iter()
            .map(|field| (field.key(), field.value(), field.line()))
            .collect();
        let written: Vec<(&str, &str, usize)> = fields
            .iter()
            .zip(3..)
            .map(|((key, value), line)| (key.as_str(), value.as_str(), line))
            .collect();
        prop_assert_eq!(read, written);
        prop_assert_eq!(block.elsewhere().len(), 0);
        Ok(())
    })
}

#[test]
fn the_views_agree_with_the_fields() -> Result<(), TestError<Vec<String>>> {
    let line = "(---|# [A-Z][a-z]{0,6}|- [A-Z][a-z]{0,3}: [a-z]{0,4}|\\*\\*[A-Z][a-z]{0,3}:\\*\\* [a-z]{0,4}|[A-Z][a-z]{0,3}: [a-z]{0,4}|  [a-z]{0,4}|\t[a-z]{1,4}|[a-z ]{0,8}|)";
    TestRunner::default().run(&vec(line, 0..14), |lines| {
        let text = lines.join("\n");
        let page = Page::read(&text);
        let mut fields = BTreeMap::new();
        for field in page.fields() {
            let written = text.lines().nth(field.line() - 1).unwrap_or_default();
            prop_assert!(written.contains(field.key()), "{field:?} on {written:?}");
            fields.insert(field.key().to_owned(), field.value().to_owned());
        }
        let view = page.wiki_view();
        prop_assert_eq!(view.fields(), &fields);
        prop_assert_eq!(
            view.title(),
            page.title().map(llm_wiki_core::page::Title::text)
        );
        let block = page.bullet_block();
        prop_assert_eq!(
            block.fields().len() + block.elsewhere().len(),
            page.fields().len()
        );
        for field in block.fields() {
            prop_assert_eq!(
                (field.block(), field.form()),
                (Block::AfterTitle, Form::Bullet)
            );
        }
        Ok(())
    })
}
