use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    pub binary_version: String,
    pub installed_at: String,
    pub files: Vec<ManifestEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestEntry {
    pub path: PathBuf,
    pub skill: String,
    pub runtime: RuntimeName,
    pub kind: FileKind,
    pub sha256: String,
    pub installed_by_version: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeName {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    Skill,
    RuntimeConfig,
}

impl Manifest {
    pub fn new(files: Vec<ManifestEntry>) -> Self {
        Self {
            binary_version: env!("CARGO_PKG_VERSION").to_string(),
            installed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            files,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest {}", path.display()))?;
        let manifest = serde_json::from_str(&input)
            .with_context(|| format!("failed to parse manifest {}", path.display()))?;
        Ok(Some(manifest))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .with_context(|| format!("manifest path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("failed to create temp manifest in {}", parent.display()))?;
        serde_json::to_writer_pretty(&mut temp, self).context("failed to serialize manifest")?;
        temp.persist(path)
            .map_err(|err| err.error)
            .with_context(|| format!("failed to persist manifest {}", path.display()))?;
        Ok(())
    }
}
