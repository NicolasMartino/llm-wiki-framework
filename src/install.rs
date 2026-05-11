use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{Timelike, Utc};
use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};
use serde::Serialize;

use crate::cli::{CliContext, InstallArgs};
use crate::embed;
use crate::manifest::collision::{Collision, classify};
use crate::manifest::hash::sha256_hex;
use crate::manifest::{
    BackupEntry, BinaryEntry, FileKind, HashAlgorithm, Manifest, ManifestEntry, Ownership,
    PartialInstall, RuntimeName,
};
use crate::path_guidance;
use crate::paths::Paths;
use crate::search_profile::{ExternalDependencies, SearchConfig};
use crate::skill_render::{apply_binary_context, managed_binary_invocation};

pub fn run(args: &InstallArgs, context: &CliContext) -> Result<()> {
    if args.disable_llm_search && !args.configure_search {
        context.diagnostic("search configuration: --disable-llm-search implies --configure-search");
    }
    context.diagnostic("command: install");
    context.diagnostic(format!("force: {}", args.force));
    context.diagnostic(format!("path guidance: {}", !args.skip_path_guidance));
    context.diagnostic(format!(
        "configure search: {}",
        should_configure_search(args)
    ));
    let paths = Paths::from_env()?;
    context.diagnostic(format!("managed home: {}", paths.managed_home().display()));
    context.diagnostic(format!(
        "managed binary: {}",
        paths.managed_binary().display()
    ));
    context.diagnostic(format!("manifest: {}", paths.manifest().display()));
    context.diagnostic(format!(
        "partial marker: {}",
        paths.partial_install().display()
    ));
    context.diagnostic(format!(
        "search config: {}",
        paths.search_config().display()
    ));
    context.diagnostic(format!(
        "external dependencies: {}",
        paths.external_dependencies().display()
    ));
    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    context.diagnostic(format!("current executable: {}", current_exe.display()));
    let current_exe_bytes = fs::read(&current_exe).with_context(|| {
        format!(
            "failed to read current executable {}",
            current_exe.display()
        )
    })?;
    let current_exe_hash = sha256_hex(&current_exe_bytes);
    let manifest = Manifest::read(&paths.manifest())?;
    context.diagnostic(format!(
        "manifest state: {}",
        if manifest.is_some() {
            "present"
        } else {
            "missing"
        }
    ));

    let partial_state = recover_or_reject_partial(
        &paths,
        manifest.as_ref(),
        &current_exe_hash,
        args.force,
        context,
    )?;
    context.diagnostic(format!(
        "partial marker recovery: {}",
        partial_state.label()
    ));

    let files = render_install_files(&paths, context)?;
    context.diagnostic(format!("rendered install files: {}", files.len()));
    preflight_managed_binary(
        &paths,
        &current_exe,
        &current_exe_bytes,
        manifest.as_ref(),
        args.force,
        partial_state,
        context,
    )?;
    preflight_install_files(&files, manifest.as_ref(), args.force, context)?;

    let partial = PartialInstall::new(
        current_exe.clone(),
        paths.managed_binary(),
        current_exe_hash.clone(),
    );
    partial.write_atomic(&paths.partial_install())?;

    let backup = write_backup_snapshot(&paths, &files, manifest.as_ref(), &current_exe_hash)?;
    let binary = install_managed_binary(
        &paths,
        &current_exe,
        &current_exe_bytes,
        &manifest,
        args.force,
        partial_state,
        context,
    )?;
    let skill_entries = install_files(files, manifest.as_ref(), args.force, context)?;
    let mut backups = manifest
        .as_ref()
        .map(|manifest| manifest.backups.clone())
        .unwrap_or_default();
    backups.push(backup);
    Manifest::new(binary, skill_entries, backups).write_atomic(&paths.manifest())?;
    if paths.partial_install().exists() {
        fs::remove_file(paths.partial_install()).with_context(|| {
            format!(
                "failed to remove partial install marker {}",
                paths.partial_install().display()
            )
        })?;
    }
    if should_configure_search(args) {
        configure_search(args, &paths, context)?;
    }
    if !args.skip_path_guidance {
        path_guidance::print_guidance(&paths);
    }
    Ok(())
}

