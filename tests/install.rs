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
            .join(".claude/skills/wiki-init/SKILL.md")
            .exists()
    );
    assert!(
        home.path()
            .join(".codex/skills/wiki/agents/openai.yaml")
            .exists()
    );
    let manifest = read_manifest(home.path());
    assert_eq!(manifest["binary"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(!home.path().join(".llm_wiki/install.partial.json").exists());
    let backup_manifest = Path::new(
        manifest["backups"][0]["path"]
            .as_str()
            .expect("backup manifest path"),
    );
    assert!(backup_manifest.exists());
    assert_eq!(
        manifest["skills"].as_array().expect("files").len(),
        installed_files(home.path())
    );
    let skill =
        fs::read_to_string(home.path().join(".claude/skills/wiki-init/SKILL.md")).expect("skill");
    assert!(skill.contains(".llm_wiki/bin/llm-wiki"));
    assert!(!skill.contains("`llm-wiki init "));
}

#[cfg(unix)]
#[test]
fn install_managed_binary_is_executable() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).arg("install").assert().success();

    let mode = fs::metadata(home.path().join(".llm_wiki/bin/llm-wiki"))
        .expect("managed binary")
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0);
}

#[test]
fn install_is_idempotent() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).arg("install").assert().success();
    let before = fs::read_to_string(home.path().join(".llm_wiki/manifest.json")).expect("manifest");
    llm_wiki(home.path()).arg("install").assert().success();
    let after = fs::read_to_string(home.path().join(".llm_wiki/manifest.json")).expect("manifest");

    let before: Value = serde_json::from_str(&before).expect("json");
    let after: Value = serde_json::from_str(&after).expect("json");
    assert_eq!(before["skills"], after["skills"]);
}

#[test]
fn install_cleans_leaked_partial_marker_after_completed_manifest() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .success();
    let manifest = read_manifest(home.path());
    let partial = serde_json::json!({
        "schema_version": 1,
        "started_at": "2026-05-06T12:00:00Z",
        "current_exe": manifest["binary"]["path"],
        "target_binary": manifest["binary"]["path"],
        "current_exe_hash_algorithm": "sha256",
        "current_exe_hash": manifest["binary"]["hash"],
        "phase": "binary-copy"
    });
    fs::write(
        home.path().join(".llm_wiki/install.partial.json"),
        serde_json::to_string_pretty(&partial).expect("partial json"),
    )
    .expect("partial");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .success();

    assert!(!home.path().join(".llm_wiki/install.partial.json").exists());
}

#[test]
fn install_rejects_stale_partial_target_without_force() {
    let home = TempDir::new().expect("home");
    write_partial(
        home.path(),
        home.path().join(".llm_wiki/bin/other-llm-wiki"),
        "different-hash",
    );

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stale partial install targets"));

    llm_wiki(home.path())
        .args(["install", "--force", "--skip-path-guidance"])
        .assert()
        .success();
}

#[test]
fn install_rejects_stale_partial_hash_without_force() {
    let home = TempDir::new().expect("home");
    write_partial(
        home.path(),
        home.path().join(".llm_wiki/bin/llm-wiki"),
        "different-hash",
    );

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "stale partial install was started by a different binary",
        ));

    llm_wiki(home.path())
        .args(["install", "--force", "--skip-path-guidance"])
        .assert()
        .success();
}

#[test]
fn install_reports_interrupted_partial_binary_without_force() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .success();
    let manifest = read_manifest(home.path());
    let managed_binary = home.path().join(".llm_wiki/bin/llm-wiki");
    fs::write(&managed_binary, "partial binary").expect("partial binary");
    fs::remove_file(home.path().join(".llm_wiki/manifest.json")).expect("remove manifest");
    write_partial(
        home.path(),
        &managed_binary,
        manifest["binary"]["hash"].as_str().expect("binary hash"),
    );

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "previous install left a partial managed binary",
        ));

    llm_wiki(home.path())
        .args(["install", "--force", "--skip-path-guidance"])
        .assert()
        .success();
}

#[test]
fn install_rejects_unsupported_manifest_schema() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .success();
    let manifest_path = home.path().join(".llm_wiki/manifest.json");
    let mut manifest = read_manifest(home.path());
    manifest["schema_version"] = serde_json::json!(3);
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest json"),
    )
    .expect("write manifest");

    llm_wiki(home.path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unsupported manifest schema_version 3",
        ));
}

#[test]
fn install_rejects_unsupported_partial_schema() {
    let home = TempDir::new().expect("home");
    write_partial(
        home.path(),
        home.path().join(".llm_wiki/bin/llm-wiki"),
        "different-hash",
    );
    let partial_path = home.path().join(".llm_wiki/install.partial.json");
    let mut partial: Value =
        serde_json::from_str(&fs::read_to_string(&partial_path).expect("partial"))
            .expect("partial json");
    partial["schema_version"] = serde_json::json!(2);
    fs::write(
        partial_path,
        serde_json::to_string_pretty(&partial).expect("partial json"),
    )
    .expect("write partial");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unsupported partial install schema_version 2",
        ));
}

