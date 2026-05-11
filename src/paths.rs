use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub cache_home: PathBuf,
    pub data_home: PathBuf,
}

impl Paths {
    pub fn from_env() -> Result<Self> {
        let home = env::var_os("HOME").context("HOME is not set")?;
        let home = PathBuf::from(home);
        let cache_home = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"))
            .join("llm-wiki");
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("llm-wiki");
        Ok(Self {
            home,
            cache_home,
            data_home,
        })
    }

    pub fn claude_skill(&self, skill: &str) -> PathBuf {
        self.home
            .join(".claude/skills")
            .join(skill)
            .join("SKILL.md")
    }

    pub fn codex_skill(&self, skill: &str) -> PathBuf {
        self.home.join(".codex/skills").join(skill).join("SKILL.md")
    }

    pub fn codex_config(&self, skill: &str) -> PathBuf {
        self.home
            .join(".codex/skills")
            .join(skill)
            .join("agents/openai.yaml")
    }

    pub fn manifest(&self) -> PathBuf {
        self.managed_home().join("manifest.json")
    }

    pub fn managed_home(&self) -> PathBuf {
        self.home.join(".llm_wiki")
    }

    pub fn managed_bin_dir(&self) -> PathBuf {
        self.managed_home().join("bin")
    }

    pub fn managed_binary(&self) -> PathBuf {
        self.managed_bin_dir().join(managed_binary_name())
    }

    pub fn partial_install(&self) -> PathBuf {
        self.managed_home().join("install.partial.json")
    }

    pub fn search_config(&self) -> PathBuf {
        self.managed_home().join("search.toml")
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
    if cfg!(windows) {
        "llm-wiki.exe"
    } else {
        "llm-wiki"
    }
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
            paths.model_cache(),
            temp.path().join(".cache/llm-wiki/models")
        );
        assert_eq!(
            paths.search_config(),
            temp.path().join(".llm_wiki/search.toml")
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
}
