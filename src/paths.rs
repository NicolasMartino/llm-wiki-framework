use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::instance;

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub cache_home: PathBuf,
    pub data_home: PathBuf,
    pub managed_home: PathBuf,
}

impl Paths {
    pub fn from_env() -> Result<Self> {
        if cfg!(windows) {
            return Self::from_windows_known_folders();
        }

        Self::from_unix_env()
    }

    #[cfg(windows)]
    fn from_windows_known_folders() -> Result<Self> {
        let home = dirs::home_dir().context("Windows Profile known folder is not available")?;
        let local_app_data =
            dirs::data_local_dir().context("Windows LocalAppData known folder is not available")?;
        let managed_home = local_app_data.join(instance::windows_managed_dir_name());
        let cache_home = managed_home.join("cache");
        let data_home = managed_home.join("data");
        Ok(Self {
            home,
            cache_home,
            data_home,
            managed_home,
        })
    }

    #[cfg(not(windows))]
    fn from_windows_known_folders() -> Result<Self> {
        unreachable!("Windows known folders are only used on Windows")
    }

    fn from_unix_env() -> Result<Self> {
        let home = env::var_os("HOME").context("HOME is not set")?;
        let home = PathBuf::from(home);
        // Per the XDG Base Directory spec, a relative value is invalid and must
        // be ignored, falling back to the HOME-based default.
        let cache_home = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".cache"))
            .join(instance::xdg_dir_name());
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"))
            .join(instance::xdg_dir_name());
        let managed_home = home.join(instance::managed_home_dir_name());
        Ok(Self {
            home,
            cache_home,
            data_home,
            managed_home,
        })
    }

    pub fn codex_config_toml(&self) -> PathBuf {
        self.home.join(".codex/config.toml")
    }

    pub fn managed_mcp_dir(&self) -> PathBuf {
        self.managed_home().join("mcp")
    }

    pub fn claude_project_mcp_config(&self) -> PathBuf {
        self.managed_mcp_dir().join("claude-project.mcp.json")
    }

    pub fn manifest(&self) -> PathBuf {
        self.managed_home().join("manifest.json")
    }

    pub fn managed_home(&self) -> PathBuf {
        self.managed_home.clone()
    }

    pub fn managed_bin_dir(&self) -> PathBuf {
        self.managed_home().join("bin")
    }

    pub fn managed_binary(&self) -> PathBuf {
        self.managed_bin_dir().join(managed_binary_name())
    }

    pub fn managed_poman(&self) -> PathBuf {
        self.managed_bin_dir().join(poman_binary_name())
    }

    pub fn partial_install(&self) -> PathBuf {
        self.managed_home().join("install.partial.json")
    }

    pub fn search_config(&self) -> PathBuf {
        self.managed_home().join("search.toml")
    }

    pub fn search_thresholds(&self) -> PathBuf {
        self.managed_home().join("search-thresholds.toml")
    }

    pub fn search_runtime_probes(&self) -> PathBuf {
        self.managed_home().join("search-runtime-probes.toml")
    }

    pub fn external_dependencies(&self) -> PathBuf {
        self.managed_home().join("external-dependencies.toml")
    }

    pub fn accepted_licenses(&self) -> PathBuf {
        self.managed_home().join("accepted-licenses.toml")
    }

    pub fn managed_model_root(&self) -> PathBuf {
        self.managed_home().join("models")
    }

    pub fn model_artifacts(&self) -> PathBuf {
        self.managed_model_root().join("artifacts.toml")
    }

    pub fn managed_index_root(&self) -> PathBuf {
        self.managed_home().join("indexes")
    }

    pub fn cache_home(&self) -> PathBuf {
        self.cache_home.clone()
    }

    pub fn index_root(&self) -> PathBuf {
        self.managed_index_root()
    }

    pub fn legacy_index_root(&self) -> PathBuf {
        self.cache_home().join("indexes")
    }

    pub fn model_cache(&self) -> PathBuf {
        self.cache_home().join("models")
    }

    pub fn project_index_dir(&self, project_key: &str) -> PathBuf {
        self.index_root().join(project_key)
    }

    pub fn legacy_project_index_dir(&self, project_key: &str) -> PathBuf {
        self.legacy_index_root().join(project_key)
    }

    pub fn qmd_rs_store_path(&self, project_key: &str) -> PathBuf {
        self.project_index_dir(project_key).join("qmd-rs.sqlite")
    }

    pub fn semantic_index_metadata(&self, project_key: &str) -> PathBuf {
        self.project_index_dir(project_key)
            .join("semantic-index.json")
    }

    pub fn semantic_vector_index(&self, project_key: &str) -> PathBuf {
        self.project_index_dir(project_key)
            .join("semantic-vectors.json")
    }

    pub fn legacy_qmd_rs_store_path(&self, project_key: &str) -> PathBuf {
        self.legacy_project_index_dir(project_key)
            .join("qmd-rs.sqlite")
    }

    pub fn data_home(&self) -> PathBuf {
        self.data_home.clone()
    }

    pub fn project_registry(&self) -> PathBuf {
        self.data_home().join("projects.json")
    }
}

