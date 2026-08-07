#[cfg(debug_assertions)]
use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::search::gguf_runtime::{
    self, GgufRuntimeBackend, GgufRuntimeError, GgufRuntimeErrorKind, GgufRuntimeReport,
    GgufRuntimeRole, GgufRuntimeStage,
};
use crate::search_models::{
    ADAPTER_SCHEMA_VERSION, ModelArtifactRecord, ModelArtifacts, ModelRole, ProfileBundle,
    QMD_RS_VERSION, SearchModel, model_by_id,
};
use crate::search_profile::{SearchProfile, timestamp, write_toml_atomic};

const RUNTIME_PROBE_SCHEMA_VERSION: u32 = 1;
#[cfg(debug_assertions)]
const TEST_RUNTIME_PROBE_ENV: &str = "LLM_WIKI_TEST_GGUF_RUNTIME_PROBE";
const PROBE_QUERY: &str = "llm wiki runtime probe";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeProbeStore {
    pub schema_version: u32,
    pub updated_at: String,
    pub binary_version: String,
    pub target_triple: String,
    pub qmd_rs_version: String,
    pub adapter_schema_version: u32,
    pub records: Vec<RuntimeProbeRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeProbeRecord {
    pub profile: String,
    pub role: String,
    #[serde(default = "default_probe_required")]
    pub required: bool,
    pub model_id: String,
    pub model_path: String,
    pub artifact_sha256: String,
    pub artifact_size_bytes: u64,
    pub requested_backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_backend: Option<String>,
    pub fallback: bool,
    pub outcome: RuntimeProbeOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub duration_ms: u64,
    pub probed_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProbeOutcome {
    Passed,
    Failed,
    Skipped,
}

impl RuntimeProbeOutcome {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProbeRun {
    pub records: Vec<RuntimeProbeRecord>,
}

impl RuntimeProbeRun {
    pub fn first_required_problem(&self) -> Option<&RuntimeProbeRecord> {
        self.records
            .iter()
            .find(|record| record.required && record.outcome != RuntimeProbeOutcome::Passed)
    }

    pub fn all_passed(&self) -> bool {
        self.records
            .iter()
            .all(|record| record.outcome == RuntimeProbeOutcome::Passed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeProbeTarget {
    pub model: SearchModel,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProbeIdentity {
    pub profile: String,
    pub role: String,
    pub model_id: String,
    pub artifact_sha256: String,
    pub requested_backend: String,
}

impl RuntimeProbeStore {
    pub fn from_records(records: Vec<RuntimeProbeRecord>) -> Self {
        Self {
            schema_version: RUNTIME_PROBE_SCHEMA_VERSION,
            updated_at: timestamp(),
            binary_version: env!("CARGO_PKG_VERSION").to_string(),
            target_triple: target_triple(),
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            records,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read runtime probes {}", path.display()))?;
        let store: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse runtime probes {}", path.display()))?;
        if store.schema_version != RUNTIME_PROBE_SCHEMA_VERSION {
            bail!(
                "unsupported runtime probe schema_version {} in {}; expected {}",
                store.schema_version,
                path.display(),
                RUNTIME_PROBE_SCHEMA_VERSION
            );
        }
        Ok(Some(store))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "runtime probes")
    }

    pub fn store_stale_reason(&self) -> Option<&'static str> {
        if self.binary_version != env!("CARGO_PKG_VERSION") {
            return Some("binary_version");
        }
        if self.target_triple != target_triple() {
            return Some("target_triple");
        }
        if self.qmd_rs_version != QMD_RS_VERSION {
            return Some("qmd_rs_version");
        }
        if self.adapter_schema_version != ADAPTER_SCHEMA_VERSION {
            return Some("adapter_schema_version");
        }
        None
    }

    pub fn record_stale_reason(
        &self,
        record: &RuntimeProbeRecord,
        identity: &RuntimeProbeIdentity,
    ) -> Option<&'static str> {
        self.store_stale_reason().or_else(|| {
            if record.profile != identity.profile {
                Some("profile")
            } else if record.role != identity.role {
                Some("role")
            } else if record.model_id != identity.model_id {
                Some("model_id")
            } else if record.artifact_sha256 != identity.artifact_sha256 {
                Some("artifact_sha256")
            } else if record.requested_backend != identity.requested_backend {
                Some("requested_backend")
            } else {
                None
            }
        })
    }

    pub fn newest_record_for(
        &self,
        identity: &RuntimeProbeIdentity,
    ) -> Option<&RuntimeProbeRecord> {
        self.records.iter().rev().find(|record| {
            record.profile == identity.profile
                && record.role == identity.role
                && record.model_id == identity.model_id
        })
    }
}

pub fn target_triple() -> String {
    match option_env!("LLM_WIKI_BUILD_TARGET") {
        Some(target) if !target.is_empty() => target.to_string(),
        _ => format!(
            "{}-{}-unknown",
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
    }
}

pub fn probe_profile_bundle(
    profile: ProfileBundle,
    artifacts: &ModelArtifacts,
) -> Result<RuntimeProbeRun> {
    let targets = probe_targets_for_profile(profile)?;
    Ok(probe_targets(profile.id, &targets, artifacts))
}

pub fn probe_search_profile(
    profile: &SearchProfile,
    artifacts: &ModelArtifacts,
) -> Result<RuntimeProbeRun> {
    let profile_id = profile
        .profile
        .as_deref()
        .context("LLM search profile is missing a profile id")?;
    let targets = probe_targets_for_search_profile(profile)?;
    Ok(probe_targets(profile_id, &targets, artifacts))
}

pub fn probe_targets_for_profile(profile: ProfileBundle) -> Result<Vec<RuntimeProbeTarget>> {
    let mut targets = Vec::new();
    targets.push(required_probe_target(profile.embedding_model)?);
    targets.push(required_probe_target(profile.query_expansion_model)?);
    if let Some(reranker_model) = profile.reranker_model {
        targets.push(advisory_probe_target(reranker_model)?);
    }
    Ok(targets)
}

pub fn probe_targets_for_search_profile(
    profile: &SearchProfile,
) -> Result<Vec<RuntimeProbeTarget>> {
    let mut targets = Vec::new();
    let embedding_model = profile
        .embedding_model
        .as_deref()
        .context("LLM search profile is missing an embedding model")?;
    targets.push(required_probe_target(embedding_model)?);
    // Query expansion is only used by hybrid profiles; embedding-only profiles
    // legitimately omit it, so probe it only when the profile declares one instead
    // of erroring.
    if let Some(query_expansion_model) = profile.query_expansion_model.as_deref() {
        targets.push(required_probe_target(query_expansion_model)?);
    }
    if let Some(reranker_model) = profile.reranker_model.as_deref() {
        targets.push(advisory_probe_target(reranker_model)?);
    }
    Ok(targets)
}

pub fn artifact_for_model<'a>(
    artifacts: &'a ModelArtifacts,
    model_id: &str,
) -> Option<&'a ModelArtifactRecord> {
    artifacts
        .artifacts
        .iter()
        .find(|artifact| artifact.model_id == model_id)
}

pub fn identity_for_model(
    profile: &str,
    model: SearchModel,
    artifact: &ModelArtifactRecord,
) -> RuntimeProbeIdentity {
    RuntimeProbeIdentity {
        profile: profile.to_string(),
        role: runtime_role_for_model(model).label().to_string(),
        model_id: model.id.to_string(),
        artifact_sha256: artifact.observed_sha256.clone(),
        requested_backend: gguf_runtime::requested_backend().label().to_string(),
    }
}

fn required_model(id: &str) -> Result<SearchModel> {
    model_by_id(id).with_context(|| format!("unknown search model id {id}"))
}

fn required_probe_target(id: &str) -> Result<RuntimeProbeTarget> {
    Ok(RuntimeProbeTarget {
        model: required_model(id)?,
        required: true,
    })
}

fn advisory_probe_target(id: &str) -> Result<RuntimeProbeTarget> {
    Ok(RuntimeProbeTarget {
        model: required_model(id)?,
        required: false,
    })
}

fn default_probe_required() -> bool {
    true
}

fn probe_targets(
    profile: &str,
    targets: &[RuntimeProbeTarget],
    artifacts: &ModelArtifacts,
) -> RuntimeProbeRun {
    let records = targets
        .iter()
        .copied()
        .map(
            |target| match artifact_for_model(artifacts, target.model.id) {
                Some(artifact) => probe_model(profile, target, artifact),
                None => skipped_record(
                    profile,
                    target,
                    None,
                    "model artifact record is missing".to_string(),
                ),
            },
        )
        .collect();
    RuntimeProbeRun { records }
}

fn probe_model(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
) -> RuntimeProbeRecord {
    let model = target.model;
    let role = runtime_role_for_model(model);
    let started = Instant::now();
    // Debug-only test seam: release binaries never consult LLM_WIKI_TEST_GGUF_RUNTIME_PROBE.
    #[cfg(debug_assertions)]
    if let Some(record) = test_probe_record(profile, target, artifact, role, started) {
        return record;
    }

    match role {
        GgufRuntimeRole::Embedding => probe_embedding(profile, target, artifact, started),
        GgufRuntimeRole::QueryExpansion => {
            probe_query_expansion(profile, target, artifact, started)
        }
        GgufRuntimeRole::Rerank => probe_rerank(profile, target, artifact, started),
    }
}

fn probe_embedding(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    started: Instant,
) -> RuntimeProbeRecord {
    let mut engine = match gguf_runtime::embedding_engine(&artifact.path) {
        Ok(engine) => engine,
        Err(error) => return failed_record(profile, target, artifact, None, &error, started),
    };
    match gguf_runtime::embed_query(&mut engine, PROBE_QUERY) {
        Ok(value) => {
            if let Some(expected) = artifact.dimensions
                && value.value.len() != expected
            {
                return failed_message_record(
                    profile,
                    target,
                    artifact,
                    FailureDetails {
                        report: Some(engine.report()),
                        stage: GgufRuntimeStage::Embedding,
                        kind: GgufRuntimeErrorKind::InferenceFailed,
                        message: format!(
                            "embedding probe returned {} dimensions; expected {expected}",
                            value.value.len()
                        ),
                    },
                    started,
                );
            }
            passed_record(profile, target, artifact, value.report, started)
        }
        Err(error) => failed_record(
            profile,
            target,
            artifact,
            Some(engine.report()),
            &error,
            started,
        ),
    }
}

fn probe_query_expansion(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    started: Instant,
) -> RuntimeProbeRecord {
    let mut engine = match gguf_runtime::generation_engine(&artifact.path) {
        Ok(engine) => engine,
        Err(error) => return failed_record(profile, target, artifact, None, &error, started),
    };
    match gguf_runtime::expand_query(&mut engine, PROBE_QUERY) {
        Ok(value) => passed_record(profile, target, artifact, value.report, started),
        Err(error) => failed_record(
            profile,
            target,
            artifact,
            Some(engine.report()),
            &error,
            started,
        ),
    }
}

fn probe_rerank(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    started: Instant,
) -> RuntimeProbeRecord {
    let mut engine = match gguf_runtime::rerank_engine(&artifact.path) {
        Ok(engine) => engine,
        Err(error) => return failed_record(profile, target, artifact, None, &error, started),
    };
    let documents = [qmd::RerankDocument {
        file: "runtime-probe.md".to_string(),
        text: "LLM Wiki runtime probe document".to_string(),
        title: Some("Runtime Probe".to_string()),
    }];
    match gguf_runtime::rerank(&mut engine, PROBE_QUERY, &documents) {
        Ok(value) => passed_record(profile, target, artifact, value.report, started),
        Err(error) => failed_record(
            profile,
            target,
            artifact,
            Some(engine.report()),
            &error,
            started,
        ),
    }
}

#[cfg(debug_assertions)]
fn test_probe_record(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    role: GgufRuntimeRole,
    started: Instant,
) -> Option<RuntimeProbeRecord> {
    let directive = env::var(TEST_RUNTIME_PROBE_ENV).ok()?;
    let directive = directive.trim().replace('-', "_");
    let role_label = role.label();
    let requested = gguf_runtime::requested_backend();
    if let Some((action, target_role)) = directive.split_once(':')
        && target_role != role_label
        && matches!(action, "pass" | "fallback" | "fail" | "skip")
    {
        return Some(passed_record(
            profile,
            target,
            artifact,
            GgufRuntimeReport::new_for_probe(requested, requested, false),
            started,
        ));
    }
    if directive == "pass" || directive == format!("pass:{role_label}") {
        return Some(passed_record(
            profile,
            target,
            artifact,
            GgufRuntimeReport::new_for_probe(requested, requested, false),
            started,
        ));
    }
    if directive == "fallback" || directive == format!("fallback:{role_label}") {
        let fallback = requested == GgufRuntimeBackend::Auto;
        return Some(passed_record(
            profile,
            target,
            artifact,
            GgufRuntimeReport::new_for_probe(requested, GgufRuntimeBackend::Cpu, fallback),
            started,
        ));
    }
    if directive == "fail" || directive == format!("fail:{role_label}") {
        return Some(failed_message_record(
            profile,
            target,
            artifact,
            FailureDetails {
                report: None,
                stage: stage_for_role(role),
                kind: GgufRuntimeErrorKind::BackendUnavailable,
                message: "forced GGUF runtime probe failure".to_string(),
            },
            started,
        ));
    }
    if directive == "skip" || directive == format!("skip:{role_label}") {
        return Some(skipped_record(
            profile,
            target,
            Some(artifact),
            "forced GGUF runtime probe skip".to_string(),
        ));
    }
    None
}

fn passed_record(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    report: GgufRuntimeReport,
    started: Instant,
) -> RuntimeProbeRecord {
    let model = target.model;
    let role = runtime_role_for_model(model);
    RuntimeProbeRecord {
        profile: profile.to_string(),
        role: role.label().to_string(),
        required: target.required,
        model_id: model.id.to_string(),
        model_path: artifact.path.display().to_string(),
        artifact_sha256: artifact.observed_sha256.clone(),
        artifact_size_bytes: artifact.size_bytes,
        requested_backend: report.requested_backend().label().to_string(),
        used_backend: Some(report.used_backend().label().to_string()),
        fallback: report.fallback(),
        outcome: RuntimeProbeOutcome::Passed,
        failure_stage: None,
        failure_kind: None,
        message: None,
        duration_ms: elapsed_ms(started),
        probed_at: timestamp(),
    }
}

fn failed_record(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    report: Option<GgufRuntimeReport>,
    error: &anyhow::Error,
    started: Instant,
) -> RuntimeProbeRecord {
    let (stage, kind, message) = match error.downcast_ref::<GgufRuntimeError>() {
        Some(error) => (
            error.stage(),
            error.kind(),
            truncate_message(error.message().to_string()),
        ),
        None => (
            stage_for_role(runtime_role_for_model(target.model)),
            GgufRuntimeErrorKind::InferenceFailed,
            truncate_message(format!("{error:#}")),
        ),
    };
    failed_message_record(
        profile,
        target,
        artifact,
        FailureDetails {
            report,
            stage,
            kind,
            message,
        },
        started,
    )
}

struct FailureDetails {
    report: Option<GgufRuntimeReport>,
    stage: GgufRuntimeStage,
    kind: GgufRuntimeErrorKind,
    message: String,
}

fn failed_message_record(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: &ModelArtifactRecord,
    details: FailureDetails,
    started: Instant,
) -> RuntimeProbeRecord {
    let model = target.model;
    let role = runtime_role_for_model(model);
    RuntimeProbeRecord {
        profile: profile.to_string(),
        role: role.label().to_string(),
        required: target.required,
        model_id: model.id.to_string(),
        model_path: artifact.path.display().to_string(),
        artifact_sha256: artifact.observed_sha256.clone(),
        artifact_size_bytes: artifact.size_bytes,
        requested_backend: gguf_runtime::requested_backend().label().to_string(),
        used_backend: details
            .report
            .as_ref()
            .map(|report| report.used_backend().label().to_string()),
        fallback: details
            .report
            .as_ref()
            .is_some_and(|report| report.fallback()),
        outcome: RuntimeProbeOutcome::Failed,
        failure_stage: Some(details.stage.label().to_string()),
        failure_kind: Some(details.kind.label().to_string()),
        message: Some(details.message),
        duration_ms: elapsed_ms(started),
        probed_at: timestamp(),
    }
}

fn skipped_record(
    profile: &str,
    target: RuntimeProbeTarget,
    artifact: Option<&ModelArtifactRecord>,
    message: String,
) -> RuntimeProbeRecord {
    let model = target.model;
    let role = runtime_role_for_model(model);
    RuntimeProbeRecord {
        profile: profile.to_string(),
        role: role.label().to_string(),
        required: target.required,
        model_id: model.id.to_string(),
        model_path: artifact
            .map(|artifact| artifact.path.display().to_string())
            .unwrap_or_default(),
        artifact_sha256: artifact
            .map(|artifact| artifact.observed_sha256.clone())
            .unwrap_or_default(),
        artifact_size_bytes: artifact.map(|artifact| artifact.size_bytes).unwrap_or(0),
        requested_backend: gguf_runtime::requested_backend().label().to_string(),
        used_backend: None,
        fallback: false,
        outcome: RuntimeProbeOutcome::Skipped,
        failure_stage: None,
        failure_kind: None,
        message: Some(message),
        duration_ms: 0,
        probed_at: timestamp(),
    }
}

fn runtime_role_for_model(model: SearchModel) -> GgufRuntimeRole {
    match model.role {
        ModelRole::Embedding => GgufRuntimeRole::Embedding,
        ModelRole::QueryExpansion => GgufRuntimeRole::QueryExpansion,
        ModelRole::Reranker => GgufRuntimeRole::Rerank,
    }
}

fn stage_for_role(role: GgufRuntimeRole) -> GgufRuntimeStage {
    match role {
        GgufRuntimeRole::Embedding => GgufRuntimeStage::Embedding,
        GgufRuntimeRole::QueryExpansion => GgufRuntimeStage::QueryExpansion,
        GgufRuntimeRole::Rerank => GgufRuntimeStage::Rerank,
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn truncate_message(message: String) -> String {
    const LIMIT: usize = 300;
    if message.len() <= LIMIT {
        return message;
    }
    let end = message
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= LIMIT)
        .last()
        .unwrap_or(0);
    format!("{}...", &message[..end])
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{
        RuntimeProbeOutcome, RuntimeProbeStore, identity_for_model, probe_profile_bundle,
        target_triple, truncate_message,
    };
    use crate::search_models::{
        BALANCED_PROFILE, EMBEDDING_GEMMA_300M, ModelArtifactRecord, ModelArtifacts, ProfileBundle,
        QMD_QUERY_EXPANSION_17B, QWEN3_RERANKER_06B,
    };
    use crate::test_env::EnvVarGuard;

    #[test]
    fn runtime_probe_store_round_trips() {
        let temp = TempDir::new().expect("temp");
        let artifacts = fixture_artifacts(temp.path());
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass");
        let run = probe_profile_bundle(BALANCED_PROFILE, &artifacts).expect("probe");
        let store = RuntimeProbeStore::from_records(run.records);
        let path = temp.path().join("search-runtime-probes.toml");

        store.write_atomic(&path).expect("write");
        let read = RuntimeProbeStore::read(&path)
            .expect("read")
            .expect("present");

        assert_eq!(read.records.len(), 2);
        assert_eq!(read.records[0].outcome, RuntimeProbeOutcome::Passed);
        assert_eq!(read.records[1].outcome, RuntimeProbeOutcome::Passed);
    }

    #[test]
    fn runtime_probe_records_forced_role_failure() {
        let temp = TempDir::new().expect("temp");
        let artifacts = fixture_artifacts(temp.path());
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "fail:embedding");

        let run = probe_profile_bundle(BALANCED_PROFILE, &artifacts).expect("probe");
        let failure = run.first_required_problem().expect("required failure");

        assert_eq!(failure.role, "embedding");
        assert_eq!(failure.outcome, RuntimeProbeOutcome::Failed);
        assert_eq!(failure.failure_kind.as_deref(), Some("backend_unavailable"));
    }

    #[test]
    fn runtime_probe_treats_reranker_failure_as_advisory() {
        let temp = TempDir::new().expect("temp");
        let profile = ProfileBundle {
            reranker_model: Some(QWEN3_RERANKER_06B.id),
            ..BALANCED_PROFILE
        };
        let mut artifacts = fixture_artifacts(temp.path());
        let reranker_path = temp.path().join("reranker.gguf");
        std::fs::write(&reranker_path, "reranker").expect("reranker");
        artifacts.artifacts.push(artifact(
            QWEN3_RERANKER_06B.id,
            "reranker",
            &reranker_path,
            None,
        ));
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "fail:rerank");

        let run = probe_profile_bundle(profile, &artifacts).expect("probe");
        let rerank = run
            .records
            .iter()
            .find(|record| record.role == "rerank")
            .expect("rerank record");

        assert!(!rerank.required);
        assert_eq!(rerank.outcome, RuntimeProbeOutcome::Failed);
        assert!(run.first_required_problem().is_none());
    }

    #[test]
    fn runtime_probe_stale_reason_detects_artifact_change() {
        let temp = TempDir::new().expect("temp");
        let artifacts = fixture_artifacts(temp.path());
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass");
        let run = probe_profile_bundle(BALANCED_PROFILE, &artifacts).expect("probe");
        let store = RuntimeProbeStore::from_records(run.records);
        let record = &store.records[0];
        let mut identity = identity_for_model(
            BALANCED_PROFILE.id,
            EMBEDDING_GEMMA_300M,
            &artifacts.artifacts[0],
        );

        assert_eq!(store.record_stale_reason(record, &identity), None);
        identity.artifact_sha256 = "different".to_string();

        assert_eq!(
            store.record_stale_reason(record, &identity),
            Some("artifact_sha256")
        );
    }

    #[test]
    fn target_triple_uses_cargo_target_when_available() {
        if let Some(target) = option_env!("LLM_WIKI_BUILD_TARGET") {
            assert_eq!(target_triple(), target);
        } else {
            assert!(target_triple().ends_with("-unknown"));
        }
    }

    #[test]
    fn truncate_message_handles_multibyte_text() {
        let message = format!("{}{}", "a".repeat(299), "é".repeat(8));
        let truncated = truncate_message(message);

        assert!(truncated.ends_with("..."));
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    fn fixture_artifacts(root: &std::path::Path) -> ModelArtifacts {
        let embedding_path = root.join("embedding.gguf");
        let expansion_path = root.join("expansion.gguf");
        std::fs::write(&embedding_path, "embedding").expect("embedding");
        std::fs::write(&expansion_path, "expansion").expect("expansion");
        ModelArtifacts::from_records(vec![
            artifact(
                EMBEDDING_GEMMA_300M.id,
                "embedding",
                &embedding_path,
                Some(768),
            ),
            artifact(
                QMD_QUERY_EXPANSION_17B.id,
                "query-expansion",
                &expansion_path,
                None,
            ),
        ])
    }

    fn artifact(
        model_id: &str,
        role: &str,
        path: &std::path::Path,
        dimensions: Option<usize>,
    ) -> ModelArtifactRecord {
        ModelArtifactRecord {
            model_id: model_id.to_string(),
            role: role.to_string(),
            profile: BALANCED_PROFILE.id.to_string(),
            repository: "example/repo".to_string(),
            revision: "revision".to_string(),
            file: path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .to_string(),
            download_url: "https://example.invalid/model.gguf".to_string(),
            path: path.to_path_buf(),
            expected_sha256: format!("{model_id}-sha256"),
            observed_sha256: format!("{model_id}-sha256"),
            size_bytes: 1,
            license: "test".to_string(),
            terms_url: None,
            dimensions,
            qmd_rs_version: crate::search_models::QMD_RS_VERSION.to_string(),
            adapter_schema_version: crate::search_models::ADAPTER_SCHEMA_VERSION,
            verified_at: "2026-05-25T00:00:00Z".to_string(),
        }
    }
}
