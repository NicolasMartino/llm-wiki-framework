use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};

use crate::cli::{CliContext, HeadroomArgs};
use crate::instance;

const HEADROOM_MCP_READ: &str = "HEADROOM_MCP_READ";
const HEADROOM_EXCLUDE_TOOLS: &str = "HEADROOM_EXCLUDE_TOOLS";
const MCP_READ_OFF: &str = "off";
const LLM_WIKI_TOOL_GLOB: &str = "*llm_wiki*";

pub fn run(args: &HeadroomArgs, context: &CliContext) -> Result<()> {
    let headroom_bin = resolve_headroom_bin(args.headroom_bin.as_deref())?;
    let child_env = ChildEnv::new(args.unsafe_mcp_read);
    emit_warning(args.unsafe_mcp_read);
    context.diagnostic(format!(
        "headroom passthrough binary: {}",
        headroom_bin.display()
    ));
    context.diagnostic(format!(
        "headroom passthrough exclude tools: {}",
        child_env.exclude_tools
    ));

    let mut command = ProcessCommand::new(&headroom_bin);
    command.args(&args.args);
    child_env.apply(&mut command);

    run_child(command, &args.args)
}

fn resolve_headroom_bin(explicit: Option<&Path>) -> Result<PathBuf> {
    let path = env::var_os("PATH");
    let pathext = env::var_os("PATHEXT");
    resolve_headroom_bin_from_env(explicit, path.as_deref(), pathext.as_deref())
}

fn resolve_headroom_bin_from_env(
    explicit: Option<&Path>,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
) -> Result<PathBuf> {
    if let Some(explicit) = explicit {
        if explicit.metadata().is_ok_and(|metadata| metadata.is_file()) {
            return Ok(explicit.to_path_buf());
        }
        bail!(
            "headroom binary path is not an existing regular file: {}",
            explicit.display()
        );
    }

    if let Some(found) = find_headroom_on_path(path, pathext) {
        return Ok(found);
    }

    bail!("headroom binary not found on PATH; pass --headroom-bin <path>")
}

fn find_headroom_on_path(path: Option<&OsStr>, pathext: Option<&OsStr>) -> Option<PathBuf> {
    let path = path?;
    let candidate_names = headroom_candidate_names(pathext);
    find_headroom_on_path_with_candidates(path, &candidate_names)
}

fn find_headroom_on_path_with_candidates(
    path: &OsStr,
    candidate_names: &[OsString],
) -> Option<PathBuf> {
    for directory in env::split_paths(path) {
        for candidate_name in candidate_names {
            let candidate = directory.join(candidate_name);
            if is_executable_candidate(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
fn headroom_candidate_names(pathext: Option<&OsStr>) -> Vec<OsString> {
    windows_headroom_candidate_names(pathext)
}

#[cfg(any(windows, test))]
fn windows_headroom_candidate_names(pathext: Option<&OsStr>) -> Vec<OsString> {
    use std::collections::HashSet;

    let pathext = pathext
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(".COM;.EXE;.BAT;.CMD");
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    push_windows_candidate(&mut candidates, &mut seen, "headroom".to_string());
    for extension in pathext.split(';').map(str::trim) {
        if extension.is_empty() {
            continue;
        }
        let extension = if extension.starts_with('.') {
            extension.to_string()
        } else {
            format!(".{extension}")
        };
        push_windows_candidate(&mut candidates, &mut seen, format!("headroom{extension}"));
    }
    candidates
}

#[cfg(any(windows, test))]
fn push_windows_candidate(
    candidates: &mut Vec<OsString>,
    seen: &mut std::collections::HashSet<String>,
    candidate: String,
) {
    if seen.insert(candidate.to_ascii_lowercase()) {
        candidates.push(OsString::from(candidate));
    }
}

#[cfg(not(windows))]
fn headroom_candidate_names(_pathext: Option<&OsStr>) -> Vec<OsString> {
    vec![OsString::from("headroom")]
}

#[cfg(unix)]
fn is_executable_candidate(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_candidate(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.is_file())
}

#[derive(Debug, Eq, PartialEq)]
struct ChildEnv {
    mcp_read: McpReadEnv,
    exclude_tools: String,
}

#[derive(Debug, Eq, PartialEq)]
enum McpReadEnv {
    SetOff,
    Remove,
}

impl ChildEnv {
    fn new(unsafe_mcp_read: bool) -> Self {
        Self {
            mcp_read: if unsafe_mcp_read {
                McpReadEnv::Remove
            } else {
                McpReadEnv::SetOff
            },
            exclude_tools: exclude_tools_value(),
        }
    }

    fn apply(&self, command: &mut ProcessCommand) {
        match self.mcp_read {
            McpReadEnv::SetOff => {
                command.env(HEADROOM_MCP_READ, MCP_READ_OFF);
            }
            McpReadEnv::Remove => {
                command.env_remove(HEADROOM_MCP_READ);
            }
        }
        command.env(HEADROOM_EXCLUDE_TOOLS, &self.exclude_tools);
    }
}

fn emit_warning(unsafe_mcp_read: bool) {
    if unsafe_mcp_read {
        eprintln!(
            "llm-wiki headroom: WARNING: HEADROOM_MCP_READ will be absent \
             in the Headroom process, even if the parent set it. Provenance depends \
             on routing wiki/raw access through llm_wiki_* MCP tools; \
             HEADROOM_EXCLUDE_TOOLS is best-effort only, not a provenance boundary."
        );
    } else {
        eprintln!(
            "llm-wiki headroom: setting HEADROOM_MCP_READ=off for the Headroom \
             process. Provenance depends on routing wiki/raw access through \
             llm_wiki_* MCP tools; HEADROOM_EXCLUDE_TOOLS is best-effort only, \
             not a provenance boundary."
        );
    }
}

fn exclude_tools_value() -> String {
    merge_exclude_tools(std::env::var(HEADROOM_EXCLUDE_TOOLS).ok())
}

/// Merge the llm-wiki exclude entries into any value the parent already set for
/// `HEADROOM_EXCLUDE_TOOLS`, preserving the existing entries (and their order)
/// and appending only the llm-wiki entries not already present. Overwriting
/// would silently drop exclusions the parent process configured.
fn merge_exclude_tools(existing: Option<String>) -> String {
    use std::collections::HashSet;

    let mut seen: HashSet<String> = HashSet::new();
    let mut merged: Vec<String> = Vec::new();
    if let Some(existing) = existing {
        for entry in existing.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if seen.insert(entry.to_string()) {
                merged.push(entry.to_string());
            }
        }
    }
    for entry in exclude_tool_entries() {
        if seen.insert(entry.clone()) {
            merged.push(entry);
        }
    }
    merged.join(",")
}

fn exclude_tool_entries() -> Vec<String> {
    let mut entries = Vec::new();
    entries.push(LLM_WIKI_TOOL_GLOB.to_string());
    for mcp_instance in instance::all_mcp_instances() {
        for tool_name in instance::mcp_tool_names() {
            let tool = tool_name.for_instance(*mcp_instance);
            entries.push(tool.to_string());
            for server in mcp_instance.server_name_variants() {
                entries.push(format!("mcp__{server}__{tool}"));
            }
        }
    }
    entries
}

#[cfg(unix)]
fn run_child(mut command: ProcessCommand, argv: &[OsString]) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let error = command.exec();
    Err(error).with_context(|| format!("execute headroom {}", format_argv(argv)))
}

#[cfg(not(unix))]
fn run_child(mut command: ProcessCommand, argv: &[OsString]) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("execute headroom {}", format_argv(argv)))?;
    std::process::exit(status.code().unwrap_or(1));
}

