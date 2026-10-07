use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use llm_wiki_core::page::Page;
use qmd::Store;
use rusqlite::{Connection, OpenFlags, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::search::adapter::{
    BackendAccessError, BackendOpenMode, BackendState, BackendStatus, Freshness, IndexOptions,
    MatchSpan, Score, SearchBackend, SearchFilters, SearchMode, SearchResult,
};
use crate::search::index_text::mask_search_ignored_spans;
use crate::search::sanitize::{query_terms, sanitize_fts_query};

const BACKEND_NAME: &str = "qmd-rs";
const COLLECTION_FALLBACK: &str = "project";
const SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, Default)]
pub struct QmdRsBackend;

impl QmdRsBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SearchBackend for QmdRsBackend {
    fn index_project(
        &self,
        project_id: &str,
        wiki_root: &Path,
        store_path: &Path,
        options: &IndexOptions,
    ) -> Result<BackendStatus> {
        if options.force {
            remove_store_files(store_path)?;
        }

        let docs = collect_wiki_documents(wiki_root)?;
        let snapshot = WikiSnapshot::from_documents(&docs)?;
        let store = Store::open(store_path)
            .with_context(|| format!("open qmd-rs store {}", store_path.display()))?;
        let collection = collection_name(project_id);

        for doc in &docs {
            let raw_body = fs::read_to_string(&doc.absolute_path)
                .with_context(|| format!("read {}", doc.absolute_path.display()))?;
            let body = mask_search_ignored_spans(&raw_body);
            let metadata = Page::read(&body).wiki_view();
            let title = metadata
                .title()
                .filter(|value| !value.is_empty())
                .map_or_else(|| Store::extract_title(&body), ToString::to_string);
            let hash = Store::hash_content(&body);
            let modified_at = rfc3339(doc.modified)?;
            store.insert_content(&hash, &body, &modified_at)?;
            store.insert_document(
                &collection,
                &doc.canonical_path,
                &title,
                &hash,
                &modified_at,
                &modified_at,
            )?;
        }

        StoreMetadata {
            schema_version: SCHEMA_VERSION,
            backend: BACKEND_NAME.to_string(),
            project_id: collection,
            file_count: snapshot.file_count(),
            max_modified_unix_seconds: snapshot.max_modified_unix_seconds(),
            files: snapshot.files,
        }
        .write(store_path)?;

        drop(store);
        checkpoint_store(store_path)?;
        let mut status = status_for_store(
            project_id,
            store_path,
            wiki_root,
            StatusPurpose::CandidateProof,
        )?;
        if matches!(status.state, BackendState::Ready) {
            status.open_mode = BackendOpenMode::ReadWriteIndexing;
        }
        Ok(status)
    }

    fn search_project(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let sanitized = sanitize_fts_query(query);
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }

        let status = status_for_store(project_id, store_path, wiki_root, StatusPurpose::LiveRead)?;
        if !matches!(status.state, BackendState::Ready | BackendState::Stale) {
            return Err(BackendAccessError::new(status).into());
        }
        let freshness = freshness_for_status(&status);
        let terms = query_terms(query);
        let filter_active = filters.is_active();

        // A fixed overfetch window can silently drop class/status matches that rank
        // below it. When a filter is active, grow the fetch window and re-query
        // until we have `limit` post-filter results or the store is exhausted
        // (fewer raw rows than requested). Cap growth to avoid unbounded loops.
        const MAX_WINDOW: usize = 100_000;
        let mut window = limit.saturating_mul(4).max(20);
        let mut results = Vec::new();
        loop {
            let raw_results =
                immutable_search_fts(store_path, project_id, &sanitized, window, &status)?;
            let raw_len = raw_results.len();
            results.clear();

            for raw in raw_results {
                let metadata = Page::read(&raw.body).wiki_view();
                let document_class = metadata.document_class().map(ToString::to_string);
                let doc_status = metadata.status().map(ToString::to_string);
                if !filters.matches(document_class.as_deref(), doc_status.as_deref()) {
                    continue;
                }

                let match_span = find_match_span(&raw.body, &terms);
                let snippet = match_span.map(|span| snippet(&raw.body, span));
                results.push(SearchResult {
                    project_id: raw.collection.clone(),
                    project_name: None,
                    path: PathBuf::from(&raw.path),
                    title: metadata
                        .title()
                        .filter(|value| !value.is_empty())
                        .map_or(raw.title, ToString::to_string),
                    document_class,
                    status: doc_status,
                    score: Score(raw.score),
                    snippet,
                    match_span,
                    backend: BACKEND_NAME.to_string(),
                    mode: SearchMode::Fts,
                    freshness,
                    lexical_rank: None,
                    lexical_score: None,
                    semantic_rank: None,
                    semantic_score: None,
                });

                if results.len() == limit {
                    break;
                }
            }

            // Stop when we have enough, when no filter can drop rows, when the store
            // is exhausted (fewer rows returned than requested), or when capped.
            if results.len() >= limit || !filter_active || raw_len < window || window >= MAX_WINDOW
            {
                break;
            }
            window = window.saturating_mul(2).min(MAX_WINDOW);
        }

