use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

mod support;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::new(support::llm_wiki_bin());
    command
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME");
    command
}

fn is_test_instance() -> bool {
    option_env!("LLM_WIKI_COMPILED_INSTANCE") == Some("test")
}

fn managed_home_dir_name() -> &'static str {
    if is_test_instance() {
        ".llm_wiki-test"
    } else {
        ".llm_wiki"
    }
}

fn binary_name() -> &'static str {
    if is_test_instance() {
        "llm-wiki-test"
    } else {
        "llm-wiki"
    }
}

/// Path `doctor` checks for the managed binary.
fn managed_binary_path(home: &Path) -> std::path::PathBuf {
    home.join(format!("{}/bin/{}", managed_home_dir_name(), binary_name()))
}

/// Place a stand-in managed binary at the path `doctor` checks, made executable
/// so wiring that points at it counts as runnable (a real install's binary is
/// executable; a non-executable file would encode a false "runnable" condition).
fn stub_managed_binary(home: &Path) {
    let bin = managed_binary_path(home);
    fs::create_dir_all(bin.parent().expect("bin dir")).expect("bin dir");
    fs::write(&bin, b"#!/bin/sh\n").expect("stub binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod +x stub");
    }
}

#[test]
fn doctor_reports_corrupt_manifest_as_finding_without_hard_error() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    let manifest = home.path().join(".llm_wiki/manifest.json");
    fs::create_dir_all(manifest.parent().expect("parent")).expect("managed home");
    fs::write(&manifest, "{ this is not valid json").expect("corrupt manifest");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest.json failed to parse"));
}

#[test]
fn doctor_reports_corrupt_registry_as_finding_without_hard_error() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    let registry = home.path().join(".local/share/llm-wiki/projects.json");
    fs::create_dir_all(registry.parent().expect("parent")).expect("data home");
    fs::write(&registry, "{ this is not valid json").expect("corrupt registry");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("projects.json failed to parse"));
}

