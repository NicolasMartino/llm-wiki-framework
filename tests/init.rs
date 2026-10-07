use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

mod support;

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
    let mut command = Command::new(support::llm_wiki_bin());
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
        insta::with_settings!({filters => vec![
            (r"\d{4}-\d{2}-\d{2}", "[date]"),
            (r#"framework_version = "\d+\.\d+\.\d+""#, r#"framework_version = "[version]""#),
            // The guidelines' block holds the render date, so its hash changes daily.
            (r#"guidelines = "[0-9a-f]{64}""#, r#"guidelines = "[sha256]""#),
        ]}, {
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
fn init_rerun_appends_schema_drift_log_and_index_entry() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Original Project",
            "--description",
            "Original description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let index = "# Wiki Index\n\nProject: Original Project\n\n## Specs\n\n- custom entry\n";
    let log = "# Wiki Log\n\n## [2026-05-13] update | custom\n\nKeep this.\n";
    fs::write(project.path().join("wiki/index.md"), index).expect("index");
    fs::write(project.path().join("wiki/log.md"), log).expect("log");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Updated Project",
            "--description",
            "Updated description.",
            "--blueprint",
            "web-product",
            "--pack",
            "api",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated LLM Wiki project"));

    let manifest = read_init_manifest(project.path());
    assert_eq!(manifest["project_name"].as_str(), Some("Updated Project"));
    assert_eq!(
        manifest["project_description"].as_str(),
        Some("Updated description.")
    );
    assert_eq!(manifest["blueprint"].as_str(), Some("web-product"));
    assert_eq!(manifest_packs(project.path()), vec!["api"]);
    assert!(project.path().join("wiki/apis").is_dir());
    assert!(!project.path().join("src").exists());
    let folders = manifest_resolved_folders(project.path());
    assert!(folders.contains(&"raw/api".to_string()));
    assert!(folders.contains(&"wiki/apis".to_string()));

    let updated_index = fs::read_to_string(project.path().join("wiki/index.md")).expect("index");
    assert!(updated_index.starts_with(index));
    assert!(updated_index.contains("## Schema Drift"));
    assert!(updated_index.contains("Newly claimed folders: raw/api, wiki/apis"));
    assert!(updated_index.contains("Orphaned folders: none"));
    assert!(updated_index.contains("Audit trail: `wiki/log.md`"));

    let updated_log = fs::read_to_string(project.path().join("wiki/log.md")).expect("log");
    assert!(updated_log.starts_with(log));
    assert!(updated_log.contains("init | schema drift | generic -> web-product"));
    assert!(updated_log.contains("Added packs: api"));
    assert!(updated_log.contains("Removed packs: none"));
    assert!(updated_log.contains("Trigger: both"));
    assert!(updated_log.contains("Added folders: raw/api, wiki/apis"));
    assert!(updated_log.contains("Orphaned folders: none"));
    assert!(
        updated_log.trim_end().ends_with(
            "Orphan content is preserved on disk. Markdown under wiki/ remains searchable; non-wiki folders are preserved but not indexed by wiki search."
        )
    );
    assert!(
        fs::read_to_string(project.path().join("AGENTS.md"))
            .expect("agents")
            .contains("This is Updated Project: Updated description.")
    );
}

#[test]
fn init_rerun_with_identical_answers_and_composition_does_not_drift_log_or_index() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Original Project",
            "--description",
            "Original description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let index = "# Wiki Index\n\nProject: Original Project\n\n## Specs\n\n- custom entry\n";
    let log = "# Wiki Log\n\n## [2026-05-13] update | custom\n\nKeep this.\n";
    fs::write(project.path().join("wiki/index.md"), index).expect("index");
    fs::write(project.path().join("wiki/log.md"), log).expect("log");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Original Project",
            "--description",
            "Original description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(project.path().join("wiki/index.md")).expect("index"),
        index
    );
    assert_eq!(
        fs::read_to_string(project.path().join("wiki/log.md")).expect("log"),
        log
    );
    assert!(!manifest_resolved_folders(project.path()).is_empty());
}

#[test]
fn init_rerun_records_same_pack_folder_composition_drift() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "API Project",
            "--description",
            "API description.",
            "--blueprint",
            "generic",
            "--pack",
            "api",
        ])
        .assert()
        .success();

    fs::write(
        project.path().join(".llm_wiki/init.toml"),
        r#"framework_version = "0.1.2"
project_name = "API Project"
project_description = "API description."
blueprint = "generic"
packs = ["api"]
resolved_folders = [
    "raw",
    "wiki/apis",
    "wiki/archive",
    "wiki/checklists",
    "wiki/decisions",
    "wiki/plans",
    "wiki/proposals",
    "wiki/references",
    "wiki/roadmaps",
    "wiki/specs",
]
"#,
    )
    .expect("manifest");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "API Project",
            "--description",
            "API description.",
            "--blueprint",
            "generic",
            "--pack",
            "api",
        ])
        .assert()
        .success();

    let log = fs::read_to_string(project.path().join("wiki/log.md")).expect("log");
    assert!(log.contains("init | schema drift | generic -> generic"));
    assert!(log.contains("Added packs: none"));
    assert!(log.contains("Removed packs: none"));
    assert!(log.contains("Trigger: composition change"));
    assert!(log.contains("Added folders: raw/api"));
    assert!(log.contains("Orphaned folders: none"));

    let index = fs::read_to_string(project.path().join("wiki/index.md")).expect("index");
    assert!(index.contains("## Schema Drift"));
    assert!(index.contains("Newly claimed folders: raw/api"));
}

