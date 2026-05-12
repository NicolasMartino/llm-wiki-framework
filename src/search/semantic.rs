use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::search::adapter::{
    Freshness, MatchSpan, Score, SearchFilters, SearchMode, SearchResult,
};
use crate::search::index_text::mask_search_ignored_spans;
use crate::search::metadata::parse_wiki_metadata;
use crate::search_models::{
    ADAPTER_SCHEMA_VERSION, ModelArtifactRecord, ModelArtifacts, QMD_RS_VERSION,
    SearchThresholdStore, SearchThresholds, model_by_id,
};
use crate::search_profile::{SearchProfile, timestamp};

pub const SEMANTIC_INDEX_SCHEMA_VERSION: u32 = 1;
pub const SEMANTIC_VECTOR_SCHEMA_VERSION: u32 = 1;
pub const CHUNK_SIZE_CHARS: usize = 3_200;
pub const CHUNK_OVERLAP_CHARS: usize = 480;
pub const CHUNKING_STRATEGY: &str = "qmd-rs-character-v1:3200:480";
const TEST_EMBEDDINGS_ENV: &str = "LLM_WIKI_TEST_EMBEDDINGS";

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

#[derive(Clone, Debug)]
pub struct SemanticCorpusSnapshot {
    pub source_files: Vec<SemanticSourceFile>,
    pub chunks: Vec<SemanticChunk>,
    pub chunk_texts: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SemanticVectorIndex {
    pub schema_version: u32,
    pub adapter_schema_version: u32,
    pub qmd_rs_version: String,
    pub project_id: String,
    pub generated_at: String,
    pub embedding_model: String,
    pub embedding_artifact_sha256: String,
    pub embedding_dimensions: usize,
    pub chunking_strategy: String,
    pub source_fingerprint: String,
    pub vectors: Vec<SemanticVector>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SemanticVector {
    pub path: String,
    pub ordinal: usize,
    pub text_hash: String,
    pub embedding: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct SemanticSearchContext<'a> {
    pub project_id: &'a str,
    pub project_name: Option<&'a str>,
    pub wiki_root: &'a Path,
    pub query_embedding: &'a [f32],
    pub filters: &'a SearchFilters,
    pub limit: usize,
    pub floor: f64,
    pub freshness: Freshness,
    pub mode: SearchMode,
}

impl SemanticIndexMetadata {
    pub fn build(
        project_id: &str,
        wiki_root: &Path,
        profile: &SearchProfile,
        artifacts: &ModelArtifacts,
    ) -> Result<Self> {
        let corpus = Self::build_corpus(wiki_root)?;
        Self::from_corpus(project_id, profile, artifacts, &corpus)
    }

    pub fn build_corpus(wiki_root: &Path) -> Result<SemanticCorpusSnapshot> {
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
        let mut chunk_texts = Vec::new();
        for doc in docs {
            let raw_body = fs::read_to_string(&doc.absolute_path)
                .with_context(|| format!("read {}", doc.absolute_path.display()))?;
            let body = mask_search_ignored_spans(&raw_body);
            let metadata = parse_wiki_metadata(&body);
            let title = metadata
                .title
                .clone()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| doc.canonical_path.clone());
            for (ordinal, chunk) in chunk_document(&body, CHUNK_SIZE_CHARS, CHUNK_OVERLAP_CHARS)
                .into_iter()
                .enumerate()
            {
                let text_hash = sha256_bytes(chunk.text.as_bytes());
                chunk_texts.push(chunk.text.to_string());
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
                    text_hash,
                });
            }
        }
        Ok(SemanticCorpusSnapshot {
            source_files,
            chunks,
            chunk_texts,
        })
    }

    pub fn from_corpus(
        project_id: &str,
        profile: &SearchProfile,
        artifacts: &ModelArtifacts,
        corpus: &SemanticCorpusSnapshot,
    ) -> Result<Self> {
        let embedding_model = profile
            .embedding_model
            .clone()
            .context("enabled LLM search profile is missing embedding_model")?;
        let embedding_dimensions = model_by_id(&embedding_model)
            .and_then(|model| model.dimensions)
            .context("embedding model dimensions are missing from the model catalog")?;

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
            source_files: corpus.source_files.clone(),
            chunks: corpus.chunks.clone(),
        })
    }

    pub fn is_fresh(&self, wiki_root: &Path) -> Result<bool> {
        let current = collect_wiki_documents(wiki_root)?
            .into_iter()
            .map(|doc| SemanticSourceFile {
                path: doc.canonical_path,
                content_hash: doc.content_hash,
                modified_unix_seconds: doc.modified_unix_seconds,
            })
            .collect::<Vec<_>>();
        Ok(self.source_files == current)
    }

    pub fn source_fingerprint(&self) -> String {
        semantic_source_fingerprint(&self.source_files, &self.chunks)
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

impl SemanticVectorIndex {
    pub fn build(
        metadata: &SemanticIndexMetadata,
        wiki_root: &Path,
        embedding_artifact: &ModelArtifactRecord,
    ) -> Result<Self> {
        let mut embedder =
            SemanticEmbedder::new(&embedding_artifact.path, metadata.embedding_dimensions)?;
        let mut vectors = Vec::with_capacity(metadata.chunks.len());
        for chunk in &metadata.chunks {
            let text = chunk_text(wiki_root, chunk)?;
            let embedding = embedder.embed_document(&text, Some(&chunk.title))?;
            validate_embedding_dimensions(&embedding, metadata.embedding_dimensions)?;
            vectors.push(SemanticVector {
                path: chunk.path.clone(),
                ordinal: chunk.ordinal,
                text_hash: chunk.text_hash.clone(),
                embedding,
            });
        }

        Ok(Self {
            schema_version: SEMANTIC_VECTOR_SCHEMA_VERSION,
            adapter_schema_version: metadata.adapter_schema_version,
            qmd_rs_version: metadata.qmd_rs_version.clone(),
            project_id: metadata.project_id.clone(),
            generated_at: timestamp(),
            embedding_model: metadata.embedding_model.clone(),
            embedding_artifact_sha256: embedding_artifact.observed_sha256.clone(),
            embedding_dimensions: metadata.embedding_dimensions,
            chunking_strategy: metadata.chunking_strategy.clone(),
            source_fingerprint: metadata.source_fingerprint(),
            vectors,
        })
    }

    pub fn build_from_corpus(
        metadata: &SemanticIndexMetadata,
        corpus: &SemanticCorpusSnapshot,
        embedding_artifact: &ModelArtifactRecord,
    ) -> Result<Self> {
        if metadata.chunks.len() != corpus.chunk_texts.len() {
            bail!(
                "semantic corpus text count {} does not match chunk count {}",
                corpus.chunk_texts.len(),
                metadata.chunks.len()
            );
        }
        let mut embedder =
            SemanticEmbedder::new(&embedding_artifact.path, metadata.embedding_dimensions)?;
        let mut vectors = Vec::with_capacity(metadata.chunks.len());
        for (chunk, text) in metadata.chunks.iter().zip(corpus.chunk_texts.iter()) {
            let embedding = embedder.embed_document(text, Some(&chunk.title))?;
            validate_embedding_dimensions(&embedding, metadata.embedding_dimensions)?;
            vectors.push(SemanticVector {
                path: chunk.path.clone(),
                ordinal: chunk.ordinal,
                text_hash: chunk.text_hash.clone(),
                embedding,
            });
        }

        Ok(Self {
            schema_version: SEMANTIC_VECTOR_SCHEMA_VERSION,
            adapter_schema_version: metadata.adapter_schema_version,
            qmd_rs_version: metadata.qmd_rs_version.clone(),
            project_id: metadata.project_id.clone(),
            generated_at: timestamp(),
            embedding_model: metadata.embedding_model.clone(),
            embedding_artifact_sha256: embedding_artifact.observed_sha256.clone(),
            embedding_dimensions: metadata.embedding_dimensions,
            chunking_strategy: metadata.chunking_strategy.clone(),
            source_fingerprint: metadata.source_fingerprint(),
            vectors,
        })
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("read semantic vector index {}", path.display()))?;
        let index: Self = serde_json::from_str(&input)
            .with_context(|| format!("parse semantic vector index {}", path.display()))?;
        if index.schema_version != SEMANTIC_VECTOR_SCHEMA_VERSION {
            bail!(
                "unsupported semantic vector schema_version {} in {}; expected {}",
                index.schema_version,
                path.display(),
                SEMANTIC_VECTOR_SCHEMA_VERSION
            );
        }
        Ok(Some(index))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path.parent().with_context(|| {
            format!(
                "semantic vector index path has no parent: {}",
                path.display()
            )
        })?;
        fs::create_dir_all(parent)
            .with_context(|| format!("create semantic vector index dir {}", parent.display()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).with_context(|| {
            format!("create temp semantic vector index in {}", parent.display())
        })?;
        serde_json::to_writer(&mut temp, self).context("serialize semantic vector index")?;
        temp.persist(path)
            .map_err(|err| err.error)
            .with_context(|| format!("persist semantic vector index {}", path.display()))?;
        Ok(())
    }

    pub fn is_compatible(
        &self,
        metadata: &SemanticIndexMetadata,
        thresholds: &SearchThresholds,
    ) -> bool {
        self.schema_version == SEMANTIC_VECTOR_SCHEMA_VERSION
            && self.adapter_schema_version == ADAPTER_SCHEMA_VERSION
            && self.qmd_rs_version == QMD_RS_VERSION
            && self.project_id == metadata.project_id
            && self.embedding_model == metadata.embedding_model
            && self.embedding_model == thresholds.embedding_model
            && self.embedding_artifact_sha256 == thresholds.embedding_artifact_sha256
            && self.embedding_dimensions == metadata.embedding_dimensions
            && self.embedding_dimensions == thresholds.embedding_dimensions
            && self.chunking_strategy == metadata.chunking_strategy
            && self.chunking_strategy == thresholds.chunking_strategy
            && self.source_fingerprint == metadata.source_fingerprint()
            && self.vectors.len() == metadata.chunks.len()
    }

    pub fn search(
        &self,
        metadata: &SemanticIndexMetadata,
        context: SemanticSearchContext<'_>,
    ) -> Result<Vec<SearchResult>> {
        if context.limit == 0 || context.query_embedding.is_empty() {
            return Ok(Vec::new());
        }
        let chunks = metadata
            .chunks
            .iter()
            .map(|chunk| ((chunk.path.as_str(), chunk.ordinal), chunk))
            .collect::<BTreeMap<_, _>>();
        let mut rolled = BTreeMap::<String, SearchResult>::new();

        for vector in &self.vectors {
            if vector.embedding.len() != self.embedding_dimensions {
                bail!(
                    "semantic vector dimensions for {}#{} are {}; expected {}",
                    vector.path,
                    vector.ordinal,
                    vector.embedding.len(),
                    self.embedding_dimensions
                );
            }
            let score = f64::from(qmd::cosine_similarity(
                context.query_embedding,
                &vector.embedding,
            ));
            if score < context.floor {
                continue;
            }
            let Some(chunk) = chunks.get(&(vector.path.as_str(), vector.ordinal)) else {
                bail!(
                    "semantic vector {}#{} has no matching chunk metadata",
                    vector.path,
                    vector.ordinal
                );
            };
            if vector.text_hash != chunk.text_hash {
                bail!(
                    "semantic vector {}#{} text hash does not match metadata",
                    vector.path,
                    vector.ordinal
                );
            }
            if !context
                .filters
                .matches(chunk.document_class.as_deref(), chunk.status.as_deref())
            {
                continue;
            }

            let text = chunk_text(context.wiki_root, chunk)?;
            let candidate = SearchResult {
                project_id: context.project_id.to_string(),
                project_name: context.project_name.map(ToString::to_string),
                path: PathBuf::from(&chunk.path),
                title: chunk.title.clone(),
                document_class: chunk.document_class.clone(),
                status: chunk.status.clone(),
                score: Score(score),
                snippet: Some(snippet_text(&text)),
                match_span: Some(MatchSpan {
                    start: chunk.source_start,
                    end: chunk.source_end,
                }),
                backend: "qmd-rs-semantic".to_string(),
                mode: context.mode,
                freshness: context.freshness,
                lexical_rank: None,
                lexical_score: None,
                semantic_rank: None,
                semantic_score: None,
            };

            rolled
                .entry(chunk.path.clone())
                .and_modify(|existing| {
                    if score > existing.score.0 {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }

        let mut results = rolled.into_values().collect::<Vec<_>>();
        results.sort_by(|left, right| {
            right
                .score
                .0
                .total_cmp(&left.score.0)
                .then_with(|| left.path.cmp(&right.path))
        });
        results.truncate(context.limit);
        Ok(results)
    }
}

pub fn embed_query(
    query: &str,
    embedding_artifact: &ModelArtifactRecord,
    dimensions: usize,
) -> Result<Vec<f32>> {
    let mut embedder = SemanticEmbedder::new(&embedding_artifact.path, dimensions)?;
    let embedding = embedder.embed_query(query)?;
    validate_embedding_dimensions(&embedding, dimensions)?;
    Ok(embedding)
}

pub fn thresholds_match_index_inputs(
    thresholds: &SearchThresholds,
    metadata: &SemanticIndexMetadata,
    embedding_artifact: &ModelArtifactRecord,
) -> bool {
    let profile_matches = metadata
        .profile
        .as_deref()
        .is_none_or(|profile| thresholds.profile == profile);
    thresholds.qmd_rs_version == QMD_RS_VERSION
        && thresholds.adapter_schema_version == ADAPTER_SCHEMA_VERSION
        && profile_matches
        && thresholds.chunking_strategy == metadata.chunking_strategy
        && thresholds.embedding_model == metadata.embedding_model
        && thresholds.embedding_artifact_sha256 == embedding_artifact.observed_sha256
        && thresholds.embedding_dimensions == metadata.embedding_dimensions
}

pub fn select_thresholds_for_index<'a>(
    store: &'a SearchThresholdStore,
    project_id: &str,
    metadata: &SemanticIndexMetadata,
    embedding_artifact: &ModelArtifactRecord,
) -> Option<&'a SearchThresholds> {
    let allow_legacy_unscoped_fallback = !store
        .thresholds()
        .iter()
        .any(|thresholds| thresholds.project_id.is_some());
    let mut unscoped_match = None;
    for thresholds in store.thresholds() {
        if !thresholds_match_index_inputs(thresholds, metadata, embedding_artifact) {
            continue;
        }
        if thresholds.project_id.as_deref() == Some(project_id) {
            return Some(thresholds);
        }
        if allow_legacy_unscoped_fallback
            && thresholds.project_id.is_none()
            && unscoped_match.is_none()
        {
            unscoped_match = Some(thresholds);
        }
    }
    unscoped_match
}

enum SemanticEmbedder {
    Deterministic { dimensions: usize },
    Qmd(qmd::EmbeddingEngine),
}

impl SemanticEmbedder {
    fn new(model_path: &Path, dimensions: usize) -> Result<Self> {
        if env::var(TEST_EMBEDDINGS_ENV)
            .ok()
            .is_some_and(|value| value == "deterministic")
        {
            return Ok(Self::Deterministic { dimensions });
        }
        Ok(Self::Qmd(qmd::EmbeddingEngine::new(model_path)?))
    }

    fn embed_document(&mut self, text: &str, title: Option<&str>) -> Result<Vec<f32>> {
        match self {
            Self::Deterministic { dimensions } => Ok(deterministic_embedding(
                &format!("{} {text}", title.unwrap_or_default()),
                *dimensions,
            )),
            Self::Qmd(engine) => Ok(engine.embed_document(text, title)?.embedding),
        }
    }

    fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
        match self {
            Self::Deterministic { dimensions } => Ok(deterministic_embedding(query, *dimensions)),
            Self::Qmd(engine) => Ok(engine.embed_query(query)?.embedding),
        }
    }
}

fn validate_embedding_dimensions(embedding: &[f32], dimensions: usize) -> Result<()> {
    if embedding.len() != dimensions {
        bail!(
            "semantic embedding dimensions are {}; expected {}",
            embedding.len(),
            dimensions
        );
    }
    Ok(())
}

fn deterministic_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut values = vec![0.0_f32; dimensions.max(1)];
    for token in text
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
    {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let digest = hasher.finalize();
        let index = u64::from_le_bytes([
            digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
        ]) as usize
            % values.len();
        values[index] += 1.0;
    }
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut values {
            *value /= norm;
        }
    }
    values
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WikiDocument {
    absolute_path: PathBuf,
    canonical_path: String,
    content_hash: String,
    modified_unix_seconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TextChunk<'a> {
    pos: usize,
    text: &'a str,
}