#[test]
fn status_reports_corrupt_manifest_as_finding_without_hard_error() {
    let home = TempDir::new().expect("home");
    let manifest = home.path().join(".llm_wiki/manifest.json");
    fs::create_dir_all(manifest.parent().expect("parent")).expect("managed home");
    fs::write(&manifest, "{ this is not valid json").expect("corrupt manifest");

    llm_wiki(home.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest.json failed to parse"));
}

#[test]
fn status_reports_installed_files() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

    llm_wiki(home.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest files: "))
        .stdout(predicate::str::contains("OK: asset"))
        .stdout(predicate::str::contains(
            "mcp server startup: host-managed stdio",
        ))
        .stdout(predicate::str::contains("llm-wiki mcp serve"));
}

#[test]
fn verbose_path_status_and_doctor_emit_diagnostics() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");

    llm_wiki(home.path())
        .args(["--verbose", "path"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: path"))
        .stderr(predicate::str::contains("managed home:"))
        .stderr(predicate::str::contains("managed binary:"));

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["--verbose", "status"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: status"))
        .stderr(predicate::str::contains("manifest:"))
        .stderr(predicate::str::contains("manifest files:"))
        .stderr(predicate::str::contains("manifest file status:"));

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .args(["--verbose", "doctor"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: doctor"))
        .stderr(predicate::str::contains("registry:"))
        .stderr(predicate::str::contains("index root:"))
        .stderr(predicate::str::contains("model cache:"));
}

#[test]
fn doctor_reports_last_and_current_runtime_probe_status() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");

    write_runtime_probe_fixture(home.path());

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .env("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("GGUF runtime:"))
        .stdout(predicate::str::contains("Last runtime probe: passed"))
        .stdout(predicate::str::contains("Current runtime probe: passed"));
}

#[test]
fn doctor_skips_current_runtime_probe_when_artifact_file_is_missing() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    write_runtime_probe_fixture(home.path());
    fs::remove_file(fixture_embedding_path(home.path())).expect("remove embedding");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .env("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Current runtime probe skipped: model artifact file missing for embeddinggemma-300m-q8_0",
        ));
}

#[test]
fn doctor_skips_current_runtime_probe_when_artifact_hash_mismatches() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");
    write_runtime_probe_fixture(home.path());
    fs::write(fixture_embedding_path(home.path()), "tampered").expect("tamper embedding");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .env("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Current runtime probe skipped: model artifact hash mismatch for embeddinggemma-300m-q8_0",
        ));
}

fn write_runtime_probe_fixture(home: &Path) {
    let managed = home.join(".llm_wiki");
    let models = managed.join("models");
    fs::create_dir_all(&models).expect("models dir");
    let embedding_path = fixture_embedding_path(home);
    let expansion_path = fixture_expansion_path(home);
    fs::create_dir_all(embedding_path.parent().expect("embedding parent"))
        .expect("embedding parent dir");
    fs::create_dir_all(expansion_path.parent().expect("expansion parent"))
        .expect("expansion parent dir");
    let embedding_sha = write_fixture_artifact(&embedding_path, b"embedding fixture");
    let expansion_sha = write_fixture_artifact(&expansion_path, b"query expansion fixture");

    fs::write(
        managed.join("search.toml"),
        format!(
            r#"schema_version = 1
updated_at = "2026-05-25T00:00:00Z"

[project_default]
llm_search_enabled = true
configured_at = "2026-05-25T00:00:00Z"
configured_by_version = "{version}"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"

[global_search]
llm_search_enabled = true
configured_at = "2026-05-25T00:00:00Z"
configured_by_version = "{version}"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"
"#,
            version = env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("search config");
    fs::write(
        managed.join("accepted-licenses.toml"),
        format!(
            r#"schema_version = 1
updated_at = "2026-05-25T00:00:00Z"

[[licenses]]
model_id = "embeddinggemma-300m-q8_0"
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
accepted_at = "2026-05-25T00:00:00Z"
accepted_by_version = "{version}"

[[licenses]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
license = "mit"
accepted_at = "2026-05-25T00:00:00Z"
accepted_by_version = "{version}"
"#,
            version = env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("accepted licenses");
    fs::write(
        models.join("artifacts.toml"),
        format!(
            r#"schema_version = 1
updated_at = "2026-05-25T00:00:00Z"

[[artifacts]]
model_id = "embeddinggemma-300m-q8_0"
role = "embedding"
profile = "balanced"
repository = "ggml-org/embeddinggemma-300M-GGUF"
revision = "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12"
file = "embeddinggemma-300M-Q8_0.gguf"
download_url = "https://example.invalid/embedding"
path = "{embedding_path}"
expected_sha256 = "{embedding_sha}"
observed_sha256 = "{embedding_sha}"
size_bytes = 333590944
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
dimensions = 768
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-25T00:00:00Z"

[[artifacts]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
role = "query-expansion"
profile = "balanced"
repository = "tobil/qmd-query-expansion-1.7B-gguf"
revision = "7816de0b72572c6c860ca1eddf97ba9e7fb8cc65"
file = "qmd-query-expansion-1.7B-q4_k_m.gguf"
download_url = "https://example.invalid/query-expansion"
path = "{expansion_path}"
expected_sha256 = "{expansion_sha}"
observed_sha256 = "{expansion_sha}"
size_bytes = 1282438912
license = "mit"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-25T00:00:00Z"
"#,
            embedding_path = toml_path(&embedding_path),
            expansion_path = toml_path(&expansion_path),
            embedding_sha = embedding_sha,
            expansion_sha = expansion_sha
        ),
    )
    .expect("artifacts");
    fs::write(
        managed.join("search-runtime-probes.toml"),
        format!(
            r#"schema_version = 1
updated_at = "2026-05-25T00:00:00Z"
binary_version = "{version}"
target_triple = "{target}"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1

[[records]]
profile = "balanced"
role = "embedding"
required = true
model_id = "embeddinggemma-300m-q8_0"
model_path = "{embedding_path}"
artifact_sha256 = "{embedding_sha}"
artifact_size_bytes = 333590944
requested_backend = "auto"
used_backend = "auto"
fallback = false
outcome = "passed"
duration_ms = 1
probed_at = "2026-05-25T00:00:00Z"

[[records]]
profile = "balanced"
role = "query_expansion"
required = true
model_id = "qmd-query-expansion-1.7b-q4_k_m"
model_path = "{expansion_path}"
artifact_sha256 = "{expansion_sha}"
artifact_size_bytes = 1282438912
requested_backend = "auto"
used_backend = "auto"
fallback = false
outcome = "passed"
duration_ms = 1
probed_at = "2026-05-25T00:00:00Z"
"#,
            version = env!("CARGO_PKG_VERSION"),
            target = build_target(),
            embedding_path = toml_path(&embedding_path),
            expansion_path = toml_path(&expansion_path),
            embedding_sha = embedding_sha,
            expansion_sha = expansion_sha
        ),
    )
    .expect("runtime probes");
}

fn fixture_embedding_path(home: &Path) -> std::path::PathBuf {
    home.join(".llm_wiki/models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf")
}

fn fixture_expansion_path(home: &Path) -> std::path::PathBuf {
    home.join(
        ".llm_wiki/models/qmd-query-expansion-1.7b-q4_k_m/qmd-query-expansion-1.7B-q4_k_m.gguf",
    )
}

fn write_fixture_artifact(path: &Path, bytes: &[u8]) -> String {
    fs::write(path, bytes).expect("fixture artifact");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn build_target() -> String {
    option_env!("LLM_WIKI_BUILD_TARGET")
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            format!(
                "{}-{}-unknown",
                std::env::consts::ARCH,
                std::env::consts::OS
            )
        })
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

#[test]
fn status_reports_drift() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    fs::write(
        home.path().join(".llm_wiki/mcp/claude-project.mcp.json"),
        "tampered",
    )
    .expect("write");

    llm_wiki(home.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Drifted"));
}

#[test]
fn doctor_reports_missing_manifest_asset() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    fs::remove_file(home.path().join(".llm_wiki/mcp/claude-project.mcp.json")).expect("remove");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Missing manifest asset"));
}

#[test]
fn path_command_prints_guidance_without_installing() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains(".llm_wiki/bin"))
        .stdout(predicate::str::contains("PATH"));

    assert!(!home.path().join(".llm_wiki/manifest.json").exists());
}

#[test]
fn doctor_reports_path_binary_drift() {
    let home = TempDir::new().expect("home");
    let fake_bin = TempDir::new().expect("fake bin");
    fs::write(fake_bin.path().join("llm-wiki"), "different binary").expect("fake binary");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

    llm_wiki(home.path())
        .env("PATH", fake_bin.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("PATH llm-wiki differs"));
}

#[test]
fn doctor_reports_project_search_skipped_outside_wiki_project() {
    let home = TempDir::new().expect("home");
    let cwd = TempDir::new().expect("cwd");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Current project:"))
        .stdout(predicate::str::contains("Registry:"))
        .stdout(predicate::str::contains("project search checks skipped"));
}

#[test]
fn doctor_reports_registry_state_and_missing_roots() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();
    let cwd = TempDir::new().expect("cwd");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();
    fs::remove_dir_all(project.path()).expect("remove project");

    llm_wiki(home.path())
        .current_dir(cwd.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Registry:"))
        .stdout(predicate::str::contains("(1 projects)"))
        .stdout(predicate::str::contains(
            "Registered project root missing: fixture",
        ));
}

fn wiki_project() -> TempDir {
    let project = TempDir::new().expect("project");
    fs::create_dir_all(project.path().join("wiki")).expect("wiki");
    fs::write(project.path().join("wiki/index.md"), "# Index").expect("index");
    fs::write(project.path().join("wiki/log.md"), "# Log").expect("log");
    fs::write(project.path().join("AGENTS.md"), "# Agents").expect("agents");
    project
}

#[test]
fn doctor_reports_missing_search_index() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();

    llm_wiki(home.path())
        .current_dir(project.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected wiki project"))
        .stdout(predicate::str::contains("Search index:"))
        .stdout(predicate::str::contains("qmd-rs FTS index missing"))
        .stdout(predicate::str::contains("Semantic models:"));
}

#[cfg(unix)]
#[test]
fn doctor_reports_search_cache_permission_denied_distinctly() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let project = wiki_project();
    fs::write(
        project.path().join("wiki/search.md"),
        "# Search\n\nReadable content.",
    )
    .expect("search page");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();
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
        .current_dir(project.path())
        .arg("doctor")
        .output()
        .expect("doctor output");
    fs::set_permissions(&store, original).expect("restore permissions");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("qmd-rs FTS index permission denied"));
    assert!(!stdout.contains("qmd-rs FTS index corrupt"));
}

#[test]
fn doctor_reports_invalid_project_mcp_json_distinctly() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--no-mcp"])
        .arg(project.path())
        .assert()
        .success();

    // A malformed .mcp.json must not be reported as merely "not wired": the
    // suggested register/init re-merge would hit the same parse failure.
    fs::write(project.path().join(".mcp.json"), "{ not valid json").expect("write bad json");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("MCP config invalid"))
        .stdout(predicate::str::contains("fix or delete it"))
        .stdout(predicate::str::contains("fixture"));
}

