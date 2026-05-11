use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::manifest::{HashAlgorithm, Manifest};
use crate::paths::Paths;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeManifest {
    pub framework_version: String,
    pub managed_home: PathBuf,
    pub managed_binary: PathBuf,
    pub install_manifest: PathBuf,
    pub install_state: RuntimeInstallState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_hash_algorithm: Option<HashAlgorithm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeInstallState {
    Present,
    Missing,
}

impl RuntimeManifest {
    pub fn from_paths(paths: &Paths) -> Result<Self> {
        let install_manifest = paths.manifest();
        let manifest = Manifest::read(&install_manifest)?;
        Ok(match manifest {
            Some(manifest) => Self {
                framework_version: env!("CARGO_PKG_VERSION").to_string(),
                managed_home: paths.managed_home(),
                managed_binary: manifest.binary.path,
                install_manifest,
                install_state: RuntimeInstallState::Present,
                installed_version: Some(manifest.binary.version),
                install_id: Some(format!("sha256:{}", manifest.binary.hash)),
                install_hash_algorithm: Some(manifest.binary.hash_algorithm),
                install_hash: Some(manifest.binary.hash),
            },
            None => Self {
                framework_version: env!("CARGO_PKG_VERSION").to_string(),
                managed_home: paths.managed_home(),
                managed_binary: paths.managed_binary(),
                install_manifest,
                install_state: RuntimeInstallState::Missing,
                installed_version: None,
                install_id: None,
                install_hash_algorithm: None,
                install_hash: None,
            },
        })
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}
