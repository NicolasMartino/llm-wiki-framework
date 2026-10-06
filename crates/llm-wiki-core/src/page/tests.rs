use super::{Block, Form, Page};

/// Each field as `(key, value, line, block, form)`, in page order.
fn found(text: &str) -> Vec<(String, String, usize, Block, Form)> {
    Page::read(text)
        .fields()
        .iter()
        .map(|field| {
            (
                field.key().to_owned(),
                field.value().to_owned(),
                field.line(),
                field.block(),
                field.form(),
            )
        })
        .collect()
}

fn field(
    key: &str,
    value: &str,
    line: usize,
    block: Block,
    form: Form,
) -> (String, String, usize, Block, Form) {
    (key.to_owned(), value.to_owned(), line, block, form)
}

// The seven tests below came with the reader from llm-wiki's search, inputs
// and expectations as they were.

#[test]
fn parses_h1_and_leading_metadata_block() {
    let metadata = Page::read(
        "# Search Backend Selection\n\n- Document Class: Decision\n- Status: Accepted\n- Sources: wiki/evals/search-backend-selection.eval.md\n  wiki/proposals/search-backend-selection.proposal.md\n\n## Choice\nBody",
    )
    .wiki_view();

    assert_eq!(metadata.title(), Some("Search Backend Selection"));
    assert_eq!(metadata.document_class(), Some("Decision"));
    assert_eq!(metadata.status(), Some("Accepted"));
    assert_eq!(
        metadata.field("Sources"),
        Some(
            "wiki/evals/search-backend-selection.eval.md wiki/proposals/search-backend-selection.proposal.md"
        )
    );
}

#[test]
fn missing_optional_fields_are_absent() {
    let metadata = Page::read("# Untyped\n\nBody").wiki_view();

    assert_eq!(metadata.title(), Some("Untyped"));
    assert_eq!(metadata.document_class(), None);
    assert_eq!(metadata.status(), None);
}

#[test]
fn ignores_later_bullets_outside_metadata_block() {
    let metadata = Page::read(
        "# Title\n\
         \n\
         - Document Class: Plan\n\
         \n\
         ## Later\n\
         \n\
         - Status: Wrong",
    )
    .wiki_view();

    assert_eq!(metadata.document_class(), Some("Plan"));
    assert_eq!(metadata.status(), None);
}

#[test]
fn parses_bare_metadata_block_before_h1() {
    // Real-world externally authored format (the `keto-diet` testbed): a bare
    // `Key: Value` block placed *before* the H1, no bullets.
    let metadata = Page::read(
        "Document Class: Checklist\n\
         Status: Active\n\
         Date: 2026-06-25\n\
         Category: Literature intake\n\
         Scope: Defines how sources are screened.\n\
         \n\
         # Review Source Screening Checklist\n\
         \n\
         Use this checklist before creating a note.",
    )
    .wiki_view();

    assert_eq!(metadata.title(), Some("Review Source Screening Checklist"));
    assert_eq!(metadata.document_class(), Some("Checklist"));
    assert_eq!(metadata.status(), Some("Active"));
    assert_eq!(metadata.field("Category"), Some("Literature intake"));
}

#[test]
fn parses_bold_metadata_variant() {
    let metadata =
        Page::read("# Bold Header\n\n**Document Class:** Spec\n**Status:** Draft\n\nBody")
            .wiki_view();

    assert_eq!(metadata.document_class(), Some("Spec"));
    assert_eq!(metadata.status(), Some("Draft"));
}

#[test]
fn parses_yaml_frontmatter_block() {
    let metadata = Page::read(
        "---\nDocument Class: Decision\nStatus: Accepted\n---\n\n# Frontmatter Doc\n\nBody",
    )
    .wiki_view();

    assert_eq!(metadata.title(), Some("Frontmatter Doc"));
    assert_eq!(metadata.document_class(), Some("Decision"));
    assert_eq!(metadata.status(), Some("Accepted"));
}