        Ok(results)
    }

    fn status(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
    ) -> Result<BackendStatus> {
        status_for_store(project_id, store_path, wiki_root, StatusPurpose::LiveRead)
    }

    fn doctor(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
        mode: SearchMode,
    ) -> Result<BackendStatus> {
        let mut status =
            status_for_store(project_id, store_path, wiki_root, StatusPurpose::LiveRead)?;
        if matches!(mode, SearchMode::Semantic | SearchMode::Hybrid)
            && matches!(status.state, BackendState::Ready)
        {
            status.message = Some("semantic model readiness is reported separately".to_string());
        }
        Ok(status)
    }

    fn rebuild_or_recover(
        &self,
        project_id: &str,
        wiki_root: &Path,
        store_path: &Path,
        force: bool,
    ) -> Result<BackendStatus> {
        self.index_project(project_id, wiki_root, store_path, &IndexOptions { force })
    }
}

#[derive(Clone, Debug)]
struct WikiDocument {
    absolute_path: PathBuf,
    canonical_path: String,
    modified: SystemTime,
    content_hash: String,
    indexed_hash: String,
}

/// Two snapshots are equal iff they contain the same canonical paths, every
/// path has the same `content_hash`, and every path has the same
/// `indexed_hash` and `modified_unix_seconds`. The hashes catch same-second
/// content edits and prove sqlite rows against metadata; the mtime catches
/// metadata-only rewrites that preserve content.
#[derive(Clone, Debug, Eq, PartialEq)]
struct WikiSnapshot {
    files: Vec<FileSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct FileSnapshot {
    path: String,
    content_hash: String,
    #[serde(default)]
    indexed_hash: String,
    modified_unix_seconds: i64,
}

impl WikiSnapshot {
    fn from_documents(docs: &[WikiDocument]) -> Result<Self> {
        let files = docs
            .iter()
            .map(|doc| {
                Ok(FileSnapshot {
                    path: doc.canonical_path.clone(),
                    content_hash: doc.content_hash.clone(),
                    indexed_hash: doc.indexed_hash.clone(),
                    modified_unix_seconds: unix_seconds(doc.modified)?,
                })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .collect();
        Ok(Self { files })
    }

    fn file_count(&self) -> usize {
        self.files.len()
    }

    fn max_modified_unix_seconds(&self) -> i64 {
        self.files
            .iter()
            .map(|file| file.modified_unix_seconds)
            .max()
            .unwrap_or(0)
    }
}

fn freshness_for_status(status: &BackendStatus) -> Freshness {
    match status.state {
        BackendState::Ready => Freshness::Fresh,
        BackendState::Stale => Freshness::Stale,
        _ => Freshness::Unknown,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatusPurpose {
    LiveRead,
    CandidateProof,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoreMetadata {
    schema_version: u32,
    backend: String,
    project_id: String,
    file_count: usize,
    max_modified_unix_seconds: i64,
    files: Vec<FileSnapshot>,
}

impl StoreMetadata {
    fn read(store_path: &Path) -> Result<Self> {
        let path = metadata_path(store_path);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("read qmd-rs metadata {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("parse qmd-rs metadata {}", path.display()))
    }

    fn write(&self, store_path: &Path) -> Result<()> {
        let path = metadata_path(store_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("write qmd-rs metadata {}", path.display()))
    }

    fn snapshot(&self) -> WikiSnapshot {
        WikiSnapshot {
            files: self.files.clone(),
        }
    }
}

fn status_for_store(
    project_id: &str,
    store_path: &Path,
    wiki_root: &Path,
    purpose: StatusPurpose,
) -> Result<BackendStatus> {
    let paths = RelatedStorePaths::new(store_path);
    let sqlite_exists = match try_path_exists(&paths.sqlite) {
        Ok(exists) => exists,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(status(
                store_path,
                BackendState::PermissionDenied,
                BackendOpenMode::NotOpened,
                0,
                false,
                "qmd-rs sqlite file cannot be inspected",
            ));
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspect qmd-rs sqlite file {}", paths.sqlite.display()));
        }
    };
    let metadata_exists = match try_path_exists(&paths.metadata) {
        Ok(exists) => exists,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(status(
                store_path,
                BackendState::PermissionDenied,
                BackendOpenMode::NotOpened,
                0,
                false,
                "qmd-rs metadata file cannot be inspected",
            ));
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!("inspect qmd-rs metadata file {}", paths.metadata.display())
            });
        }
    };

    match (sqlite_exists, metadata_exists) {
        (false, false) => {
            return Ok(status(
                store_path,
                BackendState::Missing,
                BackendOpenMode::NotOpened,
                0,
                false,
                "qmd-rs store is missing",
            ));
        }
        (true, false) | (false, true) => {
            return Ok(status(
                store_path,
                BackendState::Transient,
                BackendOpenMode::NotOpened,
                0,
                false,
                "qmd-rs store publication is incomplete; retry the read",
            ));
        }
        (true, true) => {}
    }

    let metadata = match StoreMetadata::read(store_path) {
        Ok(metadata) => metadata,
        Err(error) if is_permission_error(&error) => {
            return Ok(status(
                store_path,
                BackendState::PermissionDenied,
                BackendOpenMode::NotOpened,
                0,
                false,
                "qmd-rs metadata file cannot be read",
            ));
        }
        Err(error) => {
            return Ok(status_with_message(
                store_path,
                BackendState::SchemaMismatch,
                BackendOpenMode::NotOpened,
                0,
                false,
                error.to_string(),
            ));
        }
    };

    if metadata.schema_version != SCHEMA_VERSION || metadata.backend != BACKEND_NAME {
        return Ok(status(
            store_path,
            BackendState::SchemaMismatch,
            BackendOpenMode::NotOpened,
            metadata.file_count,
            false,
            "qmd-rs metadata schema does not match this binary",
        ));
    }
    if metadata.project_id != collection_name(project_id) {
        return Ok(status_with_message(
            store_path,
            BackendState::SchemaMismatch,
            BackendOpenMode::NotOpened,
            metadata.file_count,
            false,
            format!(
                "qmd-rs metadata project id {} does not match expected {}",
                metadata.project_id,
                collection_name(project_id)
            ),
        ));
    }

    if let Err(classification) = verify_immutable_schema(store_path, &metadata) {
        return Ok(status_with_message(
            store_path,
            classification.state,
            classification.open_mode,
            metadata.file_count,
            false,
            classification.message,
        ));
    }

    let docs = collect_wiki_documents(wiki_root)?;
    let current = WikiSnapshot::from_documents(&docs)?;
    let stale = current != metadata.snapshot();
    let state = if stale {
        BackendState::Stale
    } else {
        BackendState::Ready
    };
    if purpose == StatusPurpose::CandidateProof && stale {
        return Ok(status(
            store_path,
            BackendState::SchemaMismatch,
            BackendOpenMode::ReadOnlyImmutable,
            metadata.file_count,
            true,
            "candidate qmd-rs store does not match the current wiki snapshot",
        ));
    }
    Ok(BackendStatus {
        store_path: store_path.to_path_buf(),
        state,
        open_mode: BackendOpenMode::ReadOnlyImmutable,
        indexed_files: metadata.file_count,
        stale,
        message: None,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RelatedStorePaths {
    sqlite: PathBuf,
    metadata: PathBuf,
}

impl RelatedStorePaths {
    fn new(store_path: &Path) -> Self {
        Self {
            sqlite: store_path.to_path_buf(),
            metadata: metadata_path(store_path),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoreClassification {
    state: BackendState,
    open_mode: BackendOpenMode,
    message: String,
}

#[derive(Clone, Debug)]
struct ImmutableFtsRow {
    collection: String,
    path: String,
    title: String,
    score: f64,
    body: String,
}

fn status(
    store_path: &Path,
    state: BackendState,
    open_mode: BackendOpenMode,
    indexed_files: usize,
    stale: bool,
    message: &str,
) -> BackendStatus {
    status_with_message(
        store_path,
        state,
        open_mode,
        indexed_files,
        stale,
        message.to_string(),
    )
}

fn status_with_message(
    store_path: &Path,
    state: BackendState,
    open_mode: BackendOpenMode,
    indexed_files: usize,
    stale: bool,
    message: String,
) -> BackendStatus {
    BackendStatus {
        store_path: store_path.to_path_buf(),
        state,
        open_mode,
        indexed_files,
        stale,
        message: Some(message),
    }
}

fn try_path_exists(path: &Path) -> io::Result<bool> {
    path.try_exists()
}

fn is_permission_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io_error| io_error.kind() == io::ErrorKind::PermissionDenied)
    })
}

fn checkpoint_store(store_path: &Path) -> Result<()> {
    let conn = Connection::open(store_path)
        .with_context(|| format!("open qmd-rs store for checkpoint {}", store_path.display()))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .with_context(|| format!("checkpoint qmd-rs store {}", store_path.display()))?;
    Ok(())
}

fn verify_immutable_schema(
    store_path: &Path,
    metadata: &StoreMetadata,
) -> std::result::Result<(), StoreClassification> {
    let conn = open_immutable_connection(store_path)?;
    let required = ["content", "documents", "documents_fts"];
    for table in required {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                params![table],
                |row| row.get(0),
            )
            .map_err(|error| classify_sqlite_read_error(error, store_path))?;
        if count == 0 {
            return Err(StoreClassification {
                state: BackendState::SchemaMismatch,
                open_mode: BackendOpenMode::ReadOnlyImmutable,
                message: format!("qmd-rs sqlite schema is missing {table}"),
            });
        }
    }
    let active_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE collection = ?1 AND active = 1",
            params![&metadata.project_id],
            |row| row.get(0),
        )
        .map_err(|error| classify_sqlite_read_error(error, store_path))?;
    if active_count as usize != metadata.file_count {
        return Err(classify_mismatch_after_metadata_reread(
            store_path,
            metadata,
            format!(
                "qmd-rs sqlite active document count {active_count} does not match metadata file count {}",
                metadata.file_count
            ),
        ));
    }
    let mut stmt = conn
        .prepare(
            "SELECT path, hash FROM documents WHERE collection = ?1 AND active = 1 ORDER BY path",
        )
        .map_err(|error| classify_sqlite_read_error(error, store_path))?;
    let active_rows = stmt
        .query_map(params![&metadata.project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| classify_sqlite_read_error(error, store_path))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| classify_sqlite_read_error(error, store_path))?;
    let mut expected_rows = metadata
        .files
        .iter()
        .map(|file| (file.path.clone(), file.indexed_hash.clone()))
        .collect::<Vec<_>>();
    expected_rows.sort_by(|left, right| left.0.cmp(&right.0));
    if active_rows != expected_rows {
        return Err(classify_mismatch_after_metadata_reread(
            store_path,
            metadata,
            "qmd-rs sqlite active document rows do not match metadata snapshot".to_string(),
        ));
    }
    Ok(())
}

fn classify_mismatch_after_metadata_reread(
    store_path: &Path,
    original_metadata: &StoreMetadata,
    mismatch_message: String,
) -> StoreClassification {
    match StoreMetadata::read(store_path) {
        Ok(current_metadata) if current_metadata != *original_metadata => StoreClassification {
            state: BackendState::Transient,
            open_mode: BackendOpenMode::ReadOnlyImmutable,
            message: "qmd-rs metadata changed while sqlite was being verified; retry the read"
                .to_string(),
        },
        Err(_) => StoreClassification {
            state: BackendState::Transient,
            open_mode: BackendOpenMode::ReadOnlyImmutable,
            message: "qmd-rs metadata could not be re-read while sqlite was being verified; retry the read"
                .to_string(),
        },
        _ => StoreClassification {
            state: BackendState::SchemaMismatch,
            open_mode: BackendOpenMode::ReadOnlyImmutable,
            message: mismatch_message,
        },
    }
}

fn immutable_search_fts(
    store_path: &Path,
    project_id: &str,
    query: &str,
    limit: usize,
    last_status: &BackendStatus,
) -> Result<Vec<ImmutableFtsRow>> {
    let conn = open_immutable_connection(store_path).map_err(|classification| {
        BackendAccessError::new(status_with_message(
            store_path,
            classification.state,
            classification.open_mode,
            last_status.indexed_files,
            last_status.stale,
            format!(
                "qmd-rs immutable read failed for {}: {}",
                store_path.display(),
                classification.message
            ),
        ))
    })?;
    let collection = collection_name(project_id);
    let mut stmt = conn
        .prepare(
            r"
            SELECT
                d.collection,
                d.path,
                d.title,
                bm25(documents_fts) as score,
                c.doc
            FROM documents_fts fts
            JOIN documents d ON d.id = fts.rowid
            JOIN content c ON c.hash = d.hash
            WHERE documents_fts MATCH ?1
              AND d.collection = ?2
              AND d.active = 1
            ORDER BY score
            LIMIT ?3
            ",
        )
        .with_context(|| {
            format!(
                "prepare immutable qmd-rs FTS query {}",
                store_path.display()
            )
        })?;
    let rows = stmt
        .query_map(params![query, collection, limit as i64], |row| {
            let score: f64 = row.get(3)?;
            Ok(ImmutableFtsRow {
                collection: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                score: -score,
                body: row.get(4)?,
            })
        })
        .with_context(|| format!("run immutable qmd-rs FTS query {}", store_path.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("read immutable qmd-rs FTS rows {}", store_path.display()))?;
    Ok(rows)
}

fn open_immutable_connection(
    store_path: &Path,
) -> std::result::Result<Connection, StoreClassification> {
    let uri = immutable_sqlite_uri(store_path);
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    Connection::open_with_flags(&uri, flags)
        .map_err(|error| classify_sqlite_open_error(error, store_path))
}

fn classify_sqlite_open_error(error: rusqlite::Error, store_path: &Path) -> StoreClassification {
    let message = error.to_string();
    let state = match &error {
        rusqlite::Error::SqliteFailure(inner, _) => match inner.code {
            rusqlite::ErrorCode::CannotOpen => classify_cannot_open_state(store_path),
            rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase => {
                BackendState::Corrupt
            }
            _ => BackendState::Corrupt,
        },
        _ => BackendState::Corrupt,
    };
    StoreClassification {
        state,
        open_mode: BackendOpenMode::ReadOnlyImmutable,
        message: format!(
            "qmd-rs immutable sqlite open failed for {}: {message}",
            store_path.display()
        ),
    }
}

fn classify_cannot_open_state(store_path: &Path) -> BackendState {
    let paths = RelatedStorePaths::new(store_path);
    let sqlite_exists = try_path_exists(&paths.sqlite);
    let metadata_exists = try_path_exists(&paths.metadata);

    if sqlite_exists
        .as_ref()
        .is_err_and(|error| error.kind() == io::ErrorKind::PermissionDenied)
        || metadata_exists
            .as_ref()
            .is_err_and(|error| error.kind() == io::ErrorKind::PermissionDenied)
    {
        return BackendState::PermissionDenied;
    }

    match (sqlite_exists.ok(), metadata_exists.ok()) {
        (Some(true), Some(true)) => match fs::File::open(&paths.sqlite) {
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                BackendState::PermissionDenied
            }
            _ => BackendState::Transient,
        },
        _ => BackendState::Transient,
    }
}

fn classify_sqlite_read_error(error: rusqlite::Error, store_path: &Path) -> StoreClassification {
    let message = error.to_string();
    let state = match &error {
        rusqlite::Error::SqliteFailure(inner, _) => match inner.code {
            rusqlite::ErrorCode::PermissionDenied => BackendState::PermissionDenied,
            rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase => {
                BackendState::Corrupt
            }
            _ => BackendState::Corrupt,
        },
        _ => BackendState::Corrupt,
    };
    StoreClassification {
        state,
        open_mode: BackendOpenMode::ReadOnlyImmutable,
        message: format!(
            "qmd-rs immutable sqlite read failed for {}: {message}",
            store_path.display()
        ),
    }
}

fn immutable_sqlite_uri(store_path: &Path) -> String {
    format!(
        "file:{}?mode=ro&immutable=1",
        percent_encode_path(&store_path.to_string_lossy())
    )
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'.' | b'_' | b'-' | b':' | b'\\' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Classification of a directory entry during the markdown walk.
pub(crate) enum WalkEntry {
    /// A directory to descend into.
    Directory,
    /// A markdown file to index.
    Markdown,
    /// Anything to ignore (non-markdown, or a symlink that escapes the root or
    /// points at a directory).
    Skip,
}

/// Classifies a directory entry for the markdown walk while refusing to follow
/// symlinks that escape `canonical_root` (scope guard) and never descending into
/// symlinked directories (cycle guard) — mirroring wiki_read's symlink policy.
///
/// Shared by [`collect_markdown`] here and the copy in `semantic.rs`; keep both
/// walk call sites in sync.
pub(crate) fn classify_walk_entry(path: &Path, canonical_root: &Path) -> Result<WalkEntry> {
    let link_meta =
        fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if link_meta.file_type().is_symlink() {
        // Only index symlink targets that stay within the canonical root, and never
        // recurse through a symlinked directory (prevents escapes and cycles).
        let Ok(canonical) = fs::canonicalize(path) else {
            return Ok(WalkEntry::Skip);
        };
        if !canonical.starts_with(canonical_root) || canonical.is_dir() {
            return Ok(WalkEntry::Skip);
        }
        if is_markdown_path(&canonical) {
            return Ok(WalkEntry::Markdown);
        }
        return Ok(WalkEntry::Skip);
    }
    if link_meta.is_dir() {
        return Ok(WalkEntry::Directory);
    }
    if is_markdown_path(path) {
        return Ok(WalkEntry::Markdown);
    }
    Ok(WalkEntry::Skip)
}

pub(crate) fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

fn collect_wiki_documents(wiki_root: &Path) -> Result<Vec<WikiDocument>> {
    let wiki_root = fs::canonicalize(wiki_root)
        .with_context(|| format!("canonicalize wiki root {}", wiki_root.display()))?;
    let project_root = wiki_root.parent().unwrap_or(&wiki_root).to_path_buf();
    let mut docs = Vec::new();
    collect_markdown(&wiki_root, &project_root, &wiki_root, &mut docs)?;
    docs.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    Ok(docs)
}

fn collect_markdown(
    path: &Path,
    project_root: &Path,
    canonical_root: &Path,
    docs: &mut Vec<WikiDocument>,
) -> Result<()> {
    for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
        let entry = entry?;
        let path = entry.path();
        match classify_walk_entry(&path, canonical_root)? {
            WalkEntry::Directory => collect_markdown(&path, project_root, canonical_root, docs)?,
            WalkEntry::Skip => {}
            WalkEntry::Markdown => {
                let metadata = entry.metadata()?;
                let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
                let raw_body = fs::read_to_string(&path)
                    .with_context(|| format!("read {}", path.display()))?;
                let content_hash = hash_bytes(raw_body.as_bytes());
                let indexed_hash = Store::hash_content(&mask_search_ignored_spans(&raw_body));
                let canonical_path = path
                    .strip_prefix(project_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/");
                docs.push(WikiDocument {
                    absolute_path: path,
                    canonical_path,
                    modified,
                    content_hash,
                    indexed_hash,
                });
            }
        }
    }
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn find_match_span(body: &str, terms: &[String]) -> Option<MatchSpan> {
    terms
        .iter()
        .filter(|term| !term.is_empty())
        .find_map(|term| find_lowercase_term_span(body, term))
}

fn find_lowercase_term_span(body: &str, term: &str) -> Option<MatchSpan> {
    if term.is_empty() {
        return None;
    }

    let starts = body
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(body.len()));

    for start in starts {
        let mut probe = String::new();
        for (offset, ch) in body[start..].char_indices() {
            probe.extend(ch.to_lowercase());
            if probe == term {
                return Some(MatchSpan {
                    start,
                    end: start + offset + ch.len_utf8(),
                });
            }
            if !term.starts_with(&probe) || probe.len() > term.len() {
                break;
            }
        }
    }
    None
}

fn snippet(body: &str, span: MatchSpan) -> String {
    let start = body[..span.start]
        .char_indices()
        .rev()
        .nth(80)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let end = body[span.end..]
        .char_indices()
        .nth(80)
        .map(|(index, _)| span.end + index)
        .unwrap_or(body.len());
    body[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rfc3339(time: SystemTime) -> Result<String> {
    Ok(DateTime::<Utc>::from(time).to_rfc3339())
}

fn unix_seconds(time: SystemTime) -> Result<i64> {
    // Clamp pre-1970 mtimes to 0, matching the UNIX_EPOCH fallback elsewhere.
    Ok(time
        .duration_since(UNIX_EPOCH)
        .map(|delta| delta.as_secs() as i64)
        .unwrap_or(0))
}

fn collection_name(project_id: &str) -> String {
    if project_id.trim().is_empty() {
        COLLECTION_FALLBACK.to_string()
    } else {
        project_id.to_string()
    }
}

fn metadata_path(store_path: &Path) -> PathBuf {
    store_path.with_extension("llm-wiki.json")
}

fn remove_store_files(store_path: &Path) -> Result<()> {
    for path in [
        store_path.to_path_buf(),
        metadata_path(store_path),
        store_path.with_extension("sqlite-wal"),
        store_path.with_extension("sqlite-shm"),
    ] {
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FileSnapshot, QmdRsBackend, StoreMetadata, WikiSnapshot, classify_cannot_open_state,
        find_match_span, snippet, verify_immutable_schema,
    };
    use crate::search::adapter::{
        BackendAccessError, BackendOpenMode, BackendState, Freshness, IndexOptions, SearchBackend,
        SearchFilters, SearchMode,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn indexes_searches_filters_and_reports_staleness() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");
        fs::write(
                wiki.join("decisions/search.decision.md"),
                "# Search Backend Selection\n\n- Document Class: Decision\n- Status: Accepted\n- Date: 2026-05-07\n- Category: Search\n- Scope: qmd-rs adapter\n- Sources: raw/search.md\n\n## Choice\nqmd-rs powers search-all quality without exposing qmd docids.",
            )
            .expect("decision");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        let indexed = backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        assert_eq!(indexed.state, BackendState::Ready);
        assert_eq!(indexed.indexed_files, 3);

        let results = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "qmd-rs search-all",
                &SearchFilters {
                    document_class: Some("decision".to_string()),
                    status: Some("accepted".to_string()),
                },
                5,
            )
            .expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].path.to_string_lossy(),
            "wiki/decisions/search.decision.md"
        );
        assert_eq!(results[0].document_class.as_deref(), Some("Decision"));
        assert_eq!(results[0].status.as_deref(), Some("Accepted"));
        assert_eq!(results[0].freshness, Freshness::Fresh);
        assert!(
            results[0]
                .snippet
                .as_deref()
                .is_some_and(|s| s.contains("qmd-rs"))
        );

        let ready = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(ready.state, BackendState::Ready);
        assert_eq!(ready.open_mode, BackendOpenMode::ReadOnlyImmutable);

        fs::create_dir_all(wiki.join("plans")).expect("plans dir");
        fs::write(
                wiki.join("plans/search.plan.md"),
                "# Search Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nMore search work.",
            )
            .expect("plan");
        let stale = backend
            .doctor("fixture", &store, &wiki, SearchMode::Fts)
            .expect("doctor");
        assert_eq!(stale.state, BackendState::Stale);

        let stale_results = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "qmd-rs search-all",
                &SearchFilters::default(),
                5,
            )
            .expect("stale search");
        assert!(
            stale_results
                .iter()
                .any(|result| result.freshness == Freshness::Stale)
        );
    }

    #[test]
    fn staleness_tracks_wiki_markdown_file_snapshots() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        fs::write(wiki.join(".directory-marker"), "parent mtime only").expect("marker");
        let ready = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(ready.state, BackendState::Ready);

        fs::write(
            wiki.join("index.md"),
            "# Index\n\n## Schema Drift\n\nLatest drift: 2026-05-14\n",
        )
        .expect("index");
        fs::write(
            wiki.join("log.md"),
            "# Log\n\n## [2026-05-14] init | schema drift | generic -> web-product\n",
        )
        .expect("log");
        let stale = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(stale.state, BackendState::Stale);
    }

