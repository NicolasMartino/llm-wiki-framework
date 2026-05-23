use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;
use toml::Value as TomlValue;

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command.env("HOME", home).env_remove("RUST_LOG");
    command
}

#[test]
fn install_writes_files_and_manifest() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

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
    assert_disabled_search_profile(home.path());
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

#[test]
fn plain_noninteractive_install_requires_explicit_search_posture() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "interactive install requires a terminal for search setup",
        ))
        .stderr(predicate::str::contains("--disable-llm-search"));

    assert!(!home.path().join(".llm_wiki/manifest.json").exists());
}

#[test]
fn explicit_noninteractive_install_requires_search_posture() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--non-interactive"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "non-interactive install requires either",
        ))
        .stderr(predicate::str::contains("--enable-llm-search"))
        .stderr(predicate::str::contains("--disable-llm-search"));

    assert_no_install_or_search_writes(home.path());
}

#[test]
fn install_enable_llm_search_requires_noninteractive_profile() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--enable-llm-search", "--profile", "balanced"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--non-interactive"));

    assert_no_install_or_search_writes(home.path());
}

#[test]
fn install_enable_llm_search_conflicts_with_disable() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--disable-llm-search",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--enable-llm-search"))
        .stderr(predicate::str::contains("--disable-llm-search"));

    assert_no_install_or_search_writes(home.path());
}

#[test]
fn install_profile_and_consent_flags_require_enable_llm_search() {
    for flag in [
        &["--profile", "balanced"][..],
        &["--confirm-model-downloads"][..],
        &["--accept-profile-licenses"][..],
    ] {
        let home = TempDir::new().expect("home");
        let mut args = vec!["install", "--non-interactive"];
        args.extend_from_slice(flag);

        llm_wiki(home.path())
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("--enable-llm-search"));

        assert_no_install_or_search_writes(home.path());
    }
}

#[test]
fn noninteractive_enable_missing_download_confirmation_writes_no_state() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--accept-profile-licenses",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--confirm-model-downloads"))
        .stderr(predicate::str::contains(
            "no install or search state was changed",
        ));

    assert_no_install_or_search_writes(home.path());
}

#[test]
fn noninteractive_enable_missing_license_confirmation_writes_no_state() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--confirm-model-downloads",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--accept-profile-licenses"))
        .stderr(predicate::str::contains(
            "no install or search state was changed",
        ));

    assert_no_install_or_search_writes(home.path());
}

#[test]
fn noninteractive_enable_hash_mismatch_without_force_writes_no_install_state() {
    let home = TempDir::new().expect("home");
    let model_path = home
        .path()
        .join(".llm_wiki/models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf");
    fs::create_dir_all(model_path.parent().expect("model parent")).expect("model dir");
    fs::write(&model_path, "corrupt model bytes").expect("model bytes");

    llm_wiki(home.path())
        .args([
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--confirm-model-downloads",
            "--accept-profile-licenses",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("model artifact hash mismatch"))
        .stderr(predicate::str::contains("--force"));

    assert_no_install_or_search_writes(home.path());
    assert_eq!(
        fs::read_to_string(&model_path).expect("model remains"),
        "corrupt model bytes"
    );
}

#[test]
fn install_disable_llm_search_writes_disabled_search_profile() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    assert_disabled_search_profile(home.path());
}

#[test]
fn noninteractive_disable_llm_search_writes_disabled_search_profile() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "install",
            "--skip-path-guidance",
            "--non-interactive",
            "--disable-llm-search",
        ])
        .assert()
        .success();

    assert_disabled_search_profile(home.path());
}

#[cfg(unix)]
#[test]
fn install_managed_binary_is_executable() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

    let mode = fs::metadata(home.path().join(".llm_wiki/bin/llm-wiki"))
        .expect("managed binary")
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0);
}

#[test]
fn install_is_idempotent() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    let before = fs::read_to_string(home.path().join(".llm_wiki/manifest.json")).expect("manifest");
    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    let after = fs::read_to_string(home.path().join(".llm_wiki/manifest.json")).expect("manifest");

    let before: Value = serde_json::from_str(&before).expect("json");
    let after: Value = serde_json::from_str(&after).expect("json");
    assert_eq!(before["skills"], after["skills"]);
}

