use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub installed_by: String,
    pub installed_at: String,
    pub binary: BinaryEntry,
    pub skills: Vec<ManifestEntry>,
    pub backups: Vec<BackupEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryEntry {
    pub path: PathBuf,
    pub version: String,
    pub hash_algorithm: HashAlgorithm,
    pub hash: String,
    pub ownership: Ownership,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestEntry {
    pub path: PathBuf,
    pub skill: String,
    pub runtime: RuntimeName,
    pub kind: FileKind,
    pub hash_algorithm: HashAlgorithm,
    pub hash: String,
    pub ownership: Ownership,
    pub installed_by_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupEntry {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Sha256,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ownership {
    ManifestOwned,
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
    pub fn new(binary: BinaryEntry, skills: Vec<ManifestEntry>, backups: Vec<BackupEntry>) -> Self {
        Self {
            schema_version: 2,
            installed_by: "llm-wiki".to_string(),
            installed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            binary,
            skills,
            backups,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartialInstall {
    pub schema_version: u32,
    pub started_at: String,
    pub current_exe: PathBuf,
    pub target_binary: PathBuf,
    pub current_exe_hash_algorithm: HashAlgorithm,
    pub current_exe_hash: String,
    pub phase: PartialPhase,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PartialPhase {
    BinaryCopy,
}

impl PartialInstall {
    pub fn new(current_exe: PathBuf, target_binary: PathBuf, current_exe_hash: String) -> Self {
        Self {
            schema_version: 1,
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            current_exe,
            target_binary,
            current_exe_hash_algorithm: HashAlgorithm::Sha256,
            current_exe_hash,
            phase: PartialPhase::BinaryCopy,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read partial install {}", path.display()))?;
        let partial = serde_json::from_str(&input)
            .with_context(|| format!("failed to parse partial install {}", path.display()))?;
        Ok(Some(partial))
    }

    pub fn write_atomic(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .with_context(|| format!("partial install path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).with_context(|| {
            format!(
                "failed to create temp partial install in {}",
                parent.display()
            )
        })?;
        serde_json::to_writer_pretty(&mut temp, self)
            .context("failed to serialize partial install")?;
        temp.persist(path)
            .map_err(|err| err.error)
            .with_context(|| format!("failed to persist partial install {}", path.display()))?;
        Ok(())
    }
}
