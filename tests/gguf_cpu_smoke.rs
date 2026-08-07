use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

const PROJECT_ID: &str = "gguf-cpu-smoke";
const EMBEDDING_MODEL_ID: &str = "embeddinggemma-300m-q8_0";
const QUERY_EXPANSION_MODEL_ID: &str = "qmd-query-expansion-1.7b-q4_k_m";
const TARGET_PAGE: &str = "wiki/proposals/project-update-command.proposal.md";

fn llm_wiki(home: &Path) -> Command {
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .env("HOME", home)
        .env("LLM_WIKI_GGUF_RUNTIME", "cpu")
        .env_remove("RUST_LOG")
        .env_remove("XDG_CACHE_HOME")
        .env_remove("XDG_DATA_HOME")
        .env_remove("LLM_WIKI_TEST_EMBEDDINGS")
        .env_remove("LLM_WIKI_TEST_QUERY_EXPANSION")
        .env_remove("LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE");
    command
}

#[test]
#[ignore = "requires managed ~/.llm_wiki GGUF model artifacts; run with `cargo test --test gguf_cpu_smoke -- --ignored --nocapture`"]
fn real_gguf_cpu_semantic_hybrid_smoke_uses_tiny_fixture() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    write_real_model_state(home.path());
    register_project(home.path(), &project);

    llm_wiki(home.path())
        .args(["index", "--project", PROJECT_ID, "--force"])
        .assert()
        .success();

    assert_search_top_path(
        home.path(),
        &["search", "--project", PROJECT_ID, "--mode", "semantic"],
    );
    assert_search_top_path(
        home.path(),
        &["search", "--project", PROJECT_ID, "--mode", "hybrid"],
    );
    assert_search_top_path(home.path(), &["search-all", "--mode", "hybrid"]);
}

