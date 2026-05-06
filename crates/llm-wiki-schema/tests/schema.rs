use llm_wiki_schema::{ParseError, Runtime, parse, schema_field_names};

fn valid_doc() -> &'static str {
    r#"---
name: knowledge-query
description: Query the wiki.
runtimes: [claude, codex]
operations: [query]
arguments:
  - name: question
    required: true
    description: Question to answer.
invocation_style: namespace
---
# Knowledge Query

## Purpose

Answer from the wiki.

## Behavior

1. Read the index.
2. Read relevant pages.

## Invocation

`<knowledge-query> what is D8?`

## Notes

Stay inside documented knowledge.
"#
}

#[test]
fn parses_valid_canonical() {
    let doc = parse(valid_doc()).expect("valid doc parses");
    assert_eq!(doc.frontmatter.name, "knowledge-query");
    assert_eq!(
        doc.frontmatter.runtimes,
        vec![Runtime::Claude, Runtime::Codex]
    );
    assert_eq!(doc.body.title, "Knowledge Query");
}

#[test]
fn rejects_malformed_yaml() {
    let err = parse("---\nname: [\n---\n# T\n").expect_err("malformed yaml");
    assert!(matches!(err, ParseError::Frontmatter(_)));
}

#[test]
fn rejects_missing_required_field() {
    let input = valid_doc().replace("description: Query the wiki.\n", "");
    let err = parse(&input).expect_err("missing field");
    assert!(matches!(err, ParseError::Frontmatter(_)));
}

#[test]
fn rejects_unknown_runtime_value() {
    let input = valid_doc().replace("[claude, codex]", "[claude, cursor]");
    let err = parse(&input).expect_err("unknown runtime");
    assert!(matches!(err, ParseError::Frontmatter(_)));
}

#[test]
fn rejects_conflicting_dispatcher_fields() {
    let input = valid_doc().replace("[claude, codex]", "[claude]").replace(
        "operations: [query]",
        "operations: [query]\ndispatcher_for: [query]",
    );
    let err = parse(&input).expect_err("conflicting fields");
    assert!(matches!(err, ParseError::Validation(_)));
}

#[test]
fn rejects_missing_required_body_section() {
    let input = valid_doc().replace("## Invocation\n\n`<knowledge-query> what is D8?`\n\n", "");
    let err = parse(&input).expect_err("missing invocation section");
    assert!(matches!(err, ParseError::MissingSection("Invocation")));
}

#[test]
fn schema_field_set_is_locked() {
    insta::assert_debug_snapshot!(schema_field_names(), @r###"
    [
        "name",
        "description",
        "runtimes",
        "operations",
        "arguments",
        "invocation_style",
        "dispatcher_for",
    ]
    "###);
}
