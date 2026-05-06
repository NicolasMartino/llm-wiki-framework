use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[test]
fn redirected_home_install_manifest_matches_filesystem() {
    let home = TempDir::new().expect("home");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env("HOME", home.path())
        .arg("install")
        .assert()
        .success();

    let manifest_path = home.path().join(".llm_wiki/manifest.json");
    let manifest_raw = fs::read_to_string(&manifest_path).expect("manifest");
    let manifest: Value = serde_json::from_str(&manifest_raw).expect("manifest json");
    let files = manifest["skills"].as_array().expect("manifest files");

    assert_eq!(manifest["binary"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(files.len(), installed_files(home.path()).len());

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

    for path in installed_files(home.path()) {
        assert!(
            manifest_paths.contains(&path),
            "installed file has manifest entry: {}",
            path.display()
        );
    }

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env("HOME", home.path())
        .arg("status")
        .assert()
        .success();
}

#[test]
fn managed_binary_runs_without_path_after_install() {
    let home = TempDir::new().expect("home");
    let sanitized_path = "";

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env("HOME", home.path())
        .env("PATH", sanitized_path)
        .args(["install", "--skip-path-guidance"])
        .assert()
        .success();

    let skill =
        fs::read_to_string(home.path().join(".codex/skills/knowledge-init/SKILL.md")).expect("skill");
    assert!(skill.contains(".llm_wiki/bin/llm-wiki"));
    assert!(!skill.contains("`llm-wiki init "));

    Command::new(home.path().join(".llm_wiki/bin/llm-wiki"))
        .env("HOME", home.path())
        .env("PATH", sanitized_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("managed binary:"));
}

fn installed_files(home: &Path) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    for root in [home.join(".claude/skills"), home.join(".codex/skills")] {
        if root.exists() {
            collect_files(&root, &mut files);
        }
    }
    files
}

fn collect_files(path: &Path, files: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(path).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.insert(path);
        }
    }
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}
