//! Property tests: splitting a filename and writing it back gives the same
//! name, and only Markdown names are split.

use llm_wiki_core::types::{FilenameError, WikiFilename};
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
    let names = "[a-z0-9.-]{0,24}".prop_filter("not Markdown", |name| name.strip_suffix(".md").is_none());
    TestRunner::default().run(&names, |name| {
        prop_assert_eq!(
            WikiFilename::parse(&name),
            Err(FilenameError::NotMarkdown)
        );
        Ok(())
    })
}