fn should_configure_search(args: &InstallArgs) -> bool {
    args.configure_search || args.disable_llm_search
}

fn configure_search(args: &InstallArgs, paths: &Paths, context: &CliContext) -> Result<()> {
    context.diagnostic("search configuration action: start");
    if !args.disable_llm_search {
        let enabled = inquire::Confirm::new("Enable semantic/hybrid LLM search now?")
            .with_default(false)
            .prompt()?;
        if enabled {
            bail!(
                "LLM search enablement is not implemented yet; rerun with --disable-llm-search to save a lexical profile"
            );
        }
    }

    let config = SearchConfig::disabled();
    config.write_atomic(&paths.search_config())?;
    ExternalDependencies::empty().write_atomic(&paths.external_dependencies())?;
    context.diagnostic("search configuration action: wrote disabled LLM search profile");
    context.diagnostic(format!(
        "search config: {}",
        paths.search_config().display()
    ));
    context.diagnostic(format!(
        "external dependencies: {}",
        paths.external_dependencies().display()
    ));
    Ok(())
}

fn preflight_managed_binary(
    paths: &Paths,
    current_exe: &Path,
    current_exe_bytes: &[u8],
    manifest: Option<&Manifest>,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
) -> Result<()> {
    let managed_binary = paths.managed_binary();
    let current_hash = sha256_hex(current_exe_bytes);
    context.diagnostic(format!("managed binary path: {}", managed_binary.display()));
    if same_file_when_possible(current_exe, &managed_binary) {
        let managed_hash = sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
        context.diagnostic(format!(
            "managed binary comparison: same-file hash_match={}",
            managed_hash == current_hash
        ));
        if managed_hash != current_hash {
            bail!(
                "managed binary {} differs from current executable",
                managed_binary.display()
            );
        }
        return Ok(());
    }

    if !managed_binary.exists() {
        context.diagnostic("managed binary comparison: target missing");
        return Ok(());
    }

    let managed_hash =
        sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
    let manifest_owned = manifest.is_some_and(|manifest| manifest.binary.path == managed_binary);
    context.diagnostic(format!(
        "managed binary comparison: hash_match={}, manifest_owned={}, force={}",
        managed_hash == current_hash,
        manifest_owned,
        force
    ));
    if managed_hash != current_hash && !manifest_owned && !force {
        if partial_state == PartialState::Resuming {
            bail!(
                "previous install left a partial managed binary {}; rerun with --force to replace it",
                managed_binary.display()
            );
        }
        bail!(
            "refusing to replace unmanaged binary {}; rerun with --force to replace it",
            managed_binary.display()
        );
    }
    Ok(())
}

fn preflight_install_files(
    files: &[InstallFile],
    manifest: Option<&Manifest>,
    force: bool,
    context: &CliContext,
) -> Result<()> {
    let manifest_by_path = manifest_entries_by_path(manifest);

    for file in files {
        let collision = classify_install_file(file, &manifest_by_path)?;
        context.diagnostic(format!(
            "collision classification: skill={} runtime={} kind={} path={} -> {:?}",
            file.skill,
            file.runtime.label(),
            file.kind.label(),
            file.path.display(),
            collision
        ));
        match collision {
            Collision::FreshInstall
            | Collision::RestoreMissing
            | Collision::Upgrade
            | Collision::UpToDate => {}
            Collision::UserEdited | Collision::UserEditedAndUpgrade | Collision::UnknownFile => {
                if !force {
                    bail!(
                        "refusing to overwrite {} ({collision:?}); rerun with --force to back up and replace",
                        file.path.display()
                    );
                }
            }
            Collision::Symlink => {
                if !force {
                    bail!(
                        "refusing to replace symlink {}; run llm-wiki doctor or rerun install --force",
                        file.path.display()
                    );
                }
            }
        }
    }

    Ok(())
}

