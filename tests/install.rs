use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command.env("HOME", home);
    command
}

#[test]
fn install_writes_files_and_manifest() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).arg("install").assert().success();

    assert!(
        home.path()
            .join(".claude/skills/init-project/SKILL.md")
            .exists()
    );
    assert!(
        home.path()
            .join(".codex/skills/knowledge/agents/openai.yaml")
            .exists()
    );
    let manifest = read_manifest(home.path());
    assert_eq!(manifest["binary_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        manifest["files"].as_array().expect("files").len(),
        installed_files(home.path())
    );
}

#[test]
fn install_is_idempotent() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).arg("install").assert().success();
    let before = fs::read_to_string(home.path().join(".local/share/llm-wiki/manifest.json"))
        .expect("manifest");
    llm_wiki(home.path()).arg("install").assert().success();
    let after = fs::read_to_string(home.path().join(".local/share/llm-wiki/manifest.json"))
        .expect("manifest");

    let before: Value = serde_json::from_str(&before).expect("json");
    let after: Value = serde_json::from_str(&after).expect("json");
    assert_eq!(before["files"], after["files"]);
}

#[test]
fn install_refuses_user_authored_collision_by_default() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/init-project/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "user skill").expect("write");

    llm_wiki(home.path()).arg("install").assert().failure();
    assert_eq!(fs::read_to_string(&path).expect("read"), "user skill");
}

#[test]
fn force_install_backs_up_and_replaces_collision() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/init-project/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "user skill").expect("write");

    llm_wiki(home.path())
        .args(["install", "--force"])
        .assert()
        .success();

    let backups = fs::read_dir(path.parent().expect("parent"))
        .expect("read_dir")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("SKILL.md.bak.")
        })
        .count();
    assert_eq!(backups, 1);
    assert_ne!(fs::read_to_string(&path).expect("read"), "user skill");
}

#[test]
fn uninstall_removes_manifest_owned_files_only() {
    let home = TempDir::new().expect("home");
    let user_file = home.path().join(".codex/skills/user-skill/SKILL.md");
    fs::create_dir_all(user_file.parent().expect("parent")).expect("mkdir");
    fs::write(&user_file, "keep").expect("write");

    llm_wiki(home.path()).arg("install").assert().success();
    llm_wiki(home.path()).arg("uninstall").assert().success();

    assert!(user_file.exists());
    assert!(
        !home
            .path()
            .join(".local/share/llm-wiki/manifest.json")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".claude/skills/init-project/SKILL.md")
            .exists()
    );
}

#[test]
fn uninstall_refuses_drifted_manifest_file() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/init-project/SKILL.md");

    llm_wiki(home.path()).arg("install").assert().success();
    fs::write(&path, "user edit").expect("write");

    llm_wiki(home.path())
        .arg("uninstall")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to uninstall drifted file",
        ));

    assert_eq!(fs::read_to_string(&path).expect("read"), "user edit");
    assert!(
        home.path()
            .join(".local/share/llm-wiki/manifest.json")
            .exists()
    );
}

fn read_manifest(home: &Path) -> Value {
    let raw = fs::read_to_string(home.join(".local/share/llm-wiki/manifest.json"))
        .expect("manifest exists");
    serde_json::from_str(&raw).expect("manifest json")
}

fn installed_files(home: &Path) -> usize {
    let mut count = 0;
    for root in [home.join(".claude/skills"), home.join(".codex/skills")] {
        count += count_files(&root);
    }
    count
}

fn count_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            if entry.path().is_dir() {
                count_files(&entry.path())
            } else {
                1
            }
        })
        .sum()
}
