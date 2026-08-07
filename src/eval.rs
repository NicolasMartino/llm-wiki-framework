use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::{CliContext, EvalArgs, EvalCalibrateArgs, EvalCommand, EvalRunArgs, OutputFormat};
use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::registry::{ProjectRegistry, RegisteredProject};
use crate::search::adapter::{Freshness, Score, SearchBackend, SearchFilters, SearchMode};
use crate::search::commands::{
    RerankInputs, dedupe_by_path_preserving_rank, expand_hybrid_queries, freshness_for_status,
    fuse_hybrid_results, maybe_rerank_results, query_anchor_terms,
};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;
use crate::search::semantic::{
    CHUNKING_STRATEGY, SemanticCorpusSnapshot, SemanticIndexMetadata, SemanticSearchContext,
    SemanticVectorIndex, embed_query,
};
use crate::search_models::{
    AcceptedLicenses, ModelArtifactRecord, ModelArtifacts, SearchThresholdStore, SearchThresholds,
    model_by_id, profile_by_id,
};
use crate::search_profile::{ProjectSearchConfig, SearchConfig, SearchProfile, timestamp};

const EVAL_REPORT_SCHEMA_VERSION: u32 = 1;
const CALIBRATION_REPORT_SCHEMA_VERSION: u32 = 2;
const RAW_EVAL_DATA_MANIFEST_SCHEMA_VERSION: u32 = 1;
const MODES: [&str; 4] = ["lexical", "semantic", "hybrid", "auto"];

pub fn run(args: &EvalArgs, context: &CliContext) -> Result<()> {
    match &args.command {
        EvalCommand::Run(args) => {
            let report = run_eval(args, context)?;
            print_run_report(&report, args.format)
        }
        EvalCommand::Calibrate(args) => {
            let report = calibrate(args, context)?;
            print_calibration_report(&report, args.run.format)
        }
    }
}

fn run_eval(args: &EvalRunArgs, context: &CliContext) -> Result<EvalRunReport> {
    context.diagnostic("command: eval run");
    context.diagnostic(format!("eval page: {}", args.eval_page.display()));
    context.diagnostic(format!(
        "requested project: {}",
        args.project
            .as_deref()
            .or_else(|| args.project_root.as_ref().map(|_| "<project-root>"))
            .unwrap_or("<current directory>")
    ));
    let paths = Paths::from_env()?;
    ensure_base_install(&paths)?;
    let project = resolve_eval_project(args, &paths, context)?;
    ensure_project_root_exists(&project)?;

    let cases = parse_eval_page(&args.eval_page)?;
    let candidates = candidate_specs(args, &paths, &project)?;
    let run_id = run_id();
    let output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/evals").join(&run_id));
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create eval output dir {}", output_dir.display()))?;
    let index_root = output_dir.join("indexes");
    fs::create_dir_all(&index_root)
        .with_context(|| format!("create eval index dir {}", index_root.display()))?;

    context.diagnostic(format!("selected project: {}", project.id));
    context.diagnostic(format!("eval cases: {}", cases.len()));
    context.diagnostic(format!("eval candidates: {}", candidates.len()));
    context.diagnostic(format!("eval output dir: {}", output_dir.display()));

    let backend = QmdRsBackend::new();
    let wiki_root = project.wiki_root();
    let store_path = output_dir.join("lexical/qmd-rs.sqlite");
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create lexical store dir {}", parent.display()))?;
    }
    let lexical_started = Instant::now();
    let lexical_status = backend.rebuild_or_recover(&project.id, &wiki_root, &store_path, true)?;
    let lexical_index_ms = elapsed_ms(lexical_started);
    warn_time_budget(
        args.time_budget_warn_ms,
        &format!(
            "candidate=<shared> stage=lexical_index path={}",
            store_path.display()
        ),
        lexical_index_ms,
    );
    context.diagnostic(format!(
        "scratch lexical store: {} indexed_files={} elapsed={}ms",
        store_path.display(),
        lexical_status.indexed_files,
        lexical_index_ms
    ));
    let corpus_started = Instant::now();
    let semantic_corpus = SemanticIndexMetadata::build_corpus(&wiki_root);
    let semantic_corpus_ms = elapsed_ms(corpus_started);
    warn_time_budget(
        args.time_budget_warn_ms,
        "candidate=<shared> stage=semantic_metadata",
        semantic_corpus_ms,
    );
    context.diagnostic(match &semantic_corpus {
        Ok(corpus) => format!(
            "semantic corpus chunks={} elapsed={}ms",
            corpus.chunks.len(),
            semantic_corpus_ms
        ),
        Err(error) => format!(
            "semantic corpus failed elapsed={}ms error={error}",
            semantic_corpus_ms
        ),
    });
    let eval_context = EvalExecutionContext {
        project: &project,
        backend: &backend,
        store_path: &store_path,
        wiki_root: &wiki_root,
        limit: args.limit,
        rerank: args.rerank,
    };
    let artifacts = ModelArtifacts::read(&paths.model_artifacts())?;
    let accepted_licenses = AcceptedLicenses::read(&paths.accepted_licenses())?;
    let mut candidate_reports = Vec::new();

    for (candidate_index, candidate) in candidates.into_iter().enumerate() {
        context.diagnostic(format!("eval candidate: {}", candidate.name));
        let started = Instant::now();
        let shared_corpus = semantic_corpus.as_ref().map_err(|error| error.to_string());
        let prepared = PreparedCandidate::prepare(
            candidate,
            artifacts.as_ref(),
            accepted_licenses.as_ref(),
            &project,
            &index_root,
            SharedEvalIndexes {
                corpus: shared_corpus,
                lexical_index_ms: if candidate_index == 0 {
                    lexical_index_ms
                } else {
                    0
                },
                semantic_metadata_ms: if candidate_index == 0 {
                    semantic_corpus_ms
                } else {
                    0
                },
            },
        );
        let mut report = prepared.report.clone();
        let mut cases_out = Vec::new();
        for case in &cases {
            let mut modes: BTreeMap<String, EvalModeOutcome> = BTreeMap::new();
            for mode in MODES {
                // `auto` resolves to hybrid with identical inputs when the case
                // applies to both modes and the candidate is hybrid-ready, so
                // reuse the hybrid outcome already computed for this case rather
                // than paying for a second model inference.
                let outcome = if mode == "auto"
                    && case.applies_to_mode("auto")
                    && case.applies_to_mode("hybrid")
                    && prepared.hybrid_ready()
                    && modes.contains_key("hybrid")
                {
                    let mut reused = modes["hybrid"].clone();
                    reused.selected_mode = Some("hybrid".to_string());
                    reused
                } else {
                    run_case_mode(mode, case, &eval_context, &prepared)
                };
                warn_mode_time_budget(
                    args.time_budget_warn_ms,
                    &prepared.spec.name,
                    &case.id,
                    mode,
                    &outcome,
                );
                modes.insert(mode.to_string(), outcome);
            }
            cases_out.push(EvalCaseRun {
                id: case.id.clone(),
                split: case.split.clone(),
                query: case.query.clone(),
                purpose: case.purpose.clone(),
                expected_pages: case.expected_pages.clone(),
                modes,
            });
        }
        report.elapsed_seconds = round_seconds(started.elapsed().as_secs_f64());
        report.summary = summarize_candidate(&cases_out);
        report.results = cases_out;
        warn_time_budget(
            args.time_budget_warn_ms,
            &format!("candidate={} stage=index_build", prepared.spec.name),
            report.index_build_ms,
        );
        context.diagnostic(candidate_timing_summary(&report));
        candidate_reports.push(report);
    }

    let report_path = output_dir.join("eval-run.json");
    let summary_path = output_dir.join("eval-run-summary.md");
    let report = EvalRunReport {
        schema_version: EVAL_REPORT_SCHEMA_VERSION,
        run_id,
        generated_at: timestamp(),
        eval_page: args.eval_page.clone(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        project_root: project.root.clone(),
        wiki_root,
        query_count: cases.len(),
        limit: args.limit,
        rerank_requested: args.rerank,
        output_dir: output_dir.clone(),
        report_path: report_path.clone(),
        summary_path: summary_path.clone(),
        candidates: candidate_reports,
    };
    write_json(&report_path, &report)?;
    fs::write(&summary_path, render_run_summary(&report))
        .with_context(|| format!("write eval summary {}", summary_path.display()))?;
    Ok(report)
}

fn calibrate(args: &EvalCalibrateArgs, context: &CliContext) -> Result<EvalCalibrationReport> {
    context.diagnostic("command: eval calibrate");
    let run_report = if let Some(path) = &args.run_report {
        read_run_report(path)?
    } else {
        run_eval(&args.run, context)?
    };

    let proposals = run_report
        .candidates
        .iter()
        .map(calibrate_candidate)
        .collect::<Vec<_>>();
    let selected = select_calibration_proposal(
        &proposals,
        args.select_candidate.as_deref(),
        args.apply || args.record,
    )?;
    let report_path = calibration_report_path(args, &run_report);
    let selected_candidate = selected
        .as_ref()
        .map(|proposal| proposal.candidate_name.clone());
    let generated_at = timestamp();
    let raw_data_bundles = if args.export_raw_data {
        raw_data_bundle_paths(args, &run_report)?
    } else {
        BTreeMap::new()
    };
    let raw_data_bundle = selected_candidate
        .as_ref()
        .and_then(|candidate| raw_data_bundles.get(candidate).cloned());
    let raw_data_manifest = raw_data_bundle
        .as_ref()
        .map(|bundle| bundle.join("manifest.toml"));
    let report = EvalCalibrationReport {
        schema_version: CALIBRATION_REPORT_SCHEMA_VERSION,
        generated_at,
        source_run_id: run_report.run_id.clone(),
        source_report_path: run_report.report_path.clone(),
        eval_page: run_report.eval_page.clone(),
        project_id: run_report.project_id.clone(),
        project_name: run_report.project_name.clone(),
        proposals,
        selected_candidate,
        applied: false,
        recorded: false,
        report_path,
        raw_data_bundle,
        raw_data_manifest,
        raw_data_bundles,
    };

    if args.apply {
        let proposal = selected
            .as_ref()
            .context("no calibration proposal available to apply")?;
        if !proposal.promotable {
            bail!(
                "calibration proposal for {} is not promotable: {}",
                proposal.candidate_name,
                proposal.status
            );
        }
        if proposal.requires_apply_profile && args.apply_profile.is_none() {
            bail!(
                "`--apply-profile <scope>` is required when applying candidate {}",
                proposal.candidate_name
            );
        }
        apply_thresholds(args, &run_report, proposal)?;
    }

    let mut report = report;
    report.applied = args.apply;
    if args.record {
        let proposal = selected
            .as_ref()
            .context("no calibration proposal selected to record")?;
        record_calibration(args, &run_report, &report, proposal)?;
        report.recorded = true;
    }
    write_json(&report.report_path, &report)?;
    if args.export_raw_data {
        export_raw_eval_data(&run_report, &report, context)?;
    }
    Ok(report)
}