fn recover_or_reject_partial(
    paths: &Paths,
    manifest: Option<&Manifest>,
    current_exe_hash: &str,
    force: bool,
    context: &CliContext,
) -> Result<PartialState> {
    let partial_path = paths.partial_install();
    let Some(partial) = PartialInstall::read(&partial_path)? else {
        context.diagnostic("partial marker decision: none");
        return Ok(PartialState::None);
    };

    let managed_binary = paths.managed_binary();
    if let Some(manifest) = manifest
        && manifest.binary.path == managed_binary
        && managed_binary.exists()
        && sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?) == manifest.binary.hash
    {
        fs::remove_file(&partial_path).with_context(|| {
            format!(
                "failed to remove leaked partial install marker {}",
                partial_path.display()
            )
        })?;
        context.diagnostic("partial marker decision: leaked marker cleaned");
        return Ok(PartialState::LeakedMarkerCleaned);
    }
    if partial.target_binary != managed_binary && !force {
        context.diagnostic("partial marker decision: stale target refused");
        bail!(
            "stale partial install targets {}; rerun with --force to replace it",
            partial.target_binary.display()
        );
    }
    if partial.current_exe_hash != current_exe_hash && !force {
        context.diagnostic("partial marker decision: stale hash refused");
        bail!(
            "stale partial install was started by a different binary; rerun with --force to replace it"
        );
    }
    context.diagnostic("partial marker decision: resuming");
    Ok(PartialState::Resuming)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartialState {
    None,
    Resuming,
    LeakedMarkerCleaned,
}

impl PartialState {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Resuming => "resuming",
            Self::LeakedMarkerCleaned => "leaked-marker-cleaned",
        }
    }
}

