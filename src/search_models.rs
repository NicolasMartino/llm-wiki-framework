use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::CliContext;
use crate::progress::{ProgressOperation, ProgressReporter};
use crate::search_profile::{timestamp, write_toml_atomic};

const ACCEPTED_LICENSES_SCHEMA_VERSION: u32 = 1;
const MODEL_ARTIFACTS_SCHEMA_VERSION: u32 = 1;
const SEARCH_THRESHOLDS_SCHEMA_VERSION: u32 = 1;
const SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION: u32 = 2;
/// Names a local file the debug binary streams in place of the network, so
/// install tests can drive a download without one.
#[cfg(debug_assertions)]
const TEST_MODEL_SOURCE_ENV: &str = "LLM_WIKI_TEST_MODEL_SOURCE";
pub const DEFAULT_PROFILE_ID: &str = "balanced";
pub const QMD_RS_VERSION: &str = "0.3.2";
pub const ADAPTER_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchModel {
    pub id: &'static str,
    pub role: ModelRole,
    pub repository: &'static str,
    pub revision: &'static str,
    pub file: &'static str,
    pub license: &'static str,
    pub terms_url: Option<&'static str>,
    pub expected_sha256: &'static str,
    pub expected_size_bytes: u64,
    pub dimensions: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelRole {
    Embedding,
    QueryExpansion,
    Reranker,
}

impl ModelRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::QueryExpansion => "query-expansion",
            Self::Reranker => "reranker",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileBundle {
    pub id: &'static str,
    pub display_name: &'static str,
    pub embedding_model: &'static str,
    pub query_expansion_model: &'static str,
    pub reranker_model: Option<&'static str>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AcceptedLicenses {
    pub schema_version: u32,
    pub updated_at: String,
    pub licenses: Vec<AcceptedLicenseRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AcceptedLicenseRecord {
    pub model_id: String,
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_url: Option<String>,
    pub accepted_at: String,
    pub accepted_by_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelArtifacts {
    pub schema_version: u32,
    pub updated_at: String,
    pub artifacts: Vec<ModelArtifactRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModelArtifactRecord {
    pub model_id: String,
    pub role: String,
    pub profile: String,
    pub repository: String,
    pub revision: String,
    pub file: String,
    pub download_url: String,
    pub path: PathBuf,
    pub expected_sha256: String,
    pub observed_sha256: String,
    pub size_bytes: u64,
    pub license: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    pub qmd_rs_version: String,
    pub adapter_schema_version: u32,
    pub verified_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelArtifactClassification {
    Verified {
        record: Box<ModelArtifactRecord>,
    },
    Missing {
        path: PathBuf,
    },
    HashMismatch {
        path: PathBuf,
        observed_sha256: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchThresholds {
    pub schema_version: u32,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub profile: String,
    pub semantic_similarity_floor: f64,
    pub hybrid_pre_fusion_semantic_floor: f64,
    #[serde(default = "default_hybrid_final_semantic_floor")]
    pub hybrid_final_semantic_floor: f64,
    #[serde(default = "default_hybrid_semantic_only_floor")]
    pub hybrid_semantic_only_floor: f64,
    #[serde(default = "default_hybrid_strong_lexical_score_floor")]
    pub hybrid_strong_lexical_score_floor: f64,
    pub reranker_probability_floor: f64,
    pub lexical_exact_identifier_guard: String,
    pub qmd_rs_version: String,
    pub adapter_schema_version: u32,
    pub chunking_strategy: String,
    pub embedding_model: String,
    pub embedding_artifact_sha256: String,
    pub embedding_dimensions: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchThresholdStore {
    pub schema_version: u32,
    pub updated_at: String,
    pub thresholds: Vec<SearchThresholds>,
}

fn default_hybrid_final_semantic_floor() -> f64 {
    0.39
}

fn default_hybrid_semantic_only_floor() -> f64 {
    0.50
}

fn default_hybrid_strong_lexical_score_floor() -> f64 {
    10.0
}

pub const EMBEDDING_GEMMA_300M: SearchModel = SearchModel {
    id: "embeddinggemma-300m-q8_0",
    role: ModelRole::Embedding,
    repository: "ggml-org/embeddinggemma-300M-GGUF",
    revision: "0f741b5a6585bd53aeb15cd1372c56f2a0f65e12",
    file: "embeddinggemma-300M-Q8_0.gguf",
    license: "gemma",
    terms_url: Some("https://ai.google.dev/gemma/terms"),
    expected_sha256: "b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63",
    expected_size_bytes: 333_590_944,
    dimensions: Some(768),
};

pub const QMD_QUERY_EXPANSION_17B: SearchModel = SearchModel {
    id: "qmd-query-expansion-1.7b-q4_k_m",
    role: ModelRole::QueryExpansion,
    repository: "tobil/qmd-query-expansion-1.7B-gguf",
    revision: "7816de0b72572c6c860ca1eddf97ba9e7fb8cc65",
    file: "qmd-query-expansion-1.7B-q4_k_m.gguf",
    license: "mit",
    terms_url: None,
    expected_sha256: "000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0",
    expected_size_bytes: 1_282_438_912,
    dimensions: None,
};

pub const QWEN3_RERANKER_06B: SearchModel = SearchModel {
    id: "qwen3-reranker-0.6b-q8_0",
    role: ModelRole::Reranker,
    repository: "ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF",
    revision: "a02f48bb4f057028298c21fa033da2b30d7742d5",
    file: "qwen3-reranker-0.6b-q8_0.gguf",
    license: "apache-2.0",
    terms_url: None,
    expected_sha256: "22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48",
    expected_size_bytes: 639_000_000,
    dimensions: None,
};

pub const BALANCED_PROFILE: ProfileBundle = ProfileBundle {
    id: DEFAULT_PROFILE_ID,
    display_name: "Balanced local hybrid search",
    embedding_model: EMBEDDING_GEMMA_300M.id,
    query_expansion_model: QMD_QUERY_EXPANSION_17B.id,
    reranker_model: None,
};

pub const MODEL_CATALOG: &[SearchModel] = &[
    EMBEDDING_GEMMA_300M,
    QMD_QUERY_EXPANSION_17B,
    QWEN3_RERANKER_06B,
];

pub const PROFILE_CATALOG: &[ProfileBundle] = &[BALANCED_PROFILE];

impl SearchModel {
    pub fn download_url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repository, self.revision, self.file
        )
    }

    pub fn managed_path(&self, model_root: &Path) -> PathBuf {
        model_root.join(self.id).join(self.file)
    }
}

impl AcceptedLicenses {
    pub fn from_models(models: &[SearchModel]) -> Self {
        let accepted_at = timestamp();
        Self {
            schema_version: ACCEPTED_LICENSES_SCHEMA_VERSION,
            updated_at: accepted_at.clone(),
            licenses: models
                .iter()
                .map(|model| AcceptedLicenseRecord {
                    model_id: model.id.to_string(),
                    license: model.license.to_string(),
                    terms_url: model.terms_url.map(ToString::to_string),
                    accepted_at: accepted_at.clone(),
                    accepted_by_version: env!("CARGO_PKG_VERSION").to_string(),
                })
                .collect(),
        }
    }

    pub fn accepts_model(&self, model: SearchModel) -> bool {
        self.licenses.iter().any(|license| {
            license.model_id == model.id
                && license.license == model.license
                && license.terms_url.as_deref() == model.terms_url
        })
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read accepted licenses {}", path.display()))?;
        let accepted: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse accepted licenses {}", path.display()))?;
        if accepted.schema_version != ACCEPTED_LICENSES_SCHEMA_VERSION {
            bail!(
                "unsupported accepted licenses schema_version {} in {}; expected {}",
                accepted.schema_version,
                path.display(),
                ACCEPTED_LICENSES_SCHEMA_VERSION
            );
        }
        Ok(Some(accepted))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "accepted licenses")
    }
}

impl ModelArtifacts {
    pub fn from_records(artifacts: Vec<ModelArtifactRecord>) -> Self {
        Self {
            schema_version: MODEL_ARTIFACTS_SCHEMA_VERSION,
            updated_at: timestamp(),
            artifacts,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read model artifacts {}", path.display()))?;
        let artifacts: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse model artifacts {}", path.display()))?;
        if artifacts.schema_version != MODEL_ARTIFACTS_SCHEMA_VERSION {
            bail!(
                "unsupported model artifacts schema_version {} in {}; expected {}",
                artifacts.schema_version,
                path.display(),
                MODEL_ARTIFACTS_SCHEMA_VERSION
            );
        }
        Ok(Some(artifacts))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "model artifacts")
    }
}

impl SearchThresholds {
    pub fn calibrated(
        profile: impl Into<String>,
        embedding_model: impl Into<String>,
        embedding_artifact_sha256: impl Into<String>,
        embedding_dimensions: usize,
        chunking_strategy: impl Into<String>,
        semantic_similarity_floor: f64,
        hybrid_pre_fusion_semantic_floor: f64,
    ) -> Self {
        Self {
            schema_version: SEARCH_THRESHOLDS_SCHEMA_VERSION,
            updated_at: timestamp(),
            project_id: None,
            profile: profile.into(),
            semantic_similarity_floor,
            hybrid_pre_fusion_semantic_floor,
            hybrid_final_semantic_floor: default_hybrid_final_semantic_floor(),
            hybrid_semantic_only_floor: default_hybrid_semantic_only_floor(),
            hybrid_strong_lexical_score_floor: default_hybrid_strong_lexical_score_floor(),
            reranker_probability_floor: 0.50,
            lexical_exact_identifier_guard: "preserve_lexical_top_3".to_string(),
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            chunking_strategy: chunking_strategy.into(),
            embedding_model: embedding_model.into(),
            embedding_artifact_sha256: embedding_artifact_sha256.into(),
            embedding_dimensions,
        }
    }

    pub fn default_for_model(
        profile: impl Into<String>,
        embedding_model: impl Into<String>,
        embedding_artifact_sha256: impl Into<String>,
        embedding_dimensions: usize,
        chunking_strategy: impl Into<String>,
    ) -> Option<Self> {
        let embedding_model = embedding_model.into();
        let model = model_by_id(&embedding_model)?;
        if model.dimensions != Some(embedding_dimensions) {
            return None;
        }

        let (
            semantic_similarity_floor,
            hybrid_pre_fusion_semantic_floor,
            hybrid_final_semantic_floor,
            hybrid_semantic_only_floor,
            hybrid_strong_lexical_score_floor,
            reranker_probability_floor,
        ) = match embedding_model.as_str() {
            "embeddinggemma-300m-q8_0" => (
                0.0,
                0.0,
                default_hybrid_final_semantic_floor(),
                default_hybrid_semantic_only_floor(),
                default_hybrid_strong_lexical_score_floor(),
                0.50,
            ),
            _ => return None,
        };

        Some(Self {
            schema_version: SEARCH_THRESHOLDS_SCHEMA_VERSION,
            updated_at: timestamp(),
            project_id: None,
            profile: profile.into(),
            semantic_similarity_floor,
            hybrid_pre_fusion_semantic_floor,
            hybrid_final_semantic_floor,
            hybrid_semantic_only_floor,
            hybrid_strong_lexical_score_floor,
            reranker_probability_floor,
            lexical_exact_identifier_guard: "preserve_lexical_top_3".to_string(),
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            chunking_strategy: chunking_strategy.into(),
            embedding_model,
            embedding_artifact_sha256: embedding_artifact_sha256.into(),
            embedding_dimensions,
        })
    }

    pub fn measurement(
        profile: impl Into<String>,
        embedding_model: impl Into<String>,
        embedding_artifact_sha256: impl Into<String>,
        embedding_dimensions: usize,
        chunking_strategy: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: SEARCH_THRESHOLDS_SCHEMA_VERSION,
            updated_at: timestamp(),
            project_id: None,
            profile: profile.into(),
            semantic_similarity_floor: 0.0,
            hybrid_pre_fusion_semantic_floor: 0.0,
            hybrid_final_semantic_floor: 0.0,
            hybrid_semantic_only_floor: 0.0,
            hybrid_strong_lexical_score_floor: 0.0,
            reranker_probability_floor: 0.0,
            lexical_exact_identifier_guard: "preserve_lexical_top_3".to_string(),
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            chunking_strategy: chunking_strategy.into(),
            embedding_model: embedding_model.into(),
            embedding_artifact_sha256: embedding_artifact_sha256.into(),
            embedding_dimensions,
        }
    }

    fn validate_schema(&self, path: &Path) -> Result<()> {
        if self.schema_version != SEARCH_THRESHOLDS_SCHEMA_VERSION {
            bail!(
                "unsupported search thresholds schema_version {} in {}; expected {}",
                self.schema_version,
                path.display(),
                SEARCH_THRESHOLDS_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    pub fn for_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }
}

impl SearchThresholdStore {
    pub fn empty() -> Self {
        Self {
            schema_version: SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION,
            updated_at: timestamp(),
            thresholds: Vec::new(),
        }
    }

    pub fn from_thresholds(thresholds: Vec<SearchThresholds>) -> Self {
        Self {
            schema_version: SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION,
            updated_at: timestamp(),
            thresholds,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read search thresholds {}", path.display()))?;
        let value: toml::Value = toml::from_str(&input)
            .with_context(|| format!("failed to parse search thresholds {}", path.display()))?;
        if value.get("thresholds").is_some() {
            let store: Self = toml::from_str(&input).with_context(|| {
                format!(
                    "failed to parse scoped search thresholds {}",
                    path.display()
                )
            })?;
            store.validate_schema(path)?;
            for thresholds in &store.thresholds {
                thresholds.validate_schema(path)?;
            }
            return Ok(Some(store));
        }

        let thresholds: SearchThresholds = toml::from_str(&input).with_context(|| {
            format!(
                "failed to parse legacy search thresholds {}",
                path.display()
            )
        })?;
        thresholds.validate_schema(path)?;
        Ok(Some(Self::from_thresholds(vec![thresholds])))
    }

    fn validate_schema(&self, path: &Path) -> Result<()> {
        if self.schema_version != SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION {
            bail!(
                "unsupported search thresholds store schema_version {} in {}; expected {}",
                self.schema_version,
                path.display(),
                SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    pub fn thresholds(&self) -> &[SearchThresholds] {
        &self.thresholds
    }

    pub fn upsert(&mut self, thresholds: SearchThresholds) {
        self.updated_at = timestamp();
        if let Some(existing) = self.thresholds.iter_mut().find(|existing| {
            existing.project_id == thresholds.project_id
                && existing.profile == thresholds.profile
                && existing.qmd_rs_version == thresholds.qmd_rs_version
                && existing.adapter_schema_version == thresholds.adapter_schema_version
                && existing.chunking_strategy == thresholds.chunking_strategy
                && existing.embedding_model == thresholds.embedding_model
                && existing.embedding_artifact_sha256 == thresholds.embedding_artifact_sha256
                && existing.embedding_dimensions == thresholds.embedding_dimensions
        }) {
            *existing = thresholds;
        } else {
            self.thresholds.push(thresholds);
        }
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "search thresholds")
    }
}

pub fn profile_by_id(id: &str) -> Option<ProfileBundle> {
    PROFILE_CATALOG
        .iter()
        .copied()
        .find(|profile| profile.id == id)
}

pub fn model_by_id(id: &str) -> Option<SearchModel> {
    MODEL_CATALOG.iter().copied().find(|model| model.id == id)
}

pub fn classify_model_artifact(
    model: SearchModel,
    profile: ProfileBundle,
    model_root: &Path,
) -> Result<ModelArtifactClassification> {
    classify_model_artifact_with_progress(model, profile, model_root, None)
}

pub fn classify_model_artifact_with_progress(
    model: SearchModel,
    profile: ProfileBundle,
    model_root: &Path,
    progress: Option<&mut ProgressOperation>,
) -> Result<ModelArtifactClassification> {
    let path = model.managed_path(model_root);
    if !path.exists() {
        return Ok(ModelArtifactClassification::Missing { path });
    }

    let observed_sha256 = sha256_file_with_progress(&path, progress)?;
    if observed_sha256 == model.expected_sha256 {
        return Ok(ModelArtifactClassification::Verified {
            record: Box::new(artifact_record(model, profile, path, observed_sha256)?),
        });
    }

    Ok(ModelArtifactClassification::HashMismatch {
        path,
        observed_sha256,
    })
}

/// Downloads `model` to `path` through `download_once`, retrying from zero up to
/// three times, then checks its hash; progress for both steps goes to
/// `reporter`. Production passes [`network_download`]; tests pass a fake.
pub fn download_and_verify_model(
    model: SearchModel,
    profile: ProfileBundle,
    path: &Path,
    position: ModelPosition,
    reporter: &ProgressReporter,
    mut download_once: impl FnMut(SearchModel, &Path, &mut ProgressOperation) -> Result<()>,
) -> Result<ModelArtifactRecord> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 1;
    loop {
        let mut progress = reporter.begin(
            "download",
            model.id,
            position.index,
            position.total,
            model.expected_size_bytes,
        );
        let err = match download_once(model, path, &mut progress) {
            Ok(()) => {
                progress.finish();
                break;
            }
            Err(err) => err,
        };
        // Ends a terminal bar before the warning, so the two do not share a line.
        drop(progress);
        if attempt >= MAX_ATTEMPTS {
            return Err(err);
        }
        eprintln!(
            "warning: model download attempt {attempt}/{MAX_ATTEMPTS} for {} failed: {err:#}; retrying from zero",
            model.id
        );
        std::thread::sleep(Duration::from_secs(u64::from(attempt)));
        attempt += 1;
    }
    let mut verify = reporter.begin(
        "verify",
        model.id,
        position.index,
        position.total,
        model.expected_size_bytes,
    );
    let record = verify_downloaded_model(model, profile, path, Some(&mut verify))?;
    verify.finish();
    Ok(record)
}

/// Where a model stands in the install's list, shown as `[index/total]`.
#[derive(Clone, Copy, Debug)]
pub struct ModelPosition {
    pub index: usize,
    pub total: usize,
}

fn verify_downloaded_model(
    model: SearchModel,
    profile: ProfileBundle,
    path: &Path,
    progress: Option<&mut ProgressOperation>,
) -> Result<ModelArtifactRecord> {
    let observed = sha256_file_with_progress(path, progress)?;
    if observed != model.expected_sha256 {
        bail!(
            "downloaded model artifact hash mismatch for {}; expected {}, observed {}",
            model.id,
            model.expected_sha256,
            observed
        );
    }
    artifact_record(model, profile, path.to_path_buf(), observed)
}

fn artifact_record(
    model: SearchModel,
    profile: ProfileBundle,
    path: PathBuf,
    observed_sha256: String,
) -> Result<ModelArtifactRecord> {
    let size_bytes = fs::metadata(&path)
        .with_context(|| format!("stat model artifact {}", path.display()))?
        .len();
    Ok(ModelArtifactRecord {
        model_id: model.id.to_string(),
        role: model.role.label().to_string(),
        profile: profile.id.to_string(),
        repository: model.repository.to_string(),
        revision: model.revision.to_string(),
        file: model.file.to_string(),
        download_url: model.download_url(),
        path,
        expected_sha256: model.expected_sha256.to_string(),
        observed_sha256,
        size_bytes,
        license: model.license.to_string(),
        terms_url: model.terms_url.map(ToString::to_string),
        dimensions: model.dimensions,
        qmd_rs_version: QMD_RS_VERSION.to_string(),
        adapter_schema_version: ADAPTER_SCHEMA_VERSION,
        verified_at: timestamp(),
    })
}

/// The downloader install uses: one attempt over HTTP, streamed to a temp file
/// beside `destination` and renamed into place once complete.
pub fn network_download(
    context: &CliContext,
) -> impl FnMut(SearchModel, &Path, &mut ProgressOperation) -> Result<()> + '_ {
    move |model, destination, progress| download_model_once(model, destination, context, progress)
}

fn download_model_once(
    model: SearchModel,
    destination: &Path,
    context: &CliContext,
    progress: &mut ProgressOperation,
) -> Result<()> {
    let parent = destination.parent().with_context(|| {
        format!(
            "model artifact path has no parent: {}",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create model artifact dir {}", parent.display()))?;

    let (mut source, content_length) = open_model_source(model)?;
    if let Some(message) = content_length_mismatch(model, content_length) {
        context.diagnostic(message);
    }

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create temp model artifact in {}", parent.display()))?;
    copy_model_stream(&mut source, &mut temp, model.id, |bytes| {
        progress.advance(bytes)
    })?;
    temp.persist(destination)
        .map_err(|err| err.error)
        .with_context(|| format!("persist model artifact {}", destination.display()))?;
    Ok(())
}

fn open_model_source(model: SearchModel) -> Result<(Box<dyn Read>, Option<u64>)> {
    #[cfg(debug_assertions)]
    if let Some(source) = std::env::var_os(TEST_MODEL_SOURCE_ENV) {
        let file = fs::File::open(&source)
            .with_context(|| format!("open test model source {}", source.display()))?;
        let length = file.metadata()?.len();
        return Ok((Box::new(file), Some(length)));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .context("build model download client")?;
    let response = client
        .get(model.download_url())
        .send()
        .with_context(|| format!("download {}", model.download_url()))?;
    if !response.status().is_success() {
        bail!(
            "failed to download {}: HTTP {}",
            model.download_url(),
            response.status()
        );
    }
    let content_length = response.content_length();
    Ok((Box::new(response), content_length))
}

/// The catalog size stays the progress total; a server that disagrees is only
/// worth a `-v` line, since the hash check decides.
fn content_length_mismatch(model: SearchModel, content_length: Option<u64>) -> Option<String> {
    let content_length = content_length?;
    (content_length != model.expected_size_bytes).then(|| {
        format!(
            "search model content length mismatch: {} expected {} bytes, server reported {} bytes",
            model.id, model.expected_size_bytes, content_length
        )
    })
}

fn copy_model_stream(
    reader: &mut impl Read,
    writer: &mut impl Write,
    model_id: &str,
    mut progress_observer: impl FnMut(u64),
) -> Result<()> {
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let bytes = reader
            .read(&mut buffer)
            .with_context(|| format!("read model response for {model_id}"))?;
        if bytes == 0 {
            break;
        }
        writer
            .write_all(&buffer[..bytes])
            .with_context(|| format!("write temp model artifact for {model_id}"))?;
        progress_observer(bytes as u64);
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    sha256_file_with_progress(path, None)
}

fn sha256_file_with_progress(
    path: &Path,
    mut progress: Option<&mut ProgressOperation>,
) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("open model artifact {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let bytes = file
            .read(&mut buffer)
            .with_context(|| format!("read model artifact {}", path.display()))?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
        if let Some(progress) = progress.as_deref_mut() {
            progress.advance(bytes as u64);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{
        AcceptedLicenses, DEFAULT_PROFILE_ID, EMBEDDING_GEMMA_300M, ModelArtifactClassification,
        ModelPosition, ModelRole, ProfileBundle, QMD_QUERY_EXPANSION_17B, SearchModel,
        SearchThresholdStore, SearchThresholds, classify_model_artifact, content_length_mismatch,
        copy_model_stream, download_and_verify_model, profile_by_id,
    };
    use crate::progress::{ProgressOperation, ProgressReporter};

    #[test]
    fn model_stream_reports_every_written_chunk() {
        let input = vec![7_u8; 150_000];
        let mut reader = std::io::Cursor::new(&input);
        let mut output = Vec::new();
        let mut reported = 0_u64;

        copy_model_stream(&mut reader, &mut output, "fixture", |bytes| {
            reported += bytes;
        })
        .expect("copy model stream");

        assert_eq!(output, input);
        assert_eq!(reported, input.len() as u64);
    }
    const FIXTURE_MODEL_BYTES: &[u8] = b"fixture model";
    const BAD_MODEL_BYTES: &[u8] = b"bad model";

    const FIXTURE_MODEL: SearchModel = SearchModel {
        id: "fixture-model-q8",
        role: ModelRole::Embedding,
        repository: "example/fixture-model",
        revision: "fixture-revision",
        file: "fixture-model.gguf",
        license: "test-license",
        terms_url: Some("https://example.invalid/terms"),
        expected_sha256: "c7a3a8c7435ef8e4cf1ca2d261f7e09ca85de6cdb70f7c689a836680523a180c",
        expected_size_bytes: 13,
        dimensions: Some(4),
    };

    const FIXTURE_PROFILE: ProfileBundle = ProfileBundle {
        id: "fixture-profile",
        display_name: "Fixture profile",
        embedding_model: FIXTURE_MODEL.id,
        query_expansion_model: FIXTURE_MODEL.id,
        reranker_model: None,
    };

    #[test]
    fn default_profile_has_required_hybrid_models() {
        let profile = profile_by_id(DEFAULT_PROFILE_ID).expect("balanced profile");

        assert_eq!(profile.embedding_model, EMBEDDING_GEMMA_300M.id);
        assert_eq!(profile.query_expansion_model, QMD_QUERY_EXPANSION_17B.id);
        assert!(profile.reranker_model.is_none());
        assert_eq!(EMBEDDING_GEMMA_300M.role, ModelRole::Embedding);
        assert_eq!(EMBEDDING_GEMMA_300M.dimensions, Some(768));
    }

    #[test]
    fn accepted_license_requires_current_license_and_terms() {
        let mut accepted = AcceptedLicenses::from_models(&[EMBEDDING_GEMMA_300M]);

        assert!(accepted.accepts_model(EMBEDDING_GEMMA_300M));

        accepted.licenses[0].terms_url = Some("https://example.com/old-terms".to_string());

        assert!(!accepted.accepts_model(EMBEDDING_GEMMA_300M));
    }

    #[test]
    fn model_classification_reports_missing_file() {
        let temp = TempDir::new().expect("tempdir");

        let classification = classify_model_artifact(FIXTURE_MODEL, FIXTURE_PROFILE, temp.path())
            .expect("classification");

        assert_eq!(
            classification,
            ModelArtifactClassification::Missing {
                path: FIXTURE_MODEL.managed_path(temp.path())
            }
        );
    }

    #[test]
    fn model_classification_verifies_local_bytes_without_artifact_record() {
        let temp = TempDir::new().expect("tempdir");
        write_fixture_model(temp.path(), FIXTURE_MODEL_BYTES);

        let classification = classify_model_artifact(FIXTURE_MODEL, FIXTURE_PROFILE, temp.path())
            .expect("classification");

        let ModelArtifactClassification::Verified { record } = classification else {
            panic!("expected verified classification");
        };
        assert_eq!(record.model_id, FIXTURE_MODEL.id);
        assert_eq!(record.profile, FIXTURE_PROFILE.id);
        assert_eq!(record.observed_sha256, FIXTURE_MODEL.expected_sha256);
        assert_eq!(record.size_bytes, FIXTURE_MODEL.expected_size_bytes);
    }

    #[test]
    fn model_classification_reports_hash_mismatch() {
        let temp = TempDir::new().expect("tempdir");
        write_fixture_model(temp.path(), BAD_MODEL_BYTES);

        let classification = classify_model_artifact(FIXTURE_MODEL, FIXTURE_PROFILE, temp.path())
            .expect("classification");

        assert_eq!(
            classification,
            ModelArtifactClassification::HashMismatch {
                path: FIXTURE_MODEL.managed_path(temp.path()),
                observed_sha256: "28d2eb87e38b9c76005f04cdd1d43b411b1f835fe27454e4fced0db741561c52"
                    .to_string(),
            }
        );
    }

    fn fixture_position() -> ModelPosition {
        ModelPosition { index: 1, total: 1 }
    }

    fn fake_download(
        bytes: &'static [u8],
    ) -> impl FnMut(SearchModel, &Path, &mut ProgressOperation) -> anyhow::Result<()> {
        move |_, destination, progress| {
            fs::create_dir_all(destination.parent().expect("destination parent"))
                .expect("destination parent");
            fs::write(destination, bytes).expect("fake download");
            progress.advance(bytes.len() as u64);
            Ok(())
        }
    }

    /// Drops the rate, ETA and elapsed time, which depend on the clock.
    fn without_timings(lines: &[String]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                if let Some((head, _)) = line.split_once(" eta ") {
                    let tokens = head.split(' ').collect::<Vec<_>>();
                    tokens[..tokens.len() - 2].join(" ")
                } else {
                    line.split(" in ").next().expect("line").to_string()
                }
            })
            .collect()
    }

    #[test]
    fn download_and_verify_reports_both_steps_and_replaces_a_mismatched_file() {
        let temp = TempDir::new().expect("tempdir");
        let path = FIXTURE_MODEL.managed_path(temp.path());
        write_fixture_model(temp.path(), BAD_MODEL_BYTES);
        let (reporter, lines) = ProgressReporter::recorded();

        let record = download_and_verify_model(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            &path,
            fixture_position(),
            &reporter,
            fake_download(FIXTURE_MODEL_BYTES),
        )
        .expect("download and verify");

        assert_eq!(record.observed_sha256, FIXTURE_MODEL.expected_sha256);
        assert_eq!(fs::read(&path).expect("model bytes"), FIXTURE_MODEL_BYTES);
        let mut expected = Vec::new();
        for phase in ["download", "verify"] {
            expected.push(format!("[1/1] fixture-model-q8 {phase} start 13 B"));
            for percent in [25, 50, 75, 100] {
                expected.push(format!(
                    "[1/1] fixture-model-q8 {phase} {percent}% 13 B / 13 B"
                ));
            }
            expected.push(format!("[1/1] fixture-model-q8 {phase} done 13 B"));
        }
        assert_eq!(without_timings(&lines.borrow()), expected);
    }

    #[test]
    fn download_and_verify_rejects_a_download_with_the_wrong_bytes() {
        let temp = TempDir::new().expect("tempdir");
        let (reporter, lines) = ProgressReporter::recorded();

        let error = download_and_verify_model(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            &FIXTURE_MODEL.managed_path(temp.path()),
            fixture_position(),
            &reporter,
            fake_download(BAD_MODEL_BYTES),
        )
        .expect_err("bad download hash should fail");

        assert!(format!("{error:#}").contains("downloaded model artifact hash mismatch"));
        let lines = without_timings(&lines.borrow());
        assert!(lines.contains(&"[1/1] fixture-model-q8 verify start 13 B".to_string()));
        assert!(!lines.iter().any(|line| line.contains("verify done")));
    }

    #[test]
    fn download_and_verify_restarts_progress_for_a_retried_attempt() {
        let temp = TempDir::new().expect("tempdir");
        let (reporter, lines) = ProgressReporter::recorded();
        let mut succeed = fake_download(FIXTURE_MODEL_BYTES);
        let mut attempts = 0;

        download_and_verify_model(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            &FIXTURE_MODEL.managed_path(temp.path()),
            fixture_position(),
            &reporter,
            |model, destination, progress| {
                attempts += 1;
                if attempts == 1 {
                    progress.advance(5);
                    anyhow::bail!("connection reset");
                }
                succeed(model, destination, progress)
            },
        )
        .expect("second attempt succeeds");

        let lines = without_timings(&lines.borrow());
        let downloads = lines
            .iter()
            .filter(|line| line.contains(" download "))
            .collect::<Vec<_>>();
        assert_eq!(
            downloads.first().map(|line| line.as_str()),
            Some("[1/1] fixture-model-q8 download start 13 B")
        );
        assert_eq!(
            downloads
                .iter()
                .filter(|line| line.ends_with("download start 13 B"))
                .count(),
            2
        );
        assert_eq!(
            downloads
                .iter()
                .filter(|line| line.contains("download done"))
                .count(),
            1
        );
    }

    #[test]
    fn content_length_mismatch_is_reported_only_when_the_server_disagrees() {
        assert_eq!(content_length_mismatch(FIXTURE_MODEL, None), None);
        assert_eq!(content_length_mismatch(FIXTURE_MODEL, Some(13)), None);
        assert_eq!(
            content_length_mismatch(FIXTURE_MODEL, Some(20)).as_deref(),
            Some(
                "search model content length mismatch: fixture-model-q8 expected 13 bytes, server reported 20 bytes"
            )
        );
    }

    #[test]
    fn calibrated_thresholds_keep_non_calibrated_default_gates() {
        let thresholds = SearchThresholds::calibrated(
            "balanced",
            EMBEDDING_GEMMA_300M.id,
            "artifact-sha",
            768,
            "qmd-rs-character-v1:3200:480",
            0.328,
            0.027,
        );

        assert_eq!(thresholds.semantic_similarity_floor, 0.328);
        assert_eq!(thresholds.hybrid_pre_fusion_semantic_floor, 0.027);
        assert_eq!(thresholds.hybrid_final_semantic_floor, 0.39);
        assert_eq!(thresholds.hybrid_semantic_only_floor, 0.50);
        assert_eq!(thresholds.hybrid_strong_lexical_score_floor, 10.0);
        assert_eq!(thresholds.reranker_probability_floor, 0.50);
    }

    #[test]
    fn default_thresholds_keep_hybrid_floor_gates() {
        let thresholds = SearchThresholds::default_for_model(
            "balanced",
            EMBEDDING_GEMMA_300M.id,
            "artifact-sha",
            768,
            "qmd-rs-character-v1:3200:480",
        )
        .expect("default thresholds");

        assert_eq!(thresholds.semantic_similarity_floor, 0.0);
        assert_eq!(thresholds.hybrid_pre_fusion_semantic_floor, 0.0);
        assert_eq!(thresholds.hybrid_final_semantic_floor, 0.39);
        assert_eq!(thresholds.hybrid_semantic_only_floor, 0.50);
        assert_eq!(thresholds.hybrid_strong_lexical_score_floor, 10.0);
        assert_eq!(thresholds.reranker_probability_floor, 0.50);
    }

    #[test]
    fn threshold_store_upserts_without_clobbering_other_project_scopes() {
        let framework = SearchThresholds::calibrated(
            "balanced",
            EMBEDDING_GEMMA_300M.id,
            "artifact-sha",
            768,
            "qmd-rs-character-v1:3200:480",
            0.328,
            0.027,
        )
        .for_project("framework");
        let electric = SearchThresholds::calibrated(
            "balanced",
            EMBEDDING_GEMMA_300M.id,
            "artifact-sha",
            768,
            "qmd-rs-character-v1:3200:480",
            0.571,
            0.571,
        )
        .for_project("electric-cars");
        let framework_update = SearchThresholds::calibrated(
            "balanced",
            EMBEDDING_GEMMA_300M.id,
            "artifact-sha",
            768,
            "qmd-rs-character-v1:3200:480",
            0.400,
            0.100,
        )
        .for_project("framework");

        let mut store = SearchThresholdStore::empty();
        store.upsert(framework);
        store.upsert(electric);
        store.upsert(framework_update);

        assert_eq!(store.thresholds().len(), 2);
        let framework = store
            .thresholds()
            .iter()
            .find(|thresholds| thresholds.project_id.as_deref() == Some("framework"))
            .expect("framework thresholds");
        let electric = store
            .thresholds()
            .iter()
            .find(|thresholds| thresholds.project_id.as_deref() == Some("electric-cars"))
            .expect("electric thresholds");
        assert_eq!(framework.semantic_similarity_floor, 0.400);
        assert_eq!(electric.semantic_similarity_floor, 0.571);
    }

    fn write_fixture_model(model_root: &std::path::Path, bytes: &[u8]) {
        let path = FIXTURE_MODEL.managed_path(model_root);
        fs::create_dir_all(path.parent().expect("model parent")).expect("model dir");
        fs::write(path, bytes).expect("model bytes");
    }
}
