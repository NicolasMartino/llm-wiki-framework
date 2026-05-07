use std::fs;
use std::path::Path;

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
        home.path().join(".claude/skills/knowledge-init/SKILL.md"),
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
    fs::remove_file(home.path().join(".claude/skills/knowledge-init/SKILL.md")).expect("remove");
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
    assert!(
        !home
            .path()
            .join(".claude/skills/knowledge-init/SKILL.md")
            .exists()
    );
}

#[test]
fn doctor_reports_path_binary_drift() {
    let home = TempDir::new().expect("home");
    let fake_bin = TempDir::new().expect("fake bin");
    fs::write(fake_bin.path().join("llm-wiki"), "different binary").expect("fake binary");

    llm_wiki(home.path()).arg("install").assert().success();

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

#[cfg(not(feature = "qmd-rs"))]
#[test]
fn doctor_reports_feature_disabled_search_backend_in_wiki_project_without_qmd_rs() {
    let home = TempDir::new().expect("home");
    let project = wiki_project();

    llm_wiki(home.path())
        .current_dir(project.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected wiki project"))
        .stdout(predicate::str::contains("Search index:"))
        .stdout(predicate::str::contains(
            "qmd-rs backend feature is disabled",
        ))
        .stdout(predicate::str::contains("Semantic models:"));
}

#[cfg(feature = "qmd-rs")]
#[test]
fn doctor_reports_missing_search_index_in_qmd_rs_build() {
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
fn doctor_reports_legacy_symlink() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let path = home.path().join(".codex/skills/knowledge-init/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    symlink(
        "/tmp/software_project_management/.codex/skills/knowledge-init/SKILL.md",
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
    let path = home.path().join(".codex/skills/knowledge-init/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    symlink(
        "/tmp/custom_legacy_repo/.codex/skills/knowledge-init/SKILL.md",
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
