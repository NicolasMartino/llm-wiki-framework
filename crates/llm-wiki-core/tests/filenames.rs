//! Integration tests: the filenames this repository's own wiki uses split as
//! its conventions say.

use llm_wiki_core::types::{FilenameError, WikiFilename};

#[test]
fn the_wiki_document_types_split() -> Result<(), FilenameError> {
    for doc_type in [
        "spec",
        "decision",
        "proposal",
        "roadmap",
        "plan",
        "experiment",
        "eval",
        "checklist",
        "reference",
        "deadline",
    ] {
        let filename = format!("poman-lives-in-this-workspace.{doc_type}.md");
        let name = WikiFilename::parse(&filename)?;
        assert_eq!(name.doc_type(), doc_type);
        assert_eq!(name.slug(), "poman-lives-in-this-workspace");
    }
    Ok(())
}

#[test]
fn the_wiki_bookkeeping_files_are_not_pages() {
    for filename in ["index.md", "log.md"] {
        assert_eq!(
            WikiFilename::parse(filename),
            Err(FilenameError::MissingType)
        );
    }
}

#[test]
fn the_error_is_a_standard_error() {
    let error: Box<dyn std::error::Error> = Box::new(FilenameError::NotMarkdown);
    assert_eq!(error.to_string(), "the filename does not end in .md");
}