#[test]
fn doctor_reports_wired_project_under_mcp_wiring() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();
    // A wired project only counts as connectable when the managed binary its
    // config names actually exists.
    stub_managed_binary(home.path());

    // `register` auto-wires the project's `.mcp.json` via the shared core.
    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("MCP wiring:"))
        .stdout(predicate::str::contains(
            "registered project(s) have Claude MCP wiring",
        ))
        .stdout(predicate::str::contains("Codex MCP server configured"));
}

#[test]
fn doctor_warns_when_wired_but_managed_binary_missing() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();

    // Register wires the project (and Codex) to the managed binary path, but no
    // binary is installed there, so doctor must flag it as unrunnable.
    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Managed binary missing"))
        .stdout(predicate::str::contains("wired but managed binary missing"))
        .stdout(predicate::str::contains(
            "Codex MCP server configured but managed binary missing",
        ));
}

#[cfg(unix)]
#[test]
fn doctor_warns_when_managed_binary_present_but_not_executable() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let project = wiki_project();

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();

    // A present-but-non-executable managed binary cannot be spawned by a host.
    let bin = managed_binary_path(home.path());
    fs::create_dir_all(bin.parent().expect("bin dir")).expect("bin dir");
    fs::write(&bin, b"#!/bin/sh\n").expect("stub binary");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o644)).expect("chmod -x");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Managed binary not executable"))
        .stdout(predicate::str::contains(
            "wired but managed binary not executable",
        ));
}

