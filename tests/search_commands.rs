use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .env("HOME", home)
        .env_remove("RUST_LOG")
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
fn search_requires_base_install() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "fixture"])
        .arg(project)
        .assert()
        .success();

    llm_wiki(home.path())
        .args(["search", "reciprocal rank", "--project", "fixture"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("llm-wiki install is required"))
        .stderr(predicate::str::contains("llm-wiki install"));
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
fn verbose_index_commands_emit_diagnostics() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["--verbose", "index", "--project", "fixture", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: index"))
        .stderr(predicate::str::contains("selected project: fixture"))
        .stderr(predicate::str::contains("store path:"))
        .stderr(predicate::str::contains("lock path:"))
        .stderr(predicate::str::contains("temp store:"))
        .stderr(predicate::str::contains("indexed files:"));

    llm_wiki(home.path())
        .args(["--verbose", "index-all", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: index-all"))
        .stderr(predicate::str::contains("registered projects: 1"))
        .stderr(predicate::str::contains("index-all project: fixture"))
        .stderr(predicate::str::contains(
            "per-project outcome fixture: indexed",
        ));
}

#[test]
fn non_verbose_search_stdout_contract_stays_clean() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let search = llm_wiki(home.path())
        .args(["search", "reciprocal rank fusion", "--project", "fixture"])
        .output()
        .expect("search output");
    assert!(search.status.success());
    let stdout = String::from_utf8_lossy(&search.stdout);
    let stderr = String::from_utf8_lossy(&search.stderr);
    assert!(stdout.contains("1. [fixture] Search Decision"));
    assert!(!stdout.contains("registry:"));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");

    let search_all = llm_wiki(home.path())
        .args(["search-all", "reciprocal rank fusion"])
        .output()
        .expect("search-all output");
    assert!(search_all.status.success());
    let stdout = String::from_utf8_lossy(&search_all.stdout);
    let stderr = String::from_utf8_lossy(&search_all.stderr);
    assert!(stdout.contains("1. [fixture] Search Decision"));
    assert!(!stdout.contains("registry:"));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}

#[test]
fn verbose_search_emits_diagnostics_on_stderr() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "-v",
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("1. [fixture] Search Decision"));
    assert!(stderr.contains("command: search"));
    assert!(stderr.contains("registry:"));
    assert!(stderr.contains("project selection: --project"));
    assert!(stderr.contains("selected project: fixture"));
    assert!(stderr.contains("index store:"));
    assert!(stderr.contains("index status: ready"));
    assert!(stderr.contains("fts query: reciprocal rank fusion"));
    assert!(stderr.contains("results: 1"));
}

#[test]
fn verbose_global_flag_is_accepted_after_subcommand() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "-v",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("command: search"));
    assert!(stderr.contains("selected project: fixture"));
}

#[test]
fn verbose_search_json_keeps_stdout_parseable() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "-v",
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["project_id"], "fixture");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("registry:"));
    assert!(stderr.contains("registry:"));
    assert!(stderr.contains("results: 1"));
    assert!(
        !stdout.contains("\u{1b}["),
        "JSON stdout must not contain ANSI escapes"
    );
}

#[test]
fn search_json_reports_auto_lexical_mode_reason() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["requested_mode"], "auto");
    assert_eq!(json["selected_mode"], "lexical");
    assert_eq!(json["mode_selection_reason"], "install_profile_missing");
}

#[test]
fn explicit_hybrid_without_fallback_reports_readiness_json() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    let output = llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("readiness json");
    assert_eq!(json["requested_mode"], "hybrid");
    assert!(json["selected_mode"].is_null());
    assert_eq!(json["readiness_reason"], "install_profile_missing");
}

#[test]
fn explicit_hybrid_with_fallback_uses_lexical() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--mode",
            "hybrid",
            "--allow-lexical-fallback",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["requested_mode"], "hybrid");
    assert_eq!(json["selected_mode"], "lexical");
    assert_eq!(json["fallback_reason"], "install_profile_missing");
}

#[test]
fn verbose_search_reports_zero_result_reason() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args(["-v", "search", "missingtoken", "--project", "fixture"])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("No results."));
    assert!(stderr.contains("results: 0"));
    assert!(stderr.contains("no-result: backend returned zero hits before filters"));
}

