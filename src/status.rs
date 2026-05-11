use std::fs;

use anyhow::Result;

use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;

pub fn run(context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: status");
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    context.diagnostic(format!("managed home: {}", paths.managed_home().display()));
    context.diagnostic(format!(
        "managed binary: {}",
        paths.managed_binary().display()
    ));
    context.diagnostic(format!("manifest: {}", manifest_path.display()));
    let Some(manifest) = Manifest::read(&manifest_path)? else {
        context.diagnostic("manifest state: missing");
        println!("llm-wiki {}: not installed", env!("CARGO_PKG_VERSION"));
        return Ok(());
    };
    context.diagnostic("manifest state: present");
    context.diagnostic(format!("manifest files: {}", manifest.skills.len()));

    println!("llm-wiki {}", env!("CARGO_PKG_VERSION"));
    println!("installed version: {}", manifest.binary.version);
    println!("installed at: {}", manifest.installed_at);
    println!("managed binary: {}", manifest.binary.path.display());
    println!("manifest files: {}", manifest.skills.len());

    for entry in &manifest.skills {
        let status = if !entry.path.exists() {
            "Missing"
        } else {
            let current = sha256_hex(&fs::read(&entry.path)?);
            if current == entry.hash {
                "OK"
            } else {
                "Drifted"
            }
        };
        context.diagnostic(format!(
            "manifest file status: {} {} {} -> {}",
            entry.runtime_string(),
            entry.kind_string(),
            entry.path.display(),
            status
        ));
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