fn select_calibration_proposal(
    proposals: &[CandidateCalibrationProposal],
    requested: Option<&str>,
    require_selection: bool,
) -> Result<Option<CandidateCalibrationProposal>> {
    if let Some(requested) = requested {
        let proposal = proposals
            .iter()
            .find(|proposal| proposal.candidate_name == requested)
            .cloned()
            .with_context(|| {
                format!("calibration candidate {requested} was not found in run report")
            })?;
        return Ok(Some(proposal));
    }

    match proposals {
        [proposal] => Ok(Some(proposal.clone())),
        [] => Ok(None),
        _ if require_selection => {
            bail!("multiple calibration candidates are available; pass `--select-candidate <name>`")
        }
        _ => Ok(None),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalRunReport {
    schema_version: u32,
    run_id: String,
    generated_at: String,
    #[serde(serialize_with = "serialize_redacted_path")]
    eval_page: PathBuf,
    project_id: String,
    project_name: String,
    #[serde(serialize_with = "serialize_redacted_path")]
    project_root: PathBuf,
    #[serde(serialize_with = "serialize_redacted_path")]
    wiki_root: PathBuf,
    query_count: usize,
    limit: usize,
    rerank_requested: bool,
    #[serde(serialize_with = "serialize_redacted_path")]
    output_dir: PathBuf,
    #[serde(serialize_with = "serialize_redacted_path")]
    report_path: PathBuf,
    #[serde(serialize_with = "serialize_redacted_path")]
    summary_path: PathBuf,
    candidates: Vec<EvalCandidateRun>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalCandidateRun {
    name: String,
    source: String,
    profile: Option<String>,
    embedding_model: Option<String>,
    query_expansion_model: Option<String>,
    reranker_model: Option<String>,
    models: Vec<EvalCandidateModel>,
    readiness_reason: Option<String>,
    index_fingerprint: Option<String>,
    vector_count: usize,
    #[serde(serialize_with = "serialize_redacted_path")]
    candidate_index_dir: PathBuf,
    #[serde(default)]
    candidate_index_size_bytes: u64,
    #[serde(default)]
    index_build_ms: u64,
    #[serde(default)]
    lexical_index_ms: u64,
    #[serde(default)]
    semantic_metadata_ms: u64,
    #[serde(default)]
    semantic_vector_build_ms: u64,
    elapsed_seconds: f64,
    summary: BTreeMap<String, EvalModeSummary>,
    results: Vec<EvalCaseRun>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalCandidateModel {
    role: String,
    model_id: String,
    repository: Option<String>,
    revision: Option<String>,
    artifact_sha256: Option<String>,
    artifact_size_bytes: Option<u64>,
    dimensions: Option<usize>,
    accepted_license: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_redacted_path"
    )]
    artifact_path: Option<PathBuf>,
    readiness_reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EvalModeSummary {
    pass: usize,
    fail: usize,
    not_applicable: usize,
    error: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalCaseRun {
    id: String,
    split: String,
    query: String,
    purpose: String,
    expected_pages: Vec<String>,
    modes: BTreeMap<String, EvalModeOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalModeOutcome {
    status: String,
    reason: String,
    selected_mode: Option<String>,
    readiness_reason: Option<String>,
    elapsed_seconds: f64,
    #[serde(default)]
    query_expansion_ms: u64,
    #[serde(default)]
    embed_query_ms: u64,
    #[serde(default)]
    vector_search_ms: u64,
    #[serde(default)]
    lexical_search_ms: u64,
    #[serde(default)]
    rerank_ms: u64,
    #[serde(default)]
    rerank_applied: bool,
    result_count: usize,
    hit_rank: Option<usize>,
    top_paths: Vec<String>,
    top_scores: Vec<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    top_lexical_ranks: Vec<Option<usize>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    top_semantic_ranks: Vec<Option<usize>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    top_lexical_scores: Vec<Option<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    top_semantic_scores: Vec<Option<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    top_anchor_matches: Vec<usize>,
    error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EvalCalibrationReport {
    schema_version: u32,
    generated_at: String,
    source_run_id: String,
    #[serde(serialize_with = "serialize_redacted_path")]
    source_report_path: PathBuf,
    #[serde(serialize_with = "serialize_redacted_path")]
    eval_page: PathBuf,
    project_id: String,
    project_name: String,
    proposals: Vec<CandidateCalibrationProposal>,
    selected_candidate: Option<String>,
    applied: bool,
    recorded: bool,
    #[serde(serialize_with = "serialize_redacted_path")]
    report_path: PathBuf,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_redacted_path"
    )]
    raw_data_bundle: Option<PathBuf>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_redacted_path"
    )]
    raw_data_manifest: Option<PathBuf>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_redacted_path_map"
    )]
    raw_data_bundles: BTreeMap<String, PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CandidateCalibrationProposal {
    candidate_name: String,
    profile: Option<String>,
    embedding_model: Option<String>,
    embedding_artifact_sha256: Option<String>,
    embedding_dimensions: Option<usize>,
    index_fingerprint: Option<String>,
    model_artifact_size_bytes: u64,
    candidate_index_size_bytes: u64,
    semantic_similarity_floor: Option<f64>,
    semantic_min_expected_score: Option<f64>,
    semantic_max_no_match_score: Option<f64>,
    semantic_missing_expected_targets: Vec<String>,
    semantic_promotable: bool,
    hybrid_pre_fusion_semantic_floor: Option<f64>,
    hybrid_min_expected_score: Option<f64>,
    hybrid_max_no_match_score: Option<f64>,
    hybrid_missing_expected_targets: Vec<String>,
    hybrid_promotable: bool,
    hybrid_final_semantic_floor: f64,
    hybrid_semantic_only_floor: f64,
    hybrid_strong_lexical_score_floor: f64,
    hybrid_max_no_match_lexical_score: f64,
    calibration_expected_queries: usize,
    calibration_no_match_queries: usize,
    missing_expected_targets: Vec<String>,
    status: String,
    promotable: bool,
    #[serde(default)]
    post_apply_validation_required: bool,
    requires_apply_profile: bool,
    current_summary: BTreeMap<String, EvalModeSummary>,
    proposed_summary: BTreeMap<String, EvalModeSummary>,
    holdout_summary: BTreeMap<String, EvalModeSummary>,
    proposed_calibration_pass: bool,
    holdout_pass: bool,
    verdict_changes: Vec<VerdictChange>,
    no_match_precision: BTreeMap<String, EvalProposalMetricSummary>,
    exact_identifier_preservation: BTreeMap<String, EvalProposalMetricSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VerdictChange {
    id: String,
    split: String,
    mode: String,
    current_status: String,
    current_reason: String,
    current_hit_rank: Option<usize>,
    proposed_status: String,
    proposed_reason: String,
    proposed_hit_rank: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct EvalProposalMetricSummary {
    current_pass: usize,
    current_total: usize,
    proposed_pass: usize,
    proposed_total: usize,
}

#[derive(Clone, Debug, Serialize)]
struct RawEvalDataManifest {
    schema_version: u32,
    exported_at: String,
    source_run_id: String,
    project_id: String,
    project_name: String,
    eval_page: String,
    source_run_report: String,
    source_calibration_report: String,
    selected_candidate: Option<String>,
    candidates: Vec<RawEvalDataCandidate>,
    files: Vec<RawEvalDataFile>,
}

#[derive(Clone, Debug, Serialize)]
struct RawEvalDataCandidate {
    name: String,
    status: String,
    promotable: bool,
    semantic_similarity_floor: Option<f64>,
    hybrid_pre_fusion_semantic_floor: Option<f64>,
    proposed_calibration_pass: bool,
    holdout_pass: bool,
    verdict_changes: usize,
    post_apply_validation_required: bool,
}

#[derive(Clone, Debug, Serialize)]
struct RawEvalDataFile {
    name: String,
    sha256: String,
    size_bytes: u64,
}

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
struct CandidateSpec {
    name: String,
    source: String,
    profile: SearchProfile,
}

struct PreparedCandidate {
    spec: CandidateSpec,
    report: EvalCandidateRun,
    artifacts: Option<ModelArtifacts>,
    accepted_licenses: Option<AcceptedLicenses>,
    embedding_artifact: Option<ModelArtifactRecord>,
    query_expansion_artifact: Option<ModelArtifactRecord>,
    reranker_artifact: Option<ModelArtifactRecord>,
    metadata: Option<SemanticIndexMetadata>,
    vectors: Option<SemanticVectorIndex>,
    thresholds: Option<SearchThresholds>,
}

struct EvalExecutionContext<'a> {
    project: &'a RegisteredProject,
    backend: &'a QmdRsBackend,
    store_path: &'a Path,
    wiki_root: &'a Path,
    limit: usize,
    rerank: bool,
}

struct SharedEvalIndexes<'a> {
    corpus: std::result::Result<&'a SemanticCorpusSnapshot, String>,
    lexical_index_ms: u64,
    semantic_metadata_ms: u64,
}

impl PreparedCandidate {
    fn prepare(
        spec: CandidateSpec,
        artifacts: Option<&ModelArtifacts>,
        accepted_licenses: Option<&AcceptedLicenses>,
        project: &RegisteredProject,
        index_root: &Path,
        shared: SharedEvalIndexes<'_>,
    ) -> Self {
        let candidate_index_dir = index_root.join(slugify(&spec.name));
        let mut readiness = Vec::new();
        let mut models = Vec::new();
        let embedding_artifact = candidate_model_report(
            "embedding",
            spec.profile.embedding_model.as_deref(),
            artifacts,
            accepted_licenses,
            &mut models,
        );
        let query_expansion_artifact = candidate_model_report(
            "query-expansion",
            spec.profile.query_expansion_model.as_deref(),
            artifacts,
            accepted_licenses,
            &mut models,
        );
        let reranker_artifact = candidate_model_report(
            "reranker",
            spec.profile.reranker_model.as_deref(),
            artifacts,
            accepted_licenses,
            &mut models,
        );

        for model in &models {
            if let Some(reason) = &model.readiness_reason {
                readiness.push(format!("{}:{reason}", model.role));
            }
        }

        let mut metadata = None;
        let mut vectors = None;
        let mut thresholds = None;
        let mut index_fingerprint = None;
        let mut vector_count = 0usize;
        let mut semantic_metadata_ms = shared.semantic_metadata_ms;
        let mut semantic_vector_build_ms = 0;

        if let (Some(artifacts), Some(embedding_artifact)) =
            (artifacts, embedding_artifact.as_ref())
        {
            match shared
                .corpus
                .map_err(anyhow::Error::msg)
                .and_then(|corpus| {
                    let metadata_started = Instant::now();
                    let built_metadata = SemanticIndexMetadata::from_corpus(
                        &project.id,
                        &spec.profile,
                        artifacts,
                        corpus,
                    )?;
                    semantic_metadata_ms += elapsed_ms(metadata_started);
                    let measurement_thresholds = SearchThresholds::measurement(
                        spec.profile
                            .profile
                            .clone()
                            .unwrap_or_else(|| spec.name.clone()),
                        built_metadata.embedding_model.clone(),
                        embedding_artifact.observed_sha256.clone(),
                        built_metadata.embedding_dimensions,
                        built_metadata.chunking_strategy.clone(),
                    );
                    let vector_started = Instant::now();
                    let built_vectors = SemanticVectorIndex::build_from_corpus(
                        &built_metadata,
                        corpus,
                        embedding_artifact,
                    )?;
                    semantic_vector_build_ms = elapsed_ms(vector_started);
                    Ok((built_metadata, built_vectors, measurement_thresholds))
                }) {
                Ok((built_metadata, built_vectors, measurement_thresholds)) => {
                    if let Err(error) = built_metadata
                        .write_atomic(&candidate_index_dir.join("semantic-index.json"))
                    {
                        readiness.push(format!("candidate_index_write_failed:{error}"));
                    }
                    if let Err(error) = built_vectors
                        .write_atomic(&candidate_index_dir.join("semantic-vectors.json"))
                    {
                        readiness.push(format!("candidate_index_write_failed:{error}"));
                    }
                    index_fingerprint = Some(built_metadata.source_fingerprint());
                    vector_count = built_vectors.vectors.len();
                    metadata = Some(built_metadata);
                    vectors = Some(built_vectors);
                    thresholds = Some(measurement_thresholds);
                }
                Err(error) => {
                    readiness.push(format!("candidate_index_build_failed:{error}"));
                }
            }
        }

        let candidate_index_size_bytes = dir_size_bytes(&candidate_index_dir).unwrap_or(0);
        let index_build_ms = shared
            .lexical_index_ms
            .saturating_add(semantic_metadata_ms)
            .saturating_add(semantic_vector_build_ms);
        let readiness_reason = (!readiness.is_empty()).then(|| readiness.join(","));
        let report = EvalCandidateRun {
            name: spec.name.clone(),
            source: spec.source.clone(),
            profile: spec.profile.profile.clone(),
            embedding_model: spec.profile.embedding_model.clone(),
            query_expansion_model: spec.profile.query_expansion_model.clone(),
            reranker_model: spec.profile.reranker_model.clone(),
            models,
            readiness_reason,
            index_fingerprint,
            vector_count,
            candidate_index_dir,
            candidate_index_size_bytes,
            index_build_ms,
            lexical_index_ms: shared.lexical_index_ms,
            semantic_metadata_ms,
            semantic_vector_build_ms,
            elapsed_seconds: 0.0,
            summary: BTreeMap::new(),
            results: Vec::new(),
        };
        Self {
            spec,
            report,
            artifacts: artifacts.cloned(),
            accepted_licenses: accepted_licenses.cloned(),
            embedding_artifact,
            query_expansion_artifact,
            reranker_artifact,
            metadata,
            vectors,
            thresholds,
        }
    }

    fn semantic_ready(&self) -> bool {
        self.embedding_artifact.is_some()
            && self.metadata.is_some()
            && self.vectors.is_some()
            && self.thresholds.is_some()
    }

    fn hybrid_ready(&self) -> bool {
        self.semantic_ready() && self.query_expansion_artifact.is_some()
    }

    fn rerank_readiness_reason(&self) -> Option<String> {
        self.spec.profile.reranker_model.as_ref()?;
        if self.reranker_artifact.is_some() {
            return None;
        }
        Some(
            self.report
                .models
                .iter()
                .find(|model| model.role == "reranker")
                .and_then(|model| model.readiness_reason.clone())
                .unwrap_or_else(|| "reranker_artifact_missing".to_string()),
        )
    }
}

fn candidate_model_report(
    role: &str,
    model_id: Option<&str>,
    artifacts: Option<&ModelArtifacts>,
    accepted_licenses: Option<&AcceptedLicenses>,
    out: &mut Vec<EvalCandidateModel>,
) -> Option<ModelArtifactRecord> {
    let model_id = model_id?;
    let catalog = model_by_id(model_id);
    let accepted_license = accepted_licenses
        .zip(catalog)
        .is_some_and(|(accepted, model)| accepted.accepts_model(model));
    let artifact = artifacts.and_then(|artifacts| {
        artifacts.artifacts.iter().find(|artifact| {
            artifact.model_id == model_id
                && catalog.is_none_or(|model| {
                    artifact.expected_sha256 == model.expected_sha256
                        && artifact.observed_sha256 == model.expected_sha256
                })
                && artifact.path.exists()
        })
    });
    let readiness_reason = if catalog.is_none() {
        Some("unknown_model".to_string())
    } else if artifact.is_none() {
        Some("artifact_missing".to_string())
    } else if !accepted_license {
        Some("license_not_accepted".to_string())
    } else {
        None
    };
    out.push(EvalCandidateModel {
        role: role.to_string(),
        model_id: model_id.to_string(),
        repository: catalog.map(|model| model.repository.to_string()),
        revision: catalog.map(|model| model.revision.to_string()),
        artifact_sha256: artifact.map(|artifact| artifact.observed_sha256.clone()),
        artifact_size_bytes: artifact.map(|artifact| artifact.size_bytes),
        dimensions: catalog.and_then(|model| model.dimensions),
        accepted_license,
        artifact_path: artifact.map(|artifact| artifact.path.clone()),
        readiness_reason: readiness_reason.clone(),
    });
    if readiness_reason.is_some() {
        None
    } else {
        artifact.cloned()
    }
}

fn run_case_mode(
    mode: &str,
    case: &EvalCase,
    context: &EvalExecutionContext<'_>,
    candidate: &PreparedCandidate,
) -> EvalModeOutcome {
    if !case.applies_to_mode(mode) {
        return not_applicable_outcome(mode, "mode_not_applicable_for_case");
    }
    let started = Instant::now();
    let result = match mode {
        "lexical" => run_lexical(case, context),
        "semantic" => run_semantic(case, context, candidate),
        "hybrid" => run_hybrid(case, context, candidate),
        "auto" => {
            if candidate.hybrid_ready() {
                run_hybrid(case, context, candidate).map(|mut outcome| {
                    outcome.selected_mode = Some("hybrid".to_string());
                    outcome
                })
            } else {
                Ok(readiness_outcome(
                    "auto",
                    candidate
                        .report
                        .readiness_reason
                        .as_deref()
                        .unwrap_or("candidate_hybrid_not_ready"),
                ))
            }
        }
        other => Err(anyhow::anyhow!("unsupported eval mode {other}")),
    };
    match result {
        Ok(mut outcome) => {
            outcome.elapsed_seconds = round_seconds(started.elapsed().as_secs_f64());
            outcome
        }
        Err(error) => EvalModeOutcome {
            status: "error".to_string(),
            reason: "execution_error".to_string(),
            selected_mode: Some(mode.to_string()),
            readiness_reason: None,
            elapsed_seconds: round_seconds(started.elapsed().as_secs_f64()),
            query_expansion_ms: 0,
            embed_query_ms: 0,
            vector_search_ms: 0,
            lexical_search_ms: 0,
            rerank_ms: 0,
            rerank_applied: false,
            result_count: 0,
            hit_rank: None,
            top_paths: Vec::new(),
            top_scores: Vec::new(),
            top_lexical_ranks: Vec::new(),
            top_semantic_ranks: Vec::new(),
            top_lexical_scores: Vec::new(),
            top_semantic_scores: Vec::new(),
            top_anchor_matches: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

fn run_lexical(case: &EvalCase, context: &EvalExecutionContext<'_>) -> Result<EvalModeOutcome> {
    let search_started = Instant::now();
    let results = context.backend.search_project(
        &context.project.id,
        context.store_path,
        context.wiki_root,
        &case.query,
        &SearchFilters::default(),
        context.limit,
    )?;
    let lexical_search_ms = elapsed_ms(search_started);
    let mut outcome = outcome_from_results(
        case,
        "lexical",
        None,
        results,
        lexical_judgment_applies(case),
    );
    outcome.lexical_search_ms = lexical_search_ms;
    Ok(outcome)
}

fn run_semantic(
    case: &EvalCase,
    context: &EvalExecutionContext<'_>,
    candidate: &PreparedCandidate,
) -> Result<EvalModeOutcome> {
    if !candidate.semantic_ready() {
        return Ok(readiness_outcome(
            "semantic",
            candidate
                .report
                .readiness_reason
                .as_deref()
                .unwrap_or("candidate_semantic_not_ready"),
        ));
    }
    let status =
        context
            .backend
            .status(&context.project.id, context.store_path, context.wiki_root)?;
    let metadata = candidate
        .metadata
        .as_ref()
        .expect("semantic_ready metadata");
    let vectors = candidate.vectors.as_ref().expect("semantic_ready vectors");
    let thresholds = candidate
        .thresholds
        .as_ref()
        .expect("semantic_ready thresholds");
    let embedding_artifact = candidate
        .embedding_artifact
        .as_ref()
        .expect("semantic_ready embedding");
    let embed_started = Instant::now();
    let query_embedding = embed_query(
        &case.query,
        embedding_artifact,
        metadata.embedding_dimensions,
    )?;
    let embed_query_ms = elapsed_ms(embed_started);
    let search_started = Instant::now();
    let results = vectors.search(
        metadata,
        SemanticSearchContext {
            project_id: &context.project.id,
            project_name: Some(&context.project.name),
            wiki_root: context.wiki_root,
            query_embedding: &query_embedding,
            filters: &SearchFilters::default(),
            limit: context.limit,
            floor: thresholds.semantic_similarity_floor,
            freshness: freshness_for_status(&status),
            mode: SearchMode::Semantic,
        },
    )?;
    let vector_search_ms = elapsed_ms(search_started);
    let mut outcome = outcome_from_results(case, "semantic", None, results, true);
    outcome.embed_query_ms = embed_query_ms;
    outcome.vector_search_ms = vector_search_ms;
    Ok(outcome)
}

fn run_hybrid(
    case: &EvalCase,
    context: &EvalExecutionContext<'_>,
    candidate: &PreparedCandidate,
) -> Result<EvalModeOutcome> {
    if !candidate.hybrid_ready() {
        return Ok(readiness_outcome(
            "hybrid",
            candidate
                .report
                .readiness_reason
                .as_deref()
                .unwrap_or("candidate_hybrid_not_ready"),
        ));
    }
    let status =
        context
            .backend
            .status(&context.project.id, context.store_path, context.wiki_root)?;
    let metadata = candidate.metadata.as_ref().expect("hybrid_ready metadata");
    let vectors = candidate.vectors.as_ref().expect("hybrid_ready vectors");
    let thresholds = candidate
        .thresholds
        .as_ref()
        .expect("hybrid_ready thresholds");
    let embedding_artifact = candidate
        .embedding_artifact
        .as_ref()
        .expect("hybrid_ready embedding");
    let artifacts = candidate
        .artifacts
        .as_ref()
        .expect("hybrid_ready artifacts");
    let expansion_started = Instant::now();
    let expanded = expand_hybrid_queries(&case.query, &candidate.spec.profile, artifacts)?;
    let query_expansion_ms = elapsed_ms(expansion_started);
    let per_branch_limit = if context.limit == 0 {
        0
    } else {
        context.limit.max(20)
    };
    let lexical_started = Instant::now();
    let mut lexical_results = Vec::new();
    for query in &expanded.lexical {
        lexical_results.extend(context.backend.search_project(
            &context.project.id,
            context.store_path,
            context.wiki_root,
            query,
            &SearchFilters::default(),
            per_branch_limit,
        )?);
    }
    let lexical_search_ms = elapsed_ms(lexical_started);
    let lexical_results = dedupe_by_path_preserving_rank(lexical_results);
    let mut semantic_results = Vec::new();
    let mut embed_query_ms = 0;
    let mut vector_search_ms = 0;
    for query in &expanded.semantic {
        let embed_started = Instant::now();
        let query_embedding =
            embed_query(query, embedding_artifact, metadata.embedding_dimensions)?;
        embed_query_ms += elapsed_ms(embed_started);
        let search_started = Instant::now();
        semantic_results.extend(vectors.search(
            metadata,
            SemanticSearchContext {
                project_id: &context.project.id,
                project_name: Some(&context.project.name),
                wiki_root: context.wiki_root,
                query_embedding: &query_embedding,
                filters: &SearchFilters::default(),
                limit: per_branch_limit,
                floor: thresholds.hybrid_pre_fusion_semantic_floor,
                freshness: freshness_for_status(&status),
                mode: SearchMode::Semantic,
            },
        )?);
        vector_search_ms += elapsed_ms(search_started);
    }
    let semantic_results = dedupe_by_path_preserving_rank(semantic_results);
    let mut results = fuse_hybrid_results(
        &case.query,
        thresholds,
        lexical_results,
        semantic_results,
        context.limit,
    );
    if context.rerank && candidate.spec.profile.reranker_model.is_none() {
        return Ok(readiness_outcome("hybrid", "reranker_model_missing"));
    }
    let mut rerank_ms = 0;
    let mut rerank_applied = false;
    if context.rerank {
        if let Some(reason) = candidate.rerank_readiness_reason() {
            return Ok(readiness_outcome("hybrid", &reason));
        }
        let rerank_started = Instant::now();
        results = maybe_rerank_results(
            &case.query,
            results,
            RerankInputs {
                profile: &candidate.spec.profile,
                artifacts,
                accepted_licenses: candidate
                    .accepted_licenses
                    .as_ref()
                    .expect("rerank readiness accepted licenses"),
                thresholds,
                wiki_root: context.wiki_root,
                requested: true,
            },
        )?
        .results;
        rerank_ms = elapsed_ms(rerank_started);
        rerank_applied = true;
    }
    for result in &mut results {
        result.mode = SearchMode::Hybrid;
        if matches!(result.freshness, Freshness::Unknown) {
            result.freshness = freshness_for_status(&status);
        }
    }
    let mut outcome = outcome_from_results(case, "hybrid", None, results, true);
    outcome.query_expansion_ms = query_expansion_ms;
    outcome.embed_query_ms = embed_query_ms;
    outcome.vector_search_ms = vector_search_ms;
    outcome.lexical_search_ms = lexical_search_ms;
    outcome.rerank_ms = rerank_ms;
    outcome.rerank_applied = rerank_applied;
    Ok(outcome)
}

fn readiness_outcome(mode: &str, reason: &str) -> EvalModeOutcome {
    EvalModeOutcome {
        status: "error".to_string(),
        reason: "readiness_failure".to_string(),
        selected_mode: Some(mode.to_string()),
        readiness_reason: Some(reason.to_string()),
        elapsed_seconds: 0.0,
        query_expansion_ms: 0,
        embed_query_ms: 0,
        vector_search_ms: 0,
        lexical_search_ms: 0,
        rerank_ms: 0,
        rerank_applied: false,
        result_count: 0,
        hit_rank: None,
        top_paths: Vec::new(),
        top_scores: Vec::new(),
        top_lexical_ranks: Vec::new(),
        top_semantic_ranks: Vec::new(),
        top_lexical_scores: Vec::new(),
        top_semantic_scores: Vec::new(),
        top_anchor_matches: Vec::new(),
        error: None,
    }
}

fn not_applicable_outcome(mode: &str, reason: &str) -> EvalModeOutcome {
    EvalModeOutcome {
        status: "not_applicable".to_string(),
        reason: reason.to_string(),
        selected_mode: Some(mode.to_string()),
        readiness_reason: None,
        elapsed_seconds: 0.0,
        query_expansion_ms: 0,
        embed_query_ms: 0,
        vector_search_ms: 0,
        lexical_search_ms: 0,
        rerank_ms: 0,
        rerank_applied: false,
        result_count: 0,
        hit_rank: None,
        top_paths: Vec::new(),
        top_scores: Vec::new(),
        top_lexical_ranks: Vec::new(),
        top_semantic_ranks: Vec::new(),
        top_lexical_scores: Vec::new(),
        top_semantic_scores: Vec::new(),
        top_anchor_matches: Vec::new(),
        error: None,
    }
}

fn outcome_from_results(
    case: &EvalCase,
    selected_mode: &str,
    readiness_reason: Option<String>,
    results: Vec<crate::search::adapter::SearchResult>,
    judgment_applies: bool,
) -> EvalModeOutcome {
    let top_paths = results
        .iter()
        .take(10)
        .map(|result| result.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let top_scores = results
        .iter()
        .take(10)
        .map(|result| round_score(result.score))
        .collect::<Vec<_>>();
    let top_lexical_ranks = results
        .iter()
        .take(10)
        .map(|result| result.lexical_rank)
        .collect::<Vec<_>>();
    let top_semantic_ranks = results
        .iter()
        .take(10)
        .map(|result| result.semantic_rank)
        .collect::<Vec<_>>();
    let top_lexical_scores = results
        .iter()
        .take(10)
        .map(|result| result.lexical_score.map(round_score))
        .collect::<Vec<_>>();
    let top_semantic_scores = results
        .iter()
        .take(10)
        .map(|result| result.semantic_score.map(round_score))
        .collect::<Vec<_>>();
    let top_anchor_matches = results
        .iter()
        .take(10)
        .map(|result| result_anchor_match_count(&case.query, result))
        .collect::<Vec<_>>();
    let (status, reason, hit_rank) = judge_case(case, selected_mode, &top_paths, judgment_applies);
    EvalModeOutcome {
        status,
        reason,
        selected_mode: Some(selected_mode.to_string()),
        readiness_reason,
        elapsed_seconds: 0.0,
        query_expansion_ms: 0,
        embed_query_ms: 0,
        vector_search_ms: 0,
        lexical_search_ms: 0,
        rerank_ms: 0,
        rerank_applied: false,
        result_count: results.len(),
        hit_rank,
        top_paths,
        top_scores,
        top_lexical_ranks,
        top_semantic_ranks,
        top_lexical_scores,
        top_semantic_scores,
        top_anchor_matches,
        error: None,
    }
}

fn judge_case(
    case: &EvalCase,
    selected_mode: &str,
    top_paths: &[String],
    judgment_applies: bool,
) -> (String, String, Option<usize>) {
    judge_expected_pages(
        &case.expected_pages,
        selected_mode,
        top_paths,
        judgment_applies,
    )
}

fn judge_expected_pages(
    expected_pages: &[String],
    selected_mode: &str,
    top_paths: &[String],
    judgment_applies: bool,
) -> (String, String, Option<usize>) {
    if !judgment_applies {
        return (
            "not_applicable".to_string(),
            "lexical_no_match_floor_not_defined".to_string(),
            None,
        );
    }
    if expected_pages.is_empty() {
        return if top_paths.is_empty() {
            (
                "pass".to_string(),
                "no_expected_match_zero_results".to_string(),
                None,
            )
        } else {
            (
                "fail".to_string(),
                "no_expected_match_returned_results".to_string(),
                None,
            )
        };
    }
    let top_k = if selected_mode == "hybrid" { 5 } else { 10 };
    let hit_rank = expected_hit_rank(expected_pages, top_paths, top_k);
    if hit_rank.is_some() {
        (
            "pass".to_string(),
            format!("expected_target_in_top_{top_k}"),
            hit_rank,
        )
    } else {
        (
            "fail".to_string(),
            format!("expected_target_missing_top_{top_k}"),
            None,
        )
    }
}

fn expected_hit_rank(
    expected_pages: &[String],
    top_paths: &[String],
    top_k: usize,
) -> Option<usize> {
    top_paths
        .iter()
        .take(top_k)
        .position(|path| expected_pages.iter().any(|expected| expected == path))
        .map(|index| index + 1)
}

fn lexical_judgment_applies(case: &EvalCase) -> bool {
    !case.expected_pages.is_empty()
}

fn candidate_specs(
    args: &EvalRunArgs,
    paths: &Paths,
    project: &RegisteredProject,
) -> Result<Vec<CandidateSpec>> {
    let mut specs = Vec::new();
    let has_explicit_bundle = args.embedding_model.is_some()
        || args.query_expansion_model.is_some()
        || args.reranker_model.is_some();

    if args.candidate_profiles.is_empty() && !has_explicit_bundle {
        let profile = active_project_profile(paths, project)?
            .context("active project search profile is missing; pass --candidate-profile or explicit model ids")?;
        let name = args
            .candidate_name
            .clone()
            .or_else(|| profile.profile.clone())
            .unwrap_or_else(|| "active-project".to_string());
        specs.push(CandidateSpec {
            name,
            source: "active_project_profile".to_string(),
            profile,
        });
    }

    for profile_id in &args.candidate_profiles {
        let profile = profile_by_id(profile_id)
            .with_context(|| format!("unknown candidate profile {profile_id}"))?;
        specs.push(CandidateSpec {
            name: args
                .candidate_name
                .clone()
                .filter(|_| args.candidate_profiles.len() == 1 && !has_explicit_bundle)
                .unwrap_or_else(|| profile.id.to_string()),
            source: "catalog_profile".to_string(),
            profile: SearchProfile::enabled(
                profile.id,
                profile.embedding_model,
                Some(profile.query_expansion_model.to_string()),
                profile.reranker_model.map(ToString::to_string),
            ),
        });
    }

    if has_explicit_bundle {
        let embedding_model = args
            .embedding_model
            .as_deref()
            .context("--embedding-model is required for an explicit eval candidate bundle")?;
        let query_expansion_model = args
            .query_expansion_model
            .as_deref()
            .context("--query-expansion-model is required for an explicit eval candidate bundle")?;
        validate_model_role(embedding_model, "embedding")?;
        validate_model_role(query_expansion_model, "query-expansion")?;
        if let Some(reranker_model) = &args.reranker_model {
            validate_model_role(reranker_model, "reranker")?;
        }
        let name = args
            .candidate_name
            .clone()
            .unwrap_or_else(|| format!("explicit-{}", slugify(embedding_model)));
        specs.push(CandidateSpec {
            name,
            source: "explicit_model_bundle".to_string(),
            profile: SearchProfile::enabled(
                "explicit",
                embedding_model,
                Some(query_expansion_model.to_string()),
                args.reranker_model.clone(),
            ),
        });
    }

    ensure_unique_candidate_names(&mut specs);
    Ok(specs)
}

fn validate_model_role(model_id: &str, expected_role: &str) -> Result<()> {
    let model =
        model_by_id(model_id).with_context(|| format!("unknown search model {model_id}"))?;
    if model.role.label() != expected_role {
        bail!(
            "model {model_id} has role {}; expected {expected_role}",
            model.role.label()
        );
    }
    Ok(())
}

fn ensure_unique_candidate_names(specs: &mut [CandidateSpec]) {
    let mut seen = BTreeMap::<String, usize>::new();
    for spec in specs {
        let base = slugify(&spec.name);
        let count = seen.entry(base.clone()).or_default();
        *count += 1;
        spec.name = if *count == 1 {
            base
        } else {
            format!("{base}-{count}")
        };
    }
}

fn active_project_profile(
    paths: &Paths,
    project: &RegisteredProject,
) -> Result<Option<SearchProfile>> {
    let project_search_config = project.root.join(".llm_wiki/search.toml");
    if let Some(config) = ProjectSearchConfig::read(&project_search_config)? {
        return Ok(Some(config.project));
    }
    Ok(SearchConfig::read(&paths.search_config())?.map(|config| config.project_default))
}

fn parse_eval_page(path: &Path) -> Result<Vec<EvalCase>> {
    let input =
        fs::read_to_string(path).with_context(|| format!("read eval page {}", path.display()))?;
    let mut cases = Vec::new();
    for line in input.lines() {
        if !(line.starts_with("| C") || line.starts_with("| H")) {
            continue;
        }
        if let Some(case) =
            parse_eval_row(line).with_context(|| format!("parse eval row in {}", path.display()))?
        {
            cases.push(case);
        }
    }
    if cases.is_empty() {
        bail!(
            "eval page {} did not contain any C*/H* query rows",
            path.display()
        );
    }
    Ok(cases)
}

fn parse_eval_row(line: &str) -> Result<Option<EvalCase>> {
    let columns = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();
    let id = columns.first().copied().unwrap_or_default();
    if !(id.starts_with('C') || id.starts_with('H')) {
        return Ok(None);
    }
    if columns.len() != 5 && columns.len() != 6 {
        bail!(
            "eval row `{id}` has {} columns; expected 5 (id, split, query, purpose, expected) or 6 (id, split, query, purpose, modes, expected)",
            columns.len()
        );
    }
    let split = normalize_split(columns[1]).with_context(|| format!("eval row `{id}`"))?;
    let (applicable_modes, expected_column) = if columns.len() == 6 {
        (
            parse_mode_list(columns[4]).with_context(|| format!("eval row `{id}`"))?,
            columns[5],
        )
    } else {
        (parse_mode_list("all")?, columns[4])
    };
    let expected_pages = if expected_column.eq_ignore_ascii_case("none") {
        Vec::new()
    } else {
        backtick_values(expected_column)
    };
    Ok(Some(EvalCase {
        id: id.to_string(),
        split: split.to_string(),
        query: columns[2].trim_matches('`').to_string(),
        purpose: columns[3].to_string(),
        applicable_modes,
        expected_pages,
    }))
}

fn normalize_split(raw: &str) -> Result<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "calibration" => Ok("Calibration"),
        "hold-out" | "holdout" => Ok("Hold-out"),
        other => bail!("unrecognized eval split `{other}`; expected `Calibration` or `Hold-out`"),
    }
}

fn parse_mode_list(input: &str) -> Result<BTreeSet<String>> {
    if input.eq_ignore_ascii_case("all") {
        return Ok(MODES.iter().map(|mode| (*mode).to_string()).collect());
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
    // An empty mode cell keeps the historical "applies to every mode" meaning.
    if values.is_empty() {
        return Ok(MODES.iter().map(|mode| (*mode).to_string()).collect());
    }
    let mut modes = BTreeSet::new();
    for value in values {
        let normalized = value.to_ascii_lowercase();
        if !MODES.contains(&normalized.as_str()) {
            bail!(
                "unknown eval mode `{value}`; expected one of lexical, semantic, hybrid, auto, or all"
            );
        }
        modes.insert(normalized);
    }
    Ok(modes)
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

fn summarize_candidate(cases: &[EvalCaseRun]) -> BTreeMap<String, EvalModeSummary> {
    let mut summary = BTreeMap::<String, EvalModeSummary>::new();
    for case in cases {
        for (mode, outcome) in &case.modes {
            let entry = summary.entry(mode.clone()).or_insert(EvalModeSummary {
                pass: 0,
                fail: 0,
                not_applicable: 0,
                error: 0,
            });
            match outcome.status.as_str() {
                "pass" => entry.pass += 1,
                "fail" => entry.fail += 1,
                "not_applicable" => entry.not_applicable += 1,
                _ => entry.error += 1,
            }
        }
    }
    summary
}

fn calibrate_candidate(candidate: &EvalCandidateRun) -> CandidateCalibrationProposal {
    let semantic = derive_floor(candidate, "semantic", CalibrationScoreMetric::ResultScore);
    let hybrid = derive_floor(
        candidate,
        "hybrid",
        CalibrationScoreMetric::SemanticBranchScore,
    );
    let hybrid_final = derive_hybrid_final_floors(candidate);
    let proposed_thresholds = proposal_thresholds(
        candidate,
        semantic.floor,
        hybrid.floor,
        hybrid_final.final_semantic_floor,
        hybrid_final.semantic_only_floor,
        hybrid_final.strong_lexical_score_floor,
    );
    let diagnostics = proposal_diagnostics(candidate, proposed_thresholds.as_ref());
    let calibration_no_match_queries = calibration_no_match_count(candidate);
    let missing_expected_targets = semantic
        .missing_expected_targets
        .iter()
        .chain(hybrid.missing_expected_targets.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let promotable = calibration_no_match_queries > 0
        && missing_expected_targets.is_empty()
        && semantic.promotable
        && diagnostics.proposed_calibration_pass
        && diagnostics.holdout_pass;
    let status = if calibration_no_match_queries == 0 {
        "blocked_no_calibration_no_match".to_string()
    } else if !missing_expected_targets.is_empty() {
        "blocked_missing_expected_targets".to_string()
    } else if !semantic.promotable {
        "blocked_semantic_no_match_floor".to_string()
    } else if !diagnostics.proposed_calibration_pass {
        "blocked_proposed_regression".to_string()
    } else if !diagnostics.holdout_pass {
        "blocked_holdout_regression".to_string()
    } else {
        "promotable".to_string()
    };
    let embedding_model = candidate
        .models
        .iter()
        .find(|model| model.role == "embedding");
    let model_artifact_size_bytes = candidate
        .models
        .iter()
        .filter_map(|model| model.artifact_size_bytes)
        .sum();
    let candidate_index_size_bytes = if candidate.candidate_index_size_bytes > 0 {
        candidate.candidate_index_size_bytes
    } else {
        dir_size_bytes(&candidate.candidate_index_dir).unwrap_or(0)
    };
    CandidateCalibrationProposal {
        candidate_name: candidate.name.clone(),
        profile: candidate.profile.clone(),
        embedding_model: candidate.embedding_model.clone(),
        embedding_artifact_sha256: embedding_model.and_then(|model| model.artifact_sha256.clone()),
        embedding_dimensions: embedding_model.and_then(|model| model.dimensions),
        index_fingerprint: candidate.index_fingerprint.clone(),
        model_artifact_size_bytes,
        candidate_index_size_bytes,
        semantic_similarity_floor: semantic.floor,
        semantic_min_expected_score: semantic.min_expected_score,
        semantic_max_no_match_score: semantic.max_no_match_score,
        semantic_missing_expected_targets: semantic.missing_expected_targets.clone(),
        semantic_promotable: semantic.promotable,
        hybrid_pre_fusion_semantic_floor: hybrid.floor,
        hybrid_min_expected_score: hybrid.min_expected_score,
        hybrid_max_no_match_score: hybrid.max_no_match_score,
        hybrid_missing_expected_targets: hybrid.missing_expected_targets.clone(),
        hybrid_promotable: hybrid.promotable,
        hybrid_final_semantic_floor: hybrid_final.final_semantic_floor,
        hybrid_semantic_only_floor: hybrid_final.semantic_only_floor,
        hybrid_strong_lexical_score_floor: hybrid_final.strong_lexical_score_floor,
        hybrid_max_no_match_lexical_score: hybrid_final.max_no_match_lexical_score,
        calibration_expected_queries: semantic.expected_queries.max(hybrid.expected_queries),
        calibration_no_match_queries,
        missing_expected_targets,
        status,
        promotable,
        post_apply_validation_required: promotable,
        requires_apply_profile: candidate.source != "active_project_profile",
        current_summary: candidate.summary.clone(),
        proposed_summary: diagnostics.proposed_summary,
        holdout_summary: diagnostics.holdout_summary,
        proposed_calibration_pass: diagnostics.proposed_calibration_pass,
        holdout_pass: diagnostics.holdout_pass,
        verdict_changes: diagnostics.verdict_changes,
        no_match_precision: diagnostics.no_match_precision,
        exact_identifier_preservation: diagnostics.exact_identifier_preservation,
    }
}

struct ProposalDiagnostics {
    proposed_summary: BTreeMap<String, EvalModeSummary>,
    holdout_summary: BTreeMap<String, EvalModeSummary>,
    proposed_calibration_pass: bool,
    holdout_pass: bool,
    verdict_changes: Vec<VerdictChange>,
    no_match_precision: BTreeMap<String, EvalProposalMetricSummary>,
    exact_identifier_preservation: BTreeMap<String, EvalProposalMetricSummary>,
}

struct ProposedModeEvaluation {
    status: String,
    reason: String,
    hit_rank: Option<usize>,
    top_paths: Vec<String>,
}

fn proposal_thresholds(
    candidate: &EvalCandidateRun,
    semantic_floor: Option<f64>,
    hybrid_floor: Option<f64>,
    hybrid_final_semantic_floor: f64,
    hybrid_semantic_only_floor: f64,
    hybrid_strong_lexical_score_floor: f64,
) -> Option<SearchThresholds> {
    let embedding_model = candidate.embedding_model.clone()?;
    let embedding_artifact = candidate
        .models
        .iter()
        .find(|model| model.role == "embedding")?
        .artifact_sha256
        .clone()?;
    let embedding_dimensions = candidate
        .models
        .iter()
        .find(|model| model.role == "embedding")?
        .dimensions?;
    let mut thresholds = SearchThresholds::calibrated(
        candidate
            .profile
            .clone()
            .unwrap_or_else(|| candidate.name.clone()),
        embedding_model,
        embedding_artifact,
        embedding_dimensions,
        CHUNKING_STRATEGY,
        semantic_floor?,
        hybrid_floor?,
    );
    thresholds.hybrid_final_semantic_floor = hybrid_final_semantic_floor;
    thresholds.hybrid_semantic_only_floor = hybrid_semantic_only_floor;
    thresholds.hybrid_strong_lexical_score_floor = hybrid_strong_lexical_score_floor;
    Some(thresholds)
}

struct HybridFinalFloors {
    final_semantic_floor: f64,
    semantic_only_floor: f64,
    strong_lexical_score_floor: f64,
    max_no_match_lexical_score: f64,
}

fn derive_hybrid_final_floors(candidate: &EvalCandidateRun) -> HybridFinalFloors {
    let mut max_no_match_sem: f64 = 0.0;
    let mut max_no_match_lex: f64 = 0.0;
    let mut max_anchor_leak_sem: f64 = 0.0;
    for case in &candidate.results {
        if !case.expected_pages.is_empty() {
            continue;
        }
        let Some(outcome) = case.modes.get("hybrid") else {
            continue;
        };
        if outcome.status == "not_applicable" {
            continue;
        }
        for index in 0..outcome.top_paths.len() {
            if let Some(score) = optional_value_at(&outcome.top_semantic_scores, index)
                && score > max_no_match_sem
            {
                max_no_match_sem = score;
            }
            if let Some(score) = optional_value_at(&outcome.top_lexical_scores, index)
                && score > max_no_match_lex
            {
                max_no_match_lex = score;
            }
        }
    }
    let semantic_only_floor = (max_no_match_sem + 0.005).max(DEFAULT_HYBRID_SEMANTIC_ONLY_FLOOR);
    let strong_lexical_score_floor = (max_no_match_lex + 0.5).max(0.5);

    for case in &candidate.results {
        if !case.expected_pages.is_empty() {
            continue;
        }
        let Some(outcome) = case.modes.get("hybrid") else {
            continue;
        };
        if outcome.status == "not_applicable" {
            continue;
        }
        for index in 0..outcome.top_paths.len() {
            let Some(semantic_score) = optional_value_at(&outcome.top_semantic_scores, index)
            else {
                continue;
            };
            if semantic_score < DEFAULT_HYBRID_FINAL_SEMANTIC_FLOOR
                || semantic_score >= semantic_only_floor
            {
                continue;
            }
            let lexical_score = optional_value_at(&outcome.top_lexical_scores, index);
            if lexical_score.is_some_and(|score| score >= strong_lexical_score_floor) {
                continue;
            }
            if optional_value_at(&outcome.top_lexical_ranks, index).is_none()
                && anchor_match_count(case, outcome, index) > 0
                && semantic_score > max_anchor_leak_sem
            {
                max_anchor_leak_sem = semantic_score;
            }
        }
    }
    let final_semantic_floor = if max_anchor_leak_sem > 0.0 {
        (max_anchor_leak_sem + 0.005).max(DEFAULT_HYBRID_FINAL_SEMANTIC_FLOOR)
    } else {
        DEFAULT_HYBRID_FINAL_SEMANTIC_FLOOR
    };
    HybridFinalFloors {
        final_semantic_floor,
        semantic_only_floor,
        strong_lexical_score_floor,
        max_no_match_lexical_score: max_no_match_lex,
    }
}

const DEFAULT_HYBRID_FINAL_SEMANTIC_FLOOR: f64 = 0.39;
const DEFAULT_HYBRID_SEMANTIC_ONLY_FLOOR: f64 = 0.50;

fn proposal_diagnostics(
    candidate: &EvalCandidateRun,
    thresholds: Option<&SearchThresholds>,
) -> ProposalDiagnostics {
    let mut proposed_summary = BTreeMap::new();
    let mut holdout_summary = BTreeMap::new();
    let mut verdict_changes = Vec::new();
    let mut no_match_precision = BTreeMap::new();
    let mut exact_identifier_preservation = BTreeMap::new();
    let mut proposed_calibration_pass = thresholds.is_some();
    let mut holdout_pass = thresholds.is_some();

    for case in &candidate.results {
        for mode in MODES {
            let Some(outcome) = case.modes.get(mode) else {
                continue;
            };
            let proposed = proposed_mode_evaluation(case, mode, outcome, thresholds);
            record_summary_status(&mut proposed_summary, mode, &proposed.status);
            if case.split == "Hold-out" {
                record_summary_status(&mut holdout_summary, mode, &proposed.status);
            }
            if promotion_gate_mode(mode) && proposed.status != "not_applicable" {
                if case.split == "Calibration" && proposed.status != "pass" {
                    proposed_calibration_pass = false;
                }
                if case.split == "Hold-out" && proposed.status != "pass" {
                    holdout_pass = false;
                }
            }
            if outcome.status != proposed.status
                || outcome.reason != proposed.reason
                || outcome.hit_rank != proposed.hit_rank
            {
                verdict_changes.push(VerdictChange {
                    id: case.id.clone(),
                    split: case.split.clone(),
                    mode: mode.to_string(),
                    current_status: outcome.status.clone(),
                    current_reason: outcome.reason.clone(),
                    current_hit_rank: outcome.hit_rank,
                    proposed_status: proposed.status.clone(),
                    proposed_reason: proposed.reason.clone(),
                    proposed_hit_rank: proposed.hit_rank,
                });
            }
            record_no_match_precision(&mut no_match_precision, case, mode, outcome, &proposed);
            record_exact_identifier_preservation(
                &mut exact_identifier_preservation,
                case,
                mode,
                outcome,
                &proposed,
            );
        }
    }

    ProposalDiagnostics {
        proposed_summary,
        holdout_summary,
        proposed_calibration_pass,
        holdout_pass,
        verdict_changes,
        no_match_precision,
        exact_identifier_preservation,
    }
}

fn proposed_mode_evaluation(
    case: &EvalCaseRun,
    mode: &str,
    outcome: &EvalModeOutcome,
    thresholds: Option<&SearchThresholds>,
) -> ProposedModeEvaluation {
    if outcome.status == "error" || outcome.status == "not_applicable" {
        return ProposedModeEvaluation {
            status: outcome.status.clone(),
            reason: outcome.reason.clone(),
            hit_rank: outcome.hit_rank,
            top_paths: outcome.top_paths.clone(),
        };
    }
    let selected_mode = outcome.selected_mode.as_deref().unwrap_or(mode);
    let top_paths = proposed_top_paths(case, selected_mode, outcome, thresholds);
    let judgment_applies = selected_mode != "lexical" || !case.expected_pages.is_empty();
    let (status, reason, hit_rank) = judge_expected_pages(
        &case.expected_pages,
        selected_mode,
        &top_paths,
        judgment_applies,
    );
    ProposedModeEvaluation {
        status,
        reason,
        hit_rank,
        top_paths,
    }
}

fn proposed_top_paths(
    case: &EvalCaseRun,
    selected_mode: &str,
    outcome: &EvalModeOutcome,
    thresholds: Option<&SearchThresholds>,
) -> Vec<String> {
    let Some(thresholds) = thresholds else {
        return outcome.top_paths.clone();
    };
    match selected_mode {
        "semantic" => outcome
            .top_paths
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                outcome
                    .top_scores
                    .get(*index)
                    .is_some_and(|score| *score >= thresholds.semantic_similarity_floor)
            })
            .map(|(_, path)| path.clone())
            .collect(),
        "hybrid" => outcome
            .top_paths
            .iter()
            .enumerate()
            .filter(|(index, _)| hybrid_result_survives_proposed(case, outcome, *index, thresholds))
            .map(|(_, path)| path.clone())
            .collect(),
        _ => outcome.top_paths.clone(),
    }
}

fn hybrid_result_survives_proposed(
    case: &EvalCaseRun,
    outcome: &EvalModeOutcome,
    index: usize,
    thresholds: &SearchThresholds,
) -> bool {
    let lexical_rank = optional_value_at(&outcome.top_lexical_ranks, index);
    let lexical_score = optional_value_at(&outcome.top_lexical_scores, index);
    let semantic_score = optional_value_at(&outcome.top_semantic_scores, index);

    if query_has_exact_identifier(&case.query)
        && thresholds.lexical_exact_identifier_guard == "preserve_lexical_top_3"
        && lexical_rank.is_some_and(|rank| rank < 3)
    {
        return true;
    }
    if lexical_score.is_some_and(|score| score >= thresholds.hybrid_strong_lexical_score_floor) {
        return true;
    }
    let Some(semantic_score) = semantic_score else {
        return false;
    };
    if semantic_score < thresholds.hybrid_pre_fusion_semantic_floor {
        return false;
    }
    if semantic_score >= thresholds.hybrid_semantic_only_floor {
        return true;
    }
    if semantic_score < thresholds.hybrid_final_semantic_floor {
        return false;
    }
    lexical_rank.is_some() || anchor_match_count(case, outcome, index) > 0
}

fn optional_value_at<T: Copy>(values: &[Option<T>], index: usize) -> Option<T> {
    values.get(index).and_then(|value| *value)
}

fn anchor_match_count(case: &EvalCaseRun, outcome: &EvalModeOutcome, index: usize) -> usize {
    outcome
        .top_anchor_matches
        .get(index)
        .copied()
        .unwrap_or_else(|| path_anchor_match_count(&case.query, &outcome.top_paths[index]))
}

fn result_anchor_match_count(query: &str, result: &crate::search::adapter::SearchResult) -> usize {
    let haystack = format!(
        "{} {} {}",
        result.path.to_string_lossy().to_ascii_lowercase(),
        result.title.to_ascii_lowercase(),
        result
            .snippet
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
    );
    query_anchor_terms(query)
        .iter()
        .filter(|anchor| haystack.contains(anchor.as_str()))
        .count()
}

fn path_anchor_match_count(query: &str, path: &str) -> usize {
    let path = path.to_ascii_lowercase();
    query_anchor_terms(query)
        .iter()
        .filter(|anchor| path.contains(anchor.as_str()))
        .count()
}

fn record_summary_status(
    summary: &mut BTreeMap<String, EvalModeSummary>,
    mode: &str,
    status: &str,
) {
    let entry = summary.entry(mode.to_string()).or_default();
    match status {
        "pass" => entry.pass += 1,
        "fail" => entry.fail += 1,
        "not_applicable" => entry.not_applicable += 1,
        _ => entry.error += 1,
    }
}

fn promotion_gate_mode(mode: &str) -> bool {
    matches!(mode, "hybrid" | "auto")
}

fn record_no_match_precision(
    metrics: &mut BTreeMap<String, EvalProposalMetricSummary>,
    case: &EvalCaseRun,
    mode: &str,
    outcome: &EvalModeOutcome,
    proposed: &ProposedModeEvaluation,
) {
    if !case.expected_pages.is_empty() {
        return;
    }
    let entry = metrics.entry(mode.to_string()).or_default();
    if metric_countable(&outcome.status) {
        entry.current_total += 1;
        if outcome.status == "pass" {
            entry.current_pass += 1;
        }
    }
    if metric_countable(&proposed.status) {
        entry.proposed_total += 1;
        if proposed.status == "pass" {
            entry.proposed_pass += 1;
        }
    }
}

fn record_exact_identifier_preservation(
    metrics: &mut BTreeMap<String, EvalProposalMetricSummary>,
    case: &EvalCaseRun,
    mode: &str,
    outcome: &EvalModeOutcome,
    proposed: &ProposedModeEvaluation,
) {
    if case.expected_pages.is_empty() || !query_has_exact_identifier(&case.query) {
        return;
    }
    let Some(top_k) = exact_identifier_top_k(mode, outcome.selected_mode.as_deref()) else {
        return;
    };
    let entry = metrics.entry(mode.to_string()).or_default();
    if metric_countable(&outcome.status) {
        entry.current_total += 1;
        if expected_hit_rank(&case.expected_pages, &outcome.top_paths, top_k).is_some() {
            entry.current_pass += 1;
        }
    }
    if metric_countable(&proposed.status) {
        entry.proposed_total += 1;
        if expected_hit_rank(&case.expected_pages, &proposed.top_paths, top_k).is_some() {
            entry.proposed_pass += 1;
        }
    }
}

fn metric_countable(status: &str) -> bool {
    status != "not_applicable" && status != "error"
}

fn exact_identifier_top_k(mode: &str, selected_mode: Option<&str>) -> Option<usize> {
    match selected_mode.unwrap_or(mode) {
        "lexical" => Some(3),
        "hybrid" => Some(5),
        _ => None,
    }
}

fn query_has_exact_identifier(query: &str) -> bool {
    query.split_whitespace().any(|term| {
        term.contains("--")
            || term.contains('/')
            || term.contains('.')
            || term.contains('_')
            || term.chars().any(|ch| ch.is_ascii_digit())
    })
}

struct DerivedFloor {
    floor: Option<f64>,
    min_expected_score: Option<f64>,
    max_no_match_score: Option<f64>,
    expected_queries: usize,
    missing_expected_targets: Vec<String>,
    promotable: bool,
}

#[derive(Clone, Copy)]
enum CalibrationScoreMetric {
    ResultScore,
    SemanticBranchScore,
}

fn derive_floor(
    candidate: &EvalCandidateRun,
    mode: &str,
    metric: CalibrationScoreMetric,
) -> DerivedFloor {
    let mut expected_scores = Vec::new();
    let mut no_match_scores = Vec::new();
    let mut missing_expected_targets = Vec::new();
    for case in candidate
        .results
        .iter()
        .filter(|case| case.split == "Calibration")
    {
        let Some(outcome) = case.modes.get(mode) else {
            continue;
        };
        if outcome.status == "not_applicable" {
            continue;
        }
        if case.expected_pages.is_empty() {
            if let Some(score) = outcome_score_samples(outcome, metric)
                .into_iter()
                .reduce(f64::max)
            {
                no_match_scores.push(score);
            }
            continue;
        }
        if outcome.status != "pass" {
            missing_expected_targets.push(case.id.clone());
            continue;
        }
        let expected_score = outcome
            .top_paths
            .iter()
            .enumerate()
            .find_map(|(index, path)| {
                case.expected_pages
                    .iter()
                    .any(|expected| expected == path)
                    .then(|| outcome_score_at(outcome, index, metric))
                    .flatten()
            });
        if let Some(score) = expected_score {
            expected_scores.push(score);
        } else {
            missing_expected_targets.push(case.id.clone());
        }
    }

    let min_expected_score = expected_scores.iter().copied().reduce(f64::min);
    let max_no_match_score = no_match_scores.iter().copied().reduce(f64::max);
    let floor = min_expected_score.map(round_floor);
    let feasible = match (
        floor,
        max_no_match_score,
        missing_expected_targets.is_empty(),
    ) {
        (Some(min_expected), Some(max_no_match), true) => max_no_match < min_expected,
        (Some(_), None, true) => false,
        _ => false,
    };
    DerivedFloor {
        floor,
        min_expected_score,
        max_no_match_score,
        expected_queries: expected_scores.len() + missing_expected_targets.len(),
        missing_expected_targets,
        promotable: feasible,
    }
}

fn outcome_score_samples(outcome: &EvalModeOutcome, metric: CalibrationScoreMetric) -> Vec<f64> {
    match metric {
        CalibrationScoreMetric::ResultScore => outcome.top_scores.clone(),
        CalibrationScoreMetric::SemanticBranchScore => outcome
            .top_semantic_scores
            .iter()
            .filter_map(|score| *score)
            .collect(),
    }
}

fn outcome_score_at(
    outcome: &EvalModeOutcome,
    index: usize,
    metric: CalibrationScoreMetric,
) -> Option<f64> {
    match metric {
        CalibrationScoreMetric::ResultScore => outcome.top_scores.get(index).copied(),
        CalibrationScoreMetric::SemanticBranchScore => outcome
            .top_semantic_scores
            .get(index)
            .and_then(|score| *score),
    }
}

fn round_floor(value: f64) -> f64 {
    // Expected scores are stored round-half-up to 6 dp, so a stored value can
    // sit up to 5e-7 above the true unrounded score. Subtract that half-ULP
    // before flooring so the recorded floor is guaranteed `<=` the true
    // expected score and never filters out the very hit that justified it.
    (value * 1_000_000.0 - 0.5).floor() / 1_000_000.0
}

fn calibration_no_match_count(candidate: &EvalCandidateRun) -> usize {
    candidate
        .results
        .iter()
        .filter(|case| case.split == "Calibration" && case.expected_pages.is_empty())
        .count()
}

fn apply_thresholds(
    args: &EvalCalibrateArgs,
    run_report: &EvalRunReport,
    proposal: &CandidateCalibrationProposal,
) -> Result<()> {
    let paths = Paths::from_env()?;
    let candidate = run_report
        .candidates
        .iter()
        .find(|candidate| candidate.name == proposal.candidate_name)
        .context("selected calibration candidate missing from run report")?;
    let mut thresholds = SearchThresholds::calibrated(
        candidate
            .profile
            .clone()
            .unwrap_or_else(|| candidate.name.clone()),
        candidate
            .embedding_model
            .clone()
            .context("candidate embedding model missing")?,
        proposal
            .embedding_artifact_sha256
            .clone()
            .context("candidate embedding artifact hash missing")?,
        proposal
            .embedding_dimensions
            .context("candidate embedding dimensions missing")?,
        CHUNKING_STRATEGY,
        proposal
            .semantic_similarity_floor
            .context("semantic floor missing from proposal")?,
        proposal
            .hybrid_pre_fusion_semantic_floor
            .context("hybrid floor missing from proposal")?,
    );
    thresholds.hybrid_final_semantic_floor = proposal.hybrid_final_semantic_floor;
    thresholds.hybrid_semantic_only_floor = proposal.hybrid_semantic_only_floor;
    thresholds.hybrid_strong_lexical_score_floor = proposal.hybrid_strong_lexical_score_floor;
    thresholds = thresholds.for_project(run_report.project_id.clone());
    let path = paths.search_thresholds();
    let mut store = SearchThresholdStore::read(&path)?.unwrap_or_else(SearchThresholdStore::empty);
    store.upsert(thresholds);
    if path.exists() {
        let stamp = slugify(&timestamp());
        let mut backup = path.with_file_name(format!("search-thresholds.toml.backup-{stamp}"));
        let mut counter = 2;
        while backup.exists() {
            backup =
                path.with_file_name(format!("search-thresholds.toml.backup-{stamp}-{counter}"));
            counter += 1;
        }
        fs::copy(&path, &backup).with_context(|| {
            format!(
                "backup previous search thresholds {} to {}",
                path.display(),
                backup.display()
            )
        })?;
    }
    store.write_atomic(&path)?;
    if let Some(scope) = &args.apply_profile {
        let marker = paths
            .managed_home()
            .join(format!("last-applied-eval-profile-{scope}.txt"));
        fs::write(
            &marker,
            format!(
                "candidate={}\nsource_run_id={}\napplied_at={}\n",
                proposal.candidate_name,
                run_report.run_id,
                timestamp()
            ),
        )
        .with_context(|| format!("write apply profile marker {}", marker.display()))?;
    }
    Ok(())
}

fn record_calibration(
    args: &EvalCalibrateArgs,
    run_report: &EvalRunReport,
    report: &EvalCalibrationReport,
    proposal: &CandidateCalibrationProposal,
) -> Result<()> {
    let eval_page = &run_report.eval_page;
    let before = fs::read(eval_page).with_context(|| format!("read {}", eval_page.display()))?;
    let before_hash = sha256_bytes(&before);
    let mut file = OpenOptions::new()
        .append(true)
        .open(eval_page)
        .with_context(|| format!("open eval page for append {}", eval_page.display()))?;
    writeln!(
        file,
        "\n### {} Eval Calibration Run {}\n\n- Source run: `{}`\n- Report: `{}`\n- Pre-record source fingerprint: `{}`\n- Index stale warning: run `llm-wiki index --force` after this wiki mutation.\n",
        Utc::now().date_naive(),
        report.source_run_id,
        report.source_run_id,
        report.report_path.display(),
        before_hash
    )
    .with_context(|| format!("append calibration run to {}", eval_page.display()))?;
    writeln!(
        file,
        "- Candidate: `{}`\n- Status: `{}`\n- Promotable: `{}`\n- Post-apply validation required: `{}`\n- Proposed `semantic_similarity_floor`: `{}`\n- Proposed `hybrid_pre_fusion_semantic_floor`: `{}`\n- Current summary: {}\n- Proposed summary: {}\n- Hold-out summary: {}\n- Verdict changes: `{}`\n- Model artifact bytes: `{}`\n- Candidate index bytes: `{}`\n",
        proposal.candidate_name,
        proposal.status,
        proposal.promotable,
        proposal.post_apply_validation_required,
        optional_floor(proposal.semantic_similarity_floor),
        optional_floor(proposal.hybrid_pre_fusion_semantic_floor),
        summary_inline(&proposal.current_summary),
        summary_inline(&proposal.proposed_summary),
        summary_inline(&proposal.holdout_summary),
        proposal.verdict_changes.len(),
        proposal.model_artifact_size_bytes,
        proposal.candidate_index_size_bytes,
    )
    .with_context(|| format!("append calibration proposal to {}", eval_page.display()))?;
    if args.apply {
        writeln!(file, "- Applied: yes\n").with_context(|| {
            format!("append calibration apply status to {}", eval_page.display())
        })?;
    }
    Ok(())
}

fn raw_data_bundle_paths(
    args: &EvalCalibrateArgs,
    run_report: &EvalRunReport,
) -> Result<BTreeMap<String, PathBuf>> {
    let root = raw_data_root(args, run_report)?;
    let run_root = unique_raw_run_root(&root.join(&run_report.run_id));
    Ok(run_report
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.name.clone(),
                run_root.join(slugify(&candidate.name)),
            )
        })
        .collect())
}

fn raw_data_root(args: &EvalCalibrateArgs, run_report: &EvalRunReport) -> Result<PathBuf> {
    if let Some(path) = &args.raw_data_dir {
        return Ok(if path.is_absolute() {
            path.clone()
        } else {
            env::current_dir()
                .context("read current directory for raw eval data dir")?
                .join(path)
        });
    }
    Ok(env::current_dir()
        .context("read current directory for raw eval data root")?
        .join("raw/data/eval")
        .join(corpus_slug(run_report)))
}

fn unique_raw_run_root(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    for suffix in 2.. {
        let path = base.with_file_name(format!(
            "{}-{suffix}",
            base.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("run")
        ));
        if !path.exists() {
            return path;
        }
    }
    unreachable!("raw eval data run suffix loop is unbounded")
}

fn corpus_slug(run_report: &EvalRunReport) -> String {
    let cwd = env::current_dir().ok();
    let project_inside_cwd = cwd
        .as_ref()
        .is_some_and(|cwd| run_report.project_root.starts_with(cwd));
    if project_inside_cwd
        && let Some(stem) = run_report
            .eval_page
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.trim_end_matches(".md").trim_end_matches(".eval"))
            .filter(|value| !value.is_empty())
    {
        return slugify(stem);
    }
    slugify(&run_report.project_name)
}

