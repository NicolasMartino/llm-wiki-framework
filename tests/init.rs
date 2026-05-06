use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn init_project(path: &Path, project_type: &str, scale: &str, extra: &[&str]) {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .arg("init")
        .arg(path)
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--type",
            project_type,
            "--scale",
            scale,
        ])
        .args(extra)
        .assert()
        .success();
}

#[test]
fn init_profiles_match_snapshots() {
    let cases = [
        ("baseline", "web", "small", vec![]),
        ("ml_ai", "ml", "small", vec![]),
        ("qmd", "web", "medium", vec![]),
        ("ml_ai_qmd", "data", "large", vec![]),
        ("is_existing", "web", "small", vec!["--existing"]),
    ];
    for (name, project_type, scale, extra) in cases {
        let temp = TempDir::new().expect("tempdir");
        if name == "is_existing" {
            fs::create_dir_all(temp.path().join("app")).expect("mkdir");
            fs::write(temp.path().join("package.json"), "{}").expect("write");
        }
        init_project(temp.path(), project_type, scale, &extra);
        let snapshot = snapshot_project(temp.path());
        insta::with_settings!({filters => vec![(r"\d{4}-\d{2}-\d{2}", "[date]")]}, {
            insta::assert_snapshot!(format!("init_{name}"), snapshot);
        });
    }
}

#[test]
fn init_refuses_framework_artifact_collision() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("wiki")).expect("mkdir");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--type",
            "web",
            "--scale",
            "small",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("framework artifacts"));
}

#[test]
fn initial_sources_are_copied_without_ingest() {
    let temp = TempDir::new().expect("tempdir");
    let source_dir = TempDir::new().expect("sources");
    let source_a = source_dir.path().join("manifest.md");
    let source_b = source_dir.path().join("b.txt");
    fs::write(&source_a, "# User Manifest").expect("write");
    fs::write(&source_b, "B").expect("write");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--type",
            "web",
            "--scale",
            "small",
            "--initial-sources",
        ])
        .arg(&source_a)
        .arg("--initial-sources")
        .arg(&source_b)
        .assert()
        .success()
        .stdout(predicate::str::contains("Sources copied"));

    let raw_initial = temp.path().join("raw/initial");
    assert!(raw_initial.exists());
    assert!(find_file_with_contents(&raw_initial, "manifest.md", "# User Manifest").is_some());
    assert!(find_file(&raw_initial, "b.txt").is_some());
    assert!(find_file(&raw_initial, "manifest.md").is_some());
    assert!(
        !fs::read_to_string(temp.path().join("wiki/index.md"))
            .expect("index")
            .contains("a.md")
    );
}

fn snapshot_project(path: &Path) -> String {
    let mut output = String::new();
    output.push_str("# files\n");
    for rel in interesting_files(path) {
        output.push_str(&format!("- {}\n", rel.display()));
    }
    output.push_str("\n# project_guidelines.md\n");
    output.push_str(&fs::read_to_string(path.join("project_guidelines.md")).expect("guidelines"));
    output.push_str("\n# CLAUDE.md\n");
    output.push_str(&fs::read_to_string(path.join("CLAUDE.md")).expect("claude"));
    output.push_str("\n# wiki/index.md\n");
    output.push_str(&fs::read_to_string(path.join("wiki/index.md")).expect("index"));
    output
}

fn interesting_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(path, path, &mut files);
    files.sort();
    files
}

fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            files.push(path.strip_prefix(root).expect("rel").to_path_buf());
        }
    }
}

fn find_file(path: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Some(found) = find_file(&entry_path, name) {
                return Some(found);
            }
        } else if entry.file_name() == name {
            return Some(entry_path);
        }
    }
    None
}

fn find_file_with_contents(path: &Path, name: &str, contents: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Some(found) = find_file_with_contents(&entry_path, name, contents) {
                return Some(found);
            }
        } else if entry.file_name() == name
            && fs::read_to_string(&entry_path).ok().as_deref() == Some(contents)
        {
            return Some(entry_path);
        }
    }
    None
}
