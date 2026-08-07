//! Project-scoped MCP onboarding shared by `init`, `register`, and `install`.
//!
//! The pure rendering/merge logic lives in [`crate::mcp_config`]; this module is
//! the thin orchestrator that performs the filesystem side effects (backups,
//! atomic writes, directory creation) and reports what changed, so the two
//! project-scoped commands and the global installer cannot drift in how they
//! wire the `llm-wiki[-test]` server.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::CliContext;
use crate::instance;
use crate::mcp_config;
use crate::mcp_config::ServerWiring;
use crate::paths::Paths;

/// What happened to a single host config while ensuring it wires the server.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireOutcome {
    /// The config did not exist and was created.
    Created,
    /// The config existed and its managed server entry was added or updated.
    Updated,
    /// The config already wired the managed server; nothing was written.
    AlreadyCurrent,
}

impl WireOutcome {
    fn resolve(existed: bool, changed: bool) -> Self {
        match (existed, changed) {
            (_, false) => Self::AlreadyCurrent,
            (false, true) => Self::Created,
            (true, true) => Self::Updated,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::AlreadyCurrent => "already up to date",
        }
    }
}

/// What `wire_project_mcp` did, so the caller can print the side effects.
#[derive(Clone, Debug)]
pub struct WireSummary {
    pub claude_path: PathBuf,
    pub claude: WireOutcome,
    pub codex_path: PathBuf,
    pub codex: WireOutcome,
}

/// Wire a concrete project for both hosts: merge the project-local Claude
/// `.mcp.json` and ensure the global Codex config, non-destructively and
/// idempotently. `binary` is the managed binary the hosts will spawn.
pub fn wire_project_mcp(
    project_root: &Path,
    paths: &Paths,
    binary: &Path,
    context: &CliContext,
) -> Result<WireSummary> {
    let claude_path = project_root.join(".mcp.json");
    let claude = ensure_claude_project_mcp_config(&claude_path, binary, context)?;
    let codex_path = paths.codex_config_toml();
    let codex = ensure_codex_mcp_config(&codex_path, binary, context)?;
    Ok(WireSummary {
        claude_path,
        claude,
        codex_path,
        codex,
    })
}

/// Ensure a project-local Claude `.mcp.json` wires the managed server. Other
/// servers and root keys are preserved; a no-op when already correct.
pub fn ensure_claude_project_mcp_config(
    mcp_json_path: &Path,
    binary: &Path,
    context: &CliContext,
) -> Result<WireOutcome> {
    let existing = read_optional(mcp_json_path)?;
    if let Some(contents) = existing.as_deref()
        && mcp_config::claude_project_server_wiring(contents, binary)? == ServerWiring::Wired
    {
        return Ok(WireOutcome::AlreadyCurrent);
    }
    let merged = mcp_config::merge_claude_project_mcp_config(existing.as_deref(), binary)?;
    let changed = existing.as_deref() != Some(merged.as_str());
    if changed {
        write_atomic(mcp_json_path, &merged)?;
        context.diagnostic(format!(
            "wired Claude MCP config: {}",
            mcp_json_path.display()
        ));
    }
    Ok(WireOutcome::resolve(existing.is_some(), changed))
}

/// Ensure the global Codex `config.toml` wires the managed server. The config is
/// user-owned, so the pre-llm-wiki contents are snapshotted once to a
/// `.llm-wiki-backup` sibling before the first merge and the merged config is
/// written atomically. A no-op when already correct.
pub fn ensure_codex_mcp_config(
    codex_path: &Path,
    binary: &Path,
    context: &CliContext,
) -> Result<WireOutcome> {
    let existing = read_optional(codex_path)?;
    let merged = mcp_config::merge_codex_config(existing.as_deref(), binary)?;
    let changed = existing.as_deref() != Some(merged.as_str());
    if !changed {
        return Ok(WireOutcome::AlreadyCurrent);
    }
    if let Some(previous) = existing.as_deref() {
        // Snapshot only once: a repeated wiring must keep the original
        // pre-llm-wiki config as the recovery point, not overwrite it with the
        // already-merged state from a prior run. Uninstall removes this file.
        let backup_path = codex_config_backup_path(codex_path);
        if !backup_path.exists() {
            write_all_synced(&backup_path, previous)?;
            context.diagnostic(format!(
                "backed up Codex MCP config before first merge: {}",
                backup_path.display()
            ));
        }
    }
    mcp_config::write_codex_config_atomic(codex_path, &merged)?;
    context.diagnostic(format!("wired Codex MCP config: {}", codex_path.display()));
    Ok(WireOutcome::resolve(existing.is_some(), changed))
}

pub fn codex_config_backup_path(codex_path: &Path) -> PathBuf {
    let mut name = codex_path.as_os_str().to_os_string();
    name.push(".llm-wiki-backup");
    PathBuf::from(name)
}

/// One-line pointer printed when wiring is skipped (`--no-mcp`, or `init
/// --no-register`), so the user knows where install materializes the fallback
/// template if they need to wire Claude manually.
pub fn staged_template_pointer(paths: &Paths) -> String {
    let bin = instance::binary_stem();
    format!(
        "MCP wiring skipped; Claude fallback template path is {}. Run `{bin} install` to materialize it if needed, then `{bin} doctor` to check project wiring.",
        paths.claude_project_mcp_config().display()
    )
}

