use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;
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

#[test]
fn install_writes_mcp_config_surfaces() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let codex_config_path = home.path().join(".codex/config.toml");
    let codex_config = fs::read_to_string(&codex_config_path).expect("codex config");
    let parsed: toml::Value = toml::from_str(&codex_config).expect("codex config toml");
    let server = &parsed["mcp_servers"][mcp_server_name()];
    assert_eq!(
        server["command"].as_str().expect("command"),
        home.path()
            .join(format!("{}/bin/{}", managed_home_dir_name(), binary_name()))
            .to_string_lossy()
    );
    let args = server["args"].as_array().expect("args");
    assert_eq!(args[0].as_str().expect("arg 0"), "mcp");
    assert_eq!(args[1].as_str().expect("arg 1"), "serve");

    let claude_config_path = home.path().join(format!(
        "{}/mcp/claude-project.mcp.json",
        managed_home_dir_name()
    ));
    let claude_config: Value =
        serde_json::from_str(&fs::read_to_string(&claude_config_path).expect("claude config"))
            .expect("claude config json");
    let server = &claude_config["mcpServers"][mcp_server_name()];
    assert_eq!(server["type"], "stdio");
    assert_eq!(
        server["command"].as_str().expect("claude command"),
        home.path()
            .join(format!("{}/bin/{}", managed_home_dir_name(), binary_name()))
            .to_string_lossy()
    );
    assert_eq!(server["args"], serde_json::json!(["mcp", "serve"]));
}

#[test]
fn install_preserves_existing_codex_config_while_adding_mcp_server() {
    let home = TempDir::new().expect("home");
    let codex_dir = home.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::write(
        codex_dir.join("config.toml"),
        r#"model = "gpt-5.5"

[mcp_servers.other]
command = "other"
"#,
    )
    .expect("existing config");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let parsed: toml::Value =
        toml::from_str(&fs::read_to_string(codex_dir.join("config.toml")).expect("config"))
            .expect("toml");
    assert_eq!(parsed["model"].as_str().expect("model"), "gpt-5.5");
    assert_eq!(
        parsed["mcp_servers"]["other"]["command"]
            .as_str()
            .expect("other command"),
        "other"
    );
    assert_eq!(
        parsed["mcp_servers"][mcp_server_name()]["args"]
            .as_array()
            .expect("active llm-wiki args")
            .len(),
        2
    );
}

#[test]
fn test_instance_install_preserves_production_codex_mcp_server() {
    if !is_test_instance() {
        return;
    }

    let home = TempDir::new().expect("home");
    let config_path = home.path().join(".codex/config.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("config parent");
    fs::write(
        &config_path,
        r#"
[mcp_servers.llm-wiki]
command = "/prod/llm-wiki"
args = ["mcp", "serve"]
"#,
    )
    .expect("write config");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let parsed: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).expect("codex config"))
            .expect("codex config toml");
    assert_eq!(
        parsed["mcp_servers"]["llm-wiki"]["command"].as_str(),
        Some("/prod/llm-wiki")
    );
    assert!(
        parsed["mcp_servers"]
            .as_table()
            .expect("mcp servers")
            .contains_key(mcp_server_name())
    );

    llm_wiki(home.path()).arg("uninstall").assert().success();

    let parsed: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).expect("codex config"))
            .expect("codex config toml");
    assert_eq!(
        parsed["mcp_servers"]["llm-wiki"]["command"].as_str(),
        Some("/prod/llm-wiki")
    );
    assert!(
        !parsed["mcp_servers"]
            .as_table()
            .expect("mcp servers")
            .contains_key(mcp_server_name())
    );
}

#[test]
fn uninstall_removes_owned_codex_mcp_config_file() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let codex_config_path = home.path().join(".codex/config.toml");
    assert!(codex_config_path.exists());
    assert!(
        home.path()
            .join(format!(
                "{}/mcp/claude-project.mcp.json",
                managed_home_dir_name()
            ))
            .exists()
    );

    llm_wiki(home.path()).arg("uninstall").assert().success();

    assert!(!codex_config_path.exists());
    assert!(
        !home
            .path()
            .join(format!(
                "{}/mcp/claude-project.mcp.json",
                managed_home_dir_name()
            ))
            .exists()
    );
}

#[test]
fn uninstall_without_manifest_still_clears_codex_and_staged_claude_config() {
    let home = TempDir::new().expect("home");
    let codex_dir = home.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("codex dir");
    let codex_config_path = codex_dir.join("config.toml");
    fs::write(
        &codex_config_path,
        "model = \"gpt-5\"\n\n[mcp_servers.other]\ncommand = \"other-mcp\"\nargs = [\"serve\"]\n",
    )
    .expect("seed codex config");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let staged_claude = home.path().join(format!(
        "{}/mcp/claude-project.mcp.json",
        managed_home_dir_name()
    ));
    assert!(staged_claude.exists());

    // Simulate a corrupt/missing manifest: uninstall must still remove the MCP
    // wiring rather than leave the host spawning a now-deleted binary.
    let manifest_path = home
        .path()
        .join(format!("{}/manifest.json", managed_home_dir_name()));
    fs::remove_file(&manifest_path).expect("remove manifest");

    llm_wiki(home.path()).arg("uninstall").assert().success();

    let codex_config = fs::read_to_string(&codex_config_path).expect("codex config remains");
    let parsed: toml::Value = toml::from_str(&codex_config).expect("codex toml");
    assert_eq!(parsed["model"].as_str(), Some("gpt-5"));
    assert_eq!(
        parsed["mcp_servers"]["other"]["command"].as_str(),
        Some("other-mcp")
    );
    assert!(
        parsed["mcp_servers"].get(mcp_server_name()).is_none(),
        "stale llm-wiki MCP server must be removed even without a manifest"
    );
    assert!(
        !staged_claude.exists(),
        "staged Claude MCP config must be cleaned up even without a manifest"
    );
    // Uninstall removes everything install put in place, both binaries
    // included, with or without a manifest.
    let bin_dir = home.path().join(format!("{}/bin", managed_home_dir_name()));
    assert!(!bin_dir.join(binary_name()).exists());
    assert!(!bin_dir.join("poman").exists());
    assert!(!bin_dir.exists());
}

#[test]
fn uninstall_removes_only_owned_codex_mcp_server() {
    let home = TempDir::new().expect("home");
    let codex_dir = home.path().join(".codex");
    fs::create_dir_all(&codex_dir).expect("codex dir");
    let codex_config_path = codex_dir.join("config.toml");
    fs::write(
        &codex_config_path,
        r#"
model = "gpt-5"

[mcp_servers.other]
command = "other-mcp"
args = ["serve"]
"#,
    )
    .expect("seed codex config");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    llm_wiki(home.path()).arg("uninstall").assert().success();

    let codex_config = fs::read_to_string(&codex_config_path).expect("codex config remains");
    let parsed: toml::Value = toml::from_str(&codex_config).expect("codex toml");
    assert_eq!(parsed["model"].as_str(), Some("gpt-5"));
    assert_eq!(
        parsed["mcp_servers"]["other"]["command"].as_str(),
        Some("other-mcp")
    );
    assert!(parsed["mcp_servers"].get(mcp_server_name()).is_none());
}
