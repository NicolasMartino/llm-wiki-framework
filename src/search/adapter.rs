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
            Self::Fts => formatter.write_str("fts"),
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score(pub f64);

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
    pub indexed_files: usize,
    pub stale: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendState {
    Ready,
    Missing,
    Stale,
    Corrupt,
    SchemaMismatch,
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
        store_path: &Path,
        wiki_root: &Path,
        query: &str,
        filters: &SearchFilters,
        limit: usize,
    ) -> Result<Vec<SearchResult>>;

    fn status(&self, store_path: &Path, wiki_root: &Path) -> Result<BackendStatus>;

    fn doctor(
        &self,
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