/// Print the side effects of `wire_project_mcp` in a stable, human-readable form.
pub fn print_summary(summary: &WireSummary) {
    println!("MCP wiring:");
    println!(
        "  Claude {} ({})",
        summary.claude.label(),
        summary.claude_path.display()
    );
    println!(
        "  Codex {} ({})",
        summary.codex.label(),
        summary.codex_path.display()
    );
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

/// Write `contents` to `path` via a same-directory temp file + rename so a crash
/// mid-write cannot truncate a user-owned file.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("failed to create temp file in {}", dir.display()))?;
    temp.write_all(contents.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to persist {}", path.display()))?;
    Ok(())
}

fn write_all_synced(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> CliContext {
        CliContext::new(false)
    }

    #[test]
    fn wire_project_creates_then_is_idempotent() {
        let project = tempfile::TempDir::new().expect("project");
        let home = tempfile::TempDir::new().expect("home");
        let paths = paths_for(home.path());
        let binary = Path::new("/tmp/llm-wiki");

        let first = wire_project_mcp(project.path(), &paths, binary, &context()).expect("first");
        assert_eq!(first.claude, WireOutcome::Created);
        assert_eq!(first.codex, WireOutcome::Created);
        assert!(first.claude_path.exists());
        assert!(first.codex_path.exists());

        let before = fs::read_to_string(&first.claude_path).expect("claude");
        let second = wire_project_mcp(project.path(), &paths, binary, &context()).expect("second");
        assert_eq!(second.claude, WireOutcome::AlreadyCurrent);
        assert_eq!(second.codex, WireOutcome::AlreadyCurrent);
        assert_eq!(
            fs::read_to_string(&first.claude_path).expect("claude"),
            before,
            "idempotent wiring must not rewrite the file"
        );
    }

    #[test]
    fn wire_project_preserves_existing_claude_server() {
        let project = tempfile::TempDir::new().expect("project");
        let mcp_json = project.path().join(".mcp.json");
        fs::write(
            &mcp_json,
            r#"{"mcpServers": {"other": {"type": "stdio", "command": "other"}}}"#,
        )
        .expect("seed");

        let outcome =
            ensure_claude_project_mcp_config(&mcp_json, Path::new("/tmp/llm-wiki"), &context())
                .expect("wire");
        assert_eq!(outcome, WireOutcome::Updated);
        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&mcp_json).expect("read")).expect("json");
        assert_eq!(
            parsed["mcpServers"]["other"]["command"].as_str(),
            Some("other")
        );
        assert!(parsed["mcpServers"][instance::mcp_server_name()].is_object());
    }

    #[test]
    fn ensure_claude_project_mcp_config_preserves_compact_wired_json_bytes() {
        let project = tempfile::TempDir::new().expect("project");
        let mcp_json = project.path().join(".mcp.json");
        let compact = format!(
            r#"{{"mcpServers":{{"{name}":{{"type":"stdio","command":"/tmp/llm-wiki","args":["mcp","serve"],"env":{{}}}},"other":{{"command":"other"}}}},"custom":true}}"#,
            name = instance::mcp_server_name()
        );
        fs::write(&mcp_json, &compact).expect("seed compact .mcp.json");

        let outcome =
            ensure_claude_project_mcp_config(&mcp_json, Path::new("/tmp/llm-wiki"), &context())
                .expect("wire");

        assert_eq!(outcome, WireOutcome::AlreadyCurrent);
        assert_eq!(
            fs::read_to_string(&mcp_json).expect("read compact .mcp.json"),
            compact,
            "an already-correct project .mcp.json must not be reformatted"
        );
    }

    #[test]
    fn ensure_codex_snapshots_original_once() {
        let home = tempfile::TempDir::new().expect("home");
        let codex_path = home.path().join(".codex/config.toml");
        fs::create_dir_all(codex_path.parent().unwrap()).expect("codex dir");
        fs::write(&codex_path, "model = \"gpt-5\"\n").expect("seed");

        ensure_codex_mcp_config(&codex_path, Path::new("/tmp/llm-wiki"), &context())
            .expect("first");
        let backup = codex_config_backup_path(&codex_path);
        assert_eq!(
            fs::read_to_string(&backup).expect("backup"),
            "model = \"gpt-5\"\n",
            "backup must snapshot the pre-llm-wiki config"
        );

        // A second wiring against a new binary must not overwrite the snapshot.
        ensure_codex_mcp_config(&codex_path, Path::new("/new/llm-wiki"), &context())
            .expect("second");
        assert_eq!(
            fs::read_to_string(&backup).expect("backup"),
            "model = \"gpt-5\"\n",
            "backup snapshot must be written only once"
        );
    }

    // Construct Paths directly from a scratch HOME so tests never mutate process
    // env; only `codex_config_toml()` and `claude_project_mcp_config()` are used.
    fn paths_for(home: &Path) -> Paths {
        Paths {
            home: home.to_path_buf(),
            cache_home: home.join(".cache"),
            data_home: home.join(".local/share"),
            managed_home: home.join(instance::managed_home_dir_name()),
        }
    }
}
