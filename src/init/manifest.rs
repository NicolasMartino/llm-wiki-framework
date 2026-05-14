use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::blueprints::Blueprint;
use super::packs::Pack;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InitManifest {
    pub framework_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_description: Option<String>,
    pub blueprint: Blueprint,
    pub packs: Vec<Pack>,
    #[serde(default)]
    pub resolved_folders: Vec<String>,
}

impl InitManifest {
    pub fn new(
        project_name: impl Into<String>,
        project_description: impl Into<String>,
        blueprint: Blueprint,
        packs: Vec<Pack>,
        resolved_folders: Vec<String>,
    ) -> Self {
        Self {
            framework_version: env!("CARGO_PKG_VERSION").to_string(),
            project_name: Some(project_name.into()),
            project_description: Some(project_description.into()),
            blueprint,
            packs,
            resolved_folders,
        }
    }

    pub fn read_from_project(path: &Path) -> Result<Option<Self>> {
        let manifest_path = path.join(".llm_wiki/init.toml");
        if !manifest_path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;
        let manifest = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
        Ok(Some(manifest))
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}
