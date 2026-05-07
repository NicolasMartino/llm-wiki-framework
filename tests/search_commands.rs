use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .env("HOME", home)
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME");
    command
}

#[cfg(not(feature = "qmd-rs"))]
#[test]
fn index_reports_feature_disabled_in_default_build() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("qmd-rs-feature-disabled"));
}

#[cfg(not(feature = "qmd-rs"))]
#[test]
fn search_reports_feature_disabled_in_default_build() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["search", "reciprocal rank", "--project", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("qmd-rs-feature-disabled"));
}

#[cfg(feature = "qmd-rs")]
#[test]
fn search_refuses_missing_index_in_feature_build() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["search", "reciprocal rank", "--project", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("search index missing"))
        .stderr(predicate::str::contains("llm-wiki index --project fixture"));
}

#[cfg(feature = "qmd-rs")]
#[test]
fn index_and_search_registered_project_with_filters() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexed project: fixture"));

    llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--class",
            "Decision",
            "--status",
            "Accepted",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project_id\": \"fixture\""))
        .stdout(predicate::str::contains(
            "\"path\": \"wiki/decisions/search.decision.md\"",
        ))
        .stdout(predicate::str::contains("\"freshness\": \"fresh\""));
}

fn register_project(home: &Path, project: &Path) {
    llm_wiki(home)
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(project)
        .assert()
        .success();
}

fn fixture_project(root: &Path, name: &str) -> PathBuf {
    let project = root.join(name);
    fs::create_dir_all(project.join("wiki/decisions")).expect("wiki");
    fs::create_dir_all(project.join("wiki/plans")).expect("plans");
    fs::write(project.join("wiki/index.md"), "# Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    fs::write(project.join("AGENTS.md"), "# Agents\n").expect("agents");
    fs::write(
        project.join("wiki/decisions/search.decision.md"),
        "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-07\n- Category: Search\n- Scope: Test\n- Sources: raw/test.md\n\n## Decision\nReciprocal rank fusion keeps search-all result ordering deterministic.",
    )
    .expect("decision");
    fs::write(
        project.join("wiki/plans/search.plan.md"),
        "# Search Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nSearch work that should be filtered out.",
    )
    .expect("plan");
    project.canonicalize().expect("canonical")
}