#[test]
fn init_rerun_legacy_manifest_without_resolved_folders_seeds_then_drifts_on_pack_change() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Legacy Project",
            "--description",
            "Legacy description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    let legacy_manifest = r#"framework_version = "0.1.2"
project_name = "Legacy Project"
project_description = "Legacy description."
blueprint = "generic"
packs = []
"#;
    fs::write(project.path().join(".llm_wiki/init.toml"), legacy_manifest).expect("manifest");

    let index = "# Wiki Index\n\nProject: Legacy Project\n\n## Specs\n\n- legacy entry\n";
    let log = "# Wiki Log\n\n## [2026-05-13] update | legacy\n\nKeep this.\n";
    fs::write(project.path().join("wiki/index.md"), index).expect("index");
    fs::write(project.path().join("wiki/log.md"), log).expect("log");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Legacy Project",
            "--description",
            "Legacy description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(project.path().join("wiki/index.md")).expect("index"),
        index
    );
    assert_eq!(
        fs::read_to_string(project.path().join("wiki/log.md")).expect("log"),
        log
    );
    assert!(manifest_resolved_folders(project.path()).contains(&"wiki/specs".to_string()));

    fs::write(project.path().join(".llm_wiki/init.toml"), legacy_manifest).expect("manifest");
    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Legacy Project",
            "--description",
            "Legacy description.",
            "--blueprint",
            "generic",
            "--pack",
            "api",
        ])
        .assert()
        .success();

    let log = fs::read_to_string(project.path().join("wiki/log.md")).expect("log");
    assert!(log.contains("init | schema drift | generic -> generic"));
    assert!(log.contains("Added packs: api"));
    assert!(log.contains("Added folders: raw/api, wiki/apis"));
}

