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

#[test]
fn search_all_rejects_unknown_excluded_project() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["search-all", "reciprocal rank", "--exclude", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "project id missing is not registered",
        ));
}

#[test]
fn search_refuses_missing_index() {
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

#[test]
fn search_refuses_missing_project_root() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);
    fs::remove_dir_all(&project).expect("remove project root");

    llm_wiki(home.path())
        .args(["search", "reciprocal rank", "--project", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("project root missing for fixture"))
        .stderr(predicate::str::contains("llm-wiki forget fixture"));
}

#[test]
fn search_all_refuses_missing_project_root() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Alpha Decision",
        "Shared retrieval token appears in alpha project.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Beta Decision",
        "Shared retrieval token appears in beta project.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");
    llm_wiki(home.path())
        .args(["index-all", "--force"])
        .assert()
        .success();
    fs::remove_dir_all(&beta).expect("remove beta root");

    llm_wiki(home.path())
        .args(["search-all", "shared retrieval token"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("project root missing for beta"))
        .stderr(predicate::str::contains("llm-wiki forget beta"));
}

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

#[test]
fn projects_reports_fresh_and_stale_index_status() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"index_status\": \"index-present\"",
        ))
        .stdout(predicate::str::contains("\"freshness\": \"fresh\""));

    fs::write(
        project.join("wiki/plans/new.plan.md"),
        "# New Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nNew stale content.",
    )
    .expect("new plan");

    llm_wiki(home.path())
        .arg("projects")
        .assert()
        .success()
        .stdout(predicate::str::contains("index_status"))
        .stdout(predicate::str::contains("stale"));

    llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"freshness\": \"stale\""));
}

#[test]
fn stale_search_reports_warning_and_stale_result_freshness() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    fs::write(
        project.join("wiki/plans/new.plan.md"),
        "# New Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nNew stale content.",
    )
    .expect("new plan");

    llm_wiki(home.path())
        .args(["search", "reciprocal rank", "--project", "fixture"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Warning: search index stale"))
        .stdout(predicate::str::contains("freshness=stale"));

    llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"warning\": \"search index stale",
        ))
        .stdout(predicate::str::contains("\"freshness\": \"stale\""));
}

#[cfg(unix)]
#[test]
fn failed_force_index_preserves_previous_store() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let bad_file = project.join("wiki/decisions/unreadable.decision.md");
    fs::write(&bad_file, "# Bad\n\nThis file cannot be read.").expect("bad file");
    let mut permissions = fs::metadata(&bad_file).expect("metadata").permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&bad_file, permissions).expect("chmod unreadable");

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .failure();

    let mut permissions = fs::metadata(&bad_file).expect("metadata").permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&bad_file, permissions).expect("chmod readable");
    fs::remove_file(&bad_file).expect("remove bad file");

    llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"path\": \"wiki/decisions/search.decision.md\"",
        ));
}

#[test]
fn index_all_and_search_all_fuse_registered_projects() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Alpha Decision",
        "Shared retrieval token appears in alpha project.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Beta Decision",
        "Shared retrieval token appears in beta project.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");

    llm_wiki(home.path())
        .args(["index-all", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexed project: alpha"))
        .stdout(predicate::str::contains("Indexed project: beta"))
        .stdout(predicate::str::contains("Indexed 2 of 2 projects."));

    llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project_id\": \"alpha\""))
        .stdout(predicate::str::contains("\"project_id\": \"beta\""));

    llm_wiki(home.path())
        .args([
            "search-all",
            "shared retrieval token",
            "--include",
            "alpha",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project_id\": \"alpha\""))
        .stdout(predicate::str::contains("\"project_id\": \"beta\"").not());

    llm_wiki(home.path())
        .args([
            "search-all",
            "shared retrieval token",
            "--exclude",
            "beta",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project_id\": \"alpha\""))
        .stdout(predicate::str::contains("\"project_id\": \"beta\"").not());
}

fn register_project(home: &Path, project: &Path) {
    register_project_with_id(home, project, "fixture");
}

fn register_project_with_id(home: &Path, project: &Path, id: &str) {
    llm_wiki(home)
        .args(["register", "--id", id, "--name", id])
        .arg(project)
        .assert()
        .success();
}

fn fixture_project(root: &Path, name: &str) -> PathBuf {
    fixture_project_with_decision(
        root,
        name,
        "Search Decision",
        "Reciprocal rank fusion keeps search-all result ordering deterministic.",
    )
}

fn fixture_project_with_decision(root: &Path, name: &str, title: &str, body: &str) -> PathBuf {
    let project = root.join(name);
    fs::create_dir_all(project.join("wiki/decisions")).expect("wiki");
    fs::create_dir_all(project.join("wiki/plans")).expect("plans");
    fs::write(project.join("wiki/index.md"), "# Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    fs::write(project.join("AGENTS.md"), "# Agents\n").expect("agents");
    fs::write(
        project.join("wiki/decisions/search.decision.md"),
        format!(
            "# {title}\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-07\n- Category: Search\n- Scope: Test\n- Sources: raw/test.md\n\n## Decision\n{body}"
        ),
    )
    .expect("decision");
    fs::write(
        project.join("wiki/plans/search.plan.md"),
        "# Search Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nSearch work that should be filtered out.",
    )
    .expect("plan");
    project.canonicalize().expect("canonical")
}