#[test]
fn bare_block_does_not_swallow_prose_paragraph() {
    // A non-metadata paragraph directly under the H1 must not be parsed as
    // metadata just because one line happens to contain a colon.
    let metadata = Page::read(
        "# Prose Doc\n\nThis is prose that mentions a ratio of 4:1 in passing.\n\nMore body.",
    )
    .wiki_view();

    assert_eq!(metadata.document_class(), None);
    assert_eq!(metadata.status(), None);
}

#[test]
fn a_prose_line_under_the_title_is_read_as_a_field() {
    let metadata = Page::read("# Prose Doc\n\nA ratio of 4:1 in passing.\n").wiki_view();

    assert_eq!(metadata.field("A ratio of 4"), Some("1 in passing."));
}

#[test]
fn each_field_has_its_line_block_and_form() {
    let page = "---\n\
                Status: Draft\n\
                - Owner: Ana\n\
                ---\n\
                Category: Tools\n\
                \n\
                # Strict Gates\n\
                \n\
                - Document Class: Plan\n\
                **Scope:** The gates\n\
                - **Related**: PM1\n\
                - Sources: a\n\
                \tb\n\
                Date:**2026-10-07\n";
    assert_eq!(
        found(page),
        [
            field("Status", "Draft", 2, Block::FrontMatter, Form::Bare),
            field("Owner", "Ana", 3, Block::FrontMatter, Form::Bullet),
            field("Category", "Tools", 5, Block::BeforeTitle, Form::Bare),
            field("Document Class", "Plan", 9, Block::AfterTitle, Form::Bullet),
            field("Scope", "The gates", 10, Block::AfterTitle, Form::Bold),
            field("Related", "PM1", 11, Block::AfterTitle, Form::Bold),
            field("Sources", "a b", 12, Block::AfterTitle, Form::Bullet),
            field("Date", "2026-10-07", 14, Block::AfterTitle, Form::Bold),
        ]
    );
}

#[test]
fn the_title_has_its_line() {
    let page = Page::read("Intro\n\n#Not a title\n   #  Strict Gates  \n# Second\n");
    let title = page.title().map(|title| (title.text(), title.line()));
    assert_eq!(title, Some(("Strict Gates", 4)));
}

#[test]
fn a_page_with_no_title_reads_from_the_top() {
    assert_eq!(
        found("Status: Active\n  wrapped\nScope: x\n\nLater: no\n"),
        [
            field("Status", "Active wrapped", 1, Block::Untitled, Form::Bare),
            field("Scope", "x", 3, Block::Untitled, Form::Bare),
        ]
    );
    assert_eq!(Page::read("Status: Active\n").title(), None);
}

#[test]
fn a_page_with_no_title_reads_from_after_its_front_matter() {
    assert_eq!(
        found("---\nStatus: Draft\n---\n\nScope: x\n"),
        [
            field("Status", "Draft", 2, Block::FrontMatter, Form::Bare),
            field("Scope", "x", 5, Block::Untitled, Form::Bare),
        ]
    );
}

#[test]
fn the_title_is_looked_for_after_the_front_matter() {
    let page = Page::read("---\n# Inside\nStatus: Draft\nOwner: Ana\n---\n# Outside\n");
    assert_eq!(
        page.title().map(|title| (title.text(), title.line())),
        Some(("Outside", 6))
    );
    assert_eq!(page.wiki_view().status(), Some("Draft"));
}

#[test]
fn front_matter_reads_every_line_up_to_its_closing_line() {
    assert_eq!(
        found("---\nSources: a\n\n  b\nnot a field\n  c\ntags:\n  - x\n---\n# T\n"),
        [
            field("Sources", "a b c", 2, Block::FrontMatter, Form::Bare),
            field("tags", "- x", 7, Block::FrontMatter, Form::Bare),
        ]
    );
}

#[test]
fn front_matter_never_closed_is_not_front_matter() {
    assert_eq!(
        found("---\nStatus: Draft\n\n# T\n- Scope: x\n"),
        [
            field("Status", "Draft", 2, Block::BeforeTitle, Form::Bare),
            field("Scope", "x", 5, Block::AfterTitle, Form::Bullet),
        ]
    );
}

