use std::fs;
use std::path::{Path, PathBuf};

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
fn eval_run_records_candidate_profile_and_calibrate_reads_report() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    let eval_page = project.join("wiki/evals/tiny.eval.md");
    let output_dir = workspace.path().join("eval-output");
    let raw_data_dir = workspace.path().join("raw-data");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    write_verified_model_state(home.path());

    let run = llm_wiki(home.path())
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .args(["eval", "run", "--project-root"])
        .arg(&project)
        .args([
            "--candidate-profile",
            "balanced",
            "--candidate-profile",
            "balanced",
            "--eval-page",
        ])
        .arg(&eval_page)
        .args(["--output-dir"])
        .arg(&output_dir)
        .args(["--time-budget-warn-ms", "1"])
        .args(["--format", "json"])
        .output()
        .expect("eval run");
    assert!(
        run.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&run.stderr)
    );
    let run_json: Value = serde_json::from_slice(&run.stdout).expect("run json");
    assert_eq!(run_json["project_id"], "fixture");
    assert_eq!(run_json["candidates"][0]["name"], "balanced");
    assert_eq!(run_json["candidates"][1]["name"], "balanced-2");
    assert_eq!(run_json["candidates"][0]["source"], "catalog_profile");
    assert_eq!(
        run_json["candidates"][0]["models"][0]["accepted_license"],
        true
    );
    assert_ne!(
        run_json["candidates"][0]["candidate_index_dir"],
        run_json["candidates"][1]["candidate_index_dir"]
    );
    assert!(
        run_json["candidates"][0]["candidate_index_size_bytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert!(
        run_json["candidates"][0]["index_build_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        run_json["candidates"][0]["results"][0]["modes"]["hybrid"]["query_expansion_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        run_json["candidates"][1]["lexical_index_ms"].as_u64(),
        Some(0)
    );
    assert_eq!(
        run_json["candidates"][0]["index_fingerprint"],
        run_json["candidates"][1]["index_fingerprint"]
    );
    assert!(output_dir.join("eval-run.json").is_file());
    assert!(
        output_dir
            .join("indexes/balanced/semantic-index.json")
            .is_file()
    );
    assert!(
        output_dir
            .join("indexes/balanced-2/semantic-index.json")
            .is_file()
    );

    let calibrate = llm_wiki(home.path())
        .args(["eval", "calibrate", "--run-report"])
        .arg(output_dir.join("eval-run.json"))
        .args(["--select-candidate", "balanced-2"])
        .args(["--output-dir"])
        .arg(&output_dir)
        .args(["--export-raw-data"])
        .args(["--raw-data-dir"])
        .arg(&raw_data_dir)
        .args(["--format", "json"])
        .output()
        .expect("eval calibrate");
    assert!(
        calibrate.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&calibrate.stderr)
    );
    let calibration_json: Value =
        serde_json::from_slice(&calibrate.stdout).expect("calibration json");
    assert_eq!(
        calibration_json["proposals"][0]["candidate_name"],
        "balanced"
    );
    assert_eq!(
        calibration_json["proposals"][1]["candidate_name"],
        "balanced-2"
    );
    assert_eq!(
        calibration_json["selected_candidate"].as_str(),
        Some("balanced-2")
    );
    assert!(
        calibration_json["proposals"][0]["semantic_similarity_floor"]
            .as_f64()
            .is_some()
    );
    assert!(
        calibration_json["proposals"][0]["candidate_index_size_bytes"]
            .as_u64()
            .is_some_and(|size| size > 0)
    );
    assert!(
        calibration_json["proposals"][0]["proposed_summary"]["hybrid"]["pass"]
            .as_u64()
            .is_some()
    );
    assert!(
        calibration_json["proposals"][0]["holdout_summary"]["hybrid"]["pass"]
            .as_u64()
            .is_some()
    );
    assert!(calibration_json["proposals"][0]["verdict_changes"].is_array());
    assert!(
        calibration_json["proposals"][0]["no_match_precision"]["hybrid"]["proposed_total"]
            .as_u64()
            .is_some()
    );
    assert!(
        calibration_json["proposals"][0]["exact_identifier_preservation"]["hybrid"]
            ["proposed_total"]
            .as_u64()
            .is_some()
    );
    let raw_bundle_report_path = PathBuf::from(
        calibration_json["raw_data_bundle"]
            .as_str()
            .expect("raw data bundle"),
    );
    assert!(raw_bundle_report_path.ends_with("balanced-2"));
    let raw_bundle = raw_data_dir
        .join(run_json["run_id"].as_str().expect("run id"))
        .join("balanced-2");
    assert!(raw_bundle.join("eval-run.json").is_file());
    assert!(raw_bundle.join("eval-calibration.json").is_file());
    let raw_bundles = calibration_json["raw_data_bundles"]
        .as_object()
        .expect("raw data bundles");
    assert_eq!(raw_bundles.len(), 2);
    assert!(
        PathBuf::from(
            raw_bundles["balanced-2"]
                .as_str()
                .expect("balanced-2 bundle")
        )
        .ends_with("balanced-2")
    );
    let manifest_path = raw_bundle.join("manifest.toml");
    let manifest = fs::read_to_string(&manifest_path).expect("manifest");
    let manifest: toml::Value = toml::from_str(&manifest).expect("manifest toml");
    assert_eq!(
        manifest["source_run_id"].as_str(),
        run_json["run_id"].as_str()
    );
    assert_eq!(manifest["selected_candidate"].as_str(), Some("balanced-2"));
    assert_eq!(manifest["files"].as_array().expect("files").len(), 2);
    let raw_run_json = fs::read_to_string(raw_bundle.join("eval-run.json")).expect("raw run json");
    assert!(!raw_run_json.contains("/Users/"));
    assert!(!raw_run_json.contains("/home/"));
    assert!(output_dir.join("eval-calibration.json").is_file());
}

