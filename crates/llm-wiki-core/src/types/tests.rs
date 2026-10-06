use super::{FilenameError, WikiFilename};

#[test]
fn splits_a_plain_filename() -> Result<(), FilenameError> {
    let name = WikiFilename::parse("poman-workspace.plan.md")?;
    assert_eq!(name.index(), None);
    assert_eq!(name.slug(), "poman-workspace");
    assert_eq!(name.doc_type(), "plan");
    Ok(())
}

#[test]
fn splits_an_indexed_filename() -> Result<(), FilenameError> {
    let name = WikiFilename::parse("03-strict-gates.spec.md")?;
    assert_eq!(name.index(), Some("03"));
    assert_eq!(name.slug(), "strict-gates");
    assert_eq!(name.doc_type(), "spec");
    Ok(())
}

#[test]
fn a_leading_dash_is_part_of_the_slug() -> Result<(), FilenameError> {
    let name = WikiFilename::parse("-draft.plan.md")?;
    assert_eq!(name.index(), None);
    assert_eq!(name.slug(), "-draft");
    Ok(())
}

#[test]
fn digits_without_a_dash_are_the_slug() -> Result<(), FilenameError> {
    let name = WikiFilename::parse("2026.roadmap.md")?;
    assert_eq!(name.index(), None);
    assert_eq!(name.slug(), "2026");
    Ok(())
}

#[test]
fn letters_before_the_dash_are_not_an_index() -> Result<(), FilenameError> {
    let name = WikiFilename::parse("v2-search.decision.md")?;
    assert_eq!(name.index(), None);
    assert_eq!(name.slug(), "v2-search");
    Ok(())
}

#[test]
fn display_writes_the_filename_back() -> Result<(), FilenameError> {
    for filename in ["poman.decision.md", "12-ordered.checklist.md"] {
        assert_eq!(WikiFilename::parse(filename)?.to_string(), filename);
    }
    Ok(())
}

#[test]
fn refuses_each_broken_rule() {
    let cases = [
        ("notes.txt", FilenameError::NotMarkdown),
        ("plans/x.plan.md", FilenameError::HasPathSeparator),
        ("plans\\x.plan.md", FilenameError::HasPathSeparator),
        ("index.md", FilenameError::MissingType),
        ("v1.2.plan.md", FilenameError::ExtraDot),
        ("x..md", FilenameError::InvalidType),
        ("x.Plan.md", FilenameError::InvalidType),
        ("x.plan2.md", FilenameError::InvalidType),
        (".plan.md", FilenameError::EmptySlug),
        ("03-.plan.md", FilenameError::EmptySlug),
    ];
    for (filename, error) in cases {
        assert_eq!(WikiFilename::parse(filename), Err(error), "{filename}");
    }
}

#[test]
fn each_error_says_which_rule() {
    let cases = [
        (
            FilenameError::NotMarkdown,
            "the filename does not end in .md",
        ),
        (
            FilenameError::HasPathSeparator,
            "the filename holds a path separator",
        ),
        (
            FilenameError::MissingType,
            "the filename has no .type before .md",
        ),
        (
            FilenameError::ExtraDot,
            "the filename has more than one . before .md",
        ),
        (
            FilenameError::InvalidType,
            "the filename has a type that is not lowercase letters",
        ),
        (FilenameError::EmptySlug, "the filename has an empty slug"),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