    #[cfg(unix)]
    #[test]
    fn immutable_reader_searches_completed_store_without_write_access() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");
        fs::write(
            wiki.join("decisions/search.decision.md"),
            "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n\nImmutable cache reads work.",
        )
        .expect("decision");

        let index_dir = temp.path().join("indexes/project");
        let store = index_dir.join("qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let original_dir_permissions = fs::metadata(&index_dir)
            .expect("dir metadata")
            .permissions();
        let mut readonly_dir = original_dir_permissions.clone();
        readonly_dir.set_mode(0o555);
        fs::set_permissions(&index_dir, readonly_dir).expect("chmod readonly dir");

        let results = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "immutable cache",
                &SearchFilters::default(),
                5,
            )
            .expect("readonly search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].freshness, Freshness::Fresh);

        fs::set_permissions(&index_dir, original_dir_permissions).expect("restore permissions");
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_sqlite_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        let original = fs::metadata(&store).expect("metadata").permissions();
        let mut unreadable = original.clone();
        unreadable.set_mode(0o000);
        fs::set_permissions(&store, unreadable).expect("chmod unreadable");

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::PermissionDenied);
        assert_eq!(status.open_mode, BackendOpenMode::ReadOnlyImmutable);

        fs::set_permissions(&store, original).expect("restore permissions");
    }