#[test]
fn doctor_flags_stale_project_wiring_command() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();
    stub_managed_binary(home.path());

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(project.path())
        .assert()
        .success();

    // Rewrite the project .mcp.json so the managed server points at a different
    // binary than the managed one: doctor must call this out as stale, not "wired".
    let mcp_json = project.path().join(".mcp.json");
    let stale = fs::read_to_string(&mcp_json)
        .expect("read .mcp.json")
        .replace(
            &home
                .path()
                .join(format!("{}/bin/{}", managed_home_dir_name(), binary_name()))
                .to_string_lossy()
                .into_owned(),
            "/somewhere/else/llm-wiki",
        );
    fs::write(&mcp_json, stale).expect("write stale .mcp.json");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("MCP wiring stale"))
        .stdout(predicate::str::contains("fixture"));
}

#[test]
fn doctor_warns_about_unwired_registered_project_without_failing() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();

    // `--no-mcp` registers the project but leaves it unwired.
    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--no-mcp"])
        .arg(project.path())
        .assert()
        .success();

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        // The unwired project is a warning, never a failing exit status.
        .success()
        .stdout(predicate::str::contains("MCP wiring:"))
        .stdout(predicate::str::contains("MCP not wired: "))
        .stdout(predicate::str::contains("fixture"));
}
