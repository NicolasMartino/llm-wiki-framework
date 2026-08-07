//! Integration coverage for automatic MCP onboarding at `register`/`init`:
//! both project-scoped commands wire a project-local Claude `.mcp.json` and the
//! global Codex config via the shared core, non-destructively and idempotently,
//! honoring `--no-mcp`. See wiki/plans/mcp-onboarding-init-register.plan.md.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
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

fn is_test_instance() -> bool {
    option_env!("LLM_WIKI_COMPILED_INSTANCE") == Some("test")
}

fn mcp_server_name() -> &'static str {
    if is_test_instance() {
        "llm-wiki-test"
    } else {
        "llm-wiki"
    }
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

/// The managed binary path the wired server should spawn, relative to `home`.
fn managed_binary(home: &Path) -> String {
    home.join(format!("{}/bin/{}", managed_home_dir_name(), binary_name()))
        .to_string_lossy()
        .into_owned()
}

fn fallback_claude_template(home: &Path) -> PathBuf {
    home.join(format!(
        "{}/mcp/claude-project.mcp.json",
        managed_home_dir_name()
    ))
}

fn fixture_project(root: &Path, name: &str) -> PathBuf {
    let project = root.join(name);
    fs::create_dir_all(project.join("wiki")).expect("wiki");
    fs::write(project.join("wiki/index.md"), "# Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    fs::write(project.join("AGENTS.md"), "# Agents\n").expect("agents");
    project.canonicalize().expect("canonical")
}

fn read_claude_config(project: &Path) -> Value {
    let contents = fs::read_to_string(project.join(".mcp.json")).expect("read .mcp.json");
    serde_json::from_str(&contents).expect("parse .mcp.json")
}

#[test]
fn register_wires_project_and_codex_mcp_config() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "fixture");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();

    let config = read_claude_config(&project);
    let server = &config["mcpServers"][mcp_server_name()];
    assert_eq!(server["type"], "stdio");
    assert_eq!(
        server["command"].as_str().expect("command"),
        managed_binary(home.path())
    );
    assert_eq!(server["args"], serde_json::json!(["mcp", "serve"]));

    // The global Codex config is ensured idempotently by the same core.
    let codex = fs::read_to_string(home.path().join(".codex/config.toml")).expect("codex config");
    let parsed: toml::Value = toml::from_str(&codex).expect("codex toml");
    let codex_server = &parsed["mcp_servers"][mcp_server_name()];
    assert_eq!(
        codex_server["command"].as_str().expect("codex command"),
        managed_binary(home.path())
    );
    assert_eq!(
        codex_server["args"].as_array().expect("codex args"),
        &[
            toml::Value::String("mcp".to_string()),
            toml::Value::String("serve".to_string())
        ]
    );
}

#[test]
fn register_mcp_wiring_is_idempotent() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "fixture");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();
    let first = fs::read_to_string(project.join(".mcp.json")).expect("first .mcp.json");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();
    let second = fs::read_to_string(project.join(".mcp.json")).expect("second .mcp.json");

    assert_eq!(first, second, "re-registering must not rewrite .mcp.json");
}

#[test]
fn register_preserves_compact_already_wired_project_mcp_config_bytes() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "fixture");
    let compact = format!(
        r#"{{"mcpServers":{{"{name}":{{"type":"stdio","command":"{command}","args":["mcp","serve"],"env":{{}}}},"other":{{"command":"other-cmd"}}}},"custom":true}}"#,
        name = mcp_server_name(),
        command = managed_binary(home.path())
    );
    fs::write(project.join(".mcp.json"), &compact).expect("seed compact .mcp.json");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(project.join(".mcp.json")).expect("read .mcp.json"),
        compact,
        "a structurally correct .mcp.json must not be reformatted"
    );
}

#[test]
fn register_preserves_unrelated_project_mcp_servers() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "fixture");
    fs::write(
        project.join(".mcp.json"),
        r#"{"mcpServers": {"other": {"type": "stdio", "command": "other-cmd"}}}"#,
    )
    .expect("seed .mcp.json");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();

    let config = read_claude_config(&project);
    assert_eq!(
        config["mcpServers"]["other"]["command"].as_str(),
        Some("other-cmd"),
        "an unrelated server must be preserved"
    );
    assert!(config["mcpServers"][mcp_server_name()].is_object());
}

#[test]
fn register_no_mcp_writes_no_host_config() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "fixture");

    let output = llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--no-mcp"])
        .arg(&project)
        .output()
        .expect("register --no-mcp output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Claude fallback template path is"));
    assert!(stdout.contains("Run `llm-wiki install` to materialize it if needed"));
    assert!(!stdout.contains("Claude template staged at"));

    assert!(
        !project.join(".mcp.json").exists(),
        "--no-mcp must not write a project .mcp.json"
    );
    assert!(
        !home.path().join(".codex/config.toml").exists(),
        "--no-mcp must not wire the Codex config"
    );
    assert!(
        !fallback_claude_template(home.path()).exists(),
        "fresh-home --no-mcp must not imply the fallback template was materialized"
    );
}

#[test]
fn init_wires_fresh_project_mcp_config() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = workspace.path().join("proj");

    llm_wiki(home.path())
        .arg("init")
        .arg(&project)
        .args([
            "--non-interactive",
            "--name",
            "Proj",
            "--description",
            "A project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let project = project.canonicalize().expect("canonical project");
    let config = read_claude_config(&project);
    assert_eq!(
        config["mcpServers"][mcp_server_name()]["command"]
            .as_str()
            .expect("command"),
        managed_binary(home.path())
    );
}

#[test]
fn init_no_register_does_not_wire_project() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = workspace.path().join("proj");

    let output = llm_wiki(home.path())
        .arg("init")
        .arg(&project)
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Proj",
            "--description",
            "A project.",
            "--blueprint",
            "generic",
        ])
        .output()
        .expect("init --no-register output");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Claude fallback template path is"));
    assert!(stdout.contains("Run `llm-wiki install` to materialize it if needed"));
    assert!(!stdout.contains("Claude template staged at"));

    let project = project.canonicalize().expect("canonical project");
    assert!(
        !project.join(".mcp.json").exists(),
        "--no-register must not touch host MCP config"
    );
    assert!(
        !fallback_claude_template(home.path()).exists(),
        "fresh-home init --no-register must not imply the fallback template was materialized"
    );
}
