use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map as JsonMap, Value as JsonValue, json};
use toml::Value;
use toml::map::Map;

use crate::instance;

pub const SERVER_STARTUP: &str = "host-managed stdio";
const SERVER_ARGS: [&str; 2] = ["mcp", "serve"];

pub fn server_start_command(binary: &Path) -> String {
    format!("{} {}", binary.display(), SERVER_ARGS.join(" "))
}

/// The managed binary path is embedded verbatim as the MCP `command`. A
/// non-UTF-8 path would be silently mangled by `to_string_lossy`, producing a
/// command the host cannot spawn, so require valid UTF-8 and bail otherwise.
fn binary_command_str(binary: &Path) -> Result<&str> {
    binary.to_str().with_context(|| {
        format!(
            "managed binary path {} is not valid UTF-8; cannot write MCP command",
            binary.display()
        )
    })
}

pub fn render_claude_project_mcp_config(binary: &Path) -> Result<String> {
    merge_claude_project_mcp_config(None, binary)
}

/// Merge the managed `llm-wiki[-test]` stdio server into a project-local Claude
/// `.mcp.json`, preserving every other key and server. `existing` is the current
/// file contents (or `None`/empty for a fresh project). The merge is:
///
/// - **non-destructive**: unrelated `mcpServers` entries and root keys are kept;
/// - **reconciling**: a stale `command`/`args` for the active server is replaced
///   in place rather than duplicated (the entry is keyed by server name);
/// - **idempotent**: re-merging its own output is byte-identical.
///
/// Only the *active* instance's server key ([`instance::mcp_server_name`]) is
/// touched. The other instance's entry (the test server while running
/// production, or vice versa) is deliberately left alone so the two servers can
/// coexist in the same project; the same applies to any hand-authored name
/// variant, which the tool itself never writes.
///
/// With no existing content this returns exactly what a fresh render would, so
/// `render_claude_project_mcp_config` is just `merge(None, ..)`.
pub fn merge_claude_project_mcp_config(existing: Option<&str>, binary: &Path) -> Result<String> {
    let entry = claude_server_entry(binary)?;
    let mut root = parse_claude_project_config(existing)?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    let servers = servers
        .as_object_mut()
        .context("Claude .mcp.json `mcpServers` must be a JSON object")?;
    servers.insert(instance::mcp_server_name().to_string(), entry);
    serde_json::to_string_pretty(&JsonValue::Object(root)).context("render Claude MCP config")
}

/// The managed stdio server block hosts spawn on demand: `<binary> mcp serve`.
fn claude_server_entry(binary: &Path) -> Result<JsonValue> {
    let command = binary_command_str(binary)?;
    Ok(json!({
        "type": "stdio",
        "command": command,
        "args": SERVER_ARGS,
        "env": {}
    }))
}

fn parse_claude_project_config(existing: Option<&str>) -> Result<JsonMap<String, JsonValue>> {
    let existing = existing.unwrap_or_default();
    if existing.trim().is_empty() {
        return Ok(JsonMap::new());
    }
    match serde_json::from_str(existing).context("parse Claude .mcp.json")? {
        JsonValue::Object(map) => Ok(map),
        _ => bail!("Claude .mcp.json root must be a JSON object"),
    }
}

/// Whether a host config wires the active managed server, and if so whether the
/// wired entry still matches the managed binary. Lets `doctor` tell "never
/// wired" apart from "stale wiring" without rewriting the config, so it can
/// surface a config that points at a moved, wrong-instance, or missing binary —
/// the "MCP not connected" class this onboarding is meant to eliminate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerWiring {
    /// No entry for the active server name.
    Absent,
    /// An entry exists but its command target is not the managed binary.
    Mismatched,
    /// An entry exists and targets the managed binary.
    Wired,
}

/// How a project-local Claude `.mcp.json` string wires the active managed server
/// relative to `binary`. Used by `doctor`.
pub fn claude_project_server_wiring(existing: &str, binary: &Path) -> Result<ServerWiring> {
    let root = parse_claude_project_config(Some(existing))?;
    let Some(server) = root
        .get("mcpServers")
        .and_then(JsonValue::as_object)
        .and_then(|servers| servers.get(instance::mcp_server_name()))
    else {
        return Ok(ServerWiring::Absent);
    };
    Ok(if server == &claude_server_entry(binary)? {
        ServerWiring::Wired
    } else {
        ServerWiring::Mismatched
    })
}