fn install_managed_binary(
    paths: &Paths,
    current_exe: &Path,
    current_exe_bytes: &[u8],
    manifest: &Option<Manifest>,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
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
        context.diagnostic("managed binary action: already running managed binary");
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
            if partial_state == PartialState::Resuming {
                bail!(
                    "previous install left a partial managed binary {}; rerun with --force to replace it",
                    managed_binary.display()
                );
            }
            bail!(
                "refusing to replace unmanaged binary {}; rerun with --force to replace it",
                managed_binary.display()
            );
        }
        if managed_hash != current_hash {
            context.diagnostic("managed binary action: replace existing target");
            copy_current_exe(current_exe, &managed_binary)?;
        } else {
            context.diagnostic("managed binary action: existing target hash matches");
        }
    } else {
        context.diagnostic("managed binary action: copy new target");
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
    context: &CliContext,
) -> Result<Vec<ManifestEntry>> {
    let manifest_by_path = manifest_entries_by_path(manifest);

    let mut new_entries = Vec::with_capacity(files.len());
    for file in files {
        let collision = classify_install_file(&file, &manifest_by_path)?;
        context.diagnostic(format!(
            "install file action: skill={} runtime={} kind={} path={} collision={:?}",
            file.skill,
            file.runtime.label(),
            file.kind.label(),
            file.path.display(),
            collision
        ));

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

fn manifest_entries_by_path(manifest: Option<&Manifest>) -> HashMap<PathBuf, ManifestEntry> {
    manifest
        .map(|manifest| {
            manifest
                .skills
                .iter()
                .cloned()
                .map(|entry| (entry.path.clone(), entry))
                .collect()
        })
        .unwrap_or_default()
}

fn classify_install_file(
    file: &InstallFile,
    manifest_by_path: &HashMap<PathBuf, ManifestEntry>,
) -> Result<Collision> {
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
    Ok(classify(
        current_hash.as_deref(),
        manifest_hash,
        &file.sha256,
        symlink,
    ))
}

fn write_backup_snapshot(
    paths: &Paths,
    files: &[InstallFile],
    manifest: Option<&Manifest>,
    current_exe_hash: &str,
) -> Result<BackupEntry> {
    let id = backup_id();
    let dir = paths.managed_home().join("backups").join(&id);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let manifest_by_path = manifest_entries_by_path(manifest);

    let mut backed_up = Vec::new();
    let managed_binary = paths.managed_binary();
    if managed_binary.exists() {
        let current = fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?;
        let current_hash = sha256_hex(&current);
        if current_hash != current_exe_hash {
            let backup_path = dir.join("0000-llm-wiki");
            fs::write(&backup_path, current)
                .with_context(|| format!("failed to write {}", backup_path.display()))?;
            backed_up.push(BackupSnapshotFile {
                kind: BackupSnapshotKind::ManagedBinary,
                original_path: managed_binary,
                backup_path,
                hash_algorithm: HashAlgorithm::Sha256,
                hash: current_hash,
            });
        }
    }
    for (index, file) in files.iter().enumerate() {
        let symlink = fs::symlink_metadata(&file.path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);
        if !file.path.exists() || symlink {
            continue;
        }
        let current = fs::read(&file.path)?;
        let current_hash = sha256_hex(&current);
        let manifest_hash = manifest_by_path
            .get(&file.path)
            .map(|entry| entry.hash.as_str());
        if Some(current_hash.as_str()) == manifest_hash {
            continue;
        }

        let file_name = file
            .path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "file".into());
        let backup_path = dir.join(format!("{:04}-{file_name}", index + 1));
        fs::write(&backup_path, current)
            .with_context(|| format!("failed to write {}", backup_path.display()))?;
        backed_up.push(BackupSnapshotFile {
            kind: BackupSnapshotKind::InstallFile,
            original_path: file.path.clone(),
            backup_path,
            hash_algorithm: HashAlgorithm::Sha256,
            hash: current_hash,
        });
    }

    let manifest_path = dir.join("backup-manifest.json");
    let snapshot = BackupSnapshotManifest {
        schema_version: 1,
        id: id.clone(),
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        files: backed_up,
    };
    let mut temp = tempfile::NamedTempFile::new_in(&dir)
        .with_context(|| format!("failed to create temp backup manifest in {}", dir.display()))?;
    serde_json::to_writer_pretty(&mut temp, &snapshot)
        .context("failed to serialize backup manifest")?;
    temp.persist(&manifest_path)
        .map_err(|err| err.error)
        .with_context(|| {
            format!(
                "failed to persist backup manifest {}",
                manifest_path.display()
            )
        })?;

    Ok(BackupEntry {
        id,
        path: manifest_path,
    })
}

fn render_install_files(paths: &Paths, context: &CliContext) -> Result<Vec<InstallFile>> {
    let mut files = Vec::new();
    for asset in embed::SKILLS {
        let doc = parse(asset.skill_md)
            .with_context(|| format!("failed to parse embedded skill {}", asset.name))?;
        let binary_invocation = managed_binary_invocation(&paths.managed_binary());
        let doc = apply_binary_context(doc, &binary_invocation);
        if doc.frontmatter.runtimes.contains(&Runtime::Claude) {
            let rendered = ClaudeProjector.project(&doc)?;
            context.diagnostic(format!(
                "render target: skill={} runtime=claude path={}",
                asset.name,
                paths.claude_skill(asset.name).display()
            ));
            files.push(InstallFile::new(
                paths.claude_skill(asset.name),
                asset.name,
                RuntimeName::Claude,
                FileKind::Skill,
                rendered.skill_md,
            ));
        }
        if doc.frontmatter.runtimes.contains(&Runtime::Codex) {
            let runtime_config = llm_wiki_schema::CodexRuntimeConfig::from_yaml(asset.codex_openai)
                .with_context(|| format!("failed to parse {} Codex runtime config", asset.name))?;
            let rendered = CodexProjector::with_runtime_config(runtime_config).project(&doc)?;
            context.diagnostic(format!(
                "render target: skill={} runtime=codex path={}",
                asset.name,
                paths.codex_skill(asset.name).display()
            ));
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
    let suffix = backup_suffix();
    backup_with_suffix(path, &suffix)
}

fn backup_id() -> String {
    format!("install-{}", backup_suffix())
}

fn backup_suffix() -> String {
    let now = Utc::now();
    format!("{}{:09}Z", now.format("%Y%m%dT%H%M%S"), now.nanosecond())
}

#[derive(Serialize)]
struct BackupSnapshotManifest {
    schema_version: u32,
    id: String,
    created_at: String,
    files: Vec<BackupSnapshotFile>,
}

#[derive(Serialize)]
struct BackupSnapshotFile {
    kind: BackupSnapshotKind,
    original_path: PathBuf,
    backup_path: PathBuf,
    hash_algorithm: HashAlgorithm,
    hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum BackupSnapshotKind {
    ManagedBinary,
    InstallFile,
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

trait InstallLabel {
    fn label(self) -> &'static str;
}

impl InstallLabel for RuntimeName {
    fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

impl InstallLabel for FileKind {
    fn label(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::RuntimeConfig => "runtime-config",
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