#[test]
fn install_refuses_user_authored_collision_by_default() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/wiki-init/SKILL.md");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "user skill").expect("write");

    llm_wiki(home.path()).arg("install").assert().failure();
    assert_eq!(fs::read_to_string(&path).expect("read"), "user skill");
    assert!(!home.path().join(".llm_wiki").exists());
}

#[test]
fn install_refuses_unmanaged_binary_collision_by_default() {
    let home = TempDir::new().expect("home");
    let managed_binary = home.path().join(".llm_wiki/bin/llm-wiki");
    fs::create_dir_all(managed_binary.parent().expect("parent")).expect("mkdir");
    fs::write(&managed_binary, "foreign binary").expect("write foreign");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to replace unmanaged binary",
        ));
    assert_eq!(
        fs::read_to_string(&managed_binary).expect("foreign remains"),
        "foreign binary"
    );
    assert_no_install_metadata(home.path());

    llm_wiki(home.path())
        .args(["install", "--force", "--skip-path-guidance"])
        .assert()
        .success();

    let manifest = read_manifest(home.path());
    let backup_manifest = Path::new(
        manifest["backups"][0]["path"]
            .as_str()
            .expect("backup manifest path"),
    );
    let backup_manifest: Value =
        serde_json::from_str(&fs::read_to_string(backup_manifest).expect("backup manifest"))
            .expect("backup json");
    let binary_backup = backup_manifest["files"]
        .as_array()
        .expect("backup files")
        .iter()
        .find(|entry| entry["kind"] == "managed-binary")
        .expect("managed binary backup");
    assert_eq!(
        binary_backup["original_path"]
            .as_str()
            .expect("original path"),
        managed_binary.to_string_lossy()
    );
    assert_eq!(
        fs::read_to_string(binary_backup["backup_path"].as_str().expect("backup path"))
            .expect("backup contents"),
        "foreign binary"
    );
}

#[test]
fn force_install_backs_up_and_replaces_collision() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/wiki-init/SKILL.md");
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
    let manifest = read_manifest(home.path());
    let backup_manifest = Path::new(
        manifest["backups"][0]["path"]
            .as_str()
            .expect("backup manifest path"),
    );
    let backup_manifest: Value =
        serde_json::from_str(&fs::read_to_string(backup_manifest).expect("backup manifest"))
            .expect("backup json");
    assert_eq!(
        backup_manifest["files"][0]["original_path"]
            .as_str()
            .expect("original path"),
        path.to_string_lossy()
    );
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
    assert!(!home.path().join(".llm_wiki/manifest.json").exists());
    assert!(home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(
        !home
            .path()
            .join(".claude/skills/wiki-init/SKILL.md")
            .exists()
    );
}

#[test]
fn uninstall_include_binary_removes_managed_binary() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).arg("install").assert().success();
    llm_wiki(home.path())
        .args(["uninstall", "--include-binary"])
        .assert()
        .success();

    assert!(!home.path().join(".llm_wiki/bin/llm-wiki").exists());
}

#[test]
fn uninstall_refuses_drifted_manifest_file() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/wiki-init/SKILL.md");

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
    assert!(home.path().join(".llm_wiki/manifest.json").exists());
}

fn read_manifest(home: &Path) -> Value {
    let raw = fs::read_to_string(home.join(".llm_wiki/manifest.json")).expect("manifest exists");
    serde_json::from_str(&raw).expect("manifest json")
}

fn write_partial(home: &Path, target_binary: impl AsRef<Path>, current_exe_hash: &str) {
    let partial = serde_json::json!({
        "schema_version": 1,
        "started_at": "2026-05-06T12:00:00Z",
        "current_exe": "/tmp/llm-wiki",
        "target_binary": target_binary.as_ref(),
        "current_exe_hash_algorithm": "sha256",
        "current_exe_hash": current_exe_hash,
        "phase": "binary-copy"
    });
    let partial_path = home.join(".llm_wiki/install.partial.json");
    fs::create_dir_all(partial_path.parent().expect("parent")).expect("mkdir");
    fs::write(
        partial_path,
        serde_json::to_string_pretty(&partial).expect("partial json"),
    )
    .expect("partial");
}

fn assert_no_install_metadata(home: &Path) {
    assert!(!home.join(".llm_wiki/manifest.json").exists());
    assert!(!home.join(".llm_wiki/install.partial.json").exists());
    assert!(!home.join(".llm_wiki/backups").exists());
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
