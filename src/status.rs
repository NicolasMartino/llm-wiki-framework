use std::fs;

use anyhow::Result;

use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    let Some(manifest) = Manifest::read(&manifest_path)? else {
        println!("llm-wiki {}: not installed", env!("CARGO_PKG_VERSION"));
        return Ok(());
    };

    println!("llm-wiki {}", env!("CARGO_PKG_VERSION"));
    println!("installed version: {}", manifest.binary_version);
    println!("installed at: {}", manifest.installed_at);
    println!("manifest files: {}", manifest.files.len());

    for entry in &manifest.files {
        let status = if !entry.path.exists() {
            "Missing"
        } else {
            let current = sha256_hex(&fs::read(&entry.path)?);
            if current == entry.sha256 {
                "OK"
            } else {
                "Drifted"
            }
        };
        println!(
            "{status}: {} {} {}",
            entry.runtime_string(),
            entry.kind_string(),
            entry.path.display()
        );
    }
    Ok(())
}

trait EntryLabels {
    fn runtime_string(&self) -> &'static str;
    fn kind_string(&self) -> &'static str;
}

impl EntryLabels for crate::manifest::ManifestEntry {
    fn runtime_string(&self) -> &'static str {
        match self.runtime {
            crate::manifest::RuntimeName::Claude => "claude",
            crate::manifest::RuntimeName::Codex => "codex",
        }
    }

    fn kind_string(&self) -> &'static str {
        match self.kind {
            crate::manifest::FileKind::Skill => "skill",
            crate::manifest::FileKind::RuntimeConfig => "runtime-config",
        }
    }
}
