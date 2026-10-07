use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use assert_cmd::Command;
use serde_json::{Value, json};

const PROJECT_ID: &str = "llm-wiki-framework-semantic-search";
const MODES: &[&str] = &["lexical", "semantic", "hybrid", "auto"];

#[derive(Clone, Debug)]
struct EvalCase {
    id: String,
    split: String,
    query: String,
    purpose: String,
    applicable_modes: BTreeSet<String>,
    expected_pages: Vec<String>,
}

impl EvalCase {
    fn applies_to_mode(&self, mode: &str) -> bool {
        self.applicable_modes.contains(mode)
    }
}

#[derive(Clone, Debug)]
struct ModeOutcome {
    returncode: i32,
    elapsed_seconds: f64,
    stdout_json_parseable: bool,
    stdout_bytes: usize,
    stderr_bytes: usize,
    requested_mode: Option<String>,
    selected_mode: Option<String>,
    mode_selection_reason: Option<String>,
    fallback_reason: Option<String>,
    readiness_reason: Option<String>,
    zero_result_reason: Option<String>,
    profile: Option<String>,
    embedding_model: Option<String>,
    query_expansion_model: Option<String>,
    result_count: usize,
    top_paths: Vec<String>,
    top_scores: Vec<f64>,
    judgment: Judgment,
}

#[derive(Clone, Debug)]
struct Judgment {
    status: &'static str,
    reason: String,
    hit_rank: Option<usize>,
}

#[test]
fn natural_language_eval_query_table_is_well_formed() {
    let cases = parse_eval_cases();

    assert_eq!(cases.len(), 30);
    assert_eq!(
        cases
            .iter()
            .filter(|case| case.split == "Calibration")
            .count(),
        10
    );
    assert_eq!(
        cases.iter().filter(|case| case.split == "Hold-out").count(),
        20
    );
    assert_eq!(
        cases
            .iter()
            .filter(|case| case.expected_pages.is_empty())
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        vec!["C10", "H9", "H11", "H20"]
    );
    assert!(cases.iter().all(|case| !case.query.is_empty()));
    assert!(cases.iter().all(|case| !case.purpose.is_empty()));
    assert!(cases.iter().all(|case| !case.applicable_modes.is_empty()));
    let c1 = cases.iter().find(|case| case.id == "C1").expect("C1");
    assert!(!c1.applies_to_mode("semantic"));
    assert!(c1.applies_to_mode("hybrid"));
}

