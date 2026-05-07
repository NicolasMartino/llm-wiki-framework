use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::blueprints::Blueprint;
use super::packs::Pack;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InitManifest {
    pub framework_version: String,
    pub blueprint: Blueprint,
    pub packs: Vec<Pack>,
}

impl InitManifest {
    pub fn new(blueprint: Blueprint, packs: Vec<Pack>) -> Self {
        Self {
            framework_version: env!("CARGO_PKG_VERSION").to_string(),
            blueprint,
            packs,
        }
    }

    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}
