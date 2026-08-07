use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn llm_wiki() -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME");
    command
}

#[test]
fn redirected_home_install_manifest_matches_filesystem() {
    let home = TempDir::new().expect("home");

    llm_wiki()
        .env("HOME", home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

    let manifest_path = home.path().join(".llm_wiki/manifest.json");
    let manifest_raw = fs::read_to_string(&manifest_path).expect("manifest");
    let manifest: Value = serde_json::from_str(&manifest_raw).expect("manifest json");
    let files = manifest["skills"].as_array().expect("manifest files");

    assert_eq!(manifest["binary"]["version"], env!("CARGO_PKG_VERSION"));
    let binary_path = PathBuf::from(manifest["binary"]["path"].as_str().expect("binary path"));
    assert_eq!(
        sha256_hex(&fs::read(&binary_path).expect("read managed binary")),
        manifest["binary"]["hash"].as_str().expect("binary hash")
    );
    assert!(
        manifest["binary"]["source_hash"].as_str().is_some(),
        "manifest records source executable hash"
    );
    assert_eq!(files.len(), 0);

    let mut manifest_paths = BTreeSet::new();
    for entry in files {
        let path = PathBuf::from(entry["path"].as_str().expect("path"));
        let expected_hash = entry["hash"].as_str().expect("sha256");
        assert!(path.exists(), "manifest path exists: {}", path.display());
        assert!(
            path.starts_with(home.path()),
            "path is under redirected HOME"
        );
        assert_eq!(
            sha256_hex(&fs::read(&path).expect("read installed file")),
            expected_hash
        );
        manifest_paths.insert(path);
    }

    llm_wiki()
        .env("HOME", home.path())
        .arg("status")
        .assert()
        .success();
}

#[test]
fn managed_binary_runs_without_path_after_install() {
    let home = TempDir::new().expect("home");
    let sanitized_path = "";

    llm_wiki()
        .env("HOME", home.path())
        .env("PATH", sanitized_path)
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let config = fs::read_to_string(home.path().join(".codex/config.toml")).expect("codex config");
    let parsed: toml::Value = toml::from_str(&config).expect("codex config toml");
    let server = &parsed["mcp_servers"]["llm-wiki"];
    assert!(
        server["command"]
            .as_str()
            .expect("command")
            .contains(".llm_wiki/bin/llm-wiki")
    );
    assert_eq!(
        server["args"]
            .as_array()
            .expect("args")
            .iter()
            .map(|value| value.as_str().expect("arg"))
            .collect::<Vec<_>>(),
        vec!["mcp", "serve"]
    );

    let output = run_installed_binary_bounded(
        home.path().join(".llm_wiki/bin/llm-wiki"),
        home.path(),
        sanitized_path,
        &["status"],
    );
    assert!(
        output.status.success(),
        "managed status failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("managed binary:"),
        "status stdout did not report managed binary: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn managed_binary_can_self_install() {
    let home = TempDir::new().expect("home");

    llm_wiki()
        .env("HOME", home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    let output = run_installed_binary_bounded(
        home.path().join(".llm_wiki/bin/llm-wiki"),
        home.path(),
        "",
        &["install", "--skip-path-guidance", "--disable-llm-search"],
    );
    assert!(
        output.status.success(),
        "managed self-install failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

fn run_installed_binary_bounded(
    binary: PathBuf,
    home: &std::path::Path,
    path: &str,
    args: &[&str],
) -> Output {
    let mut child = StdCommand::new(&binary)
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env("PATH", path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {}: {error}", binary.display()));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if child
            .try_wait()
            .unwrap_or_else(|error| panic!("failed to poll {}: {error}", binary.display()))
            .is_some()
        {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout)
                    .expect("read managed binary stdout");
            }
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr)
                    .expect("read managed binary stderr");
            }
            let status = child
                .wait()
                .unwrap_or_else(|error| panic!("failed to reap {}: {error}", binary.display()));
            return Output {
                status,
                stdout,
                stderr,
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!(
                "timed out waiting for {} {}",
                binary.display(),
                args.join(" ")
            );
        }
        thread::sleep(Duration::from_millis(50));
    }
}
