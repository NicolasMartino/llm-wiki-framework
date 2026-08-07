use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
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

#[test]
fn register_is_idempotent_and_projects_lists_registry() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered project: fixture"));

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Project already registered: fixture",
        ));

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], "fixture");
    assert_eq!(projects[0]["name"], "Fixture");
    assert_eq!(projects[0]["root"], project.to_string_lossy().as_ref());
    assert_eq!(projects[0]["wiki_path"], "wiki");

    llm_wiki(home.path())
        .arg("projects")
        .assert()
        .success()
        .stdout(predicate::str::contains("fixture"))
        .stdout(predicate::str::contains("root-ok"))
        .stdout(predicate::str::contains("index-missing"));

    llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": \"fixture\""))
        .stdout(predicate::str::contains(
            "\"index_status\": \"index-missing\"",
        ))
        .stdout(predicate::str::contains("\"freshness\": \"missing\""));
}

#[test]
fn register_update_changes_existing_project_without_duplicate() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["register", "--update", "fixture", "--name", "Renamed"])
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated project: fixture"));

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "Renamed");
}

#[test]
fn register_update_changes_existing_project_without_path_argument() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["register", "--update", "fixture", "--name", "Renamed Again"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated project: fixture"));

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "Renamed Again");
    assert_eq!(projects[0]["root"], project.to_string_lossy().as_ref());
}

#[cfg(unix)]
#[test]
fn projects_reports_search_cache_permission_denied_distinctly() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let store = home.path().join(".llm_wiki/indexes/fixture/qmd-rs.sqlite");
    let original = fs::metadata(&store).expect("metadata").permissions();
    let mut unreadable = original.clone();
    unreadable.set_mode(0o000);
    fs::set_permissions(&store, unreadable).expect("chmod unreadable");

    let output = llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .output()
        .expect("projects output");
    fs::set_permissions(&store, original).expect("restore permissions");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("projects json");
    let project = json["projects"]
        .as_array()
        .expect("projects")
        .first()
        .expect("project");
    assert_eq!(project["index_status"], "index-permission-denied");
    assert_eq!(project["freshness"], "unknown");
    assert!(project["status_message"].as_str().is_some_and(|message| {
        message.contains("cannot be read") || message.contains("open failed")
    }));
}

#[test]
fn projects_reports_metadata_only_store_as_transient() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let store = home.path().join(".llm_wiki/indexes/fixture/qmd-rs.sqlite");
    fs::remove_file(&store).expect("remove sqlite");

    let output = llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .output()
        .expect("projects output");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("projects json");
    let project = json["projects"]
        .as_array()
        .expect("projects")
        .first()
        .expect("project");
    assert_eq!(project["index_status"], "index-transient");
    assert_eq!(project["freshness"], "unknown");
    assert!(
        project["status_message"]
            .as_str()
            .is_some_and(|message| message.contains("publication is incomplete"))
    );
}

#[cfg(unix)]
#[test]
fn projects_tolerates_unreadable_search_cache_directory() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture", "--name", "Fixture"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["index", "--project", "fixture", "--force"])
        .assert()
        .success();

    let cache_dir = home.path().join(".llm_wiki/indexes/fixture");
    let original = fs::metadata(&cache_dir).expect("metadata").permissions();
    let mut unreadable = original.clone();
    unreadable.set_mode(0o000);
    fs::set_permissions(&cache_dir, unreadable).expect("chmod unreadable");

    let output = llm_wiki(home.path())
        .args(["projects", "--format", "json"])
        .output()
        .expect("projects output");
    fs::set_permissions(&cache_dir, original).expect("restore permissions");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("projects json");
    let project = json["projects"]
        .as_array()
        .expect("projects")
        .first()
        .expect("project");
    assert_eq!(project["index_status"], "index-permission-denied");
    assert_eq!(project["freshness"], "unknown");
    assert!(project["cache_size_bytes"].is_null());
    assert_eq!(project["cache_size_status"], "unavailable");
    assert!(
        project["status_message"]
            .as_str()
            .is_some_and(|message| message.contains("cannot be inspected"))
    );
}

#[test]
fn verbose_registry_commands_emit_diagnostics() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args([
            "--verbose",
            "register",
            "--id",
            "fixture",
            "--name",
            "Fixture",
        ])
        .arg(&project)
        .assert()
        .success()
        .stderr(predicate::str::contains("command: register"))
        .stderr(predicate::str::contains("registry:"))
        .stderr(predicate::str::contains("canonical root:"))
        .stderr(predicate::str::contains("validation: ok"));

    llm_wiki(home.path())
        .args(["--verbose", "projects"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: projects"))
        .stderr(predicate::str::contains("registry:"))
        .stderr(predicate::str::contains("registered projects: 1"))
        .stderr(predicate::str::contains("project status: fixture"));

    llm_wiki(home.path())
        .args(["--verbose", "forget", "fixture", "--delete-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: forget"))
        .stderr(predicate::str::contains("requested project id: fixture"))
        .stderr(predicate::str::contains("cache dir:"))
        .stderr(predicate::str::contains("cache deletion:"));
}

#[test]
fn same_root_with_different_explicit_id_is_rejected() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "first"])
        .arg(&project)
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["register", "--id", "second"])
        .arg(&project)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already registered"));
}