    #[test]
    fn mixed_sqlite_and_metadata_reports_transient() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        fs::remove_file(store.with_extension("llm-wiki.json")).expect("remove metadata");

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::Transient);
        assert_eq!(status.open_mode, BackendOpenMode::NotOpened);
    }

    #[test]
    fn cannot_open_with_readable_present_files_remains_transient() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        fs::create_dir_all(store.parent().expect("store parent")).expect("mkdir");
        fs::write(&store, "readable sqlite placeholder").expect("sqlite");
        fs::write(store.with_extension("llm-wiki.json"), "{}").expect("metadata");

        assert_eq!(classify_cannot_open_state(&store), BackendState::Transient);
    }

    #[test]
    fn immutable_search_missing_sqlite_returns_typed_transient() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");
        fs::write(
            wiki.join("decisions/search.decision.md"),
            "# Search Decision\n\n- Document Class: Decision\n- Status: Accepted\n\nPromotion race token.",
        )
        .expect("decision");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        fs::remove_file(&store).expect("remove sqlite");

        let error = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "promotion race",
                &SearchFilters::default(),
                5,
            )
            .expect_err("transient immutable search");
        let access = error
            .downcast_ref::<BackendAccessError>()
            .expect("typed backend access error");
        assert_eq!(access.status().state, BackendState::Transient);
        assert_eq!(access.status().open_mode, BackendOpenMode::NotOpened);
    }

    #[test]
    fn malformed_sqlite_reports_corrupt() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        fs::write(&store, "not sqlite").expect("corrupt sqlite");

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::Corrupt);
    }

    #[test]
    fn metadata_project_mismatch_reports_schema_mismatch() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let metadata_path = store.with_extension("llm-wiki.json");
        let raw = fs::read_to_string(&metadata_path).expect("metadata");
        let mut metadata: serde_json::Value = serde_json::from_str(&raw).expect("json");
        metadata["project_id"] = serde_json::Value::String("other".to_string());
        fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&metadata).expect("json"),
        )
        .expect("write metadata");

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::SchemaMismatch);
    }

    #[test]
    fn direct_search_rejects_non_searchable_status() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index\n\nDirect search guard.").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let metadata_path = store.with_extension("llm-wiki.json");
        let raw = fs::read_to_string(&metadata_path).expect("metadata");
        let mut metadata: serde_json::Value = serde_json::from_str(&raw).expect("json");
        metadata["project_id"] = serde_json::Value::String("other".to_string());
        fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&metadata).expect("json"),
        )
        .expect("write metadata");

        let error = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "direct search guard",
                &SearchFilters::default(),
                5,
            )
            .expect_err("non-searchable direct search");
        let access = error
            .downcast_ref::<BackendAccessError>()
            .expect("typed backend access error");
        assert_eq!(access.status().state, BackendState::SchemaMismatch);
    }

    #[test]
    fn sqlite_metadata_file_count_mismatch_reports_schema_mismatch() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let metadata_path = store.with_extension("llm-wiki.json");
        let raw = fs::read_to_string(&metadata_path).expect("metadata");
        let mut metadata: serde_json::Value = serde_json::from_str(&raw).expect("json");
        metadata["file_count"] = serde_json::Value::from(999);
        fs::write(
            &metadata_path,
            serde_json::to_string_pretty(&metadata).expect("json"),
        )
        .expect("write metadata");

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::SchemaMismatch);
        assert!(
            status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("active document count"))
        );
    }

    #[test]
    fn sqlite_active_rows_must_match_metadata_snapshot() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(&wiki).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index\n\nCurrent metadata.").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        let conn = rusqlite::Connection::open(&store).expect("sqlite");
        let replacement_hash: String = conn
            .query_row(
                "SELECT hash FROM documents WHERE collection = ?1 AND path = ?2 AND active = 1",
                rusqlite::params!["fixture", "wiki/log.md"],
                |row| row.get(0),
            )
            .expect("replacement hash");
        conn.execute(
            "UPDATE documents SET hash = ?1 WHERE collection = ?2 AND path = ?3 AND active = 1",
            rusqlite::params![replacement_hash, "fixture", "wiki/index.md"],
        )
        .expect("mutate sqlite row");
        drop(conn);

        let status = backend.status("fixture", &store, &wiki).expect("status");
        assert_eq!(status.state, BackendState::SchemaMismatch);
        assert!(
            status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("active document rows"))
        );
    }

    #[test]
    fn changed_metadata_during_sqlite_verification_reports_transient() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("decisions")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index\n\nOriginal metadata.").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");
        let old_metadata = StoreMetadata::read(&store).expect("old metadata");

        fs::write(
            wiki.join("decisions/new.decision.md"),
            "# New Decision\n\n- Document Class: Decision\n- Status: Accepted\n\nNew publication.",
        )
        .expect("new decision");
        let staging = temp.path().join("indexes/staging/qmd-rs.sqlite");
        backend
            .index_project("fixture", &wiki, &staging, &IndexOptions { force: true })
            .expect("new index");
        fs::copy(&staging, &store).expect("replace sqlite");
        fs::copy(
            staging.with_extension("llm-wiki.json"),
            store.with_extension("llm-wiki.json"),
        )
        .expect("replace metadata");

        let classification =
            verify_immutable_schema(&store, &old_metadata).expect_err("mixed pair transient");

        assert_eq!(classification.state, BackendState::Transient);
        assert!(
            classification
                .message
                .contains("metadata changed while sqlite was being verified")
        );
    }

    #[test]
    fn index_masks_search_ignored_spans() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let wiki = temp.path().join("wiki");
        fs::create_dir_all(wiki.join("evals")).expect("mkdir");
        fs::write(wiki.join("index.md"), "# Index").expect("index");
        fs::write(wiki.join("log.md"), "# Log").expect("log");
        fs::write(
            wiki.join("evals/search.eval.md"),
            "# Search Eval\n\n- Document Class: Eval\n- Status: Active\n\n\
<!-- llm-wiki-search-ignore-start -->\n\
GPU shader compiler roadmap\n\
<!-- llm-wiki-search-ignore-end -->\n\n\
Visible calibration evidence.",
        )
        .expect("eval");

        let store = temp.path().join("indexes/project/qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let hidden = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "GPU shader compiler roadmap",
                &SearchFilters::default(),
                5,
            )
            .expect("hidden search");
        assert!(hidden.is_empty());

        let visible = backend
            .search_project(
                "fixture",
                &store,
                &wiki,
                "Visible calibration evidence",
                &SearchFilters::default(),
                5,
            )
            .expect("visible search");
        assert_eq!(visible.len(), 1);
        assert_eq!(
            visible[0].path.to_string_lossy(),
            "wiki/evals/search.eval.md"
        );
    }

    #[test]
    fn wiki_snapshot_equality_falsification() {
        let baseline = WikiSnapshot {
            files: vec![
                FileSnapshot {
                    path: "wiki/a.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 10,
                },
                FileSnapshot {
                    path: "wiki/b.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 20,
                },
            ],
        };
        let path_added = WikiSnapshot {
            files: vec![
                FileSnapshot {
                    path: "wiki/a.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 10,
                },
                FileSnapshot {
                    path: "wiki/b.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 20,
                },
                FileSnapshot {
                    path: "wiki/c.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 20,
                },
            ],
        };
        let content_changed_same_mtime = WikiSnapshot {
            files: vec![
                FileSnapshot {
                    path: "wiki/a.md".to_string(),
                    content_hash: "changed".to_string(),
                    indexed_hash: "changed-indexed".to_string(),
                    modified_unix_seconds: 10,
                },
                FileSnapshot {
                    path: "wiki/b.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 20,
                },
            ],
        };
        let mtime_changed_same_content = WikiSnapshot {
            files: vec![
                FileSnapshot {
                    path: "wiki/a.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 11,
                },
                FileSnapshot {
                    path: "wiki/b.md".to_string(),
                    content_hash: "same".to_string(),
                    indexed_hash: "same-indexed".to_string(),
                    modified_unix_seconds: 20,
                },
            ],
        };

        assert_ne!(baseline, path_added);
        assert_ne!(baseline, content_changed_same_mtime);
        assert_ne!(baseline, mtime_changed_same_content);
    }

    #[test]
    fn snippets_handle_unicode_lowercase_expansion() {
        let body = "Heading\n\nİstanbul and Straße stay searchable.";
        let span = find_match_span(body, &[String::from("i\u{307}stanbul")]).expect("span");

        assert_eq!(&body[span.start..span.end], "İstanbul");
        let snippet = snippet(body, span);
        assert!(snippet.contains("İstanbul"));
    }

    #[test]
    fn fixed_eval_queries_keep_expected_targets_in_top_two() {
        // A frozen copy, so the test fails when search gets worse, not when the live wiki grows.
        let wiki = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/search-eval/wiki");
        let temp = tempfile::TempDir::new().expect("tempdir");
        let store = temp.path().join("qmd-rs.sqlite");
        let backend = QmdRsBackend::new();
        backend
            .index_project("fixture", &wiki, &store, &IndexOptions { force: true })
            .expect("index");

        let cases: &[(&str, &[&str])] = &[
            (
                "managed binary runtime install manifest",
                &[
                    "wiki/plans/binary-path-bootstrap.plan.md",
                    "wiki/decisions/binary-path-bootstrap.decision.md",
                ],
            ),
            (
                "agent owns wiki humans curate raw",
                &["wiki/decisions/agent-owns-wiki.decision.md"],
            ),
            (
                "search backend selection qmd-rs eval",
                &[
                    "wiki/proposals/search-backend-selection.proposal.md",
                    "wiki/evals/search-backend-selection.eval.md",
                    "wiki/decisions/search-backend-selection.decision.md",
                ],
            ),
            (
                "knowledge research intake bundle manifest summary",
                &[
                    "wiki/plans/knowledge-research-intake.plan.md",
                    "wiki/decisions/knowledge-research-intake.decision.md",
                ],
            ),
            (
                "qmd-rs scale search llm-wiki binary",
                &[
                    "wiki/roadmaps/framework-v1.roadmap.md",
                    "wiki/specs/documentation-model.spec.md",
                ],
            ),
            (
                "three phase ingest extraction drafting bookkeeping",
                &["wiki/references/three-phase-ingest-pipeline.reference.md"],
            ),
            (
                "project registry search-all reciprocal rank fusion",
                &[
                    "wiki/plans/project-registry-search-artifacts.plan.md",
                    "wiki/proposals/project-registry-search-artifacts.proposal.md",
                ],
            ),
            (
                "D8 distribution tooling cargo dist skill projection",
                &[
                    "wiki/plans/llm-wiki-binary.plan.md",
                    "wiki/proposals/llm-wiki-binary.proposal.md",
                ],
            ),
        ];

        for (query, expected) in cases {
            let results = backend
                .search_project(
                    "fixture",
                    &store,
                    &wiki,
                    query,
                    &SearchFilters::default(),
                    2,
                )
                .unwrap_or_else(|error| panic!("{query}: {error}"));
            let paths = results
                .iter()
                .map(|result| result.path.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            assert!(
                paths
                    .iter()
                    .any(|path| expected.iter().any(|candidate| candidate == path)),
                "{query}: expected one of {expected:?} in top two, got {paths:?}",
            );
        }
    }
}
