use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use toml::Value as TomlValue;

mod support;

fn llm_wiki(home: &Path) -> Command {
    llm_wiki_at(&support::llm_wiki_bin(), home)
}

fn llm_wiki_at(bin: &Path, home: &Path) -> Command {
    let mut command = Command::new(bin);
    command
        .env("HOME", home)
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME");
    command
}

#[test]
fn install_writes_files_and_manifest() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();

    assert!(home.path().join(".codex/config.toml").exists());
    assert!(
        home.path()
            .join(".llm_wiki/mcp/claude-project.mcp.json")
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
    assert_eq!(manifest["skills"].as_array().expect("files").len(), 0);
    assert_eq!(
        manifest["assets"][0]["kind"]
            .as_str()
            .expect("managed asset kind"),
        "mcp-config"
    );
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
fn enabled_install_reports_existing_model_hashing_as_bounded_stderr_lines() {
    let home = TempDir::new().expect("home");
    let model_path = home
        .path()
        .join(".llm_wiki/models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf");
    fs::create_dir_all(model_path.parent().expect("model parent")).expect("model dir");
    fs::write(&model_path, "corrupt model bytes").expect("model bytes");

    let output = llm_wiki(home.path())
        .args([
            "--verbose",
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--confirm-model-downloads",
            "--accept-profile-licenses",
        ])
        .output()
        .expect("run install");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");

    let progress = stderr
        .lines()
        .filter(|line| line.starts_with('['))
        .collect::<Vec<_>>();
    assert_eq!(progress.len(), 2, "{stderr}");
    assert_eq!(
        progress[0],
        "[1/2] embeddinggemma-300m-q8_0 verify start 318.1 MiB"
    );
    assert!(
        progress[1].starts_with("[1/2] embeddinggemma-300m-q8_0 verify done 19 B in "),
        "{stderr}"
    );
    assert!(!stderr.contains("\x1b[") && !stderr.contains('\r'));
    assert!(
        !stdout.lines().any(|line| line.starts_with("[1/2]")),
        "{stdout}"
    );
    assert_eq!(
        stderr
            .matches("search artifact classification: embeddinggemma-300m-q8_0 hash-mismatch")
            .count(),
        1,
        "{stderr}"
    );
}

#[test]
fn verbose_enabled_install_reports_download_and_verify_progress_once() {
    let home = TempDir::new().expect("home");
    let source = home.path().join("wrong-model.gguf");
    fs::write(&source, vec![0_u8; 1000]).expect("model source");

    let output = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_MODEL_SOURCE", &source)
        .args([
            "--verbose",
            "install",
            "--non-interactive",
            "--enable-llm-search",
            "--profile",
            "balanced",
            "--confirm-model-downloads",
            "--accept-profile-licenses",
        ])
        .output()
        .expect("run install");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");

    let progress = stderr
        .lines()
        .filter(|line| line.starts_with('['))
        .map(|line| line.split(" in ").next().expect("line"))
        .collect::<Vec<_>>();
    assert_eq!(
        progress,
        [
            "[1/2] embeddinggemma-300m-q8_0 download start 318.1 MiB",
            "[1/2] embeddinggemma-300m-q8_0 download done 1000 B",
            "[1/2] embeddinggemma-300m-q8_0 verify start 318.1 MiB",
        ],
        "{stderr}"
    );
    assert!(
        stderr.contains("downloaded model artifact hash mismatch for embeddinggemma-300m-q8_0")
    );
    for diagnostic in [
        "search model materialization: embeddinggemma-300m-q8_0 -> ",
        "search model content length mismatch: embeddinggemma-300m-q8_0 expected 333590944 bytes, server reported 1000 bytes",
    ] {
        assert_eq!(stderr.matches(diagnostic).count(), 1, "{stderr}");
    }
    assert!(!stderr.contains("\x1b[") && !stderr.contains('\r'));
    assert!(
        !stdout.lines().any(|line| line.starts_with("[1/2]")),
        "{stdout}"
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
        .stderr(predicate::str::contains(
            "runtime skill projection disabled",
        ))
        .stderr(predicate::str::contains(
            "MCP server startup is host-managed stdio",
        ))
        .stderr(predicate::str::contains("wired Codex MCP config"))
        .stderr(predicate::str::contains("materialized Claude MCP config"))
        .stderr(predicate::str::contains("is a fallback for manual setups"));
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
        .stderr(predicate::str::contains("configure search: false"))
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
        manifest["binary"]["source_hash"]
            .as_str()
            .expect("binary source hash"),
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
    manifest["schema_version"] = serde_json::json!(4);
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest json"),
    )
    .expect("write manifest");

    // `status` now surfaces a corrupt/unsupported manifest as a finding and
    // exits successfully, rather than aborting — a diagnostic command must be
    // able to report exactly this corruption class instead of failing on it.
    llm_wiki(home.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "unsupported manifest schema_version 4",
        ));
}