#[test]
#[ignore = "needs the managed search models and its project registered and indexed; run it with `just search-eval`, which sets both up in a temporary home"]
fn natural_language_eval_runs_against_managed_models() {
    let cases = parse_eval_cases();
    let started = Instant::now();
    let mut results = Vec::new();

    for case in &cases {
        eprintln!("{} {}", case.id, case.query);
        let mut modes = BTreeMap::new();
        for mode in MODES {
            let outcome = run_case_mode(case, mode);
            eprintln!(
                "  {mode:<8} {:<14} results={} elapsed={:.1}s",
                outcome.judgment.status, outcome.result_count, outcome.elapsed_seconds
            );
            modes.insert((*mode).to_string(), outcome);
        }
        results.push((case.clone(), modes));
    }

    let elapsed_seconds = started.elapsed().as_secs_f64();
    write_eval_artifacts(&results, elapsed_seconds);

    let mut pass_counts = BTreeMap::<&str, usize>::new();
    let mut fail_counts = BTreeMap::<&str, usize>::new();
    let mut parse_failures = Vec::new();
    let mut command_failures = Vec::new();
    let mut auto_non_hybrid = Vec::new();

    for (case, modes) in &results {
        for mode in MODES {
            let outcome = modes.get(*mode).expect("mode outcome");
            if !outcome.stdout_json_parseable {
                parse_failures.push(format!("{} {mode}", case.id));
            }
            if outcome.returncode != 0 {
                command_failures.push(format!("{} {mode}: {}", case.id, outcome.returncode));
            }
            match outcome.judgment.status {
                "pass" => *pass_counts.entry(mode).or_default() += 1,
                "fail" => *fail_counts.entry(mode).or_default() += 1,
                _ => {}
            }
            if *mode == "auto" && outcome.selected_mode.as_deref() != Some("hybrid") {
                auto_non_hybrid.push(case.id.clone());
            }
        }
    }

    assert!(
        parse_failures.is_empty(),
        "JSON stdout must remain parseable: {parse_failures:?}"
    );
    assert!(
        command_failures.is_empty(),
        "eval commands must exit successfully: {command_failures:?}"
    );
    assert!(
        auto_non_hybrid.is_empty(),
        "auto should select hybrid for enabled ready profile: {auto_non_hybrid:?}"
    );

    let lexical_passes = pass_counts.get("lexical").copied().unwrap_or(0);
    let semantic_passes = pass_counts.get("semantic").copied().unwrap_or(0);
    let hybrid_passes = pass_counts.get("hybrid").copied().unwrap_or(0);
    let auto_passes = pass_counts.get("auto").copied().unwrap_or(0);
    let hybrid_failures = fail_counts.get("hybrid").copied().unwrap_or(0);
    let auto_failures = fail_counts.get("auto").copied().unwrap_or(0);

    assert!(
        semantic_passes >= 24,
        "seeded semantic thresholds regressed below current eval floor: {semantic_passes}/30"
    );
    assert!(
        hybrid_passes >= 22,
        "seeded hybrid thresholds regressed below current eval floor: {hybrid_passes}/30"
    );
    assert!(
        auto_passes >= 22,
        "auto/hybrid eval regressed below current eval floor: {auto_passes}/30"
    );
    assert!(
        hybrid_failures <= 8 && auto_failures <= 8,
        "seeded threshold failure budget exceeded: hybrid={hybrid_failures}, auto={auto_failures}"
    );
    assert!(
        hybrid_passes > lexical_passes && auto_passes > lexical_passes,
        "hybrid/auto should improve over lexical on the current NL eval: lexical={lexical_passes}, hybrid={hybrid_passes}, auto={auto_passes}"
    );
}

fn run_case_mode(case: &EvalCase, mode: &str) -> ModeOutcome {
    if !case.applies_to_mode(mode) {
        return ModeOutcome {
            returncode: 0,
            elapsed_seconds: 0.0,
            stdout_json_parseable: true,
            stdout_bytes: 0,
            stderr_bytes: 0,
            requested_mode: Some(mode.to_string()),
            selected_mode: Some(mode.to_string()),
            mode_selection_reason: None,
            fallback_reason: None,
            readiness_reason: None,
            zero_result_reason: None,
            profile: None,
            embedding_model: None,
            query_expansion_model: None,
            result_count: 0,
            top_paths: Vec::new(),
            top_scores: Vec::new(),
            judgment: Judgment {
                status: "not_applicable",
                reason: "mode_not_applicable_for_case".to_string(),
                hit_rank: None,
            },
        };
    }
    let mut command = Command::cargo_bin("llm-wiki").expect("binary");
    command
        .env_remove("RUST_LOG")
        .args([
            "search",
            "--project",
            PROJECT_ID,
            "--mode",
            mode,
            "--format",
            "json",
            "--",
        ])
        .arg(&case.query);

    let started = Instant::now();
    let output = command.output().expect("search output");
    let elapsed_seconds = started.elapsed().as_secs_f64();
    outcome_from_output(case, mode, output, elapsed_seconds)
}

fn outcome_from_output(
    case: &EvalCase,
    mode: &str,
    output: Output,
    elapsed_seconds: f64,
) -> ModeOutcome {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = serde_json::from_str::<Value>(&stdout).ok();
    let judgment = judge(
        case,
        mode,
        parsed.as_ref(),
        output.status.code().unwrap_or(-1),
    );
    let results = parsed
        .as_ref()
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    ModeOutcome {
        returncode: output.status.code().unwrap_or(-1),
        elapsed_seconds,
        stdout_json_parseable: parsed.is_some(),
        stdout_bytes: output.stdout.len(),
        stderr_bytes: output.stderr.len(),
        requested_mode: string_field(parsed.as_ref(), "requested_mode"),
        selected_mode: string_field(parsed.as_ref(), "selected_mode"),
        mode_selection_reason: string_field(parsed.as_ref(), "mode_selection_reason"),
        fallback_reason: string_field(parsed.as_ref(), "fallback_reason"),
        readiness_reason: string_field(parsed.as_ref(), "readiness_reason"),
        zero_result_reason: string_field(parsed.as_ref(), "zero_result_reason"),
        profile: string_field(parsed.as_ref(), "profile"),
        embedding_model: string_field(parsed.as_ref(), "embedding_model"),
        query_expansion_model: string_field(parsed.as_ref(), "query_expansion_model"),
        result_count: results.len(),
        top_paths: results
            .iter()
            .take(10)
            .filter_map(|result| string_field(Some(result), "path"))
            .collect(),
        top_scores: results
            .iter()
            .take(10)
            .filter_map(|result| result.get("score").and_then(Value::as_f64))
            .collect(),
        judgment,
    }
}

