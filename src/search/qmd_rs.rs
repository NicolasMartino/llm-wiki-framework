use std::path::Path;

use anyhow::{Result, bail};

use super::adapter::{
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