#[test]
fn eval_run_rejects_partial_explicit_model_bundle() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    llm_wiki(home.path())
        .args(["eval", "run", "--project-root"])
        .arg(&project)
        .args(["--eval-page"])
        .arg(project.join("wiki/evals/tiny.eval.md"))
        .args(["--embedding-model", "embeddinggemma-300m-q8_0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--query-expansion-model is required",
        ));
}

#[test]
fn eval_run_rerank_reorders_hybrid_results() {
    let home = TempDir::new().expect("home");
    let workspace = TempDir::new().expect("workspace");
    let project = fixture_project(workspace.path());
    let eval_page = project.join("wiki/evals/tiny.eval.md");
    let baseline_dir = workspace.path().join("baseline-output");
    let reranked_dir = workspace.path().join("reranked-output");

    llm_wiki(home.path())
        .args(["install", "--skip-path-guidance", "--disable-llm-search"])
        .assert()
        .success();
    write_verified_model_state(home.path());

    let mut baseline = llm_wiki(home.path());
    baseline
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .args(["eval", "run", "--project-root"])
        .arg(&project)
        .args(["--eval-page"])
        .arg(&eval_page)
        .args([
            "--embedding-model",
            "embeddinggemma-300m-q8_0",
            "--query-expansion-model",
            "qmd-query-expansion-1.7b-q4_k_m",
            "--reranker-model",
            "qwen3-reranker-0.6b-q8_0",
            "--candidate-name",
            "rerank-fixture",
            "--limit",
            "5",
            "--output-dir",
        ])
        .arg(&baseline_dir)
        .args(["--format", "json"]);
    let baseline = baseline.output().expect("baseline eval run");
    assert!(
        baseline.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&baseline.stderr)
    );

    let mut reranked = llm_wiki(home.path());
    reranked
        .env("LLM_WIKI_TEST_EMBEDDINGS", "deterministic")
        .env("LLM_WIKI_TEST_QUERY_EXPANSION", "deterministic")
        .env("LLM_WIKI_TEST_RERANK", "deterministic")
        .args(["eval", "run", "--project-root"])
        .arg(&project)
        .args(["--eval-page"])
        .arg(&eval_page)
        .args([
            "--embedding-model",
            "embeddinggemma-300m-q8_0",
            "--query-expansion-model",
            "qmd-query-expansion-1.7b-q4_k_m",
            "--reranker-model",
            "qwen3-reranker-0.6b-q8_0",
            "--candidate-name",
            "rerank-fixture",
            "--limit",
            "5",
            "--output-dir",
        ])
        .arg(&reranked_dir)
        .args(["--format", "json", "--rerank"]);
    let reranked = reranked.output().expect("reranked eval run");
    assert!(
        reranked.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&reranked.stderr)
    );

    let baseline_json: Value = serde_json::from_slice(&baseline.stdout).expect("baseline json");
    let reranked_json: Value = serde_json::from_slice(&reranked.stdout).expect("reranked json");
    let baseline_hybrid = &baseline_json["candidates"][0]["results"][0]["modes"]["hybrid"];
    let reranked_hybrid = &reranked_json["candidates"][0]["results"][0]["modes"]["hybrid"];
    let baseline_paths = baseline_hybrid["top_paths"]
        .as_array()
        .expect("baseline top paths");
    let reranked_paths = reranked_hybrid["top_paths"]
        .as_array()
        .expect("reranked top paths");

    assert!(
        baseline_paths.len() > 1,
        "fixture must produce multiple hybrid candidates"
    );
    assert_eq!(baseline_hybrid["rerank_applied"], false);
    assert_eq!(reranked_hybrid["rerank_applied"], true);
    assert!(reranked_hybrid["rerank_ms"].as_u64().is_some());
    assert_ne!(baseline_paths, reranked_paths);
}