fn export_raw_eval_data(
    run_report: &EvalRunReport,
    report: &EvalCalibrationReport,
    context: &CliContext,
) -> Result<()> {
    if report.raw_data_bundles.is_empty() {
        bail!("raw data export requested but report has no raw_data_bundles");
    }
    for (candidate_name, bundle) in &report.raw_data_bundles {
        if bundle.exists() {
            bail!(
                "raw eval data bundle already exists: {}; raw data is immutable",
                bundle.display()
            );
        }
        fs::create_dir_all(bundle)
            .with_context(|| format!("create raw eval data bundle {}", bundle.display()))?;
        let run_file = bundle.join("eval-run.json");
        let calibration_file = bundle.join("eval-calibration.json");
        write_json(&run_file, run_report)?;
        write_json(&calibration_file, report)?;
        let files = vec![
            raw_manifest_file("eval-run.json", &run_file)?,
            raw_manifest_file("eval-calibration.json", &calibration_file)?,
        ];
        let candidates = report
            .proposals
            .iter()
            .filter(|proposal| proposal.candidate_name == *candidate_name)
            .map(raw_manifest_candidate)
            .collect::<Vec<_>>();
        let manifest = RawEvalDataManifest {
            schema_version: RAW_EVAL_DATA_MANIFEST_SCHEMA_VERSION,
            exported_at: timestamp(),
            source_run_id: report.source_run_id.clone(),
            project_id: report.project_id.clone(),
            project_name: report.project_name.clone(),
            eval_page: redacted_path_string(&report.eval_page),
            source_run_report: redacted_path_string(&run_report.report_path),
            source_calibration_report: redacted_path_string(&report.report_path),
            selected_candidate: Some(candidate_name.clone()),
            candidates,
            files,
        };
        let manifest_path = bundle.join("manifest.toml");
        let manifest_toml =
            toml::to_string_pretty(&manifest).context("serialize raw eval manifest")?;
        fs::write(&manifest_path, manifest_toml)
            .with_context(|| format!("write raw eval manifest {}", manifest_path.display()))?;
        context.diagnostic(format!(
            "raw eval data bundle candidate={candidate_name}: {}",
            bundle.display()
        ));
    }
    Ok(())
}

