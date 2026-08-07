use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::instance;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub installed_by: String,
    pub installed_at: String,
    pub binary: BinaryEntry,
    pub skills: Vec<ManifestEntry>,
    /// Non-skill managed assets materialized into the managed runtime home
    /// (currently the MCP server config). Defaulted so existing schema-version 2
    /// manifests written before this field existed still deserialize with an
    /// empty asset list; no schema bump is required.
    #[serde(default)]
    pub assets: Vec<ManagedAssetEntry>,
    #[serde(default)]
    pub backups: Vec<BackupEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BinaryEntry {
    pub path: PathBuf,
    pub version: String,
    pub hash_algorithm: HashAlgorithm,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
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

/// A framework-owned, non-skill asset materialized verbatim into the managed
/// runtime home. Tracked separately from skill entries so the install/uninstall
/// machinery does not have to pretend a profile is a runtime skill.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManagedAssetEntry {
    pub path: PathBuf,
    pub asset: String,
    pub kind: ManagedAssetKind,
    pub hash_algorithm: HashAlgorithm,
    pub hash: String,
    pub ownership: Ownership,
    pub installed_by_version: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagedAssetKind {
    McpConfig,
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
    pub fn new(
        binary: BinaryEntry,
        skills: Vec<ManifestEntry>,
        assets: Vec<ManagedAssetEntry>,
        backups: Vec<BackupEntry>,
    ) -> Self {
        Self {
            schema_version: 2,
            installed_by: instance::binary_stem().to_string(),
            installed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            binary,
            skills,
            assets,
            backups,
        }
    }

    pub fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest {}", path.display()))?;
        let mut manifest: Self = serde_json::from_str(&input)
            .with_context(|| format!("failed to parse manifest {}", path.display()))?;
        if manifest.schema_version != 1 && manifest.schema_version != 2 {
            bail!(
                "unsupported manifest schema_version {} in {}; expected 1 or 2",
                manifest.schema_version,
                path.display()
            );
        }
        manifest.schema_version = 2;
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
        // Fsync the temp before the atomic rename so a crash cannot leave a
        // renamed-but-torn manifest. (NamedTempFile already cleans up the temp on
        // any error via drop.)
        temp.as_file()
            .sync_all()
            .with_context(|| format!("failed to fsync temp manifest in {}", parent.display()))?;
        temp.persist(path)
            .map_err(|err| err.error)
            .with_context(|| format!("failed to persist manifest {}", path.display()))?;
        // Best effort: durably record the rename in the parent directory entry.
        let _ = fs::File::open(parent).and_then(|dir| dir.sync_all());
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
    /// The current transaction model has one phase, but the manifest keeps this
    /// explicit so future recovery can distinguish later install steps.
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
        let partial: Self = serde_json::from_str(&input)
            .with_context(|| format!("failed to parse partial install {}", path.display()))?;
        if partial.schema_version != 1 {
            bail!(
                "unsupported partial install schema_version {} in {}; expected 1",
                partial.schema_version,
                path.display()
            );
        }
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

#[cfg(test)]
mod tests {
    use super::Manifest;

    /// A schema-version 2 manifest written before the `assets` field existed
    /// must still deserialize, defaulting `assets` to an empty list. This is the
    /// backward-compatibility guarantee that lets us add managed-asset tracking
    /// without a schema bump.
    #[test]
    fn legacy_v2_manifest_without_assets_field_deserializes() {
        let legacy = r#"{
            "schema_version": 2,
            "installed_by": "llm-wiki",
            "installed_at": "2026-01-01T00:00:00Z",
            "binary": {
                "path": "/home/u/.llm_wiki/bin/llm-wiki",
                "version": "0.2.0",
                "hash_algorithm": "sha256",
                "hash": "deadbeef",
                "ownership": "manifest-owned"
            },
            "skills": [],
            "backups": []
        }"#;
        let manifest: Manifest = serde_json::from_str(legacy).expect("legacy manifest parses");
        assert_eq!(manifest.schema_version, 2);
        assert!(manifest.assets.is_empty());
    }
}