#[test]
fn install_removes_unchanged_legacy_generated_skills() {
    let home = TempDir::new().expect("home");
    let skill_path = home.path().join(".codex/skills/wiki-query/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("skill parent");
    let generated = "generated wiki query skill";
    fs::write(&skill_path, generated).expect("write generated skill");
    write_legacy_manifest(
        home.path(),
        &[legacy_skill_entry(
            &skill_path,
            "wiki-query",
            "codex",
            sha256_hex(generated.as_bytes()),
        )],
    );

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();

    assert!(!skill_path.exists());
    assert!(!skill_path.parent().expect("skill parent").exists());
    let manifest = read_manifest(home.path());
    assert_eq!(manifest["schema_version"].as_u64(), Some(3));
    assert_eq!(manifest["skills"].as_array().expect("skills").len(), 0);
}

#[test]
fn install_preserves_edited_legacy_generated_skills_with_warning() {
    let home = TempDir::new().expect("home");
    let skill_path = home.path().join(".codex/skills/wiki-query/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("skill parent");
    let generated = "generated wiki query skill";
    fs::write(&skill_path, "user edited wiki query skill").expect("write edited skill");
    write_legacy_manifest(
        home.path(),
        &[legacy_skill_entry(
            &skill_path,
            "wiki-query",
            "codex",
            sha256_hex(generated.as_bytes()),
        )],
    );

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Warning: manual cleanup required: legacy generated skill was edited and was preserved",
        ));

    assert_eq!(
        fs::read_to_string(&skill_path).expect("preserved skill"),
        "user edited wiki query skill"
    );
    let manifest = read_manifest(home.path());
    assert_eq!(manifest["skills"].as_array().expect("skills").len(), 1);
}

#[test]
fn install_warns_about_manifestless_legacy_generated_skill_dirs() {
    let home = TempDir::new().expect("home");
    let skill_path = home.path().join(".codex/skills/wiki-query/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("skill parent");
    fs::write(&skill_path, "orphaned generated skill").expect("write orphaned skill");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Warning: manual cleanup required: legacy generated skill directory remains",
        ));

    assert!(skill_path.exists());
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
fn uninstall_removes_manifest_owned_files_only() {
    let home = TempDir::new().expect("home");
    let user_file = home.path().join(".codex/user-owned.txt");
    fs::create_dir_all(user_file.parent().expect("parent")).expect("mkdir");
    fs::write(&user_file, "keep").expect("write");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path()).arg("uninstall").assert().success();

    assert!(user_file.exists());
    assert!(!home.path().join(".llm_wiki/manifest.json").exists());
}

#[test]
fn uninstall_removes_both_binaries() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--disable-llm-search"])
        .assert()
        .success();
    assert!(home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(home.path().join(".llm_wiki/bin/poman").exists());
    llm_wiki(home.path()).arg("uninstall").assert().success();

    assert!(!home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(!home.path().join(".llm_wiki/bin/poman").exists());
    assert!(!home.path().join(".llm_wiki/bin").exists());
}

#[test]
fn uninstall_has_no_include_binary_flag() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["uninstall", "--include-binary"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unexpected argument '--include-binary'",
        ));
}