#[test]
fn init_rerun_updates_registry_entry_without_duplicate() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Original Project",
            "--description",
            "Original description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Project initialized. Registry: registered as original-project.",
        ));

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Renamed Project",
            "--description",
            "Renamed description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Project updated. Registry: updated original-project.",
        ));

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--non-interactive",
            "--name",
            "Renamed Project",
            "--description",
            "Renamed description.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Project updated. Registry: already registered as original-project.",
        ));

    let registry = read_registry(home.path());
    let projects = registry["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], "original-project");
    assert_eq!(projects[0]["name"], "Renamed Project");
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
fn init_rejects_retired_type_and_scale_flags() {
    let temp = TempDir::new().expect("tempdir");

    Command::new(support::llm_wiki_bin())
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

    Command::new(support::llm_wiki_bin())
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

    Command::new(support::llm_wiki_bin())
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

fn manifest_resolved_folders(project: &Path) -> Vec<String> {
    read_init_manifest(project)["resolved_folders"]
        .as_array()
        .expect("resolved folders")
        .iter()
        .map(|folder| folder.as_str().expect("folder string").to_string())
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

#[test]
fn init_recovers_from_partial_scaffold_missing_init_toml() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");

    // Simulate a mid-init failure: the project files and the tool-owned
    // `.llm_wiki/` marker exist, but `.llm_wiki/init.toml` was never committed.
    // Previously this trapped the user (Fresh mode -> collision refusal); a
    // re-run must now recover without hand-deleting anything.
    fs::create_dir_all(project.path().join(".llm_wiki")).expect("marker dir");
    fs::create_dir_all(project.path().join("wiki")).expect("wiki dir");
    fs::write(project.path().join("wiki/index.md"), "# Wiki Index\n").expect("index");
    fs::write(project.path().join("wiki/log.md"), "# Wiki Log\n").expect("log");
    fs::write(project.path().join("AGENTS.md"), "# AGENTS\n").expect("agents");
    assert!(!project.path().join(".llm_wiki/init.toml").exists());

    llm_wiki(home.path())
        .arg("init")
        .arg(project.path())
        .args([
            "--no-register",
            "--non-interactive",
            "--name",
            "Recovered Project",
            "--description",
            "A recovered project.",
            "--blueprint",
            "generic",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated LLM Wiki project"));

    assert!(
        project.path().join(".llm_wiki/init.toml").is_file(),
        "re-run should recover the init manifest"
    );
}

const BLOCK_START: &str = "<!-- llm-wiki:managed:start -->";
const BLOCK_END: &str = "<!-- llm-wiki:managed:end -->";
const ROOT_SCHEMA_FILES: [&str; 3] = ["project_guidelines.md", "AGENTS.md", "CLAUDE.md"];

fn init_in(home: &Path, project: &Path, packs: &[&str]) -> assert_cmd::assert::Assert {
    llm_wiki(home)
        .arg("init")
        .arg(project)
        .args([
            "--no-register",
            "--no-mcp",
            "--non-interactive",
            "--name",
            "Fixture Project",
            "--description",
            "A fixture project.",
            "--blueprint",
            "generic",
        ])
        .args(packs.iter().flat_map(|pack| ["--pack", *pack]))
        .assert()
}

fn read(project: &Path, file: &str) -> String {
    fs::read_to_string(project.join(file)).unwrap_or_else(|_| panic!("read {file}"))
}

fn stderr_of(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8_lossy(&assert.get_output().stderr).into_owned()
}

/// The text between a file's markers.
fn block_of(text: &str) -> &str {
    let start = text.find(BLOCK_START).expect("begin marker") + BLOCK_START.len() + 1;
    let end = text.find(BLOCK_END).expect("end marker");
    &text[start..end]
}

/// What init rendered for a file before it owned a block in it: the block
/// without its first line, the notice, and the blank line after it.
fn render_without_block(text: &str) -> String {
    block_of(text)
        .splitn(3, '\n')
        .nth(2)
        .expect("render")
        .to_string()
}

fn remove_recorded_hashes(project: &Path) {
    let mut manifest = read_init_manifest(project);
    manifest
        .as_table_mut()
        .expect("table")
        .remove("managed_blocks")
        .expect("managed_blocks recorded");
    fs::write(
        project.join(".llm_wiki/init.toml"),
        toml::to_string(&manifest).expect("toml"),
    )
    .expect("manifest");
}

fn saved_copies(project: &Path) -> Vec<PathBuf> {
    let mut copies = fs::read_dir(project.join(".llm_wiki/saved-blocks"))
        .map(|entries| {
            entries
                .map(|entry| entry.expect("entry").path())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    copies.sort();
    copies
}

/// Every folder and file under the project, with each file's bytes.
fn tree(project: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    fn walk(root: &Path, path: &Path, entries: &mut Vec<(PathBuf, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(path).expect("read_dir") {
            let path = entry.expect("entry").path();
            let rel = path.strip_prefix(root).expect("rel").to_path_buf();
            if path.is_dir() {
                entries.push((rel, None));
                walk(root, &path, entries);
            } else {
                entries.push((rel, Some(fs::read(&path).expect("bytes"))));
            }
        }
    }
    let mut entries = Vec::new();
    walk(project, project, &mut entries);
    entries.sort();
    entries
}

#[test]
fn init_writes_each_root_schema_file_as_its_block_and_records_its_hash() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();

    for file in ROOT_SCHEMA_FILES {
        let text = read(project.path(), file);
        assert!(text.starts_with(&format!("{BLOCK_START}\n<!-- llm-wiki init rewrites")));
        assert!(text.ends_with(&format!("{BLOCK_END}\n")), "{file}");
    }
    assert!(block_of(&read(project.path(), "CLAUDE.md")).ends_with("See @AGENTS.md.\n"));
    let manifest = read_init_manifest(project.path());
    for key in ["agents", "claude", "guidelines"] {
        let hash = manifest["managed_blocks"][key].as_str().expect("hash");
        assert_eq!(hash.len(), 64, "{key}");
    }
}

#[test]
fn init_rerun_keeps_text_outside_each_block_byte_for_byte() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();

    let above = "# Project notes\r\n\r\nWritten by the project, CRLF lines.\r\n\r\n";
    let below = "\r\n## Below the block\r\n\r\nNo final newline here.";
    for file in ROOT_SCHEMA_FILES {
        let text = read(project.path(), file);
        fs::write(project.path().join(file), format!("{above}{text}{below}")).expect("write");
    }
    let agents_block = block_of(&read(project.path(), "AGENTS.md")).to_string();

    let same = init_in(home.path(), project.path(), &[]).success();
    assert!(
        !stderr_of(&same).contains("Warning"),
        "{}",
        stderr_of(&same)
    );
    let adding_a_pack = init_in(home.path(), project.path(), &["api"]).success();
    assert!(!stderr_of(&adding_a_pack).contains("Warning"));

    for file in ROOT_SCHEMA_FILES {
        let text = read(project.path(), file);
        assert!(
            text.starts_with(&format!("{above}{BLOCK_START}\n")),
            "{file}"
        );
        assert!(text.ends_with(&format!("{BLOCK_END}\n{below}")), "{file}");
    }
    assert_ne!(block_of(&read(project.path(), "AGENTS.md")), agents_block);
    assert!(project.path().join("wiki/apis").is_dir());
    assert!(saved_copies(project.path()).is_empty());
}

#[test]
fn init_rerun_turns_an_unmarked_file_equal_to_the_previous_render_into_the_block_alone() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let fresh: Vec<String> = ROOT_SCHEMA_FILES
        .iter()
        .map(|file| read(project.path(), file))
        .collect();

    // The files as a release before the block wrote them, the guidelines on
    // an earlier date, and a manifest with no hashes.
    for (file, text) in ROOT_SCHEMA_FILES.iter().zip(&fresh) {
        let mut old = render_without_block(text);
        if *file == "project_guidelines.md" {
            let date_line = old
                .lines()
                .find(|line| line.starts_with("- Date: "))
                .expect("date line")
                .to_string();
            old = old.replace(&date_line, "- Date: 2025-01-02");
        }
        assert!(!old.contains(BLOCK_START));
        fs::write(project.path().join(file), old).expect("write");
    }
    remove_recorded_hashes(project.path());

    // The rerun adds a pack, so the files match the previous answers' render,
    // not this run's.
    let rerun = init_in(home.path(), project.path(), &["api"]).success();
    assert!(
        !stderr_of(&rerun).contains("Warning"),
        "{}",
        stderr_of(&rerun)
    );
    assert_eq!(read(project.path(), "CLAUDE.md"), fresh[2]);
    for file in ["project_guidelines.md", "AGENTS.md"] {
        let text = read(project.path(), file);
        assert!(text.starts_with(&format!("{BLOCK_START}\n")), "{file}");
        assert!(text.ends_with(&format!("{BLOCK_END}\n")), "{file}");
        assert!(!text.contains("Kept From Before"), "{file}");
        assert!(block_of(&text).contains("Pack Document Types"), "{file}");
    }
    assert!(saved_copies(project.path()).is_empty());
    assert!(read_init_manifest(project.path())["managed_blocks"]["agents"].is_str());
}

#[test]
fn init_rerun_compares_a_block_with_no_recorded_hash_to_the_previous_render() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let fresh_agents = read(project.path(), "AGENTS.md");
    let added = "A rule added inside the block.";
    let edited_claude = read(project.path(), "CLAUDE.md")
        .replace("See @AGENTS.md.\n", &format!("See @AGENTS.md.\n{added}\n"));
    fs::write(project.path().join("CLAUDE.md"), &edited_claude).expect("claude");
    remove_recorded_hashes(project.path());

    let rerun = init_in(home.path(), project.path(), &["api"]).success();
    let stderr = stderr_of(&rerun);

    // The unedited AGENTS block is refreshed with the pack, without a word.
    assert!(!stderr.contains("AGENTS.md"), "{stderr}");
    let agents = read(project.path(), "AGENTS.md");
    assert_ne!(agents, fresh_agents);
    assert!(block_of(&agents).contains("Pack Document Types"));
    // The edited CLAUDE block is saved, replaced and warned about.
    let copies = saved_copies(project.path());
    assert_eq!(copies.len(), 1, "{copies:?}");
    assert_eq!(
        fs::read_to_string(&copies[0]).expect("copy"),
        block_of(&edited_claude)
    );
    assert!(
        stderr.contains("the llm-wiki block in CLAUDE.md was edited"),
        "{stderr}"
    );
    assert!(stderr.contains(&format!("  {added}")), "{stderr}");
    assert!(!read(project.path(), "CLAUDE.md").contains(added));
}

#[cfg(unix)]
#[test]
fn init_rerun_writes_one_block_when_claude_md_links_to_the_agents_file() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let agents = format!(
        "{}\n## Our Own Section\n",
        read(project.path(), "AGENTS.md")
    );
    fs::write(project.path().join("AGENTS.md"), &agents).expect("agents");
    fs::remove_file(project.path().join("CLAUDE.md")).expect("remove");
    std::os::unix::fs::symlink("AGENTS.md", project.path().join("CLAUDE.md")).expect("link");

    for _ in 0..2 {
        let rerun = init_in(home.path(), project.path(), &[]).success();
        assert!(
            !stderr_of(&rerun).contains("Warning"),
            "{}",
            stderr_of(&rerun)
        );
        assert!(
            String::from_utf8_lossy(&rerun.get_output().stdout)
                .contains("CLAUDE.md links to AGENTS.md")
        );
        assert_eq!(read(project.path(), "AGENTS.md"), agents);
        assert!(project.path().join("CLAUDE.md").is_symlink());
        assert!(saved_copies(project.path()).is_empty());
    }
}

#[test]
fn init_rerun_writes_an_empty_root_file_as_a_missing_one() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let fresh = read(project.path(), "CLAUDE.md");
    fs::write(project.path().join("CLAUDE.md"), "").expect("truncate");

    let rerun = init_in(home.path(), project.path(), &[]).success();
    assert!(
        !stderr_of(&rerun).contains("Warning"),
        "{}",
        stderr_of(&rerun)
    );
    assert_eq!(read(project.path(), "CLAUDE.md"), fresh);
}

#[test]
fn init_refuses_a_fresh_folder_with_only_an_agents_file() {
    for name in ["AGENTS.md", "AGENTS.MD"] {
        let project = TempDir::new().expect("project");
        let home = TempDir::new().expect("home");
        fs::write(project.path().join(name), "# Our agents\n").expect("agents");

        let refused = init_in(home.path(), project.path(), &[]).failure();
        let stderr = stderr_of(&refused);
        assert!(stderr.contains("framework artifacts"), "{name}: {stderr}");
        assert!(stderr.contains(name), "{name}: {stderr}");
        assert_eq!(read(project.path(), name), "# Our agents\n");
        assert!(!project.path().join(".llm_wiki").exists(), "{name}");
    }
}

#[test]
fn init_rerun_keeps_an_edited_unmarked_file_below_the_block_and_warns() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();

    let agents = format!(
        "{}\n## How Work Runs\n\nThe project's own rules.\n",
        render_without_block(&read(project.path(), "AGENTS.md"))
    );
    let claude = "See @AGENTS.md.\n\nA Claude-only note.\n";
    let guidelines = render_without_block(&read(project.path(), "project_guidelines.md"));
    fs::write(project.path().join("AGENTS.md"), &agents).expect("agents");
    fs::write(project.path().join("CLAUDE.md"), claude).expect("claude");
    fs::write(project.path().join("project_guidelines.md"), &guidelines).expect("guidelines");
    remove_recorded_hashes(project.path());

    let rerun = init_in(home.path(), project.path(), &[]).success();
    let stderr = stderr_of(&rerun);
    assert!(
        stderr.contains("Warning: AGENTS.md had no llm-wiki block"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Warning: CLAUDE.md had no llm-wiki block"),
        "{stderr}"
    );
    assert!(!stderr.contains("project_guidelines.md"), "{stderr}");

    for (file, old) in [("AGENTS.md", agents.as_str()), ("CLAUDE.md", claude)] {
        let text = read(project.path(), file);
        assert!(text.starts_with(BLOCK_START), "{file}");
        let kept = text
            .find("\n## Kept From Before The llm-wiki Block (")
            .expect("heading");
        assert!(text[kept..].ends_with(&format!(")\n\n{old}")), "{file}");
    }
    assert!(!read(project.path(), "project_guidelines.md").contains("Kept From Before"));

    // The second rerun is silent and changes nothing.
    let before = tree(project.path());
    let second = init_in(home.path(), project.path(), &[]).success();
    assert!(
        !stderr_of(&second).contains("Warning"),
        "{}",
        stderr_of(&second)
    );
    for file in ROOT_SCHEMA_FILES {
        let path = PathBuf::from(file);
        let entry = |tree: &[(PathBuf, Option<Vec<u8>>)]| {
            tree.iter().find(|(rel, _)| rel == &path).cloned()
        };
        assert_eq!(entry(&before), entry(&tree(project.path())), "{file}");
    }
}

#[test]
fn init_rerun_saves_and_replaces_an_edited_block_with_a_warning() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let fresh = read(project.path(), "AGENTS.md");

    for round in 1..=2 {
        let added = format!("A rule added inside the block, round {round}.");
        let edited = fresh.replace("## Agent Role\n", &format!("## Agent Role\n\n{added}\n"));
        assert_ne!(edited, fresh);
        fs::write(project.path().join("AGENTS.md"), &edited).expect("edit");

        let rerun = init_in(home.path(), project.path(), &[]).success();
        let stderr = stderr_of(&rerun);
        let copies = saved_copies(project.path());
        assert_eq!(copies.len(), round, "{copies:?}");
        let copy = copies
            .iter()
            .find(|copy| fs::read_to_string(copy).expect("copy").contains(&added))
            .expect("a copy holds the edited text");
        assert_eq!(fs::read_to_string(copy).expect("copy"), block_of(&edited));
        let copy_name = copy
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        assert!(copy_name.starts_with("AGENTS-"), "{copy_name}");
        assert!(
            stderr.contains("the llm-wiki block in AGENTS.md was edited"),
            "{stderr}"
        );
        assert!(
            stderr.contains(&format!(".llm_wiki/saved-blocks/{copy_name}")),
            "{stderr}"
        );
        assert!(stderr.contains(&format!("  {added}")), "{stderr}");
        assert_eq!(read(project.path(), "AGENTS.md"), fresh);
    }
}