#[test]
fn verbose_install_emits_command_diagnostics() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "--verbose",
            "install",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: install"))
        .stderr(predicate::str::contains("current executable:"))
        .stderr(predicate::str::contains("managed binary:"))
        .stderr(predicate::str::contains("partial marker recovery:"))
        .stderr(predicate::str::contains("render target:"))
        .stderr(predicate::str::contains("collision classification:"));
}

#[test]
fn verbose_install_configure_search_emits_profile_diagnostics() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "--verbose",
            "install",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("configure search: true"))
        .stderr(predicate::str::contains("search config:"))
        .stderr(predicate::str::contains("external dependencies:"))
        .stderr(predicate::str::contains(
            "search configuration action: wrote disabled LLM search profile",
        ));
}

#[test]
fn verbose_noninteractive_disabled_install_reports_redundant_configure_search() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "--verbose",
            "install",
            "--skip-path-guidance",
            "--non-interactive",
            "--disable-llm-search",
            "--configure-search",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("non-interactive: true"))
        .stderr(predicate::str::contains(
            "search configuration selected posture: disabled",
        ))
        .stderr(predicate::str::contains(
            "redundant beside explicit disabled posture",
        ));
}

#[test]
fn verbose_noninteractive_enabled_refusal_reports_preflight_state() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args([
            "--verbose",
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--accept-profile-licenses",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("enable llm search: true"))
        .stderr(predicate::str::contains(
            "search profile selected: balanced",
        ))
        .stderr(predicate::str::contains(
            "search artifact classification summary:",
        ))
        .stderr(predicate::str::contains(
            "search license classification summary:",
        ))
        .stderr(predicate::str::contains(
            "search non-interactive missing runtime confirmation: --confirm-model-downloads",
        ))
        .stderr(predicate::str::contains(
            "search non-interactive refusal: no install state mutated",
        ));
}

#[test]
fn install_cleans_leaked_partial_marker_after_completed_manifest() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stale partial install targets"));

    llm_wiki(home.path())
        .args([
            "install",
            "--force",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "stale partial install was started by a different binary",
        ));

    llm_wiki(home.path())
        .args([
            "install",
            "--force",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
        .assert()
        .success();
}

#[test]
fn install_reports_interrupted_partial_binary_without_force() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "previous install left a partial managed binary",
        ));

    llm_wiki(home.path())
        .args([
            "install",
            "--force",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
        .assert()
        .success();
}

#[test]
fn install_rejects_unsupported_manifest_schema() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .failure();
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
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
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
        .args([
            "install",
            "--force",
            "--skip-path-guidance",
            "--disable-llm-search",
        ])
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
        .args(["install", "--force", "--disable-llm-search"])
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

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
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

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["uninstall", "--include-binary"])
        .assert()
        .success();

    assert!(!home.path().join(".llm_wiki/bin/llm-wiki").exists());
}

#[test]
fn uninstall_force_requires_search_artifacts() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["uninstall", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--search-artifacts"));

    llm_wiki(home.path())
        .args(["uninstall", "--include-binary", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with '--force'"));
}

#[test]
fn disable_llm_search_preserves_artifacts_and_prints_cleanup_guidance() {
    let home = TempDir::new().expect("home");
    seed_search_artifacts(home.path());

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "llm-wiki uninstall --search-artifacts",
        ));

    assert!(
        home.path()
            .join(".llm_wiki/models/fixture/model.gguf")
            .exists()
    );
    assert!(
        home.path()
            .join(".llm_wiki/indexes/fixture/semantic-index.json")
            .exists()
    );
    let accepted = read_toml(&home.path().join(".llm_wiki/accepted-licenses.toml"));
    assert!(
        accepted["licenses"]
            .as_array()
            .expect("licenses array")
            .is_empty()
    );
}

#[test]
fn search_artifact_cleanup_refuses_when_either_global_search_profile_is_enabled() {
    for (project_default_enabled, global_search_enabled) in [(true, false), (false, true)] {
        let home = TempDir::new().expect("home");
        seed_search_artifacts(home.path());
        write_search_config(home.path(), project_default_enabled, global_search_enabled);

        llm_wiki(home.path())
            .args(["uninstall", "--search-artifacts"])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "refusing to remove search artifacts while LLM search is enabled",
            ));

        assert!(
            home.path()
                .join(".llm_wiki/models/fixture/model.gguf")
                .exists()
        );
    }
}

