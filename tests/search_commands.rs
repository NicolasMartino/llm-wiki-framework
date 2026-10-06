use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

mod support;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::new(support::llm_wiki_bin());
    command
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("LLM_WIKI_GGUF_RUNTIME")
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
        .stderr(predicate::str::contains("index_missing"))
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

    let output = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");
    let warnings = json["warnings"].as_array().expect("warnings");
    assert!(warnings.iter().any(|warning| {
        warning["project_id"] == "beta"
            && warning["message"]
                .as_str()
                .is_some_and(|message| message.contains("project root missing for beta"))
            && warning["message"]
                .as_str()
                .is_some_and(|message| message.contains("llm-wiki forget beta"))
    }));
    assert!(
        json["results"]
            .as_array()
            .expect("results")
            .iter()
            .any(|result| result["project_id"] == "alpha")
    );
    let beta_report = json["projects"]
        .as_array()
        .expect("project reports")
        .iter()
        .find(|report| report["project_id"] == "beta")
        .expect("beta report");
    assert_eq!(beta_report["readiness_reason"], "project_root_missing");
    assert_eq!(beta_report["result_count"], 0);
}

#[test]
fn search_all_reports_missing_wiki_root_per_project() {
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
    fs::remove_dir_all(beta.join("wiki")).expect("remove beta wiki root");

    let output = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");
    let warnings = json["warnings"].as_array().expect("warnings");
    assert!(warnings.iter().any(|warning| {
        warning["project_id"] == "beta"
            && warning["message"]
                .as_str()
                .is_some_and(|message| message.contains("wiki root missing for beta"))
            && warning["message"]
                .as_str()
                .is_some_and(|message| message.contains("llm-wiki forget beta"))
    }));
    assert!(
        json["results"]
            .as_array()
            .expect("results")
            .iter()
            .any(|result| result["project_id"] == "alpha")
    );
    let beta_report = json["projects"]
        .as_array()
        .expect("project reports")
        .iter()
        .find(|report| report["project_id"] == "beta")
        .expect("beta report");
    assert_eq!(beta_report["readiness_reason"], "wiki_root_missing");
    assert_eq!(beta_report["result_count"], 0);
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

#[cfg(unix)]
#[test]
fn index_does_not_follow_symlinks_escaping_the_wiki_root() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    // Secret markdown that lives OUTSIDE the project's wiki/ tree.
    let outside = workspace.path().join("outside");
    fs::create_dir_all(&outside).expect("outside dir");
    fs::write(
        outside.join("secret.md"),
        "# Secret\n\n- Document Class: Decision\n- Status: Accepted\n\n## Secret\nescapedsecrettoken must never be indexed.",
    )
    .expect("secret");

    // Both a symlinked file and a symlinked directory inside wiki/ that resolve
    // outside the wiki root. The indexer must refuse to follow either.
    symlink(
        outside.join("secret.md"),
        project.join("wiki/decisions/leak.decision.md"),
    )
    .expect("file symlink");
    symlink(&outside, project.join("wiki/leaked")).expect("dir symlink");

    // Indexing succeeds — escaping symlinks are skipped, not fatal.
    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    // The escaped content is not searchable: neither the symlinked file nor the
    // symlinked directory's target may appear in results.
    llm_wiki(home.path())
        .args([
            "search",
            "escapedsecrettoken",
            "--project",
            "fixture",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("leak.decision.md").not())
        .stdout(predicate::str::contains("leaked").not())
        .stdout(predicate::str::contains("secret.md").not());

    // Positive control: genuine in-wiki content is still indexed and searchable.
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
        .stdout(predicate::str::contains("search.decision.md"));
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
    remove_search_profile(home.path());

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
    remove_search_profile(home.path());

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
    remove_search_profile(home.path());

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
fn enabled_profile_without_thresholds_fails_closed() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());

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
    assert_eq!(json["readiness_reason"], "semantic_index_missing");
}

