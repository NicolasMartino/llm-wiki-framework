use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{Timelike, Utc};
use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};

use crate::embed;
use crate::manifest::collision::{Collision, classify};
use crate::manifest::hash::sha256_hex;
use crate::manifest::{
    BackupEntry, BinaryEntry, FileKind, HashAlgorithm, Manifest, ManifestEntry, Ownership,
    PartialInstall, RuntimeName,
};
use crate::paths::Paths;

pub fn run(force: bool) -> Result<()> {
    let paths = Paths::from_env()?;
    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    let current_exe_bytes = fs::read(&current_exe).with_context(|| {
        format!(
            "failed to read current executable {}",
            current_exe.display()
        )
    })?;
    let current_exe_hash = sha256_hex(&current_exe_bytes);
    let manifest = Manifest::read(&paths.manifest())?;

    recover_or_reject_partial(&paths, manifest.as_ref(), &current_exe_hash, force)?;

    let partial = PartialInstall::new(
        current_exe.clone(),
        paths.managed_binary(),
        current_exe_hash.clone(),
    );
    partial.write_atomic(&paths.partial_install())?;

    let binary =
        install_managed_binary(&paths, &current_exe, &current_exe_bytes, &manifest, force)?;
    let files = render_install_files(&paths)?;
    let skill_entries = install_files(files, manifest.as_ref(), force)?;
    Manifest::new(binary, skill_entries, Vec::<BackupEntry>::new())
        .write_atomic(&paths.manifest())?;
    if paths.partial_install().exists() {
        fs::remove_file(paths.partial_install()).with_context(|| {
            format!(
                "failed to remove partial install marker {}",
                paths.partial_install().display()
            )
        })?;
    }
    Ok(())
}

fn recover_or_reject_partial(
    paths: &Paths,
    manifest: Option<&Manifest>,
    current_exe_hash: &str,
    force: bool,
) -> Result<()> {
    let partial_path = paths.partial_install();
    let Some(partial) = PartialInstall::read(&partial_path)? else {
        return Ok(());
    };

    let managed_binary = paths.managed_binary();
    if let Some(manifest) = manifest {
        if manifest.binary.path == managed_binary
            && managed_binary.exists()
            && sha256_hex(&fs::read(&managed_binary)?) == manifest.binary.hash
        {
            fs::remove_file(&partial_path).with_context(|| {
                format!(
                    "failed to remove leaked partial install marker {}",
                    partial_path.display()
                )
            })?;
            return Ok(());
        }
    }
    if partial.target_binary != managed_binary && !force {
        bail!(
            "stale partial install targets {}; rerun with --force to replace it",
            partial.target_binary.display()
        );
    }
    if partial.current_exe_hash != current_exe_hash && !force {
        bail!(
            "stale partial install was started by a different binary; rerun with --force to replace it"
        );
    }
    Ok(())
}

fn install_managed_binary(
    paths: &Paths,
    current_exe: &Path,
    current_exe_bytes: &[u8],
    manifest: &Option<Manifest>,
    force: bool,
) -> Result<BinaryEntry> {
    let managed_binary = paths.managed_binary();
    if let Some(parent) = managed_binary.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let current_hash = sha256_hex(current_exe_bytes);
    if same_file_when_possible(current_exe, &managed_binary) {
        let managed_hash = sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
        if managed_hash != current_hash {
            bail!(
                "managed binary {} differs from current executable",
                managed_binary.display()
            );
        }
    } else if managed_binary.exists() {
        let managed_hash = sha256_hex(&fs::read(&managed_binary)?);
        let manifest_owned = manifest
            .as_ref()
            .is_some_and(|manifest| manifest.binary.path == managed_binary);
        if managed_hash != current_hash && !manifest_owned && !force {
            bail!(
                "refusing to replace unmanaged binary {}; rerun with --force to replace it",
                managed_binary.display()
            );
        }
        if managed_hash != current_hash {
            copy_current_exe(current_exe, &managed_binary)?;
        }
    } else {
        copy_current_exe(current_exe, &managed_binary)?;
    }

    let managed_hash = sha256_hex(&fs::read(&managed_binary)?);
    if managed_hash != current_hash {
        bail!(
            "managed binary hash mismatch after copy: {}",
            managed_binary.display()
        );
    }

    Ok(BinaryEntry {
        path: managed_binary,
        version: env!("CARGO_PKG_VERSION").to_string(),
        hash_algorithm: HashAlgorithm::Sha256,
        hash: managed_hash,
        ownership: Ownership::ManifestOwned,
    })
}

fn same_file_when_possible(left: &Path, right: &Path) -> bool {
    if !right.exists() {
        return left == right;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn copy_current_exe(current_exe: &Path, managed_binary: &Path) -> Result<()> {
    fs::copy(current_exe, managed_binary).with_context(|| {
        format!(
            "failed to copy {} to {}",
            current_exe.display(),
            managed_binary.display()
        )
    })?;
    let permissions = fs::metadata(current_exe)
        .with_context(|| format!("failed to inspect {}", current_exe.display()))?
        .permissions();
    fs::set_permissions(managed_binary, permissions)
        .with_context(|| format!("failed to set permissions on {}", managed_binary.display()))?;
    Ok(())
}

fn install_files(
    files: Vec<InstallFile>,
    manifest: Option<&Manifest>,
    force: bool,
) -> Result<Vec<ManifestEntry>> {
    let manifest_by_path: HashMap<PathBuf, ManifestEntry> = manifest
        .map(|manifest| {
            manifest
                .skills
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
            .map(|entry| entry.hash.as_str());
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

    Ok(new_entries)
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
            hash_algorithm: HashAlgorithm::Sha256,
            hash: self.sha256.clone(),
            ownership: Ownership::ManifestOwned,
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
