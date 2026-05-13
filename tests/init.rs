use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn init_project(path: &Path, blueprint: &str, packs: &[&str], extra: &[&str]) {
    let home = TempDir::new().expect("home");
    let mut command = llm_wiki(home.path());
    command
        .arg("init")
        .arg(path)
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            blueprint,
        ])
        .args(packs.iter().flat_map(|pack| ["--pack", *pack]))
        .args(extra)
        .assert()
        .success();
}

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
fn init_profiles_match_snapshots() {
    let cases = [
        ("baseline", "generic", vec![], vec![]),
        ("ml_ai", "custom", vec!["ml"], vec![]),
        ("qmd_rs", "custom", vec!["qmd-rs-scale"], vec![]),
        ("ml_ai_qmd_rs", "custom", vec!["ml", "qmd-rs-scale"], vec![]),
        ("research", "research", vec![], vec![]),
        ("web_product", "web-product", vec![], vec![]),
        ("cli_tool", "cli-tool", vec![], vec![]),
        ("ml_research", "ml-research", vec![], vec![]),
        ("ops_infra", "ops-infra", vec![], vec![]),
    ];
    for (name, blueprint, packs, extra) in cases {
        let temp = TempDir::new().expect("tempdir");
        init_project(temp.path(), blueprint, &packs, &extra);
        let snapshot = snapshot_project(temp.path());
        insta::with_settings!({filters => vec![(r"\d{4}-\d{2}-\d{2}", "[date]")]}, {
            insta::assert_snapshot!(format!("init_{name}"), snapshot);
        });
    }
}

#[test]
fn init_refuses_framework_artifact_collision() {
    let temp = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("home");
    fs::create_dir_all(temp.path().join("wiki")).expect("mkdir");

    llm_wiki(home.path())
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("framework artifacts"));
}

#[test]
fn init_rejects_retired_type_and_scale_flags() {
    let temp = TempDir::new().expect("tempdir");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env_remove("RUST_LOG")
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--type",
            "web",
            "--scale",
            "small",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--blueprint"));
}

#[test]
fn init_rejects_removed_existing_flag() {
    let temp = TempDir::new().expect("tempdir");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env_remove("RUST_LOG")
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
            "--existing",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--existing"));
}

#[test]
fn init_manifest_records_resolved_blueprint_packs() {
    let temp = TempDir::new().expect("tempdir");

    init_project(temp.path(), "ml-research", &[], &[]);

    assert!(temp.path().join("wiki/experiments").is_dir());
    assert!(temp.path().join("wiki/evals").is_dir());
    assert!(temp.path().join("wiki/datasets").is_dir());
    assert!(temp.path().join("wiki/literature").is_dir());
    assert_code_dirs_exist(temp.path());

    let manifest = read_init_manifest(temp.path());
    assert_eq!(manifest["blueprint"].as_str(), Some("ml-research"));
    assert_eq!(
        manifest["packs"].as_array().expect("packs"),
        &[
            toml::Value::String("ml".to_string()),
            toml::Value::String("data".to_string()),
            toml::Value::String("research".to_string()),
            toml::Value::String("code".to_string()),
        ]
    );

    let guidelines =
        fs::read_to_string(temp.path().join("project_guidelines.md")).expect("guidelines");
    assert!(guidelines.contains("## Pack Document Types"));
    assert!(guidelines.contains("`model-card.md`"));
    assert!(guidelines.contains("`dataset-card.md`"));
    assert!(guidelines.contains("## Pack Status Vocabulary"));
}

#[test]
fn init_code_pack_controls_root_code_folders() {
    let research = TempDir::new().expect("research");
    init_project(research.path(), "research", &[], &[]);
    assert_code_dirs_absent(research.path());
    assert!(!manifest_packs(research.path()).contains(&"code".to_string()));

    let web_product = TempDir::new().expect("web");
    init_project(web_product.path(), "web-product", &[], &[]);
    assert_code_dirs_exist(web_product.path());
    assert!(manifest_packs(web_product.path()).contains(&"code".to_string()));

    let cli_tool = TempDir::new().expect("cli");
    init_project(cli_tool.path(), "cli-tool", &[], &[]);
    assert_code_dirs_exist(cli_tool.path());
    assert!(manifest_packs(cli_tool.path()).contains(&"code".to_string()));

    let generic_with_code = TempDir::new().expect("generic-code");
    init_project(generic_with_code.path(), "generic", &["code"], &[]);
    assert_code_dirs_exist(generic_with_code.path());
    assert_eq!(manifest_packs(generic_with_code.path()), vec!["code"]);
}