#[test]
fn auto_mode_uses_hybrid_defaults_when_thresholds_unconfigured() {
    // Fresh projects no longer need hand-authored calibration before hybrid can
    // run. Built-in defaults should make a freshly indexed project searchable.
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());
    // Intentionally NO write_search_thresholds: the project stays uncalibrated.

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .args([
            "search",
            "battery roadmap",
            "--project",
            "fixture",
            "--mode",
            "auto",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");

    assert!(
        output.status.success(),
        "auto must not hard-fail on uncalibrated thresholds"
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["selected_mode"], "hybrid");
    assert_eq!(json["mode_selection_reason"], "enabled_profile");
    assert_eq!(json["thresholds_source"], "default");
    assert!(json["fallback_reason"].is_null());
    assert!(json["readiness_reason"].is_null());
    assert!(
        json["results"]
            .as_array()
            .is_some_and(|results| !results.is_empty()),
        "default-threshold hybrid search should return results"
    );
}

#[test]
fn enabled_profile_without_accepted_licenses_fails_closed() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts_without_licenses(home.path());
    write_search_thresholds(home.path());

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
    assert_eq!(json["readiness_reason"], "license_not_accepted");
}

#[test]
fn index_semantic_metadata_requires_only_embedding_artifact() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_embedding_artifact_only(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let metadata: Value = serde_json::from_str(
        &fs::read_to_string(
            home.path()
                .join(".llm_wiki/indexes/fixture/semantic-index.json"),
        )
        .expect("semantic metadata"),
    )
    .expect("semantic metadata json");
    let model_artifacts = metadata["model_artifacts"]
        .as_array()
        .expect("model artifacts");
    assert_eq!(model_artifacts.len(), 1);
    assert_eq!(
        model_artifacts[0]["model_id"],
        Value::String("embeddinggemma-300m-q8_0".to_string())
    );
}