pub fn merge_codex_config(existing: Option<&str>, binary: &Path) -> Result<String> {
    let existing = existing.unwrap_or_default();
    if !existing.trim().is_empty() {
        let value = toml::from_str::<Value>(existing).context("parse Codex config.toml")?;
        let root = value
            .as_table()
            .context("Codex config.toml root must be a table")?;
        if let Some(mcp_servers) = root.get("mcp_servers")
            && !mcp_servers.is_table()
        {
            bail!("Codex config.toml mcp_servers must be a table");
        }
        if defines_mcp_servers_inline(existing) {
            bail!(
                "Codex config.toml defines `mcp_servers` as an inline table or dotted key; rewrite it as `[mcp_servers.NAME]` table sections before installing so the {} block can be merged without producing a duplicate key",
                instance::mcp_server_name()
            );
        }
    }

    let (mut merged, _) = remove_codex_mcp_server_text(existing);
    if !merged.is_empty() && !merged.ends_with('\n') {
        merged.push('\n');
    }
    merged.push_str(&render_codex_mcp_server_block(binary)?);
    if !merged.ends_with('\n') {
        merged.push('\n');
    }
    Ok(merged)
}

/// Persist a rendered Codex `config.toml` to `path` safely. The file is
/// user-owned, so a corrupt render must never land: re-parse the string first
/// (bailing while the original file and its `.llm-wiki-backup` snapshot stay
/// intact) and then write via a temp file in the same directory + rename so a
/// crash mid-write cannot truncate the config.
pub fn write_codex_config_atomic(path: &Path, contents: &str) -> Result<()> {
    toml::from_str::<Value>(contents).with_context(|| {
        format!(
            "refusing to write invalid Codex config.toml to {}",
            path.display()
        )
    })?;
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("failed to create temp Codex config in {}", dir.display()))?;
    temp.write_all(contents.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to persist {}", path.display()))?;
    Ok(())
}

fn render_codex_mcp_server_block(binary: &Path) -> Result<String> {
    let command = binary_command_str(binary)?;
    let mut server = Map::new();
    server.insert("command".to_string(), Value::String(command.to_string()));
    server.insert(
        "args".to_string(),
        Value::Array(
            SERVER_ARGS
                .iter()
                .map(|arg| Value::String((*arg).to_string()))
                .collect(),
        ),
    );
    let body =
        toml::to_string_pretty(&Value::Table(server)).context("render Codex MCP server table")?;
    Ok(format!(
        "[mcp_servers.{}]\n{}",
        instance::mcp_server_name(),
        body
    ))
}

/// How the global Codex `config.toml` string wires the active managed server
/// relative to `binary`. Like the Claude check, this compares the configured
/// `command` against the managed binary so `doctor` surfaces a stale or
/// wrong-instance Codex entry, not merely a missing one.
pub fn codex_server_wiring(existing: &str, binary: &Path) -> Result<ServerWiring> {
    if existing.trim().is_empty() {
        return Ok(ServerWiring::Absent);
    }
    let root: Value = toml::from_str(existing).context("parse Codex config.toml")?;
    let Some(server) = root
        .as_table()
        .and_then(|table| table.get("mcp_servers"))
        .and_then(Value::as_table)
        .and_then(|servers| servers.get(instance::mcp_server_name()))
        .and_then(Value::as_table)
    else {
        return Ok(ServerWiring::Absent);
    };
    // "Wired" must mean the host will actually spawn `<binary> mcp serve`, so
    // check the args too, not just the command: a stale `args = ["status"]` would
    // otherwise read as wired even though Codex would never start the server.
    let command_ok =
        server.get("command").and_then(Value::as_str) == Some(binary_command_str(binary)?);
    let args_ok = server
        .get("args")
        .and_then(Value::as_array)
        .is_some_and(|args| {
            args.len() == SERVER_ARGS.len()
                && args
                    .iter()
                    .zip(SERVER_ARGS)
                    .all(|(value, expected)| value.as_str() == Some(expected))
        });
    Ok(if command_ok && args_ok {
        ServerWiring::Wired
    } else {
        ServerWiring::Mismatched
    })
}

