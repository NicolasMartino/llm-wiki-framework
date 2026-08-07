use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::search_profile::{timestamp, write_toml_atomic};

const ACCEPTED_LICENSES_SCHEMA_VERSION: u32 = 1;
const MODEL_ARTIFACTS_SCHEMA_VERSION: u32 = 1;
const SEARCH_THRESHOLDS_SCHEMA_VERSION: u32 = 1;
const SEARCH_THRESHOLDS_STORE_SCHEMA_VERSION: u32 = 2;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedModel {
    pub record: ModelArtifactRecord,
    pub outcome: MaterializationOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterializationOutcome {
    Reused,
    Downloaded,
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

pub fn materialize_model(
    model: SearchModel,
    profile: ProfileBundle,
    model_root: &Path,
    force: bool,
) -> Result<MaterializedModel> {
    materialize_model_with_downloader(model, profile, model_root, force, download_model)
}

pub fn classify_model_artifact(
    model: SearchModel,
    profile: ProfileBundle,
    model_root: &Path,
) -> Result<ModelArtifactClassification> {
    let path = model.managed_path(model_root);
    if !path.exists() {
        return Ok(ModelArtifactClassification::Missing { path });
    }

    let observed_sha256 = sha256_file(&path)?;
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

fn materialize_model_with_downloader(
    model: SearchModel,
    profile: ProfileBundle,
    model_root: &Path,
    force: bool,
    downloader: impl Fn(SearchModel, &Path) -> Result<()>,
) -> Result<MaterializedModel> {
    match classify_model_artifact(model, profile, model_root)? {
        ModelArtifactClassification::Verified { record } => Ok(MaterializedModel {
            record: *record,
            outcome: MaterializationOutcome::Reused,
        }),
        ModelArtifactClassification::Missing { path } => {
            download_and_verify_model(model, profile, &path, downloader)
        }
        ModelArtifactClassification::HashMismatch { path, .. } => {
            if !force {
                bail!(
                    "model artifact hash mismatch for {}; rerun `llm-wiki install --configure-search --force` to replace {}",
                    model.id,
                    path.display()
                );
            }
            download_and_verify_model(model, profile, &path, downloader)
        }
    }
}

fn download_and_verify_model(
    model: SearchModel,
    profile: ProfileBundle,
    path: &Path,
    downloader: impl Fn(SearchModel, &Path) -> Result<()>,
) -> Result<MaterializedModel> {
    downloader(model, path)?;
    let observed = sha256_file(path)?;
    if observed != model.expected_sha256 {
        bail!(
            "downloaded model artifact hash mismatch for {}; expected {}, observed {}",
            model.id,
            model.expected_sha256,
            observed
        );
    }
    Ok(MaterializedModel {
        record: artifact_record(model, profile, path.to_path_buf(), observed)?,
        outcome: MaterializationOutcome::Downloaded,
    })
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

fn download_model(model: SearchModel, destination: &Path) -> Result<()> {
    // Retry transient download failures a bounded number of times. Each attempt is
    // a clean retry-from-zero (no resume-from-offset); the temp-file + atomic
    // persist below keep partial downloads from leaking, and SHA-256 verification
    // at the call site still guards integrity.
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 1;
    loop {
        match download_model_once(model, destination) {
            Ok(()) => return Ok(()),
            Err(err) if attempt < MAX_ATTEMPTS => {
                eprintln!(
                    "warning: model download attempt {attempt}/{MAX_ATTEMPTS} for {} failed: {err:#}; retrying",
                    model.id
                );
                std::thread::sleep(Duration::from_secs(u64::from(attempt)));
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

fn download_model_once(model: SearchModel, destination: &Path) -> Result<()> {
    let parent = destination.parent().with_context(|| {
        format!(
            "model artifact path has no parent: {}",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create model artifact dir {}", parent.display()))?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .context("build model download client")?;
    let mut response = client
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

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("create temp model artifact in {}", parent.display()))?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let bytes = response
            .read(&mut buffer)
            .with_context(|| format!("read model response for {}", model.id))?;
        if bytes == 0 {
            break;
        }
        temp.write_all(&buffer[..bytes])
            .with_context(|| format!("write temp model artifact for {}", model.id))?;
    }
    temp.persist(destination)
        .map_err(|err| err.error)
        .with_context(|| format!("persist model artifact {}", destination.display()))?;
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
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
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;

    use super::{
        AcceptedLicenses, DEFAULT_PROFILE_ID, EMBEDDING_GEMMA_300M, MaterializationOutcome,
        ModelArtifactClassification, ModelRole, ProfileBundle, QMD_QUERY_EXPANSION_17B,
        SearchModel, SearchThresholdStore, SearchThresholds, classify_model_artifact,
        materialize_model_with_downloader, profile_by_id,
    };
    use tempfile::TempDir;

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

    #[test]
    fn materialize_reuses_verified_model_without_downloader() {
        let temp = TempDir::new().expect("tempdir");
        write_fixture_model(temp.path(), FIXTURE_MODEL_BYTES);
        let downloader_called = Cell::new(false);

        let materialized = materialize_model_with_downloader(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            temp.path(),
            false,
            |_, _| {
                downloader_called.set(true);
                Ok(())
            },
        )
        .expect("materialized");

        assert_eq!(materialized.outcome, MaterializationOutcome::Reused);
        assert_eq!(materialized.record.model_id, FIXTURE_MODEL.id);
        assert!(!downloader_called.get());
    }

    #[test]
    fn materialize_rejects_hash_mismatch_without_force() {
        let temp = TempDir::new().expect("tempdir");
        write_fixture_model(temp.path(), BAD_MODEL_BYTES);

        let error = materialize_model_with_downloader(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            temp.path(),
            false,
            |_, _| panic!("downloader should not run"),
        )
        .expect_err("mismatch should fail");

        assert!(format!("{error:#}").contains("model artifact hash mismatch"));
    }

    #[test]
    fn materialize_rejects_downloader_that_writes_wrong_bytes() {
        let temp = TempDir::new().expect("tempdir");

        let error = materialize_model_with_downloader(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            temp.path(),
            false,
            |_, destination| {
                fs::create_dir_all(destination.parent().expect("destination parent"))
                    .expect("destination parent");
                fs::write(destination, BAD_MODEL_BYTES).expect("bad download");
                Ok(())
            },
        )
        .expect_err("bad download hash should fail");

        assert!(format!("{error:#}").contains("downloaded model artifact hash mismatch"));
    }

    #[test]
    fn materialize_force_replaces_hash_mismatch_only() {
        let temp = TempDir::new().expect("tempdir");
        let path = FIXTURE_MODEL.managed_path(temp.path());
        write_fixture_model(temp.path(), BAD_MODEL_BYTES);

        let materialized = materialize_model_with_downloader(
            FIXTURE_MODEL,
            FIXTURE_PROFILE,
            temp.path(),
            true,
            |_, destination| {
                fs::write(destination, FIXTURE_MODEL_BYTES).expect("replacement");
                Ok(())
            },
        )
        .expect("materialized");

        assert_eq!(materialized.outcome, MaterializationOutcome::Downloaded);
        assert_eq!(
            fs::read(path).expect("model bytes"),
            FIXTURE_MODEL_BYTES.to_vec()
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
