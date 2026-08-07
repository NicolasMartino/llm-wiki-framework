use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod expected {
    include!("fixtures/wikis/v1-expected.rs");
}

#[test]
fn v1_fixture_metadata_matches_expected() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wikis/v1");
    let mut actual = BTreeMap::new();
    for path in markdown_docs(&root) {
        let rel = path.strip_prefix(&root).expect("rel").to_path_buf();
        if rel == Path::new("index.md") || rel == Path::new("log.md") {
            continue;
        }
        let raw = fs::read_to_string(&path).expect("read");
        actual.insert(
            rel.to_string_lossy().to_string(),
            (
                metadata_value(&raw, "Document Class").expect("class"),
                metadata_value(&raw, "Status").expect("status"),
            ),
        );
    }

    for expected in expected::EXPECTED {
        let actual = actual.get(expected.path).expect(expected.path);
        assert_eq!(actual.0, expected.class);
        assert_eq!(actual.1, expected.status);
    }
    assert_eq!(actual.len(), expected::EXPECTED.len());
}

#[test]
fn v1_fixture_index_references_operation_specs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wikis/v1");
    let index = fs::read_to_string(root.join("index.md")).expect("index");
    for section in [
        "Specs",
        "Decisions",
        "Roadmaps",
        "References",
        "Proposals",
        "Plans",
        "Checklists",
        "Archive",
    ] {
        assert!(index.contains(&format!("## {section}")));
    }

    for spec in [
        "wiki-init-skill.spec.md",
        "wiki-query-skill.spec.md",
        "wiki-ingest-skill.spec.md",
        "wiki-research-skill.spec.md",
        "wiki-lint-skill.spec.md",
    ] {
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("wiki/specs")
                .join(spec)
                .exists()
        );
    }
}

fn markdown_docs(root: &Path) -> Vec<PathBuf> {
    let mut docs = Vec::new();
    collect(root, &mut docs);
    docs.sort();
    docs
}

fn collect(path: &Path, docs: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            collect(&path, docs);
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            docs.push(path);
        }
    }
}

fn metadata_value(input: &str, key: &str) -> Option<String> {
    let prefix = format!("- {key}: ");
    input
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(ToString::to_string))
}