fn judge(case: &EvalCase, mode: &str, parsed: Option<&Value>, returncode: i32) -> Judgment {
    let Some(parsed) = parsed else {
        return Judgment {
            status: "error",
            reason: "stdout_not_json".to_string(),
            hit_rank: None,
        };
    };
    if returncode != 0 {
        return Judgment {
            status: "error",
            reason: format!("exit_{returncode}"),
            hit_rank: None,
        };
    }

    let results = parsed
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if case.expected_pages.is_empty() {
        if mode == "lexical" {
            return Judgment {
                status: "not_applicable",
                reason: "lexical_no_match_floor_not_defined".to_string(),
                hit_rank: None,
            };
        }
        return if results.is_empty() {
            Judgment {
                status: "pass",
                reason: "no_expected_match_zero_results".to_string(),
                hit_rank: None,
            }
        } else {
            Judgment {
                status: "fail",
                reason: "no_expected_match_returned_results".to_string(),
                hit_rank: None,
            }
        };
    }

    let selected_mode = parsed
        .get("selected_mode")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let top_k = if matches!(mode, "hybrid" | "auto") && selected_mode == "hybrid" {
        5
    } else {
        10
    };

    let hit_rank = results
        .iter()
        .take(top_k)
        .filter_map(|result| result.get("path").and_then(Value::as_str))
        .position(|path| case.expected_pages.iter().any(|expected| expected == path))
        .map(|index| index + 1);

    if hit_rank.is_some() {
        Judgment {
            status: "pass",
            reason: format!("expected_target_in_top_{top_k}"),
            hit_rank,
        }
    } else {
        Judgment {
            status: "fail",
            reason: format!("expected_target_missing_top_{top_k}"),
            hit_rank: None,
        }
    }
}

fn parse_eval_cases() -> Vec<EvalCase> {
    let eval_path = repo_root().join("wiki/evals/natural-language-search.eval.md");
    fs::read_to_string(&eval_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", eval_path.display()))
        .lines()
        .filter(|line| line.starts_with("| C") || line.starts_with("| H"))
        .filter_map(parse_eval_row)
        .collect()
}

fn parse_eval_row(line: &str) -> Option<EvalCase> {
    let columns = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();
    if columns.len() < 5 {
        return None;
    }
    let id = columns[0];
    if !(id.starts_with('C') || id.starts_with('H')) {
        return None;
    }
    let (applicable_modes, expected_column) = if columns.len() >= 6 {
        (parse_mode_list(columns[4]), columns[5])
    } else {
        (parse_mode_list("all"), columns[4])
    };
    let expected_pages = if expected_column.eq_ignore_ascii_case("none") {
        Vec::new()
    } else {
        backtick_values(expected_column)
    };
    Some(EvalCase {
        id: id.to_string(),
        split: columns[1].to_string(),
        query: columns[2].trim_matches('`').to_string(),
        purpose: columns[3].to_string(),
        applicable_modes,
        expected_pages,
    })
}

fn parse_mode_list(input: &str) -> BTreeSet<String> {
    if input.eq_ignore_ascii_case("all") {
        return MODES.iter().map(|mode| (*mode).to_string()).collect();
    }

    let backticked = backtick_values(input);
    let values = if backticked.is_empty() {
        input
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.trim_matches('`').to_string())
            .collect::<Vec<_>>()
    } else {
        backticked
    };
    let modes = values
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| MODES.contains(&value.as_str()))
        .collect::<BTreeSet<_>>();
    if modes.is_empty() {
        MODES.iter().map(|mode| (*mode).to_string()).collect()
    } else {
        modes
    }
}

