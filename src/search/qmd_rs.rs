#[cfg(feature = "qmd-rs")]
mod enabled {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{Context, Result};
    use chrono::{DateTime, Utc};
    use qmd::Store;
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};

    use crate::search::adapter::{
        BackendState, BackendStatus, Freshness, IndexOptions, MatchSpan, Score, SearchBackend,
        SearchFilters, SearchMode, SearchResult,
    };
    use crate::search::metadata::parse_wiki_metadata;
    use crate::search::sanitize::{query_terms, sanitize_fts_query};

    const BACKEND_NAME: &str = "qmd-rs";
    const COLLECTION_FALLBACK: &str = "project";
    const SCHEMA_VERSION: u32 = 1;

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
            let mut current_paths = BTreeSet::new();

            for doc in &docs {
                let body = fs::read_to_string(&doc.absolute_path)
                    .with_context(|| format!("read {}", doc.absolute_path.display()))?;
                let metadata = parse_wiki_metadata(&body);
                let title = metadata
                    .title
                    .clone()
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| Store::extract_title(&body));
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
                current_paths.insert(doc.canonical_path.clone());
            }

            for active in store.get_active_document_paths(&collection)? {
                if !current_paths.contains(&active) {
                    store.deactivate_document(&collection, &active)?;
                }
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

            Ok(BackendStatus {
                store_path: store_path.to_path_buf(),
                state: BackendState::Ready,
                indexed_files: docs.len(),
                stale: false,
                message: None,
            })
        }

        fn search_project(
            &self,
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

            let store = Store::open(store_path)
                .with_context(|| format!("open qmd-rs store {}", store_path.display()))?;
            let freshness = freshness_for_status(status_for_store(store_path, wiki_root)?);
            let overfetch = limit.saturating_mul(4).max(20);
            let raw_results = store.search_fts(&sanitized, overfetch, None)?;
            let terms = query_terms(query);
            let mut results = Vec::new();

            for raw in raw_results {
                let doc = store.get_document(&raw.doc.collection_name, &raw.doc.path)?;
                let body = doc
                    .as_ref()
                    .and_then(|doc| doc.body.as_deref())
                    .unwrap_or_default();
                let metadata = parse_wiki_metadata(body);
                let document_class = metadata.document_class().map(ToString::to_string);
                let status = metadata.status().map(ToString::to_string);
                if !filters.matches(document_class.as_deref(), status.as_deref()) {
                    continue;
                }

                let match_span = find_match_span(body, &terms);
                let snippet = match_span.map(|span| snippet(body, span));
                results.push(SearchResult {
                    project_id: raw.doc.collection_name.clone(),
                    project_name: None,
                    path: PathBuf::from(&raw.doc.path),
                    title: metadata
                        .title
                        .clone()
                        .filter(|value| !value.is_empty())
                        .unwrap_or(raw.doc.title),
                    document_class,
                    status,
                    score: Score(raw.score),
                    snippet,
                    match_span,
                    backend: BACKEND_NAME.to_string(),
                    mode: SearchMode::Fts,
                    freshness,
                });

                if results.len() == limit {
                    break;
                }
            }

            Ok(results)
        }

        fn status(&self, store_path: &Path, wiki_root: &Path) -> Result<BackendStatus> {
            status_for_store(store_path, wiki_root)
        }

        fn doctor(
            &self,
            store_path: &Path,
            wiki_root: &Path,
            mode: SearchMode,
        ) -> Result<BackendStatus> {
            let mut status = status_for_store(store_path, wiki_root)?;
            if matches!(mode, SearchMode::Semantic | SearchMode::Hybrid)
                && matches!(status.state, BackendState::Ready)
            {
                status.message =
                    Some("semantic model readiness is reported separately".to_string());
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
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct WikiSnapshot {
        files: Vec<FileSnapshot>,
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct FileSnapshot {
        path: String,
        content_hash: String,
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

    fn freshness_for_status(status: BackendStatus) -> Freshness {
        match status.state {
            BackendState::Ready => Freshness::Fresh,
            BackendState::Stale => Freshness::Stale,
            _ => Freshness::Unknown,
        }
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
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

    fn status_for_store(store_path: &Path, wiki_root: &Path) -> Result<BackendStatus> {
        if !store_path.exists() {
            return Ok(BackendStatus {
                store_path: store_path.to_path_buf(),
                state: BackendState::Missing,
                indexed_files: 0,
                stale: false,
                message: Some("qmd-rs store is missing".to_string()),
            });
        }

        if Store::open(store_path).is_err() {
            return Ok(BackendStatus {
                store_path: store_path.to_path_buf(),
                state: BackendState::Corrupt,
                indexed_files: 0,
                stale: false,
                message: Some("qmd-rs store could not be opened".to_string()),
            });
        }

        let metadata = match StoreMetadata::read(store_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Ok(BackendStatus {
                    store_path: store_path.to_path_buf(),
                    state: BackendState::SchemaMismatch,
                    indexed_files: 0,
                    stale: false,
                    message: Some(error.to_string()),
                });
            }
        };

        if metadata.schema_version != SCHEMA_VERSION || metadata.backend != BACKEND_NAME {
            return Ok(BackendStatus {
                store_path: store_path.to_path_buf(),
                state: BackendState::SchemaMismatch,
                indexed_files: metadata.file_count,
                stale: false,
                message: Some("qmd-rs metadata schema does not match this binary".to_string()),
            });
        }

        let docs = collect_wiki_documents(wiki_root)?;
        let current = WikiSnapshot::from_documents(&docs)?;
        let stale = current != metadata.snapshot();
        Ok(BackendStatus {
            store_path: store_path.to_path_buf(),
            state: if stale {
                BackendState::Stale
            } else {
                BackendState::Ready
            },
            indexed_files: metadata.file_count,
            stale,
            message: None,
        })
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

    fn collect_markdown(
        path: &Path,
        project_root: &Path,
        docs: &mut Vec<WikiDocument>,
    ) -> Result<()> {
        for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_markdown(&path, project_root, docs)?;
            } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
                let metadata = entry.metadata()?;
                let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
                let content_hash = hash_file(&path)?;
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
                });
            }
        }
        Ok(())
    }

    fn hash_file(path: &Path) -> Result<String> {
        let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn find_match_span(body: &str, terms: &[String]) -> Option<MatchSpan> {
        let lower = body.to_lowercase();
        terms
            .iter()
            .filter(|term| !term.is_empty())
            .find_map(|term| {
                lower.find(term).map(|start| MatchSpan {
                    start,
                    end: start + term.len(),
                })
            })
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
        Ok(time.duration_since(UNIX_EPOCH)?.as_secs() as i64)
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
        use super::{FileSnapshot, QmdRsBackend, WikiSnapshot};
        use crate::search::adapter::{
            BackendState, Freshness, IndexOptions, SearchBackend, SearchFilters, SearchMode,
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

            let ready = backend.status(&store, &wiki).expect("status");
            assert_eq!(ready.state, BackendState::Ready);

            fs::create_dir_all(wiki.join("plans")).expect("plans dir");
            fs::write(
                wiki.join("plans/search.plan.md"),
                "# Search Plan\n\n- Document Class: Plan\n- Status: Active\n\n## Work\nMore search work.",
            )
            .expect("plan");
            let stale = backend
                .doctor(&store, &wiki, SearchMode::Fts)
                .expect("doctor");
            assert_eq!(stale.state, BackendState::Stale);

            let stale_results = backend
                .search_project(
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
        fn snapshots_detect_content_changes_even_with_same_count_and_mtime() {
            let before = WikiSnapshot {
                files: vec![
                    FileSnapshot {
                        path: "wiki/a.md".to_string(),
                        content_hash: "old".to_string(),
                        modified_unix_seconds: 10,
                    },
                    FileSnapshot {
                        path: "wiki/b.md".to_string(),
                        content_hash: "same".to_string(),
                        modified_unix_seconds: 20,
                    },
                ],
            };
            let after = WikiSnapshot {
                files: vec![
                    FileSnapshot {
                        path: "wiki/a.md".to_string(),
                        content_hash: "new".to_string(),
                        modified_unix_seconds: 10,
                    },
                    FileSnapshot {
                        path: "wiki/b.md".to_string(),
                        content_hash: "same".to_string(),
                        modified_unix_seconds: 20,
                    },
                ],
            };

            assert_eq!(before.file_count(), after.file_count());
            assert_eq!(
                before.max_modified_unix_seconds(),
                after.max_modified_unix_seconds()
            );
            assert_ne!(before, after);
        }

        #[test]
        fn fixed_eval_queries_keep_expected_targets_in_top_two() {
            let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
            let wiki = repo.join("wiki");
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
                    "QMD hybrid search MCP",
                    &["wiki/references/qmd-search-engine.reference.md"],
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
                    .search_project(&store, &wiki, query, &SearchFilters::default(), 2)
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
}

#[cfg(not(feature = "qmd-rs"))]
mod disabled {
    use std::path::Path;

    use anyhow::{Result, bail};

    use crate::search::adapter::{
        BackendState, BackendStatus, IndexOptions, SearchBackend, SearchFilters, SearchMode,
        SearchResult,
    };

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
            _project_id: &str,
            _wiki_root: &Path,
            store_path: &Path,
            _options: &IndexOptions,
        ) -> Result<BackendStatus> {
            Ok(feature_disabled_status(store_path))
        }

        fn search_project(
            &self,
            _store_path: &Path,
            _wiki_root: &Path,
            _query: &str,
            _filters: &SearchFilters,
            _limit: usize,
        ) -> Result<Vec<SearchResult>> {
            bail!("qmd-rs search backend is disabled at compile time")
        }

        fn status(&self, store_path: &Path, _wiki_root: &Path) -> Result<BackendStatus> {
            Ok(feature_disabled_status(store_path))
        }

        fn doctor(
            &self,
            store_path: &Path,
            _wiki_root: &Path,
            _mode: SearchMode,
        ) -> Result<BackendStatus> {
            Ok(feature_disabled_status(store_path))
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

    fn feature_disabled_status(store_path: &Path) -> BackendStatus {
        BackendStatus {
            store_path: store_path.to_path_buf(),
            state: BackendState::FeatureDisabled,
            indexed_files: 0,
            stale: false,
            message: Some("qmd-rs backend feature is disabled".to_string()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::QmdRsBackend;
        use crate::search::adapter::{BackendState, SearchBackend, SearchMode};

        #[test]
        fn disabled_backend_reports_stable_status() {
            let temp = tempfile::TempDir::new().expect("tempdir");
            let store = temp.path().join("qmd-rs.sqlite");
            let backend = QmdRsBackend::new();

            let status = backend
                .doctor(&store, temp.path(), SearchMode::Fts)
                .expect("doctor");

            assert_eq!(status.state, BackendState::FeatureDisabled);
            assert_eq!(status.store_path, store);
            assert!(status.message.expect("message").contains("disabled"));
        }
    }
}

#[cfg(not(feature = "qmd-rs"))]
pub use disabled::QmdRsBackend;
#[cfg(feature = "qmd-rs")]
pub use enabled::QmdRsBackend;