fn fixture_project(workspace: &Path) -> PathBuf {
    let project = workspace.join("fixture");
    fs::create_dir_all(project.join("wiki/decisions")).expect("decisions");
    fs::create_dir_all(project.join("wiki/evals")).expect("evals");
    fs::write(project.join("AGENTS.md"), "# Agent Instructions\n").expect("agents");
    fs::write(
        project.join("wiki/index.md"),
        "# Wiki Index\n\n## Decisions\n\n- [Search Decision](decisions/search.decision.md) - Accepted - Search quality evidence.\n- [Secondary Search Decision](decisions/secondary-search.decision.md) - Accepted - Secondary semantic search quality evidence.\n",
    )
    .expect("index");
    fs::write(project.join("wiki/log.md"), "# Wiki Log\n").expect("log");
    fs::write(
        project.join("wiki/decisions/search.decision.md"),
        "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-11\n- Category: Search\n- Scope: Eval fixture\n- Sources: raw/test.md\n\n## Decision\nSemantic search quality evidence uses qmd-rs retrieval and calibration reports.\n",
    )
    .expect("decision");
    fs::write(
        project.join("wiki/decisions/secondary-search.decision.md"),
        "# Secondary Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-11\n- Category: Search\n- Scope: Eval fixture\n- Sources: raw/test.md\n\n## Decision\nSemantic search quality evidence also covers reranking fixtures and secondary retrieval candidates.\n",
    )
    .expect("secondary decision");
    fs::write(
        project.join("wiki/evals/tiny.eval.md"),
        "# Tiny Eval\n\n| ID | Split | Query | Purpose | Draft expected target pages |\n| --- | --- | --- | --- | --- |\n| C1 | Calibration | `semantic search quality evidence` | Expected hit | `wiki/decisions/search.decision.md` |\n| C2 | Calibration | `banana submarine invoice` | No expected match | none |\n| H1 | Hold-out | `D9 search quality evidence` | Exact identifier hold-out | `wiki/decisions/search.decision.md` |\n| H2 | Hold-out | `banana submarine invoice` | Hold-out no expected match | none |\n",
    )
    .expect("eval");
    project
}

fn write_verified_model_state(home: &Path) {
    let model_root = home.join(".llm_wiki/models");
    let embedding_path = model_root.join("embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf");
    let expansion_path =
        model_root.join("qmd-query-expansion-1.7b-q4_k_m/qmd-query-expansion-1.7B-q4_k_m.gguf");
    let reranker_path = model_root.join("qwen3-reranker-0.6b-q8_0/qwen3-reranker-0.6b-q8_0.gguf");
    fs::create_dir_all(embedding_path.parent().expect("embedding parent")).expect("embedding dir");
    fs::create_dir_all(expansion_path.parent().expect("expansion parent")).expect("expansion dir");
    fs::create_dir_all(reranker_path.parent().expect("reranker parent")).expect("reranker dir");
    fs::write(&embedding_path, b"deterministic embedding fixture").expect("embedding file");
    fs::write(&expansion_path, b"deterministic expansion fixture").expect("expansion file");
    fs::write(&reranker_path, b"deterministic reranker fixture").expect("reranker file");

    let artifacts = format!(
        r#"schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "embeddinggemma-300m-q8_0"
role = "embedding"
profile = "balanced"
repository = "ggml-org/embeddinggemma-300M-GGUF"
revision = "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12"
file = "embeddinggemma-300M-Q8_0.gguf"
download_url = "https://example.invalid/embedding"
path = "{}"
expected_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
observed_sha256 = "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63"
size_bytes = 31
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
dimensions = 768
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
role = "query-expansion"
profile = "balanced"
repository = "tobil/qmd-query-expansion-1.7B-gguf"
revision = "7816de0b72572c6c860ca1eddf97ba9e7fb8cc65"
file = "qmd-query-expansion-1.7B-q4_k_m.gguf"
download_url = "https://example.invalid/query-expansion"
path = "{}"
expected_sha256 = "000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0"
observed_sha256 = "000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0"
size_bytes = 31
license = "mit"
dimensions = 0
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"

[[artifacts]]
model_id = "qwen3-reranker-0.6b-q8_0"
role = "reranker"
profile = "balanced"
repository = "ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF"
revision = "a02f48bb4f057028298c21fa033da2b30d7742d5"
file = "qwen3-reranker-0.6b-q8_0.gguf"
download_url = "https://example.invalid/reranker"
path = "{}"
expected_sha256 = "22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48"
observed_sha256 = "22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48"
size_bytes = 29
license = "apache-2.0"
dimensions = 0
qmd_rs_version = "0.3.2"
adapter_schema_version = 1
verified_at = "2026-05-11T00:00:00Z"
"#,
        toml_path(&embedding_path),
        toml_path(&expansion_path),
        toml_path(&reranker_path)
    );
    fs::write(model_root.join("artifacts.toml"), artifacts).expect("artifacts");

    let licenses = r#"schema_version = 1
updated_at = "2026-05-11T00:00:00Z"

[[licenses]]
model_id = "embeddinggemma-300m-q8_0"
license = "gemma"
terms_url = "https://ai.google.dev/gemma/terms"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"

[[licenses]]
model_id = "qmd-query-expansion-1.7b-q4_k_m"
license = "mit"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"

[[licenses]]
model_id = "qwen3-reranker-0.6b-q8_0"
license = "apache-2.0"
accepted_at = "2026-05-11T00:00:00Z"
accepted_by_version = "0.1.1"
"#;
    fs::write(home.join(".llm_wiki/accepted-licenses.toml"), licenses).expect("licenses");
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