#[test]
fn semantic_mode_uses_vector_index_when_thresholds_are_configured() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args([
            "search",
            "battery roadmap",
            "--project",
            "fixture",
            "--mode",
            "semantic",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["selected_mode"], "semantic");
    let result = json["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("semantic result");
    assert_eq!(result["mode"], "semantic");
    assert_eq!(result["backend"], "qmd-rs-semantic");
    assert_eq!(result["path"], "wiki/decisions/search.decision.md");
}

#[test]
fn semantic_runtime_failure_returns_parseable_json() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env(
            "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE",
            "embedding:context_create",
        )
        .args([
            "search",
            "battery roadmap",
            "--project",
            "fixture",
            "--mode",
            "semantic",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("runtime failure json");
    assert_eq!(json["selected_mode"], "semantic");
    assert_eq!(json["readiness_reason"], "runtime_backend_unavailable");
    assert_eq!(json["runtime_failure_stage"], "context_create");
    assert_eq!(json["runtime_error_kind"], "backend_unavailable");
    assert_eq!(json["backend_status"]["state"], "ready");

    let forced_cpu_output = llm_wiki(home.path())
        .env("LLM_WIKI_GGUF_RUNTIME", "cpu")
        .env(
            "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE",
            "embedding:context_create",
        )
        .args([
            "search",
            "battery roadmap",
            "--project",
            "fixture",
            "--mode",
            "semantic",
            "--format",
            "json",
        ])
        .output()
        .expect("forced cpu search output");

    assert!(!forced_cpu_output.status.success());
    let forced_cpu_json: Value =
        serde_json::from_slice(&forced_cpu_output.stdout).expect("forced cpu runtime failure json");
    assert_eq!(forced_cpu_json["runtime_backend_requested"], "cpu");
    assert!(forced_cpu_json["runtime_backend_used"].is_null());
    assert_eq!(forced_cpu_json["runtime_backend_fallback"], false);
}

#[test]
fn semantic_mode_selects_project_scoped_thresholds() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Battery Decision",
        "Battery chemistry roadmap retrieval belongs in semantic search.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_scoped_search_thresholds(home.path());

    for project_id in ["alpha", "beta"] {
        llm_wiki(home.path())
            .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
            .args(["index", "--project", project_id, "--force"])
            .assert()
            .success();
    }

    let alpha_output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args([
            "search",
            "battery roadmap",
            "--project",
            "alpha",
            "--mode",
            "semantic",
            "--format",
            "json",
        ])
        .output()
        .expect("alpha search output");
    assert!(alpha_output.status.success());
    let alpha_json: Value = serde_json::from_slice(&alpha_output.stdout).expect("alpha json");
    assert_eq!(alpha_json["selected_mode"], "semantic");
    assert_eq!(
        alpha_json["results"]
            .as_array()
            .expect("alpha results")
            .len(),
        0
    );

    let beta_output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args([
            "search",
            "battery roadmap",
            "--project",
            "beta",
            "--mode",
            "semantic",
            "--format",
            "json",
        ])
        .output()
        .expect("beta search output");
    assert!(beta_output.status.success());
    let beta_json: Value = serde_json::from_slice(&beta_output.stdout).expect("beta json");
    assert_eq!(beta_json["selected_mode"], "semantic");
    let result = beta_json["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("beta semantic result");
    assert_eq!(result["path"], "wiki/decisions/search.decision.md");
}

#[test]
fn hybrid_mode_fuses_lexical_and_semantic_results() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .args([
            "search",
            "battery roadmap",
            "--project",
            "fixture",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .expect("search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["selected_mode"], "hybrid");
    let result = json["results"]
        .as_array()
        .and_then(|results| results.first())
        .expect("hybrid result");
    assert_eq!(result["mode"], "hybrid");
    assert_eq!(result["backend"], "qmd-rs-hybrid");
    assert!(result["lexical_rank"].as_u64().is_some());
    assert!(result["lexical_score"].as_f64().is_some());
    assert!(result["semantic_rank"].as_u64().is_some());
    assert!(result["semantic_score"].as_f64().is_some());
}

#[test]
fn hybrid_runtime_failure_returns_parseable_json() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project_with_decision(
        workspace.path(),
        "Fixture Project",
        "Battery Decision",
        "Battery chemistry roadmap",
    );
    register_project(home.path(), &project);
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env(
            "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE",
            "query_expansion:context_create",
        )
        .args([
            "search",
            "battery roadmap",
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
    let json: Value = serde_json::from_slice(&output.stdout).expect("runtime failure json");
    assert_eq!(json["requested_mode"], "hybrid");
    assert_eq!(json["selected_mode"], "hybrid");
    assert_eq!(json["readiness_reason"], "runtime_backend_unavailable");
    assert_eq!(json["runtime_backend_requested"], "auto");
    assert!(json["runtime_backend_used"].is_null());
    assert_eq!(json["runtime_backend_fallback"], false);
    assert_eq!(json["runtime_failure_stage"], "context_create");
    assert_eq!(json["runtime_error_kind"], "backend_unavailable");
    assert_eq!(json["backend_status"]["state"], "ready");
}

#[test]
fn compact_search_json_returns_essential_paged_hits() {
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
            "--compact",
            "--page-size",
            "1",
        ])
        .output()
        .expect("compact search output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("compact search json");
    assert_eq!(json["project_id"], "fixture");
    assert_eq!(json["result_count"], 1);
    assert_eq!(json["offset"], 0);
    assert_eq!(json["page_size"], 1);
    assert!(json["next_offset"].is_null());

    let results = json["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    let hit = &results[0];
    assert_eq!(hit["title"], "Search Decision");
    assert!(hit.get("snippet").is_none());
    assert!(hit.get("backend").is_none());
    assert!(hit.get("freshness").is_none());
}

#[test]
fn compact_search_all_json_pages_fused_hits_and_reports_projects() {
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
        .args([
            "search-all",
            "shared retrieval token",
            "--format",
            "json",
            "--compact",
            "--page-size",
            "1",
        ])
        .output()
        .expect("compact search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("compact search-all json");
    assert_eq!(json["result_count"], 2);
    assert_eq!(json["offset"], 0);
    assert_eq!(json["page_size"], 1);
    assert_eq!(json["next_offset"], 1);
    assert_eq!(json["results"].as_array().expect("results").len(), 1);
    assert_eq!(json["projects"].as_array().expect("projects").len(), 2);
    assert!(json["results"][0].get("snippet").is_none());
    assert!(json["results"][0].get("backend").is_none());

    let second = llm_wiki(home.path())
        .args([
            "search-all",
            "shared retrieval token",
            "--format",
            "json",
            "--compact",
            "--page-size",
            "1",
            "--offset",
            "1",
        ])
        .output()
        .expect("compact search-all second page");
    assert!(second.status.success());
    let second_json: Value =
        serde_json::from_slice(&second.stdout).expect("compact search-all page json");
    assert_eq!(second_json["offset"], 1);
    assert!(second_json["next_offset"].is_null());
    assert_eq!(
        second_json["results"]
            .as_array()
            .expect("second results")
            .len(),
        1
    );
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
        .stdout(predicate::str::contains("\"project_id\": \"beta\""))
        .stdout(predicate::str::contains("\"projects\""))
        .stdout(predicate::str::contains("\"selected_mode\": \"lexical\""));

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
fn search_all_reports_per_project_readiness_and_skips_unready_projects() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Alpha Decision",
        "Battery roadmap shared token appears in alpha project.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Beta Decision",
        "Battery roadmap shared token appears in beta project.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index", "--project", "alpha", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .args([
            "search-all",
            "battery roadmap",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");
    assert_eq!(json["selected_mode"], "hybrid");
    assert_eq!(
        json["results"][0]["project_id"],
        Value::String("alpha".to_string())
    );
    let projects = json["projects"].as_array().expect("project reports");
    let alpha_report = projects
        .iter()
        .find(|report| report["project_id"] == "alpha")
        .expect("alpha report");
    assert_eq!(alpha_report["selected_mode"], "hybrid");
    assert!(alpha_report["readiness_reason"].is_null());
    let beta_report = projects
        .iter()
        .find(|report| report["project_id"] == "beta")
        .expect("beta report");
    assert!(beta_report["selected_mode"].is_null());
    assert_eq!(beta_report["readiness_reason"], "semantic_index_missing");
    assert_eq!(beta_report["result_count"], 0);
}

#[test]
fn search_all_lexical_reports_missing_index_per_project() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Alpha Decision",
        "Battery roadmap shared token appears in alpha project.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Beta Decision",
        "Battery roadmap shared token appears in beta project.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");

    llm_wiki(home.path())
        .args(["index", "--project", "alpha", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .args([
            "search-all",
            "battery roadmap",
            "--mode",
            "lexical",
            "--format",
            "json",
        ])
        .output()
        .expect("search-all output");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");
    assert_eq!(json["selected_mode"], "lexical");
    assert_eq!(
        json["results"][0]["project_id"],
        Value::String("alpha".to_string())
    );

    let projects = json["projects"].as_array().expect("project reports");
    let beta_report = projects
        .iter()
        .find(|report| report["project_id"] == "beta")
        .expect("beta report");
    assert_eq!(beta_report["selected_mode"], "lexical");
    assert_eq!(beta_report["readiness_reason"], "index_missing");
    assert_eq!(beta_report["backend_status"]["state"], "missing");
    assert_eq!(beta_report["result_count"], 0);
}

#[test]
fn search_all_runtime_failure_reports_per_project_json() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project_with_decision(
        workspace.path(),
        "Alpha Project",
        "Alpha Decision",
        "Battery roadmap shared token appears in alpha project.",
    );
    let beta = fixture_project_with_decision(
        workspace.path(),
        "Beta Project",
        "Beta Decision",
        "Battery roadmap shared token appears in beta project.",
    );
    register_project_with_id(home.path(), &alpha, "alpha");
    register_project_with_id(home.path(), &beta, "beta");
    write_enabled_search_profile_with_fake_artifacts(home.path());
    write_search_thresholds(home.path());

    llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .args(["index-all", "--force"])
        .assert()
        .success();

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env(
            "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE",
            "query_expansion:context_create",
        )
        .args([
            "search-all",
            "battery roadmap",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .expect("search-all output");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("runtime failure json");
    assert_eq!(json["readiness_reason"], "runtime_backend_unavailable");
    assert_eq!(json["runtime_backend_requested"], "auto");
    assert!(json["runtime_backend_used"].is_null());
    assert_eq!(json["runtime_backend_fallback"], false);
    assert_eq!(json["runtime_failure_stage"], "context_create");
    assert_eq!(json["runtime_error_kind"], "backend_unavailable");
    assert!(json["results"].as_array().expect("results").is_empty());

    let projects = json["projects"].as_array().expect("project reports");
    for project_id in ["alpha", "beta"] {
        let report = projects
            .iter()
            .find(|report| report["project_id"] == project_id)
            .expect("project report");
        assert_eq!(report["selected_mode"], "hybrid");
        assert_eq!(report["readiness_reason"], "runtime_backend_unavailable");
        assert_eq!(report["runtime_backend_requested"], "auto");
        assert!(report["runtime_backend_used"].is_null());
        assert_eq!(report["runtime_backend_fallback"], false);
        assert_eq!(report["runtime_failure_stage"], "context_create");
        assert_eq!(report["runtime_error_kind"], "backend_unavailable");
        assert_eq!(report["backend_status"]["state"], "ready");
    }

    let forced_cpu_output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_GGUF_RUNTIME", "cpu")
        .env(
            "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE",
            "query_expansion:context_create",
        )
        .args([
            "search-all",
            "battery roadmap",
            "--mode",
            "hybrid",
            "--format",
            "json",
        ])
        .output()
        .expect("forced cpu search-all output");

    assert!(forced_cpu_output.status.success());
    let forced_cpu_json: Value = serde_json::from_slice(&forced_cpu_output.stdout)
        .expect("forced cpu search-all runtime failure json");
    assert_eq!(forced_cpu_json["runtime_backend_requested"], "cpu");
    assert!(forced_cpu_json["runtime_backend_used"].is_null());
    assert_eq!(forced_cpu_json["runtime_backend_fallback"], false);
    let forced_cpu_projects = forced_cpu_json["projects"]
        .as_array()
        .expect("forced cpu project reports");
    for project_id in ["alpha", "beta"] {
        let report = forced_cpu_projects
            .iter()
            .find(|report| report["project_id"] == project_id)
            .expect("forced cpu project report");
        assert_eq!(report["runtime_backend_requested"], "cpu");
        assert!(report["runtime_backend_used"].is_null());
        assert_eq!(report["runtime_backend_fallback"], false);
    }
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
    assert_eq!(json["backend_status"]["state"], "ready");
    assert_eq!(json["backend_status"]["open_mode"], "read_only_immutable");
    assert!(json.get("warning").is_some());
    assert!(json.get("warnings").is_some());
    assert!(json.get("projects").is_some());
    assert!(
        json["projects"]
            .as_array()
            .is_some_and(|items| items.is_empty())
    );
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

#[cfg(unix)]
#[test]
fn search_json_reports_permission_denied_without_force_reindex_guidance() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let store = home.path().join(".llm_wiki/indexes/fixture/qmd-rs.sqlite");
    let original = fs::metadata(&store).expect("metadata").permissions();
    let mut unreadable = original.clone();
    unreadable.set_mode(0o000);
    fs::set_permissions(&store, unreadable).expect("chmod unreadable");

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
    fs::set_permissions(&store, original).expect("restore permissions");

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("permission json");
    assert_eq!(json["readiness_reason"], "permission_denied");
    assert_eq!(json["backend_status"]["state"], "permission_denied");
    assert_eq!(json["backend_status"]["open_mode"], "read_only_immutable");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("grant read access"));
    assert!(!stderr.contains("--force"));
}

#[test]
fn search_json_reports_transient_backend_status() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project");
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();
    fs::remove_file(
        home.path()
            .join(".llm_wiki/indexes/fixture/qmd-rs.llm-wiki.json"),
    )
    .expect("remove metadata");

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

    assert!(!output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("transient json");
    assert_eq!(json["readiness_reason"], "transient");
    assert_eq!(json["backend_status"]["state"], "transient");
    assert_eq!(json["backend_status"]["open_mode"], "not_opened");
}

#[cfg(unix)]
#[test]
fn search_all_json_keeps_ready_results_with_one_inaccessible_project() {
    use std::os::unix::fs::PermissionsExt;

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

    let beta_store = home.path().join(".llm_wiki/indexes/beta/qmd-rs.sqlite");
    let original = fs::metadata(&beta_store).expect("metadata").permissions();
    let mut unreadable = original.clone();
    unreadable.set_mode(0o000);
    fs::set_permissions(&beta_store, unreadable).expect("chmod unreadable");

    let output = llm_wiki(home.path())
        .args(["search-all", "shared retrieval token", "--format", "json"])
        .output()
        .expect("search-all output");
    fs::set_permissions(&beta_store, original).expect("restore permissions");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("search-all json");
    assert!(
        json["results"]
            .as_array()
            .expect("results")
            .iter()
            .any(|result| result["project_id"] == "alpha")
    );
    let beta_report = json["projects"]
        .as_array()
        .expect("project reports")
        .iter()
        .find(|report| report["project_id"] == "beta")
        .expect("beta report");
    assert_eq!(beta_report["readiness_reason"], "permission_denied");
    assert_eq!(beta_report["backend_status"]["state"], "permission_denied");
    assert_eq!(
        beta_report["backend_status"]["open_mode"],
        "read_only_immutable"
    );
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

    let binary = support::llm_wiki_bin();
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

    let binary = support::llm_wiki_bin();
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

    let binary = support::llm_wiki_bin();
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
}

fn remove_search_profile(home: &Path) {
    let search = home.join(".llm_wiki/search.toml");
    if search.exists() {
        fs::remove_file(search).expect("remove search profile");
    }
    let dependencies = home.join(".llm_wiki/external-dependencies.toml");
    if dependencies.exists() {
        fs::remove_file(dependencies).expect("remove external dependencies");
    }
}

fn write_enabled_search_profile_with_fake_artifacts(home: &Path) {
    write_enabled_search_profile_with_fake_artifacts_inner(home, true);
}

fn write_enabled_search_profile_with_fake_artifacts_without_licenses(home: &Path) {
    write_enabled_search_profile_with_fake_artifacts_inner(home, false);
}

fn write_enabled_search_profile_with_embedding_artifact_only(home: &Path) {
    write_enabled_search_profile_with_fake_artifacts(home);
    let managed = home.join(".llm_wiki");
    let embedding_path =
        managed.join("models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf");
    fs::write(
        managed.join("models/artifacts.toml"),
        format!(
            r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "embeddinggemma-300m-q8_0"
role = "embedding"
profile = "balanced"
repository = "ggml-org/embeddinggemma-300M-GGUF"
revision = "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12"
file = "embeddinggemma-300M-Q8_0.gguf"
download_url = "https://example.invalid/embedding.gguf"
path = "{}"
expected_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
observed_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
size_bytes = 1
license = "gemma"
dimensions = 768
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"
"#,
            embedding_path.display()
        ),
    )
    .expect("embedding-only artifacts");
    fs::write(
        home.join(".llm_wiki/accepted-licenses.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[licenses]]
model_id = "embeddinggemma-300m-q8_0"
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"
"#,
    )
    .expect("embedding-only licenses");
}

fn write_enabled_search_profile_with_fake_artifacts_inner(home: &Path, write_licenses: bool) {
    let managed = home.join(".llm_wiki");
    let embedding_path =
        managed.join("models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf");
    let expansion_path =
        managed.join("models/qmd-query-expansion-1.7b-q4_k_m/qmd-query-expansion-1.7B-q4_k_m.gguf");
    fs::create_dir_all(embedding_path.parent().expect("embedding parent")).expect("embedding dir");
    fs::create_dir_all(expansion_path.parent().expect("expansion parent")).expect("expansion dir");
    fs::write(&embedding_path, "fake embedding").expect("embedding file");
    fs::write(&expansion_path, "fake expansion").expect("expansion file");
    fs::write(
        managed.join("search.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[project_default]
llm_search_enabled = true
configured_at = "2026-05-11T00:00:00Z"
configured_by_version = "test"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"

[global_search]
llm_search_enabled = true
configured_at = "2026-05-11T00:00:00Z"
configured_by_version = "test"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"
"#,
    )
    .expect("search profile");
    fs::create_dir_all(managed.join("models")).expect("models dir");
    fs::write(
        managed.join("models/artifacts.toml"),
        format!(
            r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "embeddinggemma-300m-q8_0"
role = "embedding"
profile = "balanced"
repository = "ggml-org/embeddinggemma-300M-GGUF"
revision = "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12"
file = "embeddinggemma-300M-Q8_0.gguf"
download_url = "https://example.invalid/embedding.gguf"
path = "{}"
expected_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
observed_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
size_bytes = 1
license = "gemma"
dimensions = 768
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
role = "query-expansion"
profile = "balanced"
repository = "tobil/qmd-query-expansion-1.7B-gguf"
revision = "7816de0b72572c6c860ca1eddf97ba9e7fb8cc65"
file = "qmd-query-expansion-1.7B-q4_k_m.gguf"
download_url = "https://example.invalid/expansion.gguf"
path = "{}"
expected_sha256 = "000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0"
observed_sha256 = "000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0"
size_bytes = 1
license = "mit"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"
"#,
            embedding_path.display(),
            expansion_path.display()
        ),
    )
    .expect("artifacts");
    if write_licenses {
        write_search_licenses(home);
    }
}

fn write_search_licenses(home: &Path) {
    fs::write(
        home.join(".llm_wiki/accepted-licenses.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[licenses]]
model_id = "embeddinggemma-300m-q8_0"
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"

[[licenses]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
license = "mit"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"
"#,
    )
    .expect("licenses");
}

fn write_search_thresholds(home: &Path) {
    fs::write(
        home.join(".llm_wiki/search-thresholds.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-11T00:00:00Z"
profile = "balanced"
semantic_similarity_floor = 0.1
hybrid_pre_fusion_semantic_floor = 0.1
hybrid_final_semantic_floor = 0.1
hybrid_semantic_only_floor = 0.1
hybrid_strong_lexical_score_floor = 1.0
reranker_probability_floor = 0.1
lexical_exact_identifier_guard = "preserve_lexical_top_3"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
chunking_strategy = "qmd-rs-character-v1:3200:480"
embedding_model = "embeddinggemma-300m-q8_0"
embedding_artifact_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
embedding_dimensions = 768
"#,
    )
    .expect("thresholds");
}

fn write_scoped_search_thresholds(home: &Path) {
    fs::write(
        home.join(".llm_wiki/search-thresholds.toml"),
        r#"
schema_version = 2
updated_at = "2026-05-12T00:00:00Z"

[[thresholds]]
schema_version = 1
updated_at = "2026-05-12T00:00:00Z"
project_id = "alpha"
profile = "balanced"
semantic_similarity_floor = 2.0
hybrid_pre_fusion_semantic_floor = 2.0
hybrid_final_semantic_floor = 2.0
hybrid_semantic_only_floor = 2.0
hybrid_strong_lexical_score_floor = 1.0
reranker_probability_floor = 0.1
lexical_exact_identifier_guard = "preserve_lexical_top_3"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
chunking_strategy = "qmd-rs-character-v1:3200:480"
embedding_model = "embeddinggemma-300m-q8_0"
embedding_artifact_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
embedding_dimensions = 768

[[thresholds]]
schema_version = 1
updated_at = "2026-05-12T00:00:00Z"
project_id = "beta"
profile = "balanced"
semantic_similarity_floor = 0.1
hybrid_pre_fusion_semantic_floor = 0.1
hybrid_final_semantic_floor = 0.1
hybrid_semantic_only_floor = 0.1
hybrid_strong_lexical_score_floor = 1.0
reranker_probability_floor = 0.1
lexical_exact_identifier_guard = "preserve_lexical_top_3"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
chunking_strategy = "qmd-rs-character-v1:3200:480"
embedding_model = "embeddinggemma-300m-q8_0"
embedding_artifact_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
embedding_dimensions = 768
"#,
    )
    .expect("scoped thresholds");
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