#[test]
fn forget_removes_registry_entry_and_cache_when_requested() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);
    let cache_dir = home.path().join(".llm_wiki/indexes/fixture");
    let legacy_cache_dir = home.path().join(".cache/llm-wiki/indexes/fixture");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();
    fs::create_dir_all(&cache_dir).expect("cache dir");
    fs::write(cache_dir.join("qmd-rs.sqlite"), "sqlite").expect("cache file");
    fs::create_dir_all(&legacy_cache_dir).expect("legacy cache dir");
    fs::write(legacy_cache_dir.join("qmd-rs.sqlite"), "sqlite").expect("legacy cache file");

    llm_wiki(home.path())
        .args(["forget", "fixture", "--delete-cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Forgot project: fixture"));

    let registry = read_registry(home.path());
    assert_eq!(registry["projects"].as_array().expect("projects").len(), 0);
    assert!(!cache_dir.exists());
    assert!(!legacy_cache_dir.exists());
}

#[cfg(unix)]
#[test]
fn forget_delete_cache_refuses_to_escape_cache_home() {
    use std::os::unix::fs::symlink;

    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let escape = TempDir::new().expect("escape");
    let project = fixture_project(workspace.path(), "Fixture Project", true);
    let cache_dir = home.path().join(".llm_wiki/indexes/fixture");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();

    fs::create_dir_all(cache_dir.parent().expect("parent")).expect("cache parent");
    symlink(escape.path(), &cache_dir).expect("symlink");

    llm_wiki(home.path())
        .args(["forget", "fixture", "--delete-cache"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to delete cache outside"));
}

#[test]
fn concurrent_registry_writers_do_not_lose_updates() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let alpha = fixture_project(workspace.path(), "Alpha Project", true);
    let beta = fixture_project(workspace.path(), "Beta Project", true);

    llm_wiki(home.path())
        .args(["register", "--id", "alpha"])
        .arg(&alpha)
        .assert()
        .success();

    let binary = assert_cmd::cargo::cargo_bin("llm-wiki");
    let mut register = StdCommand::new(&binary);
    register
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS", "200")
        .args(["register", "--id", "beta"])
        .arg(&beta);
    let register = register.spawn().expect("spawn register");

    thread::sleep(Duration::from_millis(50));

    let mut forget = StdCommand::new(&binary);
    let forget_output = forget
        .env("HOME", home.path())
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .args(["forget", "alpha"])
        .output()
        .expect("forget output");
    let register_output = register.wait_with_output().expect("register output");

    assert!(register_output.status.success());
    assert!(forget_output.status.success());

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], "beta");
}

#[test]
fn three_concurrent_registry_writers_do_not_lose_updates() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");

    let binary = assert_cmd::cargo::cargo_bin("llm-wiki");
    let ids = ["alpha", "beta", "gamma"];
    let mut children = Vec::new();
    for id in ids {
        let project = fixture_project(workspace.path(), &format!("{id} project"), true);
        let mut register = StdCommand::new(&binary);
        register
            .env("HOME", home.path())
            .env_remove("RUST_LOG")
            .env_remove("XDG_CACHE_HOME")
            .env_remove("XDG_DATA_HOME")
            // Hold the critical section long enough that all three overlap: if the
            // lock inode were unlinked on drop, a lost update would surface here.
            .env("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS", "200")
            .args(["register", "--id", id])
            .arg(&project);
        children.push(register.spawn().expect("spawn register"));
    }

    for child in children {
        let output = child.wait_with_output().expect("register output");
        assert!(output.status.success());
    }

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    let mut got = projects
        .iter()
        .map(|project| project["id"].as_str().expect("id").to_string())
        .collect::<Vec<_>>();
    got.sort();
    assert_eq!(got, ids);
}

#[test]
fn register_rejects_project_without_orientation_file() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", false);

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing an orientation file"));
}

#[test]
fn register_rejects_unsafe_explicit_project_ids() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path(), "Fixture Project", true);

    for id in [
        "../models",
        "/tmp/escape",
        "a/b",
        "..",
        "C:\\temp",
        "C:temp",
    ] {
        llm_wiki(home.path())
            .args(["register", "--id", id])
            .arg(&project)
            .assert()
            .failure()
            .stderr(predicate::str::contains("invalid project id"));
    }
}

#[test]
fn register_accepts_uppercase_agents_orientation_file() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = workspace.path().join("Fixture Project");
    fs::create_dir_all(project.join("wiki")).expect("wiki");
    fs::write(project.join("wiki/index.md"), "# Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    fs::write(project.join("AGENTS.MD"), "# Agents\n").expect("agents");
    let project = project.canonicalize().expect("canonical");

    llm_wiki(home.path())
        .args(["register", "--id", "fixture"])
        .arg(&project)
        .assert()
        .success();
}

fn read_registry(home: &Path) -> Value {
    let path = home.join(".local/share/llm-wiki/projects.json");
    serde_json::from_str(&fs::read_to_string(path).expect("registry json")).expect("json")
}

fn fixture_project(root: &Path, name: &str, orientation: bool) -> PathBuf {
    let project = root.join(name);
    fs::create_dir_all(project.join("wiki")).expect("wiki");
    fs::write(project.join("wiki/index.md"), "# Index\n").expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    if orientation {
        fs::write(project.join("AGENTS.md"), "# Agents\n").expect("agents");
    }
    project.canonicalize().expect("canonical")
}