fn raw_manifest_candidate(proposal: &CandidateCalibrationProposal) -> RawEvalDataCandidate {
    RawEvalDataCandidate {
        name: proposal.candidate_name.clone(),
        status: proposal.status.clone(),
        promotable: proposal.promotable,
        semantic_similarity_floor: proposal.semantic_similarity_floor,
        hybrid_pre_fusion_semantic_floor: proposal.hybrid_pre_fusion_semantic_floor,
        proposed_calibration_pass: proposal.proposed_calibration_pass,
        holdout_pass: proposal.holdout_pass,
        verdict_changes: proposal.verdict_changes.len(),
        post_apply_validation_required: proposal.post_apply_validation_required,
    }
}

fn raw_manifest_file(name: &str, path: &Path) -> Result<RawEvalDataFile> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(RawEvalDataFile {
        name: name.to_string(),
        sha256: sha256_bytes(&bytes),
        size_bytes: bytes.len() as u64,
    })
}

fn optional_floor(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "none".to_string())
}

fn summary_inline(summary: &BTreeMap<String, EvalModeSummary>) -> String {
    if summary.is_empty() {
        return "`none`".to_string();
    }
    summary
        .iter()
        .map(|(mode, summary)| {
            format!(
                "`{} {}/{}/{}/{}`",
                mode, summary.pass, summary.fail, summary.not_applicable, summary.error
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_run_report(report: &EvalRunReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputFormat::Text => {
            println!("Eval run: {}", report.run_id);
            println!("Project: {} ({})", report.project_id, report.project_name);
            println!("Queries: {}", report.query_count);
            for candidate in &report.candidates {
                println!("Candidate: {}", candidate.name);
                for mode in MODES {
                    if let Some(summary) = candidate.summary.get(mode) {
                        println!(
                            "  {mode}: pass={} fail={} n/a={} error={}",
                            summary.pass, summary.fail, summary.not_applicable, summary.error
                        );
                    }
                }
            }
            println!("Report: {}", report.report_path.display());
            println!("Summary: {}", report.summary_path.display());
        }
    }
    Ok(())
}

fn print_calibration_report(report: &EvalCalibrationReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputFormat::Text => {
            println!("Calibration report: {}", report.source_run_id);
            for proposal in &report.proposals {
                println!(
                    "Candidate: {} status={} promotable={}",
                    proposal.candidate_name, proposal.status, proposal.promotable
                );
                println!(
                    "  post_apply_validation_required={}",
                    proposal.post_apply_validation_required
                );
                if let Some(floor) = proposal.semantic_similarity_floor {
                    println!("  semantic_similarity_floor={floor:.6}");
                }
                if let Some(score) = proposal.semantic_min_expected_score {
                    println!("  semantic_min_expected_score={score:.6}");
                }
                if let Some(score) = proposal.semantic_max_no_match_score {
                    println!("  semantic_max_no_match_score={score:.6}");
                }
                if let Some(floor) = proposal.hybrid_pre_fusion_semantic_floor {
                    println!("  hybrid_pre_fusion_semantic_floor={floor:.6}");
                }
                if let Some(score) = proposal.hybrid_min_expected_score {
                    println!("  hybrid_min_expected_score={score:.6}");
                }
                if let Some(score) = proposal.hybrid_max_no_match_score {
                    println!("  hybrid_max_no_match_score={score:.6}");
                }
                println!(
                    "  proposed_calibration_pass={} holdout_pass={}",
                    proposal.proposed_calibration_pass, proposal.holdout_pass
                );
                println!(
                    "  model_artifact_size_bytes={} candidate_index_size_bytes={}",
                    proposal.model_artifact_size_bytes, proposal.candidate_index_size_bytes
                );
                println!("  verdict_changes={}", proposal.verdict_changes.len());
                if !proposal.missing_expected_targets.is_empty() {
                    println!(
                        "  blocked_expected_targets={}",
                        proposal.missing_expected_targets.join(",")
                    );
                }
                if proposal.calibration_no_match_queries == 0 {
                    println!("  blocked: calibration split has no no-match query");
                }
            }
            println!("Report: {}", report.report_path.display());
            if let Some(bundle) = &report.raw_data_bundle {
                println!("Raw data bundle: {}", bundle.display());
            }
        }
    }
    Ok(())
}

fn render_run_summary(report: &EvalRunReport) -> String {
    let mut output = String::new();
    output.push_str("# Search Eval Run Summary\n\n");
    output.push_str(&format!("- Run: `{}`\n", report.run_id));
    output.push_str(&format!("- Project: `{}`\n", report.project_id));
    output.push_str(&format!("- Queries: `{}`\n\n", report.query_count));
    output.push_str("| Candidate | Mode | Pass | Fail | N/A | Error |\n");
    output.push_str("| --- | --- | ---: | ---: | ---: | ---: |\n");
    for candidate in &report.candidates {
        for mode in MODES {
            if let Some(summary) = candidate.summary.get(mode) {
                output.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    candidate.name,
                    mode,
                    summary.pass,
                    summary.fail,
                    summary.not_applicable,
                    summary.error
                ));
            }
        }
    }
    output
}