#[test]
fn verbose_init_emits_command_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("home");
    let sources = TempDir::new().expect("sources");
    let source = sources.path().join("seed.md");
    fs::write(&source, "# Seed\n").expect("source");

    llm_wiki(home.path())
        .args(["--verbose", "init"])
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "ml-research",
            "--initial-sources",
        ])
        .arg(&source)
        .assert()
        .success()
        .stderr(predicate::str::contains("command: init"))
        .stderr(predicate::str::contains("project path:"))
        .stderr(predicate::str::contains("blueprint: ml-research"))
        .stderr(predicate::str::contains(
            "resolved packs: ml, data, research, code",
        ))
        .stderr(predicate::str::contains("initial source:"))
        .stderr(predicate::str::contains("registry:"))
        .stderr(predicate::str::contains("canonical root:"))
        .stderr(predicate::str::contains("validation: ok"));
}

#[test]
fn verbose_init_reports_cli_tool_code_pack() {
    let temp = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["--verbose", "init"])
        .arg(temp.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "CLI Fixture",
            "--description",
            "A CLI fixture project.",
            "--blueprint",
            "cli-tool",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("command: init"))
        .stderr(predicate::str::contains("no register: true"))
        .stderr(predicate::str::contains("blueprint: cli-tool"))
        .stderr(predicate::str::contains("resolved packs: code"));
}

#[test]
fn init_runtime_manifest_records_managed_install() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Runtime Fixture",
            "--description",
            "A runtime fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let runtime = read_runtime_manifest(project.path());
    let managed_home = home.path().join(".llm_wiki").to_string_lossy().to_string();
    let managed_binary = home
        .path()
        .join(".llm_wiki/bin/llm-wiki")
        .to_string_lossy()
        .to_string();
    assert_eq!(
        runtime["framework_version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(runtime["install_state"].as_str(), Some("present"));
    assert_eq!(
        runtime["managed_home"].as_str(),
        Some(managed_home.as_str())
    );
    assert_eq!(
        runtime["managed_binary"].as_str(),
        Some(managed_binary.as_str())
    );
    assert_eq!(
        runtime["installed_version"].as_str(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert!(
        runtime["install_id"]
            .as_str()
            .expect("install id")
            .starts_with("sha256:")
    );
    assert!(runtime["install_hash"].as_str().is_some());
}

#[test]
fn init_seeds_project_search_profile_from_global_default() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Search Profile Fixture",
            "--description",
            "A search profile fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let runtime = read_runtime_manifest(project.path());
    let project_search = read_project_search_manifest(project.path());
    assert_eq!(project_search["schema_version"].as_integer(), Some(1));
    assert_eq!(
        project_search["project"]["llm_search_enabled"].as_bool(),
        Some(false)
    );
    assert_eq!(
        project_search["project"]["reason"].as_str(),
        Some("llm_search_disabled")
    );
    assert_eq!(
        project_search["project"]["source"].as_str(),
        Some("project_default")
    );
    assert_eq!(
        project_search["project"]["source_install_id"].as_str(),
        runtime["install_id"].as_str()
    );
}

#[test]
fn init_agents_lists_pack_document_types_for_pack_driven_projects() {
    let temp = TempDir::new().expect("tempdir");

    init_project(temp.path(), "ops-infra", &[], &[]);

    let agents = fs::read_to_string(temp.path().join("AGENTS.md")).expect("agents");
    assert!(agents.contains("## Pack Document Types"));
    assert!(agents.contains("| Runbook | `runbook.md` | `wiki/runbooks` |"));
    assert!(agents.contains("| SLO | `slo.md` | `wiki/slos` |"));
    assert!(agents.contains("| Postmortem | `postmortem.md` | `wiki/postmortems` |"));
}

#[test]
fn initial_sources_are_copied_without_ingest() {
    let temp = TempDir::new().expect("tempdir");
    let home = TempDir::new().expect("home");
    let source_dir = TempDir::new().expect("sources");
    let source_a = source_dir.path().join("manifest.md");
    let source_b = source_dir.path().join("b.txt");
    fs::write(&source_a, "# User Manifest").expect("write");
    fs::write(&source_b, "B").expect("write");

    llm_wiki(home.path())
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
            "--initial-sources",
        ])
        .arg(&source_a)
        .arg("--initial-sources")
        .arg(&source_b)
        .assert()
        .success()
        .stdout(predicate::str::contains("Sources copied"));

    let raw_initial = temp.path().join("raw/initial");
    assert!(raw_initial.exists());
    assert!(find_file_with_contents(&raw_initial, "manifest.md", "# User Manifest").is_some());
    assert!(find_file(&raw_initial, "b.txt").is_some());
    assert!(find_file(&raw_initial, "manifest.md").is_some());
    assert!(
        !fs::read_to_string(temp.path().join("wiki/index.md"))
            .expect("index")
            .contains("a.md")
    );
}