#[test]
fn forced_search_artifact_cleanup_deletes_artifacts_without_rewriting_search_config() {
    let home = TempDir::new().expect("home");
    seed_search_artifacts(home.path());
    write_search_config(home.path(), true, true);

    llm_wiki(home.path())
        .args(["uninstall", "--search-artifacts", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed LLM search artifacts"));

    assert!(!home.path().join(".llm_wiki/models").exists());
    assert!(
        !home
            .path()
            .join(".llm_wiki/accepted-licenses.toml")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".llm_wiki/search-thresholds.toml")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".llm_wiki/indexes/fixture/semantic-index.json")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".llm_wiki/indexes/fixture/semantic-vectors.json")
            .exists()
    );
    assert!(
        home.path()
            .join(".llm_wiki/indexes/fixture/qmd-rs.sqlite")
            .exists()
    );
    let search = read_toml(&home.path().join(".llm_wiki/search.toml"));
    assert_eq!(
        search["project_default"]["llm_search_enabled"].as_bool(),
        Some(true)
    );
    assert_eq!(
        search["global_search"]["llm_search_enabled"].as_bool(),
        Some(true)
    );
}

#[test]
fn search_artifact_cleanup_preserves_install_registry_and_lexical_indexes() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    seed_search_artifacts(home.path());
    write_project_registry(home.path());

    llm_wiki(home.path())
        .args(["uninstall", "--search-artifacts"])
        .assert()
        .success();

    assert!(home.path().join(".llm_wiki/manifest.json").exists());
    assert!(home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(
        home.path()
            .join(".claude/skills/wiki-init/SKILL.md")
            .exists()
    );
    assert!(
        home.path()
            .join(".local/share/llm-wiki/projects.json")
            .exists()
    );
    assert!(
        home.path()
            .join(".llm_wiki/indexes/fixture/qmd-rs.sqlite")
            .exists()
    );
    assert!(!home.path().join(".llm_wiki/models").exists());
    assert!(
        !home
            .path()
            .join(".llm_wiki/accepted-licenses.toml")
            .exists()
    );
}

#[test]
fn full_uninstall_removes_global_runtime_state_but_keeps_project_local_state() {
    let home = TempDir::new().expect("home");
    let project = TempDir::new().expect("project");
    fs::create_dir_all(project.path().join(".llm_wiki")).expect("project state");
    fs::write(project.path().join(".llm_wiki/keep"), "project-local").expect("project keep");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    seed_search_artifacts(home.path());
    write_project_registry(home.path());

    llm_wiki(home.path()).arg("uninstall").assert().success();

    assert!(!home.path().join(".llm_wiki/manifest.json").exists());
    assert!(!home.path().join(".llm_wiki/models").exists());
    assert!(!home.path().join(".llm_wiki/indexes").exists());
    assert!(!home.path().join(".llm_wiki/search.toml").exists());
    assert!(
        !home
            .path()
            .join(".llm_wiki/accepted-licenses.toml")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".local/share/llm-wiki/projects.json")
            .exists()
    );
    assert!(home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(project.path().join(".llm_wiki/keep").exists());
}

#[test]
fn verbose_uninstall_emits_command_diagnostics() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["--verbose", "uninstall"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: uninstall"))
        .stderr(predicate::str::contains("manifest:"))
        .stderr(predicate::str::contains("include managed binary: false"))
        .stderr(predicate::str::contains("consider file:"))
        .stderr(predicate::str::contains("drift check:"));
}

#[test]
fn uninstall_refuses_drifted_manifest_file() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".claude/skills/wiki-init/SKILL.md");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
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

fn read_toml(path: &Path) -> TomlValue {
    toml::from_str(&fs::read_to_string(path).expect("toml exists")).expect("toml")
}

