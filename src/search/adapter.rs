use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchMode {
    Fts,
    Semantic,
    Hybrid,
}

impl fmt::Display for SearchMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fts => formatter.write_str("lexical"),
            Self::Semantic => formatter.write_str("semantic"),
            Self::Hybrid => formatter.write_str("hybrid"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchFilters {
    pub document_class: Option<String>,
    pub status: Option<String>,
}

impl SearchFilters {
    pub fn matches(&self, document_class: Option<&str>, status: Option<&str>) -> bool {
        let class_matches = self.document_class.as_deref().is_none_or(|expected| {
            document_class.is_some_and(|actual| same_token(expected, actual))
        });
        let status_matches = self
            .status
            .as_deref()
            .is_none_or(|expected| status.is_some_and(|actual| same_token(expected, actual)));
        class_matches && status_matches
    }

    /// True when at least one filter is set and can therefore drop candidates.
    pub fn is_active(&self) -> bool {
        self.document_class.is_some() || self.status.is_some()
    }
}

fn same_token(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexOptions {
    pub force: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    pub project_id: String,
    pub project_name: Option<String>,
    pub path: PathBuf,
    pub title: String,
    pub document_class: Option<String>,
    pub status: Option<String>,
    pub score: Score,
    pub snippet: Option<String>,
    pub match_span: Option<MatchSpan>,
    pub backend: String,
    pub mode: SearchMode,
    pub freshness: Freshness,
    pub lexical_rank: Option<usize>,
    pub lexical_score: Option<Score>,
    pub semantic_rank: Option<usize>,
    pub semantic_score: Option<Score>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score(pub f64);

// SAFETY(invariant): scores are cosine similarities and BM25 ranks — always
// finite, never NaN — so reflexivity holds and this `Eq` is sound. A NaN would
// violate `Eq`; guard against introducing one when constructing `Score`.
impl Eq for Score {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendStatus {
    pub store_path: PathBuf,
    pub state: BackendState,
    pub open_mode: BackendOpenMode,
    pub indexed_files: usize,
    pub stale: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendAccessError {
    status: BackendStatus,
}

impl BackendAccessError {
    pub fn new(status: BackendStatus) -> Self {
        Self { status }
    }

    pub fn status(&self) -> &BackendStatus {
        &self.status
    }
}

impl fmt::Display for BackendAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status.message.as_deref() {
            Some(message) => write!(formatter, "{message}"),
            None => write!(
                formatter,
                "search backend access failed with state {:?}",
                self.status.state
            ),
        }
    }
}

impl std::error::Error for BackendAccessError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendState {
    Ready,
    Missing,
    Stale,
    Transient,
    PermissionDenied,
    Corrupt,
    SchemaMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendOpenMode {
    ReadOnlyImmutable,
    ReadWriteIndexing,
    NotOpened,
}

impl BackendOpenMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnlyImmutable => "read_only_immutable",
            Self::ReadWriteIndexing => "read_write_indexing",
            Self::NotOpened => "not_opened",
        }
    }
}

pub trait SearchBackend {
    fn index_project(
        &self,
        project_id: &str,
        wiki_root: &Path,
        store_path: &Path,
        options: &IndexOptions,
    ) -> Result<BackendStatus>;

    fn search_project(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<SearchResult>>;

    fn status(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
    ) -> Result<BackendStatus>;

    fn doctor(
        &self,
        project_id: &str,
        store_path: &Path,
        wiki_root: &Path,
        mode: SearchMode,
    ) -> Result<BackendStatus>;

    fn rebuild_or_recover(
        &self,
        project_id: &str,
        wiki_root: &Path,
        store_path: &Path,
        force: bool,
    ) -> Result<BackendStatus>;
}

#[cfg(test)]
mod tests {
    use super::SearchFilters;

    #[test]
    fn filters_match_case_insensitively() {
        let filters = SearchFilters {
            document_class: Some("decision".to_string()),
            status: Some("ACCEPTED".to_string()),
        };

        assert!(filters.matches(Some("Decision"), Some("Accepted")));
        assert!(!filters.matches(Some("Plan"), Some("Accepted")));
        assert!(!filters.matches(Some("Decision"), None));
    }
}
