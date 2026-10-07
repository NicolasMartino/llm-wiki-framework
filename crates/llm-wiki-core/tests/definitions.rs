//! Integration tests: llm-wiki's type definitions agree with the filename
//! splitter and with one another.

use std::collections::BTreeSet;

use llm_wiki_core::types::llm_wiki::{CORE, FIELDS, ML};
use llm_wiki_core::types::{DocumentType, FilenameError, WikiFilename};

fn all() -> impl Iterator<Item = &'static DocumentType> {
    CORE.iter().chain(ML)
}

#[test]
fn every_suffix_is_one_the_splitter_accepts() -> Result<(), FilenameError> {
    for doc_type in all() {
        let filename = format!("strict-gates.{}", doc_type.suffix);
        let name = WikiFilename::parse(&filename)?;
        assert_eq!(format!("{}.md", name.doc_type()), doc_type.suffix);
        assert_eq!(name.to_string(), filename);
        if doc_type.indexed {
            let indexed = WikiFilename::parse(&format!("03-{filename}"))?;
            assert_eq!(indexed.index(), Some("03"), "{}", doc_type.name);
        }
    }
    Ok(())
}

#[test]
fn the_nine_types_are_distinct() {
    let distinct =
        |part: fn(&DocumentType) -> &'static str| all().map(part).collect::<BTreeSet<_>>().len();
    assert_eq!(all().count(), 9);
    assert_eq!(distinct(|doc_type| doc_type.name), 9);
    assert_eq!(distinct(|doc_type| doc_type.plural), 9);
    assert_eq!(distinct(|doc_type| doc_type.suffix), 9);
    assert_eq!(distinct(|doc_type| doc_type.folder), 9);
}

#[test]
fn each_type_lives_in_the_wiki_folder_named_after_its_pages() {
    for doc_type in all() {
        assert_eq!(
            doc_type.folder,
            format!("wiki/{}", doc_type.plural.to_lowercase())
        );
    }
}

#[test]
fn every_page_carries_the_six_fields_first() {
    let required: Vec<&str> = FIELDS
        .iter()
        .take_while(|field| field.required)
        .map(|field| field.key)
        .collect();
    assert_eq!(
        required,
        [
            "Document Class",
            "Status",
            "Date",
            "Category",
            "Scope",
            "Sources"
        ]
    );
    assert_eq!(FIELDS.iter().filter(|field| field.required).count(), 6);
    for doc_type in all() {
        assert_eq!(doc_type.fields, FIELDS);
        assert!(!doc_type.statuses.is_empty(), "{}", doc_type.name);
    }
}