fn read_run_report(path: &Path) -> Result<EvalRunReport> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("read eval run report {}", path.display()))?;
    let mut report: EvalRunReport =
        serde_json::from_str(&input).with_context(|| format!("parse {}", path.display()))?;
    if report.schema_version != EVAL_REPORT_SCHEMA_VERSION {
        bail!(
            "unsupported eval run report schema_version {}; expected {}",
            report.schema_version,
            EVAL_REPORT_SCHEMA_VERSION
        );
    }
    // Paths are serialized in a lossy, redacted form (`~/…`, `./…`). Rehydrate
    // them so consumers (`record_calibration`, `calibration_report_path`,
    // `dir_size_bytes`, …) resolve against the right location instead of the
    // caller's current working directory, which may differ from where the run
    // report was produced.
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    report.eval_page = rehydrate_report_path(&report.eval_page, base);
    report.project_root = rehydrate_report_path(&report.project_root, base);
    report.wiki_root = rehydrate_report_path(&report.wiki_root, base);
    report.output_dir = rehydrate_report_path(&report.output_dir, base);
    report.report_path = rehydrate_report_path(&report.report_path, base);
    report.summary_path = rehydrate_report_path(&report.summary_path, base);
    for candidate in &mut report.candidates {
        candidate.candidate_index_dir = rehydrate_report_path(&candidate.candidate_index_dir, base);
    }
    Ok(report)
}