fn backtick_values(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.push(after_start[..end].to_string());
        rest = &after_start[end + 1..];
    }
    values
}

fn string_field(value: Option<&Value>, field: &str) -> Option<String> {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn write_eval_artifacts(results: &[(EvalCase, BTreeMap<String, ModeOutcome>)], elapsed: f64) {
    let out_dir = repo_root().join("target/evals");
    fs::create_dir_all(&out_dir).expect("create eval output dir");
    let json_path = out_dir.join("natural-language-search-results.json");
    let summary_path = out_dir.join("natural-language-search-summary.md");
    fs::write(&json_path, result_json(results, elapsed).to_string()).expect("write eval json");
    fs::write(&summary_path, result_markdown(results, elapsed)).expect("write eval summary");
    eprintln!("wrote {}", json_path.display());
    eprintln!("wrote {}", summary_path.display());
}

fn result_json(results: &[(EvalCase, BTreeMap<String, ModeOutcome>)], elapsed: f64) -> Value {
    json!({
        "generated_unix_seconds": unix_seconds(),
        "project": PROJECT_ID,
        "query_count": results.len(),
        "modes": MODES,
        "elapsed_seconds": elapsed,
        "results": results.iter().map(|(case, modes)| {
            json!({
                "id": case.id,
                "split": case.split,
                "query": case.query,
                "purpose": case.purpose,
                "applicable_modes": case.applicable_modes,
                "expected_pages": case.expected_pages,
                "modes": modes.iter().map(|(mode, outcome)| {
                    (mode.clone(), json!({
                        "returncode": outcome.returncode,
                        "elapsed_seconds": outcome.elapsed_seconds,
                        "stdout_json_parseable": outcome.stdout_json_parseable,
                        "stdout_bytes": outcome.stdout_bytes,
                        "stderr_bytes": outcome.stderr_bytes,
                        "requested_mode": outcome.requested_mode,
                        "selected_mode": outcome.selected_mode,
                        "mode_selection_reason": outcome.mode_selection_reason,
                        "fallback_reason": outcome.fallback_reason,
                        "readiness_reason": outcome.readiness_reason,
                        "zero_result_reason": outcome.zero_result_reason,
                        "profile": outcome.profile,
                        "embedding_model": outcome.embedding_model,
                        "query_expansion_model": outcome.query_expansion_model,
                        "result_count": outcome.result_count,
                        "top_paths": outcome.top_paths,
                        "top_scores": outcome.top_scores,
                        "judgment": {
                            "status": outcome.judgment.status,
                            "reason": outcome.judgment.reason,
                            "hit_rank": outcome.judgment.hit_rank,
                        },
                    }))
                }).collect::<serde_json::Map<_, _>>()
            })
        }).collect::<Vec<_>>()
    })
}

fn result_markdown(results: &[(EvalCase, BTreeMap<String, ModeOutcome>)], elapsed: f64) -> String {
    let mut output = String::new();
    output.push_str("# Natural-Language Search Eval Summary\n\n");
    output.push_str(&format!("- Project: `{PROJECT_ID}`\n"));
    output.push_str(&format!("- Queries: {}\n", results.len()));
    output.push_str(&format!("- Elapsed seconds: {:.3}\n\n", elapsed));
    output.push_str(
        "| ID | Lexical | Semantic | Hybrid | Auto | Auto selected | Auto top result |\n",
    );
    output.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for (case, modes) in results {
        let lexical = status_cell(modes, "lexical");
        let semantic = status_cell(modes, "semantic");
        let hybrid = status_cell(modes, "hybrid");
        let auto = status_cell(modes, "auto");
        let auto_outcome = modes.get("auto").expect("auto outcome");
        let auto_top = auto_outcome.top_paths.first().cloned().unwrap_or_default();
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            case.id,
            lexical,
            semantic,
            hybrid,
            auto,
            auto_outcome.selected_mode.as_deref().unwrap_or(""),
            auto_top
        ));
    }
    output
}

fn status_cell(modes: &BTreeMap<String, ModeOutcome>, mode: &str) -> String {
    let outcome = modes.get(mode).expect("mode outcome");
    format!("{} ({})", outcome.judgment.status, outcome.judgment.reason)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before epoch")
        .as_secs()
}