#[test]
fn init_rerun_refreshes_an_unedited_block_after_a_template_change_silently() {
    use sha2::{Digest, Sha256};

    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let fresh = read(project.path(), "AGENTS.md");

    // The block as an older template rendered it, with the hash init recorded.
    let old_inner = "<!-- an older notice -->\n\n# AGENTS.md - An Older Template\n";
    fs::write(
        project.path().join("AGENTS.md"),
        format!("Above.\n{BLOCK_START}\n{old_inner}{BLOCK_END}\nBelow.\n"),
    )
    .expect("agents");
    let mut manifest = read_init_manifest(project.path());
    manifest["managed_blocks"]["agents"] =
        toml::Value::String(format!("{:x}", Sha256::digest(old_inner.as_bytes())));
    fs::write(
        project.path().join(".llm_wiki/init.toml"),
        toml::to_string(&manifest).expect("toml"),
    )
    .expect("manifest");

    let rerun = init_in(home.path(), project.path(), &[]).success();
    assert!(
        !stderr_of(&rerun).contains("Warning"),
        "{}",
        stderr_of(&rerun)
    );
    assert_eq!(
        read(project.path(), "AGENTS.md"),
        format!("Above.\n{fresh}Below.\n")
    );
    assert!(saved_copies(project.path()).is_empty());
}