fn assert_search_top_path(home: &Path, command_prefix: &[&str]) {
    let mut args = command_prefix.to_vec();
    args.extend(["--format", "json", "what is project update"]);
    let output = llm_wiki(home).args(args).output().expect("search output");
    assert!(
        output.status.success(),
        "search failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("search json");
    assert_eq!(json["runtime_backend_requested"], "cpu");
    assert_eq!(json["runtime_backend_used"], "cpu");
    assert_eq!(json["runtime_backend_fallback"], false);
    assert_eq!(json["results"][0]["path"], TARGET_PAGE);
}

fn register_project(home: &Path, project: &Path) {
    llm_wiki(home)
        .args(["register", "--id", PROJECT_ID, "--name", PROJECT_ID])
        .arg(project)
        .assert()
        .success();
}

fn fixture_project(root: &Path) -> PathBuf {
    let project = root.join("gguf-cpu-smoke-project");
    fs::create_dir_all(project.join("wiki/proposals")).expect("proposals");
    fs::create_dir_all(project.join("wiki/decisions")).expect("decisions");
    fs::write(project.join("AGENTS.md"), "# Agents\n").expect("agents");
    fs::write(
        project.join("wiki/index.md"),
        "# Index\n\n- [Project Update Command](proposals/project-update-command.proposal.md)\n",
    )
    .expect("index");
    fs::write(project.join("wiki/log.md"), "# Log\n").expect("log");
    fs::write(
        project.join(TARGET_PAGE),
        "# Project Update Command\n\n- Document Class: Proposal\n- Status: Proposed\n- Date: 2026-05-25\n- Category: CLI\n- Scope: Test fixture\n- Sources: test fixture\n\n## Summary\nThe project update command refreshes project-scoped wiki maintenance, registry state, index freshness, and bookkeeping without modifying global model artifacts.",
    )
    .expect("target page");
    fs::write(
        project.join("wiki/decisions/unrelated.decision.md"),
        "# Unrelated Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-25\n- Category: Fixture\n- Scope: Test fixture\n- Sources: test fixture\n\n## Decision\nA different document discusses unrelated fixture material so retrieval has a non-target candidate.",
    )
    .expect("other page");
    project.canonicalize().expect("canonical project")
}

fn write_real_model_state(home: &Path) {
    let managed = home.join(".llm_wiki");
    let source = real_managed_home();
    let artifacts_path = source.join("models/artifacts.toml");
    let licenses_path = source.join("accepted-licenses.toml");
    let artifacts_toml = fs::read_to_string(&artifacts_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", artifacts_path.display()));
    let licenses_toml = fs::read_to_string(&licenses_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", licenses_path.display()));
    let parsed: toml::Value = toml::from_str(&artifacts_toml).expect("artifacts toml");
    let embedding = artifact(&parsed, EMBEDDING_MODEL_ID);
    let query_expansion = artifact(&parsed, QUERY_EXPANSION_MODEL_ID);
    assert!(
        embedding.path.exists(),
        "managed embedding artifact is missing: {}",
        embedding.path.display()
    );
    assert!(
        query_expansion.path.exists(),
        "managed query-expansion artifact is missing: {}",
        query_expansion.path.display()
    );

    fs::create_dir_all(managed.join("models")).expect("models dir");
    fs::write(managed.join("models/artifacts.toml"), artifacts_toml).expect("artifacts");
    fs::write(managed.join("accepted-licenses.toml"), licenses_toml).expect("licenses");
    fs::write(
        managed.join("search.toml"),
        r#"
schema_version = 1
updated_at = "2026-05-25T00:00:00Z"

[project_default]
llm_search_enabled = true
configured_at = "2026-05-25T00:00:00Z"
configured_by_version = "test"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"

[global_search]
llm_search_enabled = true
configured_at = "2026-05-25T00:00:00Z"
configured_by_version = "test"
profile = "balanced"
embedding_model = "embeddinggemma-300m-q8_0"
query_expansion_model = "qmd-query-expansion-1.7b-q4_k_m"
"#,
    )
    .expect("search config");
    fs::write(
        managed.join("search-thresholds.toml"),
        format!(
            r#"
schema_version = 1
updated_at = "2026-05-25T00:00:00Z"
profile = "balanced"
semantic_similarity_floor = 0.0
hybrid_pre_fusion_semantic_floor = 0.0
hybrid_final_semantic_floor = 0.0
hybrid_semantic_only_floor = 0.0
hybrid_strong_lexical_score_floor = 0.0
reranker_probability_floor = 0.0
lexical_exact_identifier_guard = "preserve_lexical_top_3"
qmd_rs_version = "{}"
adapter_schema_version = {}
chunking_strategy = "qmd-rs-character-v1:3200:480"
embedding_model = "{}"
embedding_artifact_sha256 = "{}"
embedding_dimensions = {}
"#,
            embedding.qmd_rs_version,
            embedding.adapter_schema_version,
            EMBEDDING_MODEL_ID,
            embedding.observed_sha256,
            embedding.dimensions
        ),
    )
    .expect("thresholds");
}

fn real_managed_home() -> PathBuf {
    let home = std::env::var_os("LLM_WIKI_REAL_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .expect("HOME must be set to find managed GGUF artifacts");
    PathBuf::from(home).join(".llm_wiki")
}

struct Artifact {
    path: PathBuf,
    observed_sha256: String,
    dimensions: i64,
    qmd_rs_version: String,
    adapter_schema_version: i64,
}

fn artifact(parsed: &toml::Value, model_id: &str) -> Artifact {
    let artifacts = parsed["artifacts"].as_array().expect("artifacts array");
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact["model_id"].as_str() == Some(model_id))
        .unwrap_or_else(|| panic!("missing managed artifact record for {model_id}"));
    Artifact {
        path: PathBuf::from(artifact["path"].as_str().expect("artifact path")),
        observed_sha256: artifact["observed_sha256"]
            .as_str()
            .expect("observed sha")
            .to_string(),
        dimensions: artifact
            .get("dimensions")
            .and_then(toml::Value::as_integer)
            .unwrap_or(768),
        qmd_rs_version: artifact["qmd_rs_version"]
            .as_str()
            .expect("qmd-rs version")
            .to_string(),
        adapter_schema_version: artifact["adapter_schema_version"]
            .as_integer()
            .expect("adapter schema version"),
    }
}