pub fn remove_codex_mcp_server(existing: &str) -> Result<Option<String>> {
    let root: Value = toml::from_str(existing).context("parse Codex config.toml")?;
    let Some(root_table) = root.as_table() else {
        bail!("Codex config.toml root must be a table");
    };
    let Some(mcp_servers) = root_table.get("mcp_servers") else {
        return Ok(None);
    };
    if !mcp_servers.is_table() {
        bail!("Codex config.toml mcp_servers must be a table");
    }
    let (updated, removed) = remove_codex_mcp_server_text(existing);
    Ok(removed.then_some(updated))
}

fn remove_codex_mcp_server_text(existing: &str) -> (String, bool) {
    let mut updated = String::with_capacity(existing.len());
    let lines = existing.split_inclusive('\n').collect::<Vec<_>>();
    let mut removed = false;
    let mut index = 0;
    while index < lines.len() {
        if is_active_codex_mcp_server_header(lines[index]) {
            removed = true;
            index += 1;
            // The block body runs to the next table header (or EOF). Consume all of
            // it — including any blank/comment lines inside the block — but preserve
            // a trailing run of blank/comment lines that directly precedes the next
            // table, since that run is the next table's leading trivia, not ours.
            let mut body_end = index;
            while body_end < lines.len() && !is_toml_table_header(lines[body_end]) {
                body_end += 1;
            }
            let mut keep_from = body_end;
            while keep_from > index && is_blank_or_comment(lines[keep_from - 1]) {
                keep_from -= 1;
            }
            index = keep_from;
            continue;
        }
        updated.push_str(lines[index]);
        index += 1;
    }
    (updated, removed)
}

/// A top-level `mcp_servers = { … }` inline table or `mcp_servers.foo = …` dotted
/// key cannot be merged by the line-based block rewrite without risking a
/// duplicate `mcp_servers` key. Only `[mcp_servers.NAME]` table sections are
/// supported; detect the unsupported forms so the caller can bail.
///
/// Only assignments in the *root* table count: once a `[table]` header is seen,
/// later dotted keys like `mcp_servers.enabled` belong to that table, not root.
fn defines_mcp_servers_inline(existing: &str) -> bool {
    let mut in_root = true;
    for line in existing.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_root = false;
            continue;
        }
        if !in_root || !trimmed.contains('=') {
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        // Match the root dotted-segment with any surrounding quotes stripped so
        // `mcp_servers`, `"mcp_servers"`, `mcp_servers.x`, and `"mcp_servers".x`
        // are all detected (a quoted key otherwise bypassed this guard).
        let root_segment = key.split('.').next().unwrap_or("").trim();
        let unquoted = root_segment.trim_matches('"').trim_matches('\'');
        if unquoted == "mcp_servers" {
            return true;
        }
    }
    false
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Strip an unquoted trailing `# comment` from a TOML line and return the rest.
/// A `#` inside a quoted key/value (basic `"..."` or literal `'...'` string) is
/// content, not a comment, so quoted spans are skipped. This lets header
/// detection see `[mcp_servers.other] # note` as the header it really is.
fn strip_toml_trailing_comment(line: &str) -> &str {
    let mut in_basic = false;
    let mut in_literal = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if in_basic {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_basic = false;
            }
        } else if in_literal {
            if ch == '\'' {
                in_literal = false;
            }
        } else {
            match ch {
                '"' => in_basic = true,
                '\'' => in_literal = true,
                '#' => return &line[..idx],
                _ => {}
            }
        }
    }
    line
}

fn is_active_codex_mcp_server_header(line: &str) -> bool {
    let trimmed = strip_toml_trailing_comment(line).trim();
    trimmed == format!("[mcp_servers.{}]", instance::mcp_server_name())
        || trimmed == format!("[mcp_servers.\"{}\"]", instance::mcp_server_name())
}