#[test]
fn uninstall_force_requires_search_artifacts() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["uninstall", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--search-artifacts"));
}

/// A folder of its own holding `llm-wiki` and, when given, a poman script.
fn bin_folder(poman: Option<&str>) -> (TempDir, PathBuf) {
    let dir = TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).expect("bin folder");
    let bin = dir.path().join("llm-wiki");
    if fs::hard_link(support::llm_wiki_bin(), &bin).is_err() {
        fs::copy(support::llm_wiki_bin(), &bin).expect("copy llm-wiki");
    }
    if let Some(script) = poman {
        support::write_executable(&dir.path().join("poman"), script);
    }
    (dir, bin)
}

const INSTALL: [&str; 3] = ["install", "--skip-path-guidance", "--disable-llm-search"];

#[test]
fn install_puts_poman_beside_the_managed_binary_and_records_it() {
    let home = TempDir::new().expect("home");

    llm_wiki(home.path()).args(INSTALL).assert().success();

    let managed_poman = home.path().join(".llm_wiki/bin/poman");
    assert_eq!(
        fs::read_to_string(&managed_poman).expect("managed poman"),
        support::POMAN_SCRIPT
    );
    let poman_hash = sha256_hex(support::POMAN_SCRIPT.as_bytes());
    let manifest = read_manifest(home.path());
    assert_eq!(manifest["schema_version"], 3);
    assert_eq!(
        manifest["poman"]["path"].as_str().expect("poman path"),
        managed_poman.to_string_lossy()
    );
    assert_eq!(manifest["poman"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["poman"]["source_hash"], poman_hash.as_str());
    #[cfg(not(target_os = "macos"))]
    assert_eq!(manifest["poman"]["hash"], poman_hash.as_str());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&managed_poman)
            .expect("poman metadata")
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "managed poman is executable");
    }
}

#[cfg(unix)]
#[test]
fn a_second_install_copies_neither_binary() {
    use std::os::unix::fs::MetadataExt;

    let home = TempDir::new().expect("home");
    let inode = |name: &str| {
        fs::metadata(home.path().join(".llm_wiki/bin").join(name))
            .expect("managed file")
            .ino()
    };

    llm_wiki(home.path()).args(INSTALL).assert().success();
    let (binary, poman) = (inode("llm-wiki"), inode("poman"));
    llm_wiki(home.path()).args(INSTALL).assert().success();

    // Install copies through a temporary file it renames into place, so an
    // unchanged inode means nothing was copied.
    assert_eq!(inode("llm-wiki"), binary);
    assert_eq!(inode("poman"), poman);
}

#[test]
fn install_refuses_without_poman_when_none_is_recorded() {
    let home = TempDir::new().expect("home");
    let (_dir, bin) = bin_folder(None);

    llm_wiki_at(&bin, home.path())
        .args(INSTALL)
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "llm-wiki install needs poman {} beside it: there is no",
            env!("CARGO_PKG_VERSION")
        )))
        .stderr(predicate::str::contains("cargo install poman"));

    assert_no_install_metadata(home.path());
    assert!(!home.path().join(".llm_wiki/bin").exists());
}