#[test]
fn verbose_search_reports_empty_sanitized_query_reason() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args(["-v", "search", "!!!", "--project", "fixture"])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("No results."));
    assert!(stderr.contains("results: 0"));
    assert!(stderr.contains("no-result: zero terms after FTS sanitization"));
}

#[test]
fn verbose_search_reports_filter_exclusion_reason() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "-v",
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--class",
            "NoSuchClass",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("No results."));
    assert!(stderr.contains("results: 0"));
    assert!(stderr.contains("no-result: filters excluded all matched hits"));
}

#[test]
fn verbose_search_reports_limit_zero_reason() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "-v",
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--limit",
            "0",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("No results."));
    assert!(stderr.contains("results: 0"));
    assert!(stderr.contains("no-result: limit was 0"));
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

#[test]
fn verbose_search_all_reports_project_diagnostics() {
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

    let output = llm_wiki(home.path())
        .args(["-v", "search-all", "shared retrieval token"])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("[alpha] Alpha Decision"));
    assert!(stdout.contains("[beta] Beta Decision"));
    assert!(stderr.contains("command: search-all"));
    assert!(stderr.contains("selected projects: alpha, beta"));
    assert!(stderr.contains("project alpha:"));
    assert!(stderr.contains("project beta:"));
    assert!(stderr.contains("index status alpha: ready"));
    assert!(stderr.contains("index status beta: ready"));
    assert!(stderr.contains("per-project results alpha: 1"));
    assert!(stderr.contains("per-project results beta: 1"));
    assert!(stderr.contains("fused results: 2"));
}

#[test]
fn search_all_honors_limits_above_default_per_project_fetch() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    for index in 0..25 {
        fs::write(
            project.join(format!("wiki/decisions/bulk-{index:02}.decision.md")),
            format!(
                "# Bulk Decision {index:02}\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-09\n- Category: Search\n- Scope: Test\n- Sources: raw/test.md\n\n## Decision\nNeedle limit expansion token {index:02}."
            ),
        )
        .expect("bulk decision");
    }
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index-all", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search-all",
            "needle limit expansion",
            "--limit",
            "25",
            "--format",
            "json",
        ])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");

    assert_eq!(json["results"].as_array().expect("results").len(), 25);
}

#[test]
fn search_json_envelope_has_all_contract_fields() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search",
            "reciprocal rank fusion",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");

    assert_eq!(json["query"], "reciprocal rank fusion");
    assert_eq!(json["project_id"], "fixture");
    assert_eq!(json["project_name"], "fixture");
    assert_eq!(json["requested_mode"], "auto");
    assert_eq!(json["selected_mode"], "lexical");
    assert!(json.get("warning").is_some());
    assert!(json.get("warnings").is_some());
    let result = json["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("first result");
    for field in [
        "project_id",
        "project_name",
        "path",
        "title",
        "document_class",
        "status",
        "score",
        "snippet",
        "backend",
        "mode",
        "freshness",
    ] {
        assert!(result.get(field).is_some(), "missing result field {field}");
    }
}

#[test]
fn search_all_warnings_are_structured_per_project() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    for project_id in ["alpha", "beta", "gamma", "delta", "epsilon"] {
        let project = fixture_project_with_decision(
            workspace.path(),
            &format!("{project_id} project"),
            &format!("{project_id} decision"),
            "Shared retrieval token appears here.",
        );
        register_project_with_id(home.path(), &project, project_id);
    }

    llm_wiki(home.path())
        .args(["index-all", "--force"])
        .assert()
        .success();

    for project_id in ["beta", "gamma", "delta", "epsilon"] {
        fs::write(
            workspace
                .path()
                .join(format!("{project_id} project/wiki/plans/stale.plan.md")),
            "# Stale Plan\n\n- Document Class: Plan\n- Status: Active\n\nShared retrieval token changed.",
        )
        .expect("stale write");
    }

    let output = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");

    assert!(json["warning"].is_null());
    let warnings = json["warnings"].as_array().expect("warnings array");
    assert_eq!(warnings.len(), 4);
    for warning in warnings {
        assert!(warning["project_id"].is_string());
        assert!(
            warning["message"]
                .as_str()
                .is_some_and(|msg| msg.contains("search index stale"))
        );
    }
}