#[test]
fn init_refuses_broken_markers_before_writing_anything() {
    let cases = [
        ("AGENTS.md", "begin without end"),
        ("CLAUDE.md", "end before begin"),
        ("project_guidelines.md", "two blocks"),
    ];
    for (file, case) in cases {
        let project = TempDir::new().expect("project");
        let home = TempDir::new().expect("home");
        init_in(home.path(), project.path(), &[]).success();
        let text = read(project.path(), file);
        let (broken, line) = match case {
            "begin without end" => (format!("one\ntwo\n{BLOCK_START}\nno end\n"), 3),
            "end before begin" => (format!("{BLOCK_END}\n{text}"), 1),
            _ => {
                let lines = text.lines().count();
                (format!("{text}\n{text}"), lines + 2)
            }
        };
        fs::write(project.path().join(file), &broken).expect("break");
        let before = tree(project.path());

        // A run that adds a pack would write folders, the manifest and the
        // schema-drift audit if it got that far.
        let refused = init_in(home.path(), project.path(), &["api"]).failure();
        let stderr = stderr_of(&refused);
        assert!(
            stderr.contains(&format!("{file} line {line}:")),
            "{case}: {stderr}"
        );
        assert!(stderr.contains("Nothing was written"), "{case}: {stderr}");
        assert_eq!(tree(project.path()), before, "{case}");
        assert!(!project.path().join("wiki/apis").exists(), "{case}");
    }
}