#[test]
fn init_with_invalid_initial_sources_leaves_no_partial_scaffold() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("missing-source.md");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .env_remove("RUST_LOG")
        .arg("init")
        .arg(temp.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
            "--initial-sources",
        ])
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicate::str::contains("initial source does not exist"));

    assert!(!temp.path().join("AGENTS.md").exists());
    assert!(!temp.path().join("project_guidelines.md").exists());
    assert!(!temp.path().join("wiki").exists());
    assert!(!temp.path().join(".llm_wiki").exists());
}

#[test]
fn init_auto_registers_successful_project() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Project initialized. Registry: registered as fixture-project.",
        ));

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], "fixture-project");
    assert_eq!(
        projects[0]["root"],
        project
            .path()
            .canonicalize()
            .expect("root")
            .to_string_lossy()
            .as_ref()
    );
}

#[test]
fn init_no_register_leaves_registry_untouched() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    assert!(
        !home
            .path()
            .join(".local/share/llm-wiki/projects.json")
            .exists()
    );
}

#[test]
fn init_registry_write_failure_is_recoverable_warning() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    let data_file = home.path().join("xdg-data-file");
    fs::write(&data_file, "not a directory").expect("data file");

    let mut command = llm_wiki(home.path());
    command.env("XDG_DATA_HOME", &data_file);
    command
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized LLM Wiki project"))
        .stdout(predicate::str::contains(
            "Project initialized. Registry: failed",
        ))
        .stderr(predicate::str::contains("registry update failed"))
        .stderr(predicate::str::contains("llm-wiki register"));

    assert!(project.path().join("wiki/index.md").exists());
}

fn snapshot_project(path: &Path) -> String {
    let mut output = String::new();
    output.push_str("# files\n");
    for rel in interesting_files(path) {
        output.push_str(&format!("- {}\n", rel.display()));
    }
    output.push_str("\n# project_guidelines.md\n");
    output.push_str(&fs::read_to_string(path.join("project_guidelines.md")).expect("guidelines"));
    output.push_str("\n# AGENTS.md\n");
    output.push_str(&fs::read_to_string(path.join("AGENTS.md")).expect("agents"));
    output.push_str("\n# CLAUDE.md\n");
    output.push_str(&fs::read_to_string(path.join("CLAUDE.md")).expect("claude"));
    output.push_str("\n# .llm_wiki/init.toml\n");
    output.push_str(&fs::read_to_string(path.join(".llm_wiki/init.toml")).expect("manifest"));
    output.push_str("\n# wiki/index.md\n");
    output.push_str(&fs::read_to_string(path.join("wiki/index.md")).expect("index"));
    output
}

fn interesting_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(path, path, &mut files);
    files.sort();
    files
}

fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("read_dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            files.push(path.strip_prefix(root).expect("rel").to_path_buf());
        }
    }
}

fn find_file(path: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Some(found) = find_file(&entry_path, name) {
                return Some(found);
            }
        } else if entry.file_name() == name {
            return Some(entry_path);
        }
    }
    None
}

fn find_file_with_contents(path: &Path, name: &str, contents: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Some(found) = find_file_with_contents(&entry_path, name, contents) {
                return Some(found);
            }
        } else if entry.file_name() == name
            && fs::read_to_string(&entry_path).ok().as_deref() == Some(contents)
        {
            return Some(entry_path);
        }
    }
    None
}

fn read_registry(home: &Path) -> Value {
    let path = home.join(".local/share/llm-wiki/projects.json");
    serde_json::from_str(&fs::read_to_string(path).expect("registry json")).expect("json")
}

fn read_init_manifest(project: &Path) -> toml::Value {
    let manifest = fs::read_to_string(project.join(".llm_wiki/init.toml")).expect("manifest");
    toml::from_str(&manifest).expect("toml")
}

fn read_runtime_manifest(project: &Path) -> toml::Value {
    let manifest = fs::read_to_string(project.join(".llm_wiki/runtime.toml")).expect("manifest");
    toml::from_str(&manifest).expect("toml")
}

fn read_project_search_manifest(project: &Path) -> toml::Value {
    let manifest = fs::read_to_string(project.join(".llm_wiki/search.toml")).expect("manifest");
    toml::from_str(&manifest).expect("toml")
}

fn manifest_packs(project: &Path) -> Vec<String> {
    read_init_manifest(project)["packs"]
        .as_array()
        .expect("packs")
        .iter()
        .map(|pack| pack.as_str().expect("pack string").to_string())
        .collect()
}

fn assert_code_dirs_exist(project: &Path) {
    for folder in ["src", "tests", "scripts", "infra"] {
        assert!(project.join(folder).is_dir(), "{folder} should exist");
    }
}

fn assert_code_dirs_absent(project: &Path) {
    for folder in ["src", "tests", "scripts", "infra"] {
        assert!(!project.join(folder).exists(), "{folder} should not exist");
    }
}