/// Reconstruct an absolute path from a redacted serialized path.
///
/// A leading `~` is expanded to `$HOME` (an exact round-trip), and other
/// relative paths are resolved against `base` (the run report's own directory)
/// rather than the process working directory.
fn rehydrate_report_path(raw: &Path, base: &Path) -> PathBuf {
    let text = raw.to_string_lossy();
    if text == "~"
        && let Some(home) = env::var_os("HOME").map(PathBuf::from)
    {
        return home;
    }
    if let Some(rest) = text.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME").map(PathBuf::from)
    {
        return home.join(rest);
    }
    if raw.is_absolute() {
        return raw.to_path_buf();
    }
    let relative = text.strip_prefix("./").map(Path::new).unwrap_or(raw);
    base.join(relative)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("json path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create temp json in {}", parent.display()))?;
    serde_json::to_writer_pretty(&mut temp, value).context("serialize json report")?;
    temp.write_all(b"\n").context("write trailing newline")?;
    temp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("persist json report {}", path.display()))?;
    Ok(())
}

fn calibration_report_path(args: &EvalCalibrateArgs, run_report: &EvalRunReport) -> PathBuf {
    args.run
        .output_dir
        .clone()
        .unwrap_or_else(|| run_report.output_dir.clone())
        .join("eval-calibration.json")
}