#[test]
fn search_all_rrf_ordering_is_deterministic() {
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
        "Shared retrieval token appears in beta project. Shared retrieval token also appears here.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");

    llm_wiki(home.path())
        .args(["index-all", "--force"])
        .assert()
        .success();

    let first = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("first output");
    let second = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("second output");
    assert!(first.status.success());
    assert!(second.status.success());

    let first_json: Value = serde_json::from_slice(&first.stdout).expect("first json");
    let second_json: Value = serde_json::from_slice(&second.stdout).expect("second json");
    assert_eq!(first_json["results"], second_json["results"]);
}

#[test]
fn search_retries_once_during_concurrent_promotion() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    fs::write(
        project.join("wiki/decisions/search.decision.md"),
        "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-07\n- Category: Search\n- Scope: Test\n- Sources: raw/test.md\n\n## Decision\nReciprocal rank fusion changes during promotion.",
    )
    .expect("decision update");

    let binary = assert_cmd::cargo::cargo_bin("llm-wiki");
    let marker = workspace.path().join("promote.marker");
    let mut search = StdCommand::new(&binary);
    let mut index = StdCommand::new(&binary);
    let index = index
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env("LLM_WIKI_TEST_PROMOTE_MARKER", &marker)
        .env("LLM_WIKI_TEST_PROMOTE_PRE_COMMIT_SLEEP_MS", "200")
        .args(["index", "--project", "fixture", "--force"])
        .spawn()
        .expect("spawn index");

    for _ in 0..50 {
        if marker.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "promotion marker was never created");
    thread::sleep(Duration::from_millis(170));

    search
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .args(["search", "reciprocal rank fusion", "--project", "fixture"]);
    let search_output = search.output().expect("search output");
    let index_output = index.wait_with_output().expect("index output");
    assert!(index_output.status.success());
    assert!(search_output.status.success());
    let stdout = String::from_utf8(search_output.stdout).expect("stdout");
    assert!(stdout.contains("Search Decision"));
}

#[test]
fn cross_process_index_lock_is_exclusive() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    let binary = assert_cmd::cargo::cargo_bin("llm-wiki");
    let mut first = StdCommand::new(&binary);
    first
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env("LLM_WIKI_TEST_INDEX_SLEEP_MS", "300")
        .args(["index", "--project", "fixture", "--force"]);
    let first = first.spawn().expect("spawn first");

    thread::sleep(Duration::from_millis(50));

    let mut second = StdCommand::new(&binary);
    let second_output = second
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .args(["index", "--project", "fixture", "--force"])
        .output()
        .expect("second output");
    let first_output = first.wait_with_output().expect("first output");

    let outputs = [first_output, second_output];
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1
    );
    assert!(outputs.iter().any(|output| {
        String::from_utf8_lossy(&output.stderr).contains("project index is already locked")
    }));
}

#[test]
fn crashed_indexer_does_not_block_next_acquire() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    let binary = assert_cmd::cargo::cargo_bin("llm-wiki");
    let mut child = StdCommand::new(&binary)
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env("LLM_WIKI_TEST_INDEX_SLEEP_MS", "5000")
        .args(["index", "--project", "fixture", "--force"])
        .spawn()
        .expect("spawn indexer");
    thread::sleep(Duration::from_millis(100));
    child.kill().expect("kill child");
    let _ = child.wait();

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexed project: fixture"));
}

fn register_project(home: &Path, project: &Path) {
    register_project_with_id(home, project, "fixture");
}

fn register_project_with_id(home: &Path, project: &Path, id: &str) {
    ensure_installed(home);
    llm_wiki(home)
        .args(["register", "--id", id, "--name", id])
        .arg(project)
        .assert()
        .success();
}

fn ensure_installed(home: &Path) {
    if home.join(".llm_wiki/manifest.json").exists() {
        return;
    }
    llm_wiki(home)
        .args(["install", "--skip-path-guidance"])
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