#[test]
fn init_rerun_writes_the_block_into_an_existing_upper_case_agents_file() {
    let project = TempDir::new().expect("project");
    let home = TempDir::new().expect("home");
    init_in(home.path(), project.path(), &[]).success();
    let agents = read(project.path(), "AGENTS.md");
    fs::rename(
        project.path().join("AGENTS.md"),
        project.path().join("AGENTS.MD"),
    )
    .expect("rename");
    fs::write(
        project.path().join("AGENTS.MD"),
        format!("{agents}\n## The Project's Own Rules\n"),
    )
    .expect("agents");

    let rerun = init_in(home.path(), project.path(), &["api"]).success();
    assert!(
        !stderr_of(&rerun).contains("Warning"),
        "{}",
        stderr_of(&rerun)
    );

    let names: Vec<String> = fs::read_dir(project.path())
        .expect("read_dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.eq_ignore_ascii_case("agents.md"))
        .collect();
    assert_eq!(names, vec!["AGENTS.MD"]);
    let upper = read(project.path(), "AGENTS.MD");
    assert!(upper.ends_with(&format!("{BLOCK_END}\n\n## The Project's Own Rules\n")));
    assert!(block_of(&upper).contains("Pack Document Types"));
    assert!(block_of(&read(project.path(), "CLAUDE.md")).ends_with("See @AGENTS.MD.\n"));
}