fn is_toml_table_header(line: &str) -> bool {
    let trimmed = strip_toml_trailing_comment(line).trim();
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_merge_preserves_existing_keys() {
        let merged = merge_codex_config(
            Some(
                r#"
model = "gpt-5.5"

[mcp_servers.other]
command = "other"
"#,
            ),
            Path::new("/tmp/llm-wiki"),
        )
        .expect("merge");
        let parsed: Value = toml::from_str(&merged).expect("toml");
        assert_eq!(parsed["model"].as_str(), Some("gpt-5.5"));
        assert_eq!(
            parsed["mcp_servers"]["other"]["command"].as_str(),
            Some("other")
        );
        assert_eq!(
            parsed["mcp_servers"][instance::mcp_server_name()]["command"].as_str(),
            Some("/tmp/llm-wiki")
        );
    }

    #[test]
    fn codex_merge_remove_round_trip_preserves_existing_text() {
        let existing = r#"# external provider comment
model_provider = "custom"
notify = ["/tmp/notifier", "turn-ended"]

[projects."/tmp/project with spaces"]
trust_level = "trusted"

[mcp_servers.other]
command = "other"
args = ["serve"]
"#;

        let merged = merge_codex_config(Some(existing), Path::new("/tmp/llm-wiki")).expect("merge");
        assert!(merged.starts_with(existing));
        assert!(merged.contains("[mcp_servers.llm-wiki]"));
        assert!(merged.contains("command = \"/tmp/llm-wiki\""));

        let removed = remove_codex_mcp_server(&merged)
            .expect("remove")
            .expect("updated");
        assert_eq!(removed, existing);
    }

    #[test]
    fn codex_merge_replaces_only_active_server_block() {
        let existing = r#"model = "gpt-5"

[mcp_servers.llm-wiki]
command = "/old/llm-wiki"
args = ["mcp", "serve"]
env = {}

[mcp_servers.other]
command = "other"
"#;

        let merged = merge_codex_config(Some(existing), Path::new("/new/llm-wiki")).expect("merge");
        assert!(!merged.contains("/old/llm-wiki"));
        assert!(merged.contains("/new/llm-wiki"));
        assert!(merged.contains("[mcp_servers.other]"));
    }

    #[test]
    fn codex_remove_preserves_trailing_comment_before_next_table() {
        let existing = format!(
            "model = \"gpt-5\"\n\n[mcp_servers.{name}]\ncommand = \"/old/llm-wiki\"\nargs = [\"mcp\", \"serve\"]\n\n# belongs to the next table\n[mcp_servers.other]\ncommand = \"other\"\n",
            name = instance::mcp_server_name()
        );

        let removed = remove_codex_mcp_server(&existing)
            .expect("remove")
            .expect("updated");
        assert!(!removed.contains("/old/llm-wiki"));
        assert!(removed.contains("# belongs to the next table"));
        assert!(removed.contains("[mcp_servers.other]"));
    }

    #[test]
    fn codex_remove_consumes_block_with_internal_blank_and_comment() {
        // A managed block that was hand-edited to contain a blank line and a comment
        // must still be fully removed up to the next table, not left half-deleted.
        let existing = format!(
            "[mcp_servers.{name}]\ncommand = \"/old/llm-wiki\"\n# hand-added note\n\nargs = [\"mcp\", \"serve\"]\n[mcp_servers.other]\ncommand = \"other\"\n",
            name = instance::mcp_server_name()
        );
        let removed = remove_codex_mcp_server(&existing)
            .expect("remove")
            .expect("updated");
        assert!(!removed.contains("/old/llm-wiki"));
        assert!(!removed.contains("hand-added note"));
        assert!(!removed.contains("args = [\"mcp\", \"serve\"]"));
        assert!(removed.contains("[mcp_servers.other]"));
    }

    #[test]
    fn codex_merge_allows_table_local_mcp_servers_key() {
        // `mcp_servers.enabled` under another table is unrelated to the root
        // `mcp_servers` table and must not block the merge.
        let existing = r#"model = "gpt-5"

[feature.flags]
mcp_servers.enabled = true

[mcp_servers.other]
command = "other"
"#;
        let merged = merge_codex_config(Some(existing), Path::new("/tmp/llm-wiki"))
            .expect("table-local key must not be rejected");
        assert!(merged.contains("[feature.flags]"));
        assert!(merged.contains(&format!("[mcp_servers.{}]", instance::mcp_server_name())));
    }

    #[test]
    fn codex_merge_rejects_inline_mcp_servers_table() {
        let existing = r#"model = "gpt-5"
mcp_servers = { other = { command = "other" } }
"#;
        let error = merge_codex_config(Some(existing), Path::new("/tmp/llm-wiki"))
            .expect_err("inline mcp_servers should be rejected");
        assert!(error.to_string().contains("inline table or dotted key"));
    }

    #[test]
    fn toml_table_header_tolerates_trailing_comment() {
        assert!(is_toml_table_header("[mcp_servers.other] # trailing note"));
        assert!(is_toml_table_header("  [mcp_servers.other]\t# note\n"));
        assert!(!is_toml_table_header("command = \"x\" # not a header"));
        // A `#` inside a quoted key must not be mistaken for a comment.
        assert!(is_toml_table_header("[mcp_servers.\"a#b\"]"));
    }

    #[test]
    fn active_header_tolerates_trailing_comment() {
        let line = format!(
            "[mcp_servers.{}] # managed by installer",
            instance::mcp_server_name()
        );
        assert!(is_active_codex_mcp_server_header(&line));
    }

    #[test]
    fn codex_remove_preserves_next_table_with_trailing_comment_header() {
        let existing = format!(
            "[mcp_servers.{name}]\ncommand = \"/old/llm-wiki\"\nargs = [\"mcp\", \"serve\"]\n[mcp_servers.other] # keep me\ncommand = \"other\"\n",
            name = instance::mcp_server_name()
        );
        let removed = remove_codex_mcp_server(&existing)
            .expect("remove")
            .expect("updated");
        assert!(!removed.contains("/old/llm-wiki"));
        assert!(removed.contains("[mcp_servers.other] # keep me"));
        assert!(removed.contains("command = \"other\""));
    }

    #[test]
    fn claude_merge_into_empty_matches_fresh_render() {
        let binary = Path::new("/tmp/llm-wiki");
        let rendered = render_claude_project_mcp_config(binary).expect("render");
        for empty in [None, Some(""), Some("   \n")] {
            assert_eq!(
                merge_claude_project_mcp_config(empty, binary).expect("merge"),
                rendered,
                "empty/absent input must equal a fresh render"
            );
        }
    }

    #[test]
    fn claude_merge_preserves_unrelated_servers_and_keys() {
        let existing = r#"{
  "someOtherKey": {"keep": true},
  "mcpServers": {
    "other": {"type": "stdio", "command": "other", "args": ["run"]}
  }
}"#;
        let merged = merge_claude_project_mcp_config(Some(existing), Path::new("/tmp/llm-wiki"))
            .expect("merge");
        let parsed: JsonValue = serde_json::from_str(&merged).expect("json");
        assert_eq!(parsed["someOtherKey"]["keep"], JsonValue::Bool(true));
        assert_eq!(
            parsed["mcpServers"]["other"]["command"].as_str(),
            Some("other")
        );
        assert_eq!(
            parsed["mcpServers"][instance::mcp_server_name()]["command"].as_str(),
            Some("/tmp/llm-wiki")
        );
    }

    #[test]
    fn claude_merge_is_idempotent() {
        let binary = Path::new("/tmp/llm-wiki");
        let first = merge_claude_project_mcp_config(
            Some(r#"{"mcpServers": {"other": {"command": "other"}}}"#),
            binary,
        )
        .expect("first merge");
        let second = merge_claude_project_mcp_config(Some(&first), binary).expect("second merge");
        assert_eq!(
            first, second,
            "re-merging own output must be byte-identical"
        );
    }

    #[test]
    fn claude_merge_reconciles_stale_entry_without_duplicating() {
        let existing = format!(
            r#"{{"mcpServers": {{"{name}": {{"type": "stdio", "command": "/old/llm-wiki", "args": ["mcp", "serve"], "env": {{}}}}}}}}"#,
            name = instance::mcp_server_name()
        );
        let merged = merge_claude_project_mcp_config(Some(&existing), Path::new("/new/llm-wiki"))
            .expect("merge");
        let parsed: JsonValue = serde_json::from_str(&merged).expect("json");
        let servers = parsed["mcpServers"].as_object().expect("servers");
        assert_eq!(
            servers.len(),
            1,
            "stale entry must be replaced, not duplicated"
        );
        assert_eq!(
            servers[instance::mcp_server_name()]["command"].as_str(),
            Some("/new/llm-wiki")
        );
    }

    #[test]
    fn claude_merge_rejects_non_object_root() {
        let error = merge_claude_project_mcp_config(Some("[]"), Path::new("/tmp/llm-wiki"))
            .expect_err("array root must be rejected");
        assert!(error.to_string().contains("must be a JSON object"));
    }

    #[test]
    fn claude_server_wiring_distinguishes_wired_stale_and_absent() {
        let binary = Path::new("/tmp/llm-wiki");
        let wired = render_claude_project_mcp_config(binary).expect("render");
        assert_eq!(
            claude_project_server_wiring(&wired, binary).expect("wired"),
            ServerWiring::Wired
        );
        // Same file, different managed binary path => stale, not wired to this binary.
        assert_eq!(
            claude_project_server_wiring(&wired, Path::new("/other/llm-wiki")).expect("stale"),
            ServerWiring::Mismatched
        );
        // Only an unrelated server present => absent.
        assert_eq!(
            claude_project_server_wiring(r#"{"mcpServers": {"other": {}}}"#, binary)
                .expect("unrelated"),
            ServerWiring::Absent
        );
    }

    #[test]
    fn codex_server_wiring_distinguishes_wired_stale_and_absent() {
        let binary = Path::new("/tmp/llm-wiki");
        let merged = merge_codex_config(None, binary).expect("merge");
        assert_eq!(
            codex_server_wiring(&merged, binary).expect("wired"),
            ServerWiring::Wired
        );
        // A stale command target for the active server => mismatched, not absent.
        assert_eq!(
            codex_server_wiring(&merged, Path::new("/other/llm-wiki")).expect("stale"),
            ServerWiring::Mismatched
        );
        assert_eq!(
            codex_server_wiring("", binary).expect("empty"),
            ServerWiring::Absent
        );
        assert_eq!(
            codex_server_wiring("model = \"gpt-5\"\n", binary).expect("no servers"),
            ServerWiring::Absent
        );
    }

    #[test]
    fn codex_server_wiring_rejects_non_serve_args() {
        let binary = Path::new("/tmp/llm-wiki");
        // Right command, but args that would not start the MCP server.
        let wrong_args = format!(
            "[mcp_servers.{name}]\ncommand = \"/tmp/llm-wiki\"\nargs = [\"status\"]\n",
            name = instance::mcp_server_name()
        );
        assert_eq!(
            codex_server_wiring(&wrong_args, binary).expect("wrong args"),
            ServerWiring::Mismatched
        );
        // Command present but args key missing entirely.
        let missing_args = format!(
            "[mcp_servers.{name}]\ncommand = \"/tmp/llm-wiki\"\n",
            name = instance::mcp_server_name()
        );
        assert_eq!(
            codex_server_wiring(&missing_args, binary).expect("missing args"),
            ServerWiring::Mismatched
        );
    }

    #[test]
    fn active_instance_mcp_config_uses_active_server_name_only() {
        let merged =
            merge_codex_config(None, Path::new("/tmp/llm-wiki")).expect("merge codex config");
        let parsed: Value = toml::from_str(&merged).expect("toml");
        let servers = parsed["mcp_servers"].as_table().expect("servers table");
        assert!(servers.contains_key(instance::mcp_server_name()));
        assert_eq!(servers.len(), 1);

        let claude = render_claude_project_mcp_config(Path::new("/tmp/llm-wiki"))
            .expect("render claude config");
        let parsed: JsonValue = serde_json::from_str(&claude).expect("json");
        let servers = parsed["mcpServers"].as_object().expect("servers object");
        assert!(servers.contains_key(instance::mcp_server_name()));
        assert_eq!(servers.len(), 1);
    }
}