fn format_argv(argv: &[OsString]) -> String {
    argv.iter()
        .map(|arg| display_arg(arg.as_os_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn display_arg(arg: &OsStr) -> String {
    arg.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        ChildEnv, LLM_WIKI_TOOL_GLOB, McpReadEnv, exclude_tool_entries,
        find_headroom_on_path_with_candidates, merge_exclude_tools, resolve_headroom_bin_from_env,
        windows_headroom_candidate_names,
    };
    use crate::instance;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs::{self, File};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    #[test]
    fn resolver_skips_directories_finds_path_candidate_and_honors_explicit_bin() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let first_path_dir = temp.path().join("first");
        let second_path_dir = temp.path().join("second");
        fs::create_dir_all(&first_path_dir).expect("first path dir");
        fs::create_dir_all(&second_path_dir).expect("second path dir");

        let candidate_name = platform_candidate_name();
        fs::create_dir(first_path_dir.join(candidate_name)).expect("directory candidate");
        let executable = second_path_dir.join(candidate_name);
        File::create(&executable).expect("executable candidate");
        make_executable(&executable);

        let path = env::join_paths([first_path_dir.as_path(), second_path_dir.as_path()])
            .expect("join PATH");
        let resolved = resolve_headroom_bin_from_env(None, Some(path.as_os_str()), None)
            .expect("resolve headroom from PATH");
        assert_eq!(resolved, executable);

        let explicit = temp.path().join("explicit-headroom");
        File::create(&explicit).expect("explicit headroom");
        let resolved = resolve_headroom_bin_from_env(Some(&explicit), Some(OsStr::new("")), None)
            .expect("resolve explicit headroom");
        assert_eq!(resolved, explicit);

        let missing = resolve_headroom_bin_from_env(None, Some(OsStr::new("")), None)
            .expect_err("missing PATH candidate should fail");
        assert!(
            missing.to_string().contains("--headroom-bin"),
            "missing error should name --headroom-bin: {missing:?}"
        );
    }

    #[cfg(windows)]
    fn platform_candidate_name() -> &'static str {
        "headroom.EXE"
    }

    #[cfg(not(windows))]
    fn platform_candidate_name() -> &'static str {
        "headroom"
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("set executable");
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {}

    #[test]
    fn windows_candidates_try_plain_headroom_before_pathext_names() {
        let candidates =
            windows_headroom_candidate_names(Some(OsStr::new(".EXE;.CMD;BAT;.exe; ;")));
        let rendered: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec!["headroom", "headroom.EXE", "headroom.CMD", "headroom.BAT"]
        );
    }

    #[test]
    fn windows_candidates_use_default_pathext_when_missing_or_blank() {
        for pathext in [None, Some(OsStr::new("  "))] {
            let candidates = windows_headroom_candidate_names(pathext);
            let rendered: Vec<_> = candidates
                .iter()
                .map(|candidate| candidate.to_string_lossy().into_owned())
                .collect();
            assert_eq!(
                rendered,
                vec![
                    "headroom",
                    "headroom.COM",
                    "headroom.EXE",
                    "headroom.BAT",
                    "headroom.CMD"
                ]
            );
        }
    }

    #[test]
    fn path_search_checks_candidate_names_in_order() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let path_dir = temp.path().join("bin");
        fs::create_dir(&path_dir).expect("path dir");
        let plain = path_dir.join("headroom");
        let exe = path_dir.join("headroom.exe");
        File::create(&plain).expect("plain candidate");
        File::create(&exe).expect("extension candidate");
        make_executable(&plain);
        make_executable(&exe);

        let path = env::join_paths([path_dir.as_path()]).expect("join PATH");
        let candidate_names = vec![OsString::from("headroom"), OsString::from("headroom.exe")];
        let resolved = find_headroom_on_path_with_candidates(path.as_os_str(), &candidate_names)
            .expect("resolve candidate");
        assert_eq!(resolved, plain);
    }

    #[test]
    fn path_search_skips_directory_before_extension_candidate() {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let path_dir = temp.path().join("bin");
        fs::create_dir(&path_dir).expect("path dir");
        fs::create_dir(path_dir.join("headroom")).expect("directory candidate");
        let exe = path_dir.join("headroom.exe");
        File::create(&exe).expect("extension candidate");
        make_executable(&exe);

        let path = env::join_paths([path_dir.as_path()]).expect("join PATH");
        let candidate_names = vec![OsString::from("headroom"), OsString::from("headroom.exe")];
        let resolved = find_headroom_on_path_with_candidates(path.as_os_str(), &candidate_names)
            .expect("resolve extension candidate");
        assert_eq!(resolved, exe);
    }

    #[test]
    fn merge_preserves_preexisting_entries_alongside_llm_wiki_entries() {
        let merged = merge_exclude_tools(Some("custom_tool_a, custom_tool_b".to_string()));
        let entries: HashSet<_> = merged.split(',').collect();
        // Pre-existing exclusions survive.
        assert!(
            entries.contains("custom_tool_a"),
            "dropped custom_tool_a: {merged}"
        );
        assert!(
            entries.contains("custom_tool_b"),
            "dropped custom_tool_b: {merged}"
        );
        // llm-wiki entries are appended.
        for entry in exclude_tool_entries() {
            assert!(
                entries.contains(entry.as_str()),
                "missing llm-wiki entry {entry}"
            );
        }
    }

    #[test]
    fn merge_dedupes_overlapping_entries() {
        let first = exclude_tool_entries()
            .into_iter()
            .next()
            .expect("at least one llm-wiki entry");
        let merged = merge_exclude_tools(Some(first.clone()));
        let occurrences = merged.split(',').filter(|entry| *entry == first).count();
        assert_eq!(occurrences, 1, "overlapping entry duplicated: {merged}");
    }

    #[test]
    fn exclude_tools_include_broad_llm_wiki_glob() {
        let entries = exclude_tool_entries();
        assert!(
            entries.contains(&LLM_WIKI_TOOL_GLOB.to_string()),
            "missing broad llm-wiki glob"
        );
    }

    #[test]
    fn default_child_env_disables_headroom_mcp_read() {
        let env = ChildEnv::new(false);
        assert_eq!(env.mcp_read, McpReadEnv::SetOff);
        assert!(!env.exclude_tools.is_empty());
    }

    #[test]
    fn unsafe_child_env_removes_headroom_mcp_read_instead_of_inheriting() {
        let env = ChildEnv::new(true);
        assert_eq!(env.mcp_read, McpReadEnv::Remove);
        assert!(!env.exclude_tools.is_empty());
    }

    #[test]
    fn exclude_tools_cover_every_mcp_tool_for_both_instances() {
        let entries = exclude_tool_entries();
        let unique: HashSet<_> = entries.iter().collect();
        assert!(
            entries.contains(&LLM_WIKI_TOOL_GLOB.to_string()),
            "missing broad llm-wiki glob"
        );
        assert_eq!(
            unique.len(),
            entries.len(),
            "exclude entries must be unique"
        );

        for mcp_instance in instance::all_mcp_instances() {
            for tool_name in instance::mcp_tool_names() {
                let tool = tool_name.for_instance(*mcp_instance).to_string();
                assert!(entries.contains(&tool), "missing plain tool {tool}");
                for server in mcp_instance.server_name_variants() {
                    let qualified = format!("mcp__{server}__{tool}");
                    assert!(
                        entries.contains(&qualified),
                        "missing qualified tool {qualified}"
                    );
                }
            }
        }
    }
}