#[test]
fn the_block_before_the_title_stops_at_the_front_matter() {
    assert_eq!(
        found("---\nStatus: Draft\n  ---\nScope: x\n# T\n"),
        [
            field("Status", "Draft", 2, Block::FrontMatter, Form::Bare),
            field("Scope", "x", 4, Block::BeforeTitle, Form::Bare),
        ]
    );
}

#[test]
fn the_block_before_the_title_stops_at_a_line_that_is_neither() {
    assert_eq!(
        found("Status: Draft\nprose\n  wrapped\nScope: x\n  more\n\n\n# T\n"),
        [field("Scope", "x more", 4, Block::BeforeTitle, Form::Bare)]
    );
}

#[test]
fn the_block_after_the_title_starts_with_a_field() {
    assert_eq!(found("# T\n\n  indented\n- Status: Active\n"), []);
    assert_eq!(
        found("# T\n\n\n- Status: Active\n  wrapped\n - one space ends it\n- Scope: x\n"),
        [field(
            "Status",
            "Active wrapped",
            4,
            Block::AfterTitle,
            Form::Bullet
        )]
    );
}

#[test]
fn continuations_join_with_one_space() {
    assert_eq!(
        found("# T\nKey:\n  first\n   \n\tsecond\n"),
        [field(
            "Key",
            "first second",
            2,
            Block::AfterTitle,
            Form::Bare
        )]
    );
}

#[test]
fn a_later_field_overrides_an_earlier_one_in_the_wiki_view() {
    let page = Page::read(
        "---\nStatus: Superseded\nCategory: front\n---\nScope: before\nStatus: Draft\n\n# T\n\n- Status: Active\n- Status: Completed\n",
    );
    let view = page.wiki_view();
    let fields: Vec<(&str, &str)> = view
        .fields()
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    assert_eq!(
        fields,
        [
            ("Category", "front"),
            ("Scope", "before"),
            ("Status", "Completed")
        ]
    );
    assert_eq!(page.fields().len(), 6);
}

#[test]
fn keys_lose_their_marks() {
    assert_eq!(
        found("# T\n- ** Key **  :   **  value  \n"),
        [field("Key", "value", 2, Block::AfterTitle, Form::Bold)]
    );
    assert_eq!(found("# T\n- : empty\n- Scope: x\n"), []);
    assert_eq!(found("# T\n**: empty\n- Scope: x\n"), []);
    assert_eq!(
        found("# T\nKey: a: b\n"),
        [field("Key", "a: b", 2, Block::AfterTitle, Form::Bare)]
    );
}

#[test]
fn windows_line_endings_read_the_same() {
    assert_eq!(
        found("# T\r\n\r\n- Status: Active\r\n  wrapped\r\n"),
        [field(
            "Status",
            "Active wrapped",
            3,
            Block::AfterTitle,
            Form::Bullet
        )]
    );
}

#[test]
fn the_bullet_block_holds_only_bullets_after_the_title() {
    let page = Page::read(
        "---\n- Status: Draft\n---\n- Owner: Ana\n# T\n- Status: Active\nScope: bare\n**Date:** bold\n- Sources: s\n",
    );
    let block = page.bullet_block();
    let keys = |fields: &[&super::Field]| {
        fields
            .iter()
            .map(|field| (field.key().to_owned(), field.line()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(block.fields()),
        [("Status".to_owned(), 6), ("Sources".to_owned(), 9)]
    );
    assert_eq!(
        keys(block.elsewhere()),
        [
            ("Status".to_owned(), 2),
            ("Owner".to_owned(), 4),
            ("Scope".to_owned(), 7),
            ("Date".to_owned(), 8)
        ]
    );
}

#[test]
fn a_page_with_no_title_has_no_bullet_block() {
    let page = Page::read("- Status: Active\n");
    let block = page.bullet_block();
    assert_eq!(block.fields().len(), 0);
    assert_eq!(block.elsewhere().len(), 1);
}

#[test]
fn an_empty_page_has_nothing() {
    assert_eq!(Page::read(""), Page::default());
    assert_eq!(Page::read("").wiki_view(), super::WikiView::default());
}
