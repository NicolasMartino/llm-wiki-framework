use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::search::metadata::parse_wiki_metadata;
use crate::search_models::{ADAPTER_SCHEMA_VERSION, ModelArtifacts, QMD_RS_VERSION, model_by_id};
use crate::search_profile::{SearchProfile, timestamp};

pub const SEMANTIC_INDEX_SCHEMA_VERSION: u32 = 1;
pub const CHUNK_SIZE_CHARS: usize = 3_200;
pub const CHUNK_OVERLAP_CHARS: usize = 480;
pub const CHUNKING_STRATEGY: &str = "qmd-rs-character-v1:3200:480";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticIndexMetadata {
    pub schema_version: u32,
    pub adapter_schema_version: u32,
    pub qmd_rs_version: String,
    pub project_id: String,
    pub generated_at: String,
    pub profile: Option<String>,
    pub embedding_model: String,
    pub query_expansion_model: Option<String>,
    pub reranker_model: Option<String>,
    pub model_artifacts: Vec<SemanticModelArtifact>,
    pub embedding_dimensions: usize,
    pub chunking_strategy: String,
    pub source_files: Vec<SemanticSourceFile>,
    pub chunks: Vec<SemanticChunk>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticModelArtifact {
    pub model_id: String,
    pub role: String,
    pub observed_sha256: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticSourceFile {
    pub path: String,
    pub content_hash: String,
    pub modified_unix_seconds: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticChunk {
    pub path: String,
    pub ordinal: usize,
    pub source_start: usize,
    pub source_end: usize,
    pub title: String,
    pub document_class: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub sources: Option<String>,
    pub source_content_hash: String,
    pub text_hash: String,
}

impl SemanticIndexMetadata {
    pub fn build(
        project_id: &str,
        wiki_root: &Path,
        profile: &SearchProfile,
        artifacts: &ModelArtifacts,
    ) -> Result<Self> {
        let embedding_model = profile
            .embedding_model
            .clone()
            .context("enabled LLM search profile is missing embedding_model")?;
        let embedding_dimensions = model_by_id(&embedding_model)
            .and_then(|model| model.dimensions)
            .context("embedding model dimensions are missing from the model catalog")?;
        let docs = collect_wiki_documents(wiki_root)?;
        let source_files = docs
            .iter()
            .map(|doc| SemanticSourceFile {
                path: doc.canonical_path.clone(),
                content_hash: doc.content_hash.clone(),
                modified_unix_seconds: doc.modified_unix_seconds,
            })
            .collect::<Vec<_>>();
        let mut chunks = Vec::new();
        for doc in docs {
            let body = fs::read_to_string(&doc.absolute_path)
                .with_context(|| format!("read {}", doc.absolute_path.display()))?;
            let metadata = parse_wiki_metadata(&body);
            let title = metadata
                .title
                .clone()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| doc.canonical_path.clone());
            for (ordinal, chunk) in
                qmd::chunk_document(&body, CHUNK_SIZE_CHARS, CHUNK_OVERLAP_CHARS)
                    .into_iter()
                    .enumerate()
            {
                chunks.push(SemanticChunk {
                    path: doc.canonical_path.clone(),
                    ordinal,
                    source_start: chunk.pos,
                    source_end: chunk.pos + chunk.text.len(),
                    title: title.clone(),
                    document_class: metadata.document_class().map(ToString::to_string),
                    status: metadata.status().map(ToString::to_string),
                    category: metadata.field("Category").map(ToString::to_string),
                    scope: metadata.field("Scope").map(ToString::to_string),
                    sources: metadata.field("Sources").map(ToString::to_string),
                    source_content_hash: doc.content_hash.clone(),
                    text_hash: sha256_bytes(chunk.text.as_bytes()),
                });
            }
        }

        Ok(Self {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            project_id: project_id.to_string(),
            generated_at: timestamp(),
            profile: profile.profile.clone(),
            embedding_model,
            query_expansion_model: profile.query_expansion_model.clone(),
            reranker_model: profile.reranker_model.clone(),
            model_artifacts: artifact_records(profile, artifacts)?,
            embedding_dimensions,
            chunking_strategy: CHUNKING_STRATEGY.to_string(),
            source_files,
            chunks,
        })
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("read semantic index metadata {}", path.display()))?;
        let metadata: Self = serde_json::from_str(&input)
            .with_context(|| format!("parse semantic index metadata {}", path.display()))?;
        if metadata.schema_version != SEMANTIC_INDEX_SCHEMA_VERSION {
            bail!(
                "unsupported semantic index schema_version {} in {}; expected {}",
                metadata.schema_version,
                path.display(),
                SEMANTIC_INDEX_SCHEMA_VERSION
            );
        }
        Ok(Some(metadata))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path.parent().with_context(|| {
            format!(
                "semantic index metadata path has no parent: {}",
                path.display()
            )
        })?;
        fs::create_dir_all(parent)
            .with_context(|| format!("create semantic index dir {}", parent.display()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).with_context(|| {
            format!(
                "create temp semantic index metadata in {}",
                parent.display()
            )
        })?;
        serde_json::to_writer_pretty(&mut temp, self)
            .context("serialize semantic index metadata")?;
        temp.persist(path)
            .map_err(|err| err.error)
            .with_context(|| format!("persist semantic index metadata {}", path.display()))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WikiDocument {
    absolute_path: PathBuf,
    canonical_path: String,
    content_hash: String,
    modified_unix_seconds: i64,
}

fn collect_wiki_documents(wiki_root: &Path) -> Result<Vec<WikiDocument>> {
    let wiki_root = fs::canonicalize(wiki_root)
        .with_context(|| format!("canonicalize wiki root {}", wiki_root.display()))?;
    let project_root = wiki_root.parent().unwrap_or(&wiki_root).to_path_buf();
    let mut docs = Vec::new();
    collect_markdown(&wiki_root, &project_root, &mut docs)?;
    docs.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    Ok(docs)
}

fn collect_markdown(path: &Path, project_root: &Path, docs: &mut Vec<WikiDocument>) -> Result<()> {
    for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, project_root, docs)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            let metadata = entry.metadata()?;
            let modified = unix_seconds(metadata.modified().unwrap_or(UNIX_EPOCH))?;
            let content_hash = sha256_file(&path)?;
            let canonical_path = path
                .strip_prefix(project_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            docs.push(WikiDocument {
                absolute_path: path,
                canonical_path,
                content_hash,
                modified_unix_seconds: modified,
            });
        }
    }
    Ok(())
}

fn artifact_records(
    profile: &SearchProfile,
    artifacts: &ModelArtifacts,
) -> Result<Vec<SemanticModelArtifact>> {
    let mut model_ids = Vec::new();
    if let Some(model_id) = &profile.embedding_model {
        model_ids.push(model_id);
    }
    if let Some(model_id) = &profile.query_expansion_model {
        model_ids.push(model_id);
    }
    if let Some(model_id) = &profile.reranker_model {
        model_ids.push(model_id);
    }

    let mut records = Vec::new();
    for model_id in model_ids {
        let record = artifacts
            .artifacts
            .iter()
            .find(|artifact| artifact.model_id == *model_id)
            .with_context(|| format!("model artifact record missing for {model_id}"))?;
        records.push(SemanticModelArtifact {
            model_id: record.model_id.clone(),
            role: record.role.clone(),
            observed_sha256: record.observed_sha256.clone(),
            path: record.path.clone(),
        });
    }
    records.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    Ok(records)
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn unix_seconds(time: SystemTime) -> Result<i64> {
    Ok(time.duration_since(UNIX_EPOCH)?.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::search_models::ModelArtifactRecord;

    use super::*;

    #[test]
    fn semantic_metadata_captures_chunks_and_profile_inputs() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index\n").expect("index");
        fs::write(wiki.join("log.md"), "# Log\n").expect("log");
        fs::write(
            wiki.join("decisions/search.decision.md"),
            "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n- Category: Search\n- Scope: Test\n- Sources: raw/test.md\n\n## Decision\nSemantic metadata records source spans.",
        )
        .expect("decision");
        let profile = SearchProfile::enabled(
            "balanced",
            "embeddinggemma-300m-q8_0",
            Some("qmd-query-expansion-1.7b-q4_k_m".to_string()),
            None,
        );
        let artifacts = ModelArtifacts::from_records(vec![
            artifact("embeddinggemma-300m-q8_0", "embedding"),
            artifact("qmd-query-expansion-1.7b-q4_k_m", "query-expansion"),
        ]);

        let metadata =
            SemanticIndexMetadata::build("fixture", &wiki, &profile, &artifacts).expect("metadata");

        assert_eq!(metadata.project_id, "fixture");
        assert_eq!(metadata.embedding_dimensions, 768);
        assert_eq!(metadata.model_artifacts.len(), 2);
        assert!(metadata.chunks.iter().any(|chunk| {
            chunk.path == "wiki/decisions/search.decision.md"
                && chunk.document_class.as_deref() == Some("Decision")
                && chunk.status.as_deref() == Some("Accepted")
        }));
    }

    fn artifact(model_id: &str, role: &str) -> ModelArtifactRecord {
        ModelArtifactRecord {
            model_id: model_id.to_string(),
            role: role.to_string(),
            profile: "balanced".to_string(),
            repository: "repo/model".to_string(),
            revision: "main".to_string(),
            file: "model.gguf".to_string(),
            download_url: "https://example.com/model.gguf".to_string(),
            path: PathBuf::from("/tmp/model.gguf"),
            expected_sha256: "hash".to_string(),
            observed_sha256: "hash".to_string(),
            size_bytes: 1,
            license: "mit".to_string(),
            terms_url: None,
            dimensions: None,
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            verified_at: timestamp(),
        }
    }
}
