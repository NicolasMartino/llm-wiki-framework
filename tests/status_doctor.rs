use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command.env("HOME", home);
    command
}

#[test]
fn status_reports_installed_files() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path()).arg("install").assert().success();

    llm_wiki(home.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest files: "))
        .stdout(predicate::str::contains("OK: claude skill"));
}

#[test]
fn status_reports_drift() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path()).arg("install").assert().success();
    fs::write(
        home.path().join(".claude/skills/init-project/SKILL.md"),
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
fn doctor_reports_missing_and_unknown_files() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path()).arg("install").assert().success();
    fs::remove_file(home.path().join(".claude/skills/init-project/SKILL.md")).expect("remove");
    fs::write(
        home.path().join(".claude/skills/knowledge-query/SKILL.md"),
        "tampered",
    )
    .expect("write");
    let unknown = home.path().join(".claude/skills/knowledge-lint/SKILL.md");
    fs::remove_file(&unknown).expect("remove");
    let manifest = home.path().join(".llm_wiki/manifest.json");
    fs::remove_file(manifest).expect("remove manifest");
    fs::write(&unknown, "unknown").expect("write");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Unknown framework-shaped file"));
}

#[cfg(unix)]
#[test]
fn doctor_reports_legacy_symlink() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let path = home.path().join(".codex/skills/init-project/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    symlink(
        "/tmp/software_project_management/.codex/skills/init-project/SKILL.md",
        &path,
    )
    .expect("symlink");

    llm_wiki(home.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Legacy symlink residue"));
}

#[cfg(unix)]
#[test]
fn doctor_legacy_symlink_marker_can_be_overridden() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let path = home.path().join(".codex/skills/init-project/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    symlink(
        "/tmp/custom_legacy_repo/.codex/skills/init-project/SKILL.md",
        &path,
    )
    .expect("symlink");

    llm_wiki(home.path())
        .env("LLM_WIKI_LEGACY_SYMLINK_MARKER", "custom_legacy_repo")
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Legacy symlink residue"));
}