fn assert_disabled_search_profile(home: &Path) {
    let search = read_toml(&home.join(".llm_wiki/search.toml"));
    assert_eq!(search["schema_version"].as_integer(), Some(1));
    assert_eq!(
        search["project_default"]["llm_search_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        search["project_default"]["reason"].as_str(),
        Some("llm_search_disabled")
    );
    assert_eq!(
        search["global_search"]["llm_search_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        search["global_search"]["reason"].as_str(),
        Some("llm_search_disabled")
    );

    let external = read_toml(&home.join(".llm_wiki/external-dependencies.toml"));
    assert_eq!(external["schema_version"].as_integer(), Some(1));
    assert!(
        external["dependencies"]
            .as_array()
            .expect("dependencies array")
            .is_empty()
    );
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

fn assert_no_install_or_search_writes(home: &Path) {
    assert!(!home.join(".llm_wiki/manifest.json").exists());
    assert!(!home.join(".llm_wiki/install.partial.json").exists());
    assert!(!home.join(".llm_wiki/backups").exists());
    assert!(!home.join(".llm_wiki/bin/llm-wiki").exists());
    assert!(!home.join(".llm_wiki/accepted-licenses.toml").exists());
    assert!(!home.join(".llm_wiki/models/artifacts.toml").exists());
    assert!(!home.join(".llm_wiki/search.toml").exists());
    assert!(!home.join(".llm_wiki/external-dependencies.toml").exists());
    assert!(!home.join(".claude/skills/wiki-init/SKILL.md").exists());
    assert!(!home.join(".codex/skills/wiki/SKILL.md").exists());
    assert!(!home.join(".codex/skills/wiki/agents/openai.yaml").exists());
}

fn seed_search_artifacts(home: &Path) {
    let model = home.join(".llm_wiki/models/fixture/model.gguf");
    fs::create_dir_all(model.parent().expect("model parent")).expect("model dir");
    fs::write(&model, "model-bytes").expect("model");
    fs::write(
        home.join(".llm_wiki/models/artifacts.toml"),
        "schema_version = 1\n",
    )
    .expect("artifacts");
    fs::write(
        home.join(".llm_wiki/accepted-licenses.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-14T00:00:00Z"

[[licenses]]
model_id = "fixture"
license = "test"
accepted_at = "2026-05-14T00:00:00Z"
accepted_by_version = "test"
"#,
    )
    .expect("accepted licenses");
    fs::write(
        home.join(".llm_wiki/search-thresholds.toml"),
        "schema_version = 2\nupdated_at = \"2026-05-14T00:00:00Z\"\nthresholds = []\n",
    )
    .expect("thresholds");
    let index = home.join(".llm_wiki/indexes/fixture");
    fs::create_dir_all(&index).expect("index dir");
    fs::write(index.join("semantic-index.json"), "{}").expect("semantic metadata");
    fs::write(index.join("semantic-vectors.json"), "{}").expect("semantic vectors");
    fs::write(index.join("qmd-rs.sqlite"), "lexical").expect("qmd store");
}

fn write_project_registry(home: &Path) {
    let registry = home.join(".local/share/llm-wiki/projects.json");
    fs::create_dir_all(registry.parent().expect("registry parent")).expect("registry dir");
    fs::write(
        registry,
        r#"
{
  "schema_version": 1,
  "updated_at": "2026-05-14T00:00:00Z",
  "projects": []
}
"#,
    )
    .expect("registry");
}

fn write_search_config(home: &Path, project_default_enabled: bool, global_search_enabled: bool) {
    let search = home.join(".llm_wiki/search.toml");
    fs::create_dir_all(search.parent().expect("search parent")).expect("search dir");
    fs::write(
        search,
        format!(
            r#"
schema_version = 1
updated_at = "2026-05-14T00:00:00Z"

[project_default]
{}

[global_search]
{}
"#,
            search_profile_toml(project_default_enabled),
            search_profile_toml(global_search_enabled)
        ),
    )
    .expect("search config");
}

fn search_profile_toml(enabled: bool) -> &'static str {
    if enabled {
        r#"llm_search_enabled = true
configured_at = "2026-05-14T00:00:00Z"
configured_by_version = "test"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m""#
    } else {
        r#"llm_search_enabled = false
configured_at = "2026-05-14T00:00:00Z"
configured_by_version = "test"
reason = "llm_search_disabled""#
    }
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