fn chunk_document(text: &str, chunk_size_chars: usize, overlap_chars: usize) -> Vec<TextChunk<'_>> {
    if text.is_empty() || chunk_size_chars == 0 {
        return Vec::new();
    }

    let step_chars = chunk_size_chars.saturating_sub(overlap_chars).max(1);
    let mut char_boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    char_boundaries.push(text.len());

    let total_chars = char_boundaries.len() - 1;
    let mut chunks = Vec::new();
    let mut start_char = 0;
    while start_char < total_chars {
        let end_char = (start_char + chunk_size_chars).min(total_chars);
        let start = char_boundaries[start_char];
        let end = char_boundaries[end_char];
        chunks.push(TextChunk {
            pos: start,
            text: &text[start..end],
        });
        if end_char == total_chars {
            break;
        }
        start_char += step_chars;
    }

    chunks
}

fn chunk_text(wiki_root: &Path, chunk: &SemanticChunk) -> Result<String> {
    let project_root = wiki_root.parent().unwrap_or(wiki_root);
    let path = project_root.join(&chunk.path);
    let raw_body = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let body = mask_search_ignored_spans(&raw_body);
    let text = body
        .get(chunk.source_start..chunk.source_end)
        .with_context(|| {
            format!(
                "semantic chunk span {}..{} is invalid for {}",
                chunk.source_start,
                chunk.source_end,
                path.display()
            )
        })?;
    Ok(text.to_string())
}

