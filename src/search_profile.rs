use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

const SEARCH_CONFIG_SCHEMA_VERSION: u32 = 1;
const EXTERNAL_DEPENDENCIES_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchConfig {
    pub schema_version: u32,
    pub updated_at: String,
    pub project_default: SearchProfile,
    pub global_search: SearchProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchProfile {
    pub llm_search_enabled: bool,
    pub configured_at: String,
    pub configured_by_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_expansion_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reranker_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_install_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectSearchConfig {
    pub schema_version: u32,
    pub updated_at: String,
    pub project: SearchProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalDependencies {
    pub schema_version: u32,
    pub updated_at: String,
    pub dependencies: Vec<ExternalDependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalDependency {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver: Option<String>,
    pub reason: String,
    pub owning_tool: String,
    pub removable_by_llm_wiki: bool,
    pub cleanup_guidance: String,
}

impl SearchConfig {
    pub fn disabled() -> Self {
        Self::disabled_with_reason("llm_search_disabled")
    }

    pub fn disabled_with_reason(reason: impl Into<String>) -> Self {
        let updated_at = timestamp();
        let reason = reason.into();
        Self {
            schema_version: SEARCH_CONFIG_SCHEMA_VERSION,
            updated_at: updated_at.clone(),
            project_default: SearchProfile::disabled(&updated_at, reason.clone()),
            global_search: SearchProfile::disabled(&updated_at, reason),
        }
    }

    pub fn enabled(profile: SearchProfile) -> Self {
        let updated_at = timestamp();
        Self {
            schema_version: SEARCH_CONFIG_SCHEMA_VERSION,
            updated_at,
            project_default: profile.clone(),
            global_search: profile,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read search config {}", path.display()))?;
        let config: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse search config {}", path.display()))?;
        if config.schema_version != SEARCH_CONFIG_SCHEMA_VERSION {
            bail!(
                "unsupported search config schema_version {} in {}; expected {}",
                config.schema_version,
                path.display(),
                SEARCH_CONFIG_SCHEMA_VERSION
            );
        }
        Ok(Some(config))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "search config")
    }
}

impl SearchProfile {
    fn disabled(configured_at: &str, reason: String) -> Self {
        Self {
            llm_search_enabled: false,
            configured_at: configured_at.to_string(),
            configured_by_version: env!("CARGO_PKG_VERSION").to_string(),
            reason: Some(reason),
            profile: None,
            embedding_model: None,
            query_expansion_model: None,
            reranker_model: None,
            source: None,
            source_install_id: None,
        }
    }

    pub fn enabled(
        profile: impl Into<String>,
        embedding_model: impl Into<String>,
        query_expansion_model: Option<String>,
        reranker_model: Option<String>,
    ) -> Self {
        let configured_at = timestamp();
        Self {
            llm_search_enabled: true,
            configured_at,
            configured_by_version: env!("CARGO_PKG_VERSION").to_string(),
            reason: None,
            profile: Some(profile.into()),
            embedding_model: Some(embedding_model.into()),
            query_expansion_model,
            reranker_model,
            source: None,
            source_install_id: None,
        }
    }
}

impl ProjectSearchConfig {
    pub fn from_project_default(
        global_config: &SearchConfig,
        source_install_id: Option<String>,
    ) -> Self {
        let mut project = global_config.project_default.clone();
        project.source = Some("project_default".to_string());
        project.source_install_id = source_install_id;
        Self {
            schema_version: SEARCH_CONFIG_SCHEMA_VERSION,
            updated_at: timestamp(),
            project,
        }
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "project search config")
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read project search config {}", path.display()))?;
        let config: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse project search config {}", path.display()))?;
        if config.schema_version != SEARCH_CONFIG_SCHEMA_VERSION {
            bail!(
                "unsupported project search config schema_version {} in {}; expected {}",
                config.schema_version,
                path.display(),
                SEARCH_CONFIG_SCHEMA_VERSION
            );
        }
        Ok(Some(config))
    }
}

impl ExternalDependencies {
    pub fn empty() -> Self {
        Self {
            schema_version: EXTERNAL_DEPENDENCIES_SCHEMA_VERSION,
            updated_at: timestamp(),
            dependencies: Vec::new(),
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read external dependencies {}", path.display()))?;
        let dependencies: Self = toml::from_str(&input)
            .with_context(|| format!("failed to parse external dependencies {}", path.display()))?;
        if dependencies.schema_version != EXTERNAL_DEPENDENCIES_SCHEMA_VERSION {
            bail!(
                "unsupported external dependencies schema_version {} in {}; expected {}",
                dependencies.schema_version,
                path.display(),
                EXTERNAL_DEPENDENCIES_SCHEMA_VERSION
            );
        }
        Ok(Some(dependencies))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        write_toml_atomic(path, self, "external dependencies")
    }
}

pub(crate) fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn write_toml_atomic<T: Serialize>(path: &Path, value: &T, label: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{label} path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp {label} in {}", parent.display()))?;
    let rendered =
        toml::to_string_pretty(value).with_context(|| format!("failed to serialize {label}"))?;
    use std::io::Write;
    temp.write_all(rendered.as_bytes())
        .with_context(|| format!("failed to write temp {label}"))?;
    temp.persist(path)
        .map_err(|err| err.error)
        .with_context(|| format!("failed to persist {label} {}", path.display()))?;
    Ok(())
}