pub fn managed_binary_name() -> &'static str {
    instance::binary_name()
}

/// poman keeps its own name in every instance: the test instance's managed
/// home already keeps it apart from the production one.
pub fn poman_binary_name() -> &'static str {
    if cfg!(windows) { "poman.exe" } else { "poman" }
}

#[cfg(test)]
mod tests {
    use super::Paths;
    use std::env;
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        value: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &Path) -> Self {
            let previous = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key,
                value: previous,
            }
        }

        fn remove(key: &'static str) -> Self {
            let previous = env::var_os(key);
            unsafe {
                env::remove_var(key);
            }
            Self {
                key,
                value: previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = &self.value {
                    env::set_var(self.key, value);
                } else {
                    env::remove_var(self.key);
                }
            }
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn search_paths_use_default_home_cache() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let temp = tempfile::TempDir::new().expect("tempdir");
        let _home = EnvGuard::set("HOME", temp.path());
        let _cache = EnvGuard::remove("XDG_CACHE_HOME");
        let _data = EnvGuard::remove("XDG_DATA_HOME");

        let paths = Paths::from_env().expect("paths");

        assert_eq!(paths.cache_home(), temp.path().join(".cache/llm-wiki"));
        assert_eq!(
            paths.qmd_rs_store_path("fixture"),
            temp.path().join(".llm_wiki/indexes/fixture/qmd-rs.sqlite")
        );
        assert_eq!(
            paths.legacy_qmd_rs_store_path("fixture"),
            temp.path()
                .join(".cache/llm-wiki/indexes/fixture/qmd-rs.sqlite")
        );
        assert_eq!(
            paths.semantic_index_metadata("fixture"),
            temp.path()
                .join(".llm_wiki/indexes/fixture/semantic-index.json")
        );
        assert_eq!(
            paths.model_cache(),
            temp.path().join(".cache/llm-wiki/models")
        );
        assert_eq!(
            paths.search_config(),
            temp.path().join(".llm_wiki/search.toml")
        );
        assert_eq!(
            paths.search_thresholds(),
            temp.path().join(".llm_wiki/search-thresholds.toml")
        );
        assert_eq!(
            paths.external_dependencies(),
            temp.path().join(".llm_wiki/external-dependencies.toml")
        );
        assert_eq!(
            paths.accepted_licenses(),
            temp.path().join(".llm_wiki/accepted-licenses.toml")
        );
        assert_eq!(
            paths.managed_model_root(),
            temp.path().join(".llm_wiki/models")
        );
        assert_eq!(
            paths.model_artifacts(),
            temp.path().join(".llm_wiki/models/artifacts.toml")
        );
        assert_eq!(
            paths.managed_index_root(),
            temp.path().join(".llm_wiki/indexes")
        );
        assert_eq!(
            paths.project_registry(),
            temp.path().join(".local/share/llm-wiki/projects.json")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn search_paths_honor_xdg_cache_home() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let home = tempfile::TempDir::new().expect("home");
        let cache = tempfile::TempDir::new().expect("cache");
        let data = tempfile::TempDir::new().expect("data");
        let _home = EnvGuard::set("HOME", home.path());
        let _cache = EnvGuard::set("XDG_CACHE_HOME", cache.path());
        let _data = EnvGuard::set("XDG_DATA_HOME", data.path());

        let paths = Paths::from_env().expect("paths");

        assert_eq!(paths.cache_home(), cache.path().join("llm-wiki"));
        assert_eq!(
            paths.project_index_dir("fixture"),
            home.path().join(".llm_wiki/indexes/fixture")
        );
        assert_eq!(
            paths.legacy_project_index_dir("fixture"),
            cache.path().join("llm-wiki/indexes/fixture")
        );
        assert_eq!(
            paths.project_registry(),
            data.path().join("llm-wiki/projects.json")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_use_known_folder_roots_without_unix_env() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let _home = EnvGuard::remove("HOME");
        let _cache = EnvGuard::remove("XDG_CACHE_HOME");
        let _data = EnvGuard::remove("XDG_DATA_HOME");
        let _local_app_data = EnvGuard::remove("LOCALAPPDATA");

        let paths = Paths::from_env().expect("paths");
        let managed_home = paths.managed_home();

        assert_eq!(
            managed_home.file_name().and_then(|name| name.to_str()),
            Some("llm_wiki")
        );
        assert_ne!(managed_home, paths.home.join(".llm_wiki"));
        assert_eq!(paths.cache_home(), managed_home.join("cache"));
        assert_eq!(paths.data_home(), managed_home.join("data"));
        assert_eq!(
            paths.project_registry(),
            managed_home.join("data/projects.json")
        );
        assert_eq!(
            paths.qmd_rs_store_path("fixture"),
            managed_home.join("indexes/fixture/qmd-rs.sqlite")
        );
        assert_eq!(
            paths.legacy_qmd_rs_store_path("fixture"),
            managed_home.join("cache/indexes/fixture/qmd-rs.sqlite")
        );
    }
}