fn resolve_eval_project(
    args: &EvalRunArgs,
    paths: &Paths,
    _context: &CliContext,
) -> Result<RegisteredProject> {
    if let Some(project_root) = &args.project_root {
        return local_eval_project(project_root);
    }
    if let Some(project_id) = args.project.as_deref() {
        eprintln!(
            "warning: `llm-wiki eval run --project <id>` is deprecated; use `--project-root <path>`"
        );
        let registry = ProjectRegistry::read(&paths.project_registry())?;
        return registry
            .project_by_id(project_id)
            .cloned()
            .with_context(|| format!("project id {project_id} is not registered"));
    }
    let discovered =
        discover_from_cwd()?.context("not inside a wiki project; pass --project-root <path>")?;
    local_eval_project(&discovered.project_root)
}

fn local_eval_project(root: &Path) -> Result<RegisteredProject> {
    let root = fs::canonicalize(root)
        .with_context(|| format!("canonicalize eval project root {}", root.display()))?;
    let wiki_root = root.join("wiki");
    if !wiki_root.join("index.md").is_file() {
        bail!(
            "eval project root {} is missing wiki/index.md",
            root.display()
        );
    }
    if !wiki_root.join("log.md").is_file() {
        bail!(
            "eval project root {} is missing wiki/log.md",
            root.display()
        );
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "project".to_string());
    Ok(RegisteredProject {
        id: slugify(&name),
        name,
        root,
        wiki_path: PathBuf::from("wiki"),
        registered_at: timestamp(),
        last_indexed_at: None,
        last_indexed_wiki_max_mtime: None,
        indexed_file_count: 0,
        backend: "qmd-rs".to_string(),
        index_schema_version: 1,
    })
}

fn ensure_project_root_exists(project: &RegisteredProject) -> Result<()> {
    if !project.root.exists() {
        bail!(
            "project root missing for {}; run `llm-wiki projects` or `llm-wiki forget {}`",
            project.id,
            project.id
        );
    }
    Ok(())
}

fn ensure_base_install(paths: &Paths) -> Result<()> {
    if Manifest::read(&paths.manifest())?.is_none() {
        bail!(
            "llm-wiki install is required before eval; run `llm-wiki install` or `llm-wiki install --configure-search`"
        );
    }
    Ok(())
}

fn run_id() -> String {
    format!("{}-{}", Utc::now().format("%Y%m%dT%H%M%SZ"), process::id())
}

fn round_seconds(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn warn_time_budget(budget_ms: Option<u64>, label: &str, elapsed_ms: u64) {
    if budget_ms.is_some_and(|budget| elapsed_ms > budget) {
        eprintln!("warning: eval timing budget exceeded: {label} elapsed={elapsed_ms}ms");
    }
}

fn warn_mode_time_budget(
    budget_ms: Option<u64>,
    candidate: &str,
    case_id: &str,
    mode: &str,
    outcome: &EvalModeOutcome,
) {
    warn_time_budget(
        budget_ms,
        &format!("candidate={candidate} case={case_id} mode={mode} stage=query_expansion"),
        outcome.query_expansion_ms,
    );
    warn_time_budget(
        budget_ms,
        &format!("candidate={candidate} case={case_id} mode={mode} stage=embed_query"),
        outcome.embed_query_ms,
    );
    warn_time_budget(
        budget_ms,
        &format!("candidate={candidate} case={case_id} mode={mode} stage=vector_search"),
        outcome.vector_search_ms,
    );
    warn_time_budget(
        budget_ms,
        &format!("candidate={candidate} case={case_id} mode={mode} stage=lexical_search"),
        outcome.lexical_search_ms,
    );
    warn_time_budget(
        budget_ms,
        &format!("candidate={candidate} case={case_id} mode={mode} stage=rerank"),
        outcome.rerank_ms,
    );
}

fn candidate_timing_summary(report: &EvalCandidateRun) -> String {
    let mut mode_ms = BTreeMap::<&str, u64>::new();
    for case in &report.results {
        for (mode, outcome) in &case.modes {
            *mode_ms.entry(mode.as_str()).or_default() +=
                (outcome.elapsed_seconds * 1000.0).round() as u64;
        }
    }
    format!(
        "candidate={} index_build={}ms lexical_index={}ms semantic_metadata={}ms semantic_vector={}ms lexical={}ms semantic={}ms hybrid={}ms auto={}ms",
        report.name,
        report.index_build_ms,
        report.lexical_index_ms,
        report.semantic_metadata_ms,
        report.semantic_vector_build_ms,
        mode_ms.get("lexical").copied().unwrap_or(0),
        mode_ms.get("semantic").copied().unwrap_or(0),
        mode_ms.get("hybrid").copied().unwrap_or(0),
        mode_ms.get("auto").copied().unwrap_or(0),
    )
}

fn round_score(score: Score) -> f64 {
    (score.0 * 1_000_000.0).round() / 1_000_000.0
}

fn serialize_redacted_path<S>(path: &Path, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&redacted_path_string(path))
}

fn serialize_optional_redacted_path<S>(
    path: &Option<PathBuf>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match path {
        Some(path) => serializer.serialize_some(&redacted_path_string(path)),
        None => serializer.serialize_none(),
    }
}

fn serialize_redacted_path_map<S>(
    paths: &BTreeMap<String, PathBuf>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let redacted = paths
        .iter()
        .map(|(key, path)| (key, redacted_path_string(path)))
        .collect::<BTreeMap<_, _>>();
    redacted.serialize(serializer)
}

fn redacted_path_string(path: &Path) -> String {
    let normalized = path_string(path);
    let prefixes = path_redaction_prefixes();
    for (prefix, replacement) in prefixes {
        if let Ok(stripped) = path.strip_prefix(&prefix) {
            return join_redacted_path(replacement, stripped);
        }
        let prefix_string = path_string(&prefix);
        if normalized == prefix_string {
            return replacement.to_string();
        }
        if let Some(rest) = normalized.strip_prefix(&(prefix_string + "/")) {
            return format!("{replacement}/{rest}");
        }
    }
    // No home-relative anchor matched. Keep the path verbatim — absolute paths
    // stay absolute so they reconstruct exactly on read. A lossy `./…` form here
    // would silently resolve against the wrong base when a run report is consumed
    // from a different working directory (e.g. `eval calibrate --run-report`
    // pointed at a report produced elsewhere), corrupting the wrong project.
    normalized
}

fn path_redaction_prefixes() -> Vec<(PathBuf, &'static str)> {
    let mut prefixes = Vec::new();
    // Only redact against $HOME: it is the single anchor that reconstructs
    // exactly on read (via `~` expansion). The working directory at write time
    // is not recorded in the report, so a cwd-relative form could not be
    // resolved correctly when the report is read from elsewhere.
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        prefixes.push((home.clone(), "~"));
        if let Ok(canonical) = fs::canonicalize(&home)
            && canonical != home
        {
            prefixes.push((canonical, "~"));
        }
    }
    prefixes.sort_by(|left, right| {
        right
            .0
            .components()
            .count()
            .cmp(&left.0.components().count())
    });
    prefixes
}