#[test]
fn install_refuses_a_poman_of_another_version() {
    let home = TempDir::new().expect("home");
    let (_dir, bin) = bin_folder(Some("#!/bin/sh\necho \"poman 0.0.1\"\n"));

    llm_wiki_at(&bin, home.path())
        .args(INSTALL)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is `poman 0.0.1`, not poman"))
        .stderr(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    assert_no_install_metadata(home.path());
}

#[test]
fn install_keeps_a_recorded_poman_when_none_is_beside_it() {
    let home = TempDir::new().expect("home");
    llm_wiki(home.path()).args(INSTALL).assert().success();
    let recorded = read_manifest(home.path())["poman"].clone();

    for poman in [None, Some("#!/bin/sh\necho \"poman 0.0.1\"\n")] {
        let (_dir, bin) = bin_folder(poman);
        llm_wiki_at(&bin, home.path())
            .args(INSTALL)
            .assert()
            .success()
            .stdout(predicate::str::contains("Warning: poman was not updated"));

        assert_eq!(read_manifest(home.path())["poman"], recorded);
        assert_eq!(
            fs::read_to_string(home.path().join(".llm_wiki/bin/poman")).expect("kept poman"),
            support::POMAN_SCRIPT
        );
    }
}

#[test]
fn install_refuses_an_unmanaged_poman_without_force() {
    let home = TempDir::new().expect("home");
    let managed_poman = home.path().join(".llm_wiki/bin/poman");
    fs::create_dir_all(managed_poman.parent().expect("parent")).expect("mkdir");
    fs::write(&managed_poman, "foreign poman").expect("write foreign");

    llm_wiki(home.path())
        .args(INSTALL)
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "refusing to replace unmanaged binary {}",
            managed_poman.display()
        )));
    assert_eq!(
        fs::read_to_string(&managed_poman).expect("foreign remains"),
        "foreign poman"
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
    assert_eq!(
        fs::read_to_string(&managed_poman).expect("managed poman"),
        support::POMAN_SCRIPT
    );
    let manifest = read_manifest(home.path());
    let backup_manifest: Value = serde_json::from_str(
        &fs::read_to_string(
            manifest["backups"][0]["path"]
                .as_str()
                .expect("backup path"),
        )
        .expect("backup manifest"),
    )
    .expect("backup json");
    let poman_backup = backup_manifest["files"]
        .as_array()
        .expect("backup files")
        .iter()
        .find(|entry| entry["original_path"] == managed_poman.to_string_lossy().as_ref())
        .expect("poman backup");
    assert_eq!(
        fs::read_to_string(poman_backup["backup_path"].as_str().expect("backup path"))
            .expect("backup contents"),
        "foreign poman"
    );
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
            .join(".llm_wiki/search-runtime-probes.toml")
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
            .join(".llm_wiki/mcp/claude-project.mcp.json")
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
    assert!(
        !home
            .path()
            .join(".llm_wiki/search-runtime-probes.toml")
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
            .join(".llm_wiki/search-runtime-probes.toml")
            .exists()
    );
    assert!(
        !home
            .path()
            .join(".local/share/llm-wiki/projects.json")
            .exists()
    );
    assert!(!home.path().join(".llm_wiki/bin/llm-wiki").exists());
    assert!(!home.path().join(".llm_wiki/bin/poman").exists());
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
        .stderr(predicate::str::contains("consider file:"))
        .stderr(predicate::str::contains("drift check:"));
}

#[test]
fn uninstall_refuses_drifted_manifest_file() {
    let home = TempDir::new().expect("home");
    let path = home.path().join(".llm_wiki/mcp/claude-project.mcp.json");

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

fn write_legacy_manifest(home: &Path, skills: &[Value]) {
    let manifest_path = home.join(".llm_wiki/manifest.json");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("manifest parent");
    let manifest = serde_json::json!({
        "schema_version": 1,
        "installed_by": "llm-wiki",
        "installed_at": "2026-06-23T00:00:00Z",
        "binary": {
            "path": home.join(".llm_wiki/bin/llm-wiki").to_string_lossy(),
            "version": "0.0.0",
            "hash_algorithm": "sha256",
            "hash": sha256_hex(b"old binary"),
            "ownership": "manifest-owned"
        },
        "skills": skills
    });
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest json"),
    )
    .expect("write legacy manifest");
}

fn legacy_skill_entry(path: &Path, skill: &str, runtime: &str, hash: String) -> Value {
    serde_json::json!({
        "path": path.to_string_lossy(),
        "skill": skill,
        "runtime": runtime,
        "kind": "skill",
        "hash_algorithm": "sha256",
        "hash": hash,
        "ownership": "manifest-owned",
        "installed_by_version": "0.1.0"
    })
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
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
    fs::write(
        home.join(".llm_wiki/search-runtime-probes.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-14T00:00:00Z"
binary_version = "test"
target_triple = "test"
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
records = []
"#,
    )
    .expect("runtime probes");
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
