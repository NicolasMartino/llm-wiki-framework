use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{Timelike, Utc};
use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};

use crate::embed;
use crate::manifest::collision::{Collision, classify};
use crate::manifest::hash::sha256_hex;
use crate::manifest::{FileKind, Manifest, ManifestEntry, RuntimeName};
use crate::paths::Paths;

pub fn run(force: bool) -> Result<()> {
    let paths = Paths::from_env()?;
    let files = render_install_files(&paths)?;
    install_files(&paths, files, force)
}

fn install_files(paths: &Paths, files: Vec<InstallFile>, force: bool) -> Result<()> {
    let manifest_path = paths.manifest();
    let manifest = Manifest::read(&manifest_path)?;
    let manifest_by_path: HashMap<PathBuf, ManifestEntry> = manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .cloned()
                .map(|entry| (entry.path.clone(), entry))
                .collect()
        })
        .unwrap_or_default();

    let mut new_entries = Vec::with_capacity(files.len());
    for file in files {
        let symlink = fs::symlink_metadata(&file.path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);
        let current_hash = if file.path.exists() && !symlink {
            Some(sha256_hex(&fs::read(&file.path)?))
        } else {
            None
        };
        let manifest_hash = manifest_by_path
            .get(&file.path)
            .map(|entry| entry.sha256.as_str());
        let collision = classify(
            current_hash.as_deref(),
            manifest_hash,
            &file.sha256,
            symlink,
        );

        match collision {
            Collision::FreshInstall | Collision::RestoreMissing | Collision::Upgrade => {
                write_file(&file.path, &file.contents)?;
            }
            Collision::UpToDate => {}
            Collision::UserEdited | Collision::UserEditedAndUpgrade | Collision::UnknownFile => {
                if !force {
                    bail!(
                        "refusing to overwrite {} ({collision:?}); rerun with --force to back up and replace",
                        file.path.display()
                    );
                }
                backup(&file.path)?;
                write_file(&file.path, &file.contents)?;
            }
            Collision::Symlink => {
                if !force {
                    bail!(
                        "refusing to replace symlink {}; run llm-wiki doctor or rerun install --force",
                        file.path.display()
                    );
                }
                backup(&file.path)?;
                write_file(&file.path, &file.contents)?;
            }
        }

        new_entries.push(file.entry());
    }

    Manifest::new(new_entries).write_atomic(&manifest_path)
}

fn render_install_files(paths: &Paths) -> Result<Vec<InstallFile>> {
    let mut files = Vec::new();
    for asset in embed::SKILLS {
        let doc = parse(asset.skill_md)
            .with_context(|| format!("failed to parse embedded skill {}", asset.name))?;
        if doc.frontmatter.runtimes.contains(&Runtime::Claude) {
            let rendered = ClaudeProjector.project(&doc)?;
            files.push(InstallFile::new(
                paths.claude_skill(asset.name),
                asset.name,
                RuntimeName::Claude,
                FileKind::Skill,
                rendered.skill_md,
            ));
        }
        if doc.frontmatter.runtimes.contains(&Runtime::Codex) {
            let rendered =
                CodexProjector::with_runtime_config_template(asset.codex_openai).project(&doc)?;
            files.push(InstallFile::new(
                paths.codex_skill(asset.name),
                asset.name,
                RuntimeName::Codex,
                FileKind::Skill,
                rendered.skill_md,
            ));
            files.push(InstallFile::new(
                paths.codex_config(asset.name),
                asset.name,
                RuntimeName::Codex,
                FileKind::RuntimeConfig,
                rendered.runtime_config.context("missing runtime config")?,
            ));
        }
    }
    Ok(files)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn backup(path: &Path) -> Result<PathBuf> {
    let now = Utc::now();
    let suffix = format!("{}{:09}Z", now.format("%Y%m%dT%H%M%S"), now.nanosecond());
    backup_with_suffix(path, &suffix)
}

fn backup_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf> {
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            path.with_file_name(format!(
                "{}.bak.{suffix}",
                path.file_name().unwrap().to_string_lossy()
            ))
        } else {
            path.with_file_name(format!(
                "{}.bak.{suffix}.{attempt}",
                path.file_name().unwrap().to_string_lossy()
            ))
        };
        if !candidate.exists() {
            fs::rename(path, &candidate).with_context(|| {
                format!(
                    "failed to back up {} to {}",
                    path.display(),
                    candidate.display()
                )
            })?;
            return Ok(candidate);
        }
    }
    bail!("failed to create unique backup name for {}", path.display())
}

struct InstallFile {
    path: PathBuf,
    skill: String,
    runtime: RuntimeName,
    kind: FileKind,
    contents: String,
    sha256: String,
}

impl InstallFile {
    fn new(
        path: PathBuf,
        skill: impl Into<String>,
        runtime: RuntimeName,
        kind: FileKind,
        contents: String,
    ) -> Self {
        let sha256 = sha256_hex(contents.as_bytes());
        Self {
            path,
            skill: skill.into(),
            runtime,
            kind,
            contents,
            sha256,
        }
    }

    fn entry(&self) -> ManifestEntry {
        ManifestEntry {
            path: self.path.clone(),
            skill: self.skill.clone(),
            runtime: self.runtime,
            kind: self.kind,
            sha256: self.sha256.clone(),
            installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::backup_with_suffix;

    #[test]
    fn backup_retries_when_first_candidate_exists() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("SKILL.md");
        let first_backup = temp.path().join("SKILL.md.bak.fixed");
        fs::write(&path, "current").expect("write current");
        fs::write(&first_backup, "existing").expect("write first backup");

        let backup = backup_with_suffix(&path, "fixed").expect("backup");

        assert_eq!(backup, temp.path().join("SKILL.md.bak.fixed.1"));
        assert_eq!(fs::read_to_string(backup).expect("read backup"), "current");
        assert_eq!(
            fs::read_to_string(first_backup).expect("read first backup"),
            "existing"
        );
        assert!(!path.exists());
    }
}