fn snippet_text(text: &str) -> String {
    const MAX_CHARS: usize = 220;
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= MAX_CHARS {
        return normalized;
    }
    normalized.chars().take(MAX_CHARS).collect::<String>()
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

fn semantic_source_fingerprint(sources: &[SemanticSourceFile], chunks: &[SemanticChunk]) -> String {
    let mut hasher = Sha256::new();
    for source in sources {
        hasher.update(source.path.as_bytes());
        hasher.update(source.content_hash.as_bytes());
        hasher.update(source.modified_unix_seconds.to_le_bytes());
    }
    for chunk in chunks {
        hasher.update(chunk.path.as_bytes());
        hasher.update(chunk.ordinal.to_le_bytes());
        hasher.update(chunk.source_start.to_le_bytes());
        hasher.update(chunk.source_end.to_le_bytes());
        hasher.update(chunk.text_hash.as_bytes());
    }
    format!("{:x}", hasher.finalize())
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

    #[test]
    fn chunk_document_uses_utf8_boundary_spans() {
        let text = "abc│def🙂ghi";
        let chunks = chunk_document(text, 5, 1);

        assert_eq!(chunks.len(), 3);
        for chunk in chunks {
            let end = chunk.pos + chunk.text.len();
            assert!(text.is_char_boundary(chunk.pos));
            assert!(text.is_char_boundary(end));
            assert_eq!(&text[chunk.pos..end], chunk.text);
        }
    }

    #[test]
    fn vector_index_search_rolls_chunks_up_to_documents() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index\n").expect("index");
        fs::write(wiki.join("log.md"), "# Log\n").expect("log");
        fs::write(
            wiki.join("decisions/search.decision.md"),
            "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n\n## Decision\nBattery chemistry roadmap retrieval belongs in semantic search.",
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
        let embedding = deterministic_embedding("battery chemistry roadmap", 768);
        let query_embedding = deterministic_embedding("battery roadmap", 768);
        let matching_chunk = metadata
            .chunks
            .iter()
            .find(|chunk| chunk.path == "wiki/decisions/search.decision.md")
            .expect("decision chunk");
        let index = SemanticVectorIndex {
            schema_version: SEMANTIC_VECTOR_SCHEMA_VERSION,
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            project_id: "fixture".to_string(),
            generated_at: timestamp(),
            embedding_model: "embeddinggemma-300m-q8_0".to_string(),
            embedding_artifact_sha256: "hash".to_string(),
            embedding_dimensions: 768,
            chunking_strategy: CHUNKING_STRATEGY.to_string(),
            source_fingerprint: metadata.source_fingerprint(),
            vectors: vec![SemanticVector {
                path: matching_chunk.path.clone(),
                ordinal: matching_chunk.ordinal,
                text_hash: matching_chunk.text_hash.clone(),
                embedding,
            }],
        };

        let results = index
            .search(
                &metadata,
                SemanticSearchContext {
                    project_id: "fixture",
                    project_name: Some("fixture"),
                    wiki_root: &wiki,
                    query_embedding: &query_embedding,
                    filters: &Default::default(),
                    limit: 5,
                    floor: 0.1,
                    freshness: Freshness::Fresh,
                    mode: SearchMode::Semantic,
                },
            )
            .expect("search");

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].path.to_string_lossy(),
            "wiki/decisions/search.decision.md"
        );
        assert_eq!(results[0].mode, SearchMode::Semantic);
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