fn join_redacted_path(prefix: &str, stripped: &Path) -> String {
    let stripped = path_string(stripped);
    if stripped.is_empty() || stripped == "." {
        prefix.to_string()
    } else {
        format!("{prefix}/{stripped}")
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "candidate".to_string()
    } else {
        output
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn dir_size_bytes(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0;
    for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
        let entry = entry.with_context(|| format!("read dir entry in {}", path.display()))?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("stat {}", entry.path().display()))?;
        if metadata.is_dir() {
            total += dir_size_bytes(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_eval_row_extracts_expected_pages() {
        let row = "| C1 | Calibration | `what happens` | Purpose | `wiki/a.md`, `wiki/b.md` |";
        let case = parse_eval_row(row).expect("row").expect("case");

        assert_eq!(case.id, "C1");
        assert_eq!(case.split, "Calibration");
        assert_eq!(case.query, "what happens");
        assert!(case.applies_to_mode("semantic"));
        assert_eq!(case.expected_pages, vec!["wiki/a.md", "wiki/b.md"]);
    }

    #[test]
    fn parse_eval_row_extracts_mode_applicability() {
        let row = "| C1 | Calibration | `what happens` | Purpose | `lexical`, `hybrid`, `auto` | `wiki/a.md` |";
        let case = parse_eval_row(row).expect("row").expect("case");

        assert!(case.applies_to_mode("lexical"));
        assert!(!case.applies_to_mode("semantic"));
        assert!(case.applies_to_mode("hybrid"));
        assert!(case.applies_to_mode("auto"));
    }

    #[test]
    fn parse_eval_row_rejects_extra_columns() {
        let row = "| C1 | Calibration | `what happens` | Purpose | all | `wiki/a.md` | notes |";
        let error = parse_eval_row(row).expect_err("extra column must fail");
        assert!(
            error.to_string().contains("7 columns"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parse_eval_row_rejects_unknown_mode_token() {
        let row = "| C1 | Calibration | `what happens` | Purpose | `semnatic` | `wiki/a.md` |";
        let error = parse_eval_row(row).expect_err("unknown mode must fail");
        assert!(
            format!("{error:#}").contains("unknown eval mode"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn parse_eval_row_normalizes_split_label_case() {
        let row = "| H1 | holdout | `what happens` | Purpose | all | `wiki/a.md` |";
        let case = parse_eval_row(row).expect("row").expect("case");
        assert_eq!(case.split, "Hold-out");
    }

    #[test]
    fn parse_eval_row_rejects_unknown_split_label() {
        let row = "| C1 | Bogus | `what happens` | Purpose | all | `wiki/a.md` |";
        let error = parse_eval_row(row).expect_err("unknown split must fail");
        assert!(
            format!("{error:#}").contains("split"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn eval_report_serialization_redacts_absolute_path_prefixes() {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        // Anchor the project under $HOME so redaction is deterministic regardless
        // of where the test runs from.
        let project = home.join("proj");
        let report = EvalRunReport {
            schema_version: EVAL_REPORT_SCHEMA_VERSION,
            run_id: "run".to_string(),
            generated_at: "2026-05-12T00:00:00Z".to_string(),
            eval_page: project.join("wiki/evals/test.eval.md"),
            project_id: "project".to_string(),
            project_name: "Project".to_string(),
            project_root: project.clone(),
            wiki_root: project.join("wiki"),
            query_count: 0,
            limit: 10,
            rerank_requested: false,
            output_dir: project.join("target/evals/run"),
            report_path: project.join("target/evals/run/eval-run.json"),
            summary_path: project.join("target/evals/run/eval-run-summary.md"),
            candidates: vec![EvalCandidateRun {
                name: "balanced".to_string(),
                source: "catalog_profile".to_string(),
                profile: Some("balanced".to_string()),
                embedding_model: Some("embeddinggemma-300m-q8_0".to_string()),
                query_expansion_model: Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
                reranker_model: None,
                models: vec![EvalCandidateModel {
                    role: "embedding".to_string(),
                    model_id: "embeddinggemma-300m-q8_0".to_string(),
                    repository: None,
                    revision: None,
                    artifact_sha256: Some("sha".to_string()),
                    artifact_size_bytes: Some(1),
                    dimensions: Some(768),
                    accepted_license: true,
                    artifact_path: Some(home.join(".llm_wiki/models/model.gguf")),
                    readiness_reason: None,
                }],
                readiness_reason: None,
                index_fingerprint: Some("fingerprint".to_string()),
                vector_count: 1,
                candidate_index_dir: project.join("target/evals/run/indexes/balanced"),
                candidate_index_size_bytes: 1,
                index_build_ms: 0,
                lexical_index_ms: 0,
                semantic_metadata_ms: 0,
                semantic_vector_build_ms: 0,
                elapsed_seconds: 0.0,
                summary: BTreeMap::new(),
                results: Vec::new(),
            }],
        };

        let json = serde_json::to_string(&report).expect("report json");

        // Home-anchored paths redact to `~/…` and no absolute home path leaks.
        assert!(!json.contains("/Users/"));
        assert!(!json.contains("/home/"));
        assert!(json.contains("~/proj/wiki/evals/test.eval.md"));
        assert!(json.contains("~/.llm_wiki/models/model.gguf"));
    }

    #[test]
    fn out_of_home_absolute_paths_round_trip_through_redaction() {
        // A report produced from a project outside $HOME must serialize paths
        // verbatim (not a lossy `./…` form) so `eval calibrate --run-report`
        // reconstructs the exact eval page rather than a wrong sibling path.
        let outside = PathBuf::from("/opt/data/example-project/wiki/evals/x.eval.md");
        let redacted = redacted_path_string(&outside);
        assert_eq!(redacted, path_string(&outside));

        let unrelated_report_dir = PathBuf::from("/var/reports/run");
        assert_eq!(
            rehydrate_report_path(Path::new(&redacted), &unrelated_report_dir),
            outside
        );
    }

    #[test]
    fn calibration_blocks_without_calibration_no_match() {
        let mut modes = BTreeMap::new();
        modes.insert(
            "semantic".to_string(),
            EvalModeOutcome {
                status: "pass".to_string(),
                reason: "expected_target_in_top_10".to_string(),
                selected_mode: Some("semantic".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 1,
                hit_rank: Some(1),
                top_paths: vec!["wiki/a.md".to_string()],
                top_scores: vec![0.82],
                top_lexical_ranks: vec![None],
                top_semantic_ranks: vec![Some(0)],
                top_lexical_scores: vec![None],
                top_semantic_scores: vec![Some(0.82)],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "pass".to_string(),
                reason: "expected_target_in_top_5".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 1,
                hit_rank: Some(1),
                top_paths: vec!["wiki/a.md".to_string()],
                top_scores: vec![0.12],
                top_lexical_ranks: vec![Some(0)],
                top_semantic_ranks: vec![Some(0)],
                top_lexical_scores: vec![Some(2.0)],
                top_semantic_scores: vec![Some(0.62)],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        let candidate = EvalCandidateRun {
            name: "balanced".to_string(),
            source: "active_project_profile".to_string(),
            profile: Some("balanced".to_string()),
            embedding_model: Some("embeddinggemma-300m-q8_0".to_string()),
            query_expansion_model: Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            reranker_model: None,
            models: Vec::new(),
            readiness_reason: None,
            index_fingerprint: Some("fingerprint".to_string()),
            vector_count: 1,
            candidate_index_dir: PathBuf::from("target/evals/run/indexes/balanced"),
            candidate_index_size_bytes: 0,
            index_build_ms: 0,
            lexical_index_ms: 0,
            semantic_metadata_ms: 0,
            semantic_vector_build_ms: 0,
            elapsed_seconds: 0.0,
            summary: BTreeMap::new(),
            results: vec![EvalCaseRun {
                id: "C1".to_string(),
                split: "Calibration".to_string(),
                query: "query".to_string(),
                purpose: "purpose".to_string(),
                expected_pages: vec!["wiki/a.md".to_string()],
                modes,
            }],
        };

        let proposal = calibrate_candidate(&candidate);

        assert_eq!(proposal.status, "blocked_no_calibration_no_match");
        assert!(!proposal.promotable);
        // `round_floor` subtracts a half-ULP guard so the recorded floor never
        // exceeds the true (unrounded) expected score of 0.82.
        assert_eq!(proposal.semantic_similarity_floor, Some(0.819999));
        assert_eq!(proposal.semantic_min_expected_score, Some(0.82));
    }

    #[test]
    fn calibration_does_not_use_failed_low_rank_expected_hits() {
        let mut expected_modes = BTreeMap::new();
        expected_modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "fail".to_string(),
                reason: "expected_target_missing_top_5".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 6,
                hit_rank: None,
                top_paths: vec![
                    "wiki/other-1.md".to_string(),
                    "wiki/other-2.md".to_string(),
                    "wiki/other-3.md".to_string(),
                    "wiki/other-4.md".to_string(),
                    "wiki/other-5.md".to_string(),
                    "wiki/a.md".to_string(),
                ],
                top_scores: vec![0.09, 0.08, 0.07, 0.06, 0.05, 0.04],
                top_lexical_ranks: vec![None, None, None, None, None, None],
                top_semantic_ranks: vec![Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)],
                top_lexical_scores: vec![None, None, None, None, None, None],
                top_semantic_scores: vec![
                    Some(0.61),
                    Some(0.60),
                    Some(0.59),
                    Some(0.58),
                    Some(0.57),
                    Some(0.56),
                ],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        let mut no_match_modes = BTreeMap::new();
        no_match_modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "fail".to_string(),
                reason: "no_expected_match_returned_results".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 1,
                hit_rank: None,
                top_paths: vec!["wiki/noise.md".to_string()],
                top_scores: vec![0.02],
                top_lexical_ranks: vec![None],
                top_semantic_ranks: vec![Some(0)],
                top_lexical_scores: vec![None],
                top_semantic_scores: vec![Some(0.55)],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        let candidate = EvalCandidateRun {
            name: "balanced".to_string(),
            source: "active_project_profile".to_string(),
            profile: Some("balanced".to_string()),
            embedding_model: Some("embeddinggemma-300m-q8_0".to_string()),
            query_expansion_model: Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            reranker_model: None,
            models: Vec::new(),
            readiness_reason: None,
            index_fingerprint: Some("fingerprint".to_string()),
            vector_count: 1,
            candidate_index_dir: PathBuf::from("target/evals/run/indexes/balanced"),
            candidate_index_size_bytes: 0,
            index_build_ms: 0,
            lexical_index_ms: 0,
            semantic_metadata_ms: 0,
            semantic_vector_build_ms: 0,
            elapsed_seconds: 0.0,
            summary: BTreeMap::new(),
            results: vec![
                EvalCaseRun {
                    id: "C1".to_string(),
                    split: "Calibration".to_string(),
                    query: "expected query".to_string(),
                    purpose: "purpose".to_string(),
                    expected_pages: vec!["wiki/a.md".to_string()],
                    modes: expected_modes,
                },
                EvalCaseRun {
                    id: "C2".to_string(),
                    split: "Calibration".to_string(),
                    query: "no match query".to_string(),
                    purpose: "purpose".to_string(),
                    expected_pages: Vec::new(),
                    modes: no_match_modes,
                },
            ],
        };

        let floor = derive_floor(
            &candidate,
            "hybrid",
            CalibrationScoreMetric::SemanticBranchScore,
        );

        assert_eq!(floor.floor, None);
        assert_eq!(floor.missing_expected_targets, vec!["C1"]);
        assert!(!floor.promotable);
    }

    #[test]
    fn calibration_preserves_narrow_six_decimal_score_separation() {
        let mut expected_modes = BTreeMap::new();
        expected_modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "pass".to_string(),
                reason: "expected_target_in_top_5".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 3,
                hit_rank: Some(3),
                top_paths: vec![
                    "wiki/other-1.md".to_string(),
                    "wiki/other-2.md".to_string(),
                    "wiki/specs/documentation-model.spec.md".to_string(),
                ],
                top_scores: vec![0.032491, 0.028161, 0.0208919],
                top_lexical_ranks: vec![None, None, None],
                top_semantic_ranks: vec![Some(0), Some(1), Some(2)],
                top_lexical_scores: vec![None, None, None],
                top_semantic_scores: vec![Some(0.032491), Some(0.028161), Some(0.0208919)],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        let mut no_match_modes = BTreeMap::new();
        no_match_modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "fail".to_string(),
                reason: "no_expected_match_returned_results".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 1,
                hit_rank: None,
                top_paths: vec!["wiki/search/noise.md".to_string()],
                top_scores: vec![0.020492],
                top_lexical_ranks: vec![None],
                top_semantic_ranks: vec![Some(0)],
                top_lexical_scores: vec![None],
                top_semantic_scores: vec![Some(0.020492)],
                top_anchor_matches: Vec::new(),
                error: None,
            },
        );
        let candidate = EvalCandidateRun {
            name: "balanced".to_string(),
            source: "active_project_profile".to_string(),
            profile: Some("balanced".to_string()),
            embedding_model: Some("embeddinggemma-300m-q8_0".to_string()),
            query_expansion_model: Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            reranker_model: None,
            models: Vec::new(),
            readiness_reason: None,
            index_fingerprint: Some("fingerprint".to_string()),
            vector_count: 1,
            candidate_index_dir: PathBuf::from("target/evals/run/indexes/balanced"),
            candidate_index_size_bytes: 0,
            index_build_ms: 0,
            lexical_index_ms: 0,
            semantic_metadata_ms: 0,
            semantic_vector_build_ms: 0,
            elapsed_seconds: 0.0,
            summary: BTreeMap::new(),
            results: vec![
                EvalCaseRun {
                    id: "C5".to_string(),
                    split: "Calibration".to_string(),
                    query: "large-index search-scale query".to_string(),
                    purpose: "purpose".to_string(),
                    expected_pages: vec!["wiki/specs/documentation-model.spec.md".to_string()],
                    modes: expected_modes,
                },
                EvalCaseRun {
                    id: "C10".to_string(),
                    split: "Calibration".to_string(),
                    query: "unrelated no-match query".to_string(),
                    purpose: "purpose".to_string(),
                    expected_pages: Vec::new(),
                    modes: no_match_modes,
                },
            ],
        };

        let floor = derive_floor(
            &candidate,
            "hybrid",
            CalibrationScoreMetric::SemanticBranchScore,
        );

        assert_eq!(floor.floor, Some(0.020891));
        assert_eq!(floor.min_expected_score, Some(0.0208919));
        assert_eq!(floor.max_no_match_score, Some(0.020492));
        assert!(floor.missing_expected_targets.is_empty());
        assert!(floor.promotable);
    }

    #[test]
    fn hybrid_final_floor_raises_above_anchor_leaking_no_match() {
        let mut no_match_modes = BTreeMap::new();
        no_match_modes.insert(
            "hybrid".to_string(),
            EvalModeOutcome {
                status: "fail".to_string(),
                reason: "no_expected_match_returned_results".to_string(),
                selected_mode: Some("hybrid".to_string()),
                readiness_reason: None,
                elapsed_seconds: 0.0,
                query_expansion_ms: 0,
                embed_query_ms: 0,
                vector_search_ms: 0,
                lexical_search_ms: 0,
                rerank_ms: 0,
                rerank_applied: false,
                result_count: 2,
                hit_rank: None,
                top_paths: vec!["wiki/log.md".to_string(), "wiki/other.md".to_string()],
                top_scores: vec![0.024161, 0.020492],
                top_lexical_ranks: vec![None, None],
                top_semantic_ranks: vec![Some(1), Some(0)],
                top_lexical_scores: vec![None, None],
                top_semantic_scores: vec![Some(0.394904), Some(0.405622)],
                top_anchor_matches: vec![1, 0],
                error: None,
            },
        );
        let candidate = EvalCandidateRun {
            name: "balanced".to_string(),
            source: "active_project_profile".to_string(),
            profile: Some("balanced".to_string()),
            embedding_model: Some("embeddinggemma-300m-q8_0".to_string()),
            query_expansion_model: Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            reranker_model: None,
            models: Vec::new(),
            readiness_reason: None,
            index_fingerprint: Some("fingerprint".to_string()),
            vector_count: 1,
            candidate_index_dir: PathBuf::from("target/evals/run/indexes/balanced"),
            candidate_index_size_bytes: 0,
            index_build_ms: 0,
            lexical_index_ms: 0,
            semantic_metadata_ms: 0,
            semantic_vector_build_ms: 0,
            elapsed_seconds: 0.0,
            summary: BTreeMap::new(),
            results: vec![EvalCaseRun {
                id: "H11".to_string(),
                split: "Hold-out".to_string(),
                query: "browser automation plugin release checklist".to_string(),
                purpose: "purpose".to_string(),
                expected_pages: Vec::new(),
                modes: no_match_modes,
            }],
        };

        let floors = derive_hybrid_final_floors(&candidate);

        assert!((floors.final_semantic_floor - 0.399904).abs() < 0.000001);
        assert_eq!(
            floors.semantic_only_floor,
            DEFAULT_HYBRID_SEMANTIC_ONLY_FLOOR
        );
        assert_eq!(floors.strong_lexical_score_floor, 0.5);
    }

    #[test]
    fn proposed_hybrid_replay_keeps_path_anchor_matches() {
        let thresholds = SearchThresholds::calibrated(
            "balanced",
            "embeddinggemma-300m-q8_0",
            "sha256",
            768,
            "qmd-rs-character-v1:3200:480",
            0.10,
            0.10,
        );
        let case = EvalCaseRun {
            id: "C1".to_string(),
            split: "Calibration".to_string(),
            query: "search profile tuning".to_string(),
            purpose: "purpose".to_string(),
            expected_pages: vec!["wiki/plans/search-profile.plan.md".to_string()],
            modes: BTreeMap::new(),
        };
        let outcome = EvalModeOutcome {
            status: "pass".to_string(),
            reason: "expected_target_in_top_5".to_string(),
            selected_mode: Some("hybrid".to_string()),
            readiness_reason: None,
            elapsed_seconds: 0.0,
            query_expansion_ms: 0,
            embed_query_ms: 0,
            vector_search_ms: 0,
            lexical_search_ms: 0,
            rerank_ms: 0,
            rerank_applied: false,
            result_count: 1,
            hit_rank: Some(1),
            top_paths: vec!["wiki/plans/search-profile.plan.md".to_string()],
            top_scores: vec![0.04],
            top_lexical_ranks: vec![None],
            top_semantic_ranks: vec![Some(0)],
            top_lexical_scores: vec![None],
            top_semantic_scores: vec![Some(0.42)],
            top_anchor_matches: Vec::new(),
            error: None,
        };

        assert!(hybrid_result_survives_proposed(
            &case,
            &outcome,
            0,
            &thresholds
        ));

        let unrelated_case = EvalCaseRun {
            query: "postgresql connection pooling".to_string(),
            ..case
        };
        assert!(!hybrid_result_survives_proposed(
            &unrelated_case,
            &outcome,
            0,
            &thresholds
        ));
    }

    #[test]
    fn proposed_hybrid_replay_uses_recorded_anchor_matches() {
        let thresholds = SearchThresholds::calibrated(
            "balanced",
            "embeddinggemma-300m-q8_0",
            "sha256",
            768,
            "qmd-rs-character-v1:3200:480",
            0.10,
            0.10,
        );
        let case = EvalCaseRun {
            id: "H11".to_string(),
            split: "Hold-out".to_string(),
            query: "browser automation plugin release checklist".to_string(),
            purpose: "purpose".to_string(),
            expected_pages: Vec::new(),
            modes: BTreeMap::new(),
        };
        let mut outcome = EvalModeOutcome {
            status: "fail".to_string(),
            reason: "unexpected_results".to_string(),
            selected_mode: Some("hybrid".to_string()),
            readiness_reason: None,
            elapsed_seconds: 0.0,
            query_expansion_ms: 0,
            embed_query_ms: 0,
            vector_search_ms: 0,
            lexical_search_ms: 0,
            rerank_ms: 0,
            rerank_applied: false,
            result_count: 1,
            hit_rank: None,
            top_paths: vec!["wiki/log.md".to_string()],
            top_scores: vec![0.04],
            top_lexical_ranks: vec![None],
            top_semantic_ranks: vec![Some(0)],
            top_lexical_scores: vec![None],
            top_semantic_scores: vec![Some(0.42)],
            top_anchor_matches: vec![1],
            error: None,
        };

        assert!(hybrid_result_survives_proposed(
            &case,
            &outcome,
            0,
            &thresholds
        ));

        outcome.top_anchor_matches = vec![0];
        assert!(!hybrid_result_survives_proposed(
            &case,
            &outcome,
            0,
            &thresholds
        ));
    }

    #[test]
    fn duplicate_candidate_names_are_stabilized() {
        let profile = SearchProfile::enabled(
            "balanced",
            "embeddinggemma-300m-q8_0",
            Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            None,
        );
        let mut specs = vec![
            CandidateSpec {
                name: "Same".to_string(),
                source: "test".to_string(),
                profile: profile.clone(),
            },
            CandidateSpec {
                name: "same".to_string(),
                source: "test".to_string(),
                profile,
            },
        ];

        ensure_unique_candidate_names(&mut specs);

        assert_eq!(specs[0].name, "same");
        assert_eq!(specs[1].name, "same-2");
    }
}
