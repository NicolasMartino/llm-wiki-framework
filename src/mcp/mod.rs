use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cli::{CliContext, McpArgs, McpCommand};
use crate::instance;
use crate::manifest::Manifest;
use crate::paths::Paths;
use crate::payload_integrity::{json_payload_integrity_error, read_content_integrity_error};
use crate::registry::ProjectRegistry;
use crate::search::adapter::{BackendState, SearchBackend};
use crate::search::commands::project_search_readiness;
use crate::search::qmd_rs::QmdRsBackend;
use crate::wiki_read::{WikiReadRequest, discover_current_project_root, read};

const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION: &str = "2025-06-18";
fn server_instructions() -> String {
    format!(
        "Use {} to read wiki/index.md first for project knowledge tasks. Use {} for registered-project queries, then read cited wiki pages directly with {}. Use resources/list for canonical project guidance and prompts/list for optional operation prompts when the host supports MCP prompts.",
        instance::mcp_read_tool_name(),
        instance::mcp_search_tool_name(),
        instance::mcp_read_tool_name()
    )
}

pub fn run(args: &McpArgs, _context: &CliContext) -> Result<()> {
    match &args.command {
        McpCommand::Serve => serve(io::stdin().lock(), io::stdout().lock()),
    }
}

/// Upper bound on a single newline-delimited JSON-RPC frame. `BufRead::lines`
/// buffers an entire line with no cap, so a multi-GB line (or a no-newline
/// stream) would grow the buffer until the process OOMs. We cap the buffered
/// frame here; oversized frames get a JSON-RPC parse error and are skipped.
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

enum FrameRead {
    Line(Vec<u8>),
    TooLarge,
    Eof,
}

/// Read one newline-delimited frame, capping the buffered bytes so a single
/// unbounded line cannot exhaust memory. When a frame exceeds the cap without a
/// newline, the remainder of the line is drained (discarded, never buffered) and
/// `TooLarge` is returned so the caller can report a parse error and continue.
fn read_capped_frame<R: BufRead>(reader: &mut R) -> io::Result<FrameRead> {
    let mut buf = Vec::new();
    // Read at most one byte past the cap so we can tell a full-cap line (with its
    // newline) apart from a line that overflows the cap without one.
    let read = reader
        .by_ref()
        .take(MAX_MESSAGE_BYTES as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(FrameRead::Eof);
    }
    if buf.len() > MAX_MESSAGE_BYTES && buf.last() != Some(&b'\n') {
        drain_to_newline(reader)?;
        return Ok(FrameRead::TooLarge);
    }
    Ok(FrameRead::Line(buf))
}

/// Discard bytes up to and including the next newline (or EOF) without buffering
/// them, so an oversized frame does not keep growing memory.
fn drain_to_newline<R: BufRead>(reader: &mut R) -> io::Result<()> {
    loop {
        let (found, used) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(());
            }
            match available.iter().position(|&byte| byte == b'\n') {
                Some(index) => (true, index + 1),
                None => (false, available.len()),
            }
        };
        reader.consume(used);
        if found {
            return Ok(());
        }
    }
}

fn serve<R, W>(mut reader: R, mut writer: W) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    loop {
        let buf = match read_capped_frame(&mut reader)? {
            FrameRead::Eof => break,
            FrameRead::TooLarge => {
                let response = error_response(
                    Value::Null,
                    -32700,
                    format!("parse error: message exceeds {MAX_MESSAGE_BYTES} byte limit"),
                );
                serde_json::to_writer(&mut writer, &response)?;
                writer.write_all(b"\n")?;
                writer.flush()?;
                continue;
            }
            FrameRead::Line(buf) => buf,
        };

        // A stray non-UTF-8 byte on stdin must not tear down the whole session;
        // skip the malformed frame and keep serving.
        let line = match String::from_utf8(buf) {
            Ok(line) => line,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(request),
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                format!("parse error: {error}"),
            )),
        };

        if let Some(response) = response {
            serde_json::to_writer(&mut writer, &response)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn handle_request(request: Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => return Some(error_response(id, -32600, "missing method")),
    };

    // JSON-RPC notifications carry no `id` (conventionally `notifications/*`) and
    // must never receive a reply — even for an otherwise-known method like
    // `tools/list`. Guard before dispatching so no method arm can answer one.
    if request.get("id").is_none() || method.starts_with("notifications/") {
        return None;
    }

    let response = match method {
        // The MCP spec requires a receiver to reply to `ping` with an empty result.
        "ping" => ok_response(id, json!({})),
        "initialize" => ok_response(id, initialize_result()),
        "tools/list" => ok_response(id, tools_list_result()),
        "tools/call" => handle_tool_call(id, request.get("params").cloned()),
        "resources/list" => handle_resources_list(id),
        "resources/read" => handle_resources_read(id, request.get("params").cloned()),
        "prompts/list" => ok_response(id, prompts_list_result()),
        "prompts/get" => handle_prompts_get(id, request.get("params").cloned()),
        _ => error_response(id, -32601, format!("unknown method {method}")),
    };
    Some(response)
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"subscribe": false, "listChanged": false},
                "prompts": {"listChanged": false}
            },
        "serverInfo": {
            "name": instance::mcp_server_name(),
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": server_instructions()
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": instance::mcp_read_tool_name(),
                "description": "Read a UTF-8 file from the active LLM Wiki project's wiki/ or raw/ tree with provenance metadata.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Project-relative path under wiki/ or raw/, or an absolute path inside those trees."
                        },
                        "project": {
                            "type": "string",
                            "description": "Optional registered project id. Defaults to the project discovered from the MCP server current directory."
                        }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }
            },
            {
                "name": instance::mcp_search_tool_name(),
                "description": "Search one registered LLM Wiki project using the same deterministic search path as the CLI. With compact, the reply carries one page of the hits: up to page_size of them from offset; next_offset is where the next page starts, and has_more is true when more hits follow this page, whether kept or past limit.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "project": {"type": "string"},
                        "mode": {"type": "string", "enum": ["auto", "lexical", "semantic", "hybrid"]},
                        "class": {"type": "string"},
                        "status": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1, "default": 10, "description": "The most hits the search keeps; result_count counts them."},
                        "compact": {"type": "boolean"},
                        "page_size": {"type": "integer", "minimum": 1, "description": "With compact, the most results one reply carries; defaults to limit."},
                        "offset": {"type": "integer", "minimum": 0, "default": 0, "description": "With compact, where the page starts among the kept hits; pass next_offset for the next page."},
                        "allow_lexical_fallback": {"type": "boolean"},
                        "rerank": {"type": "boolean"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }
            },
            {
                "name": instance::mcp_search_all_tool_name(),
                "description": "Search across registered LLM Wiki projects using the same deterministic search-all path as the CLI. With compact, the reply carries one page of the hits: up to page_size of them from offset; next_offset is where the next page starts, and has_more is true when more hits follow this page, whether kept or past limit.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "include": {"type": "array", "items": {"type": "string"}},
                        "exclude": {"type": "array", "items": {"type": "string"}},
                        "mode": {"type": "string", "enum": ["auto", "lexical", "semantic", "hybrid"]},
                        "class": {"type": "string"},
                        "status": {"type": "string"},
                        "limit": {"type": "integer", "minimum": 1, "default": 10, "description": "The most hits the search keeps; result_count counts them."},
                        "compact": {"type": "boolean"},
                        "page_size": {"type": "integer", "minimum": 1, "description": "With compact, the most results one reply carries; defaults to limit."},
                        "offset": {"type": "integer", "minimum": 0, "default": 0, "description": "With compact, where the page starts among the kept hits; pass next_offset for the next page."},
                        "allow_lexical_fallback": {"type": "boolean"},
                        "rerank": {"type": "boolean"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }
            },
            {
                "name": instance::mcp_index_tool_name(),
                "description": "Build or refresh the search index for a registered LLM Wiki project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": {"type": "string"},
                        "force": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": instance::mcp_register_tool_name(),
                "description": "Register or update an LLM Wiki project in the project registry.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "id": {"type": "string"},
                        "name": {"type": "string"},
                        "update": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": instance::mcp_status_tool_name(),
                "description": "Return installation, project registration, index freshness, and search readiness for the current llm-wiki instance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "project": {
                            "type": "string",
                            "description": "Optional registered project id. Defaults to the project discovered from the MCP server current directory."
                        }
                    },
                    "additionalProperties": false
                }
            }
        ]
    })
}

fn prompts_list_result() -> Value {
    json!({
        "prompts": [
            {
                "name": "wiki_query",
                "description": "Answer a project knowledge question using wiki-first MCP retrieval.",
                "arguments": [
                    {
                        "name": "query",
                        "description": "The user's project knowledge question.",
                        "required": false
                    }
                ]
            },
            {
                "name": "wiki_ingest",
                "description": "Ingest curated raw material into typed wiki pages and bookkeeping.",
                "arguments": [
                    {
                        "name": "source",
                        "description": "Raw source path, directory, or short source description.",
                        "required": false
                    }
                ]
            },
            {
                "name": "wiki_lint",
                "description": "Check wiki consistency, stale claims, missing links, and bookkeeping.",
                "arguments": [
                    {
                        "name": "scope",
                        "description": "Optional wiki area or concern to lint.",
                        "required": false
                    }
                ]
            },
            {
                "name": "wiki_research",
                "description": "Prepare external or local research material for later wiki ingest.",
                "arguments": [
                    {
                        "name": "topic",
                        "description": "Research topic, URL, or local material to collect.",
                        "required": false
                    }
                ]
            },
            {
                "name": "wiki_init",
                "description": "Initialize or update an LLM Wiki project using project guidance.",
                "arguments": [
                    {
                        "name": "path",
                        "description": "Project path to initialize or update.",
                        "required": false
                    }
                ]
            }
        ]
    })
}

#[derive(Deserialize)]
struct PromptGetParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

fn handle_prompts_get(id: Value, params: Option<Value>) -> Value {
    let params = match params {
        Some(params) => params,
        None => return error_response(id, -32602, "missing prompt get params"),
    };
    let params: PromptGetParams = match serde_json::from_value(params) {
        Ok(params) => params,
        Err(error) => {
            return error_response(id, -32602, format!("invalid prompt get params: {error}"));
        }
    };

    let Some(text) = prompt_text(&params.name, &params.arguments) else {
        return error_response(id, -32602, format!("unknown prompt {}", params.name));
    };

    ok_response(
        id,
        json!({
            "description": prompt_description(&params.name),
            "messages": [
                {
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": text
                    }
                }
            ]
        }),
    )
}

fn prompt_description(name: &str) -> &'static str {
    match name {
        "wiki_query" => "Answer a project knowledge question using wiki-first MCP retrieval.",
        "wiki_ingest" => "Ingest curated raw material into typed wiki pages and bookkeeping.",
        "wiki_lint" => "Check wiki consistency, stale claims, missing links, and bookkeeping.",
        "wiki_research" => "Prepare external or local research material for later wiki ingest.",
        "wiki_init" => "Initialize or update an LLM Wiki project using project guidance.",
        _ => "LLM Wiki prompt",
    }
}

fn prompt_arg(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn prompt_text(name: &str, arguments: &Value) -> Option<String> {
    let text = match name {
        "wiki_query" => {
            let query =
                prompt_arg(arguments, "query").unwrap_or_else(|| "<user query>".to_string());
            format!(
                "Answer this project knowledge question: {query}\n\nRead `wiki/index.md` first with `{}`, then use `{}` when the project is registered. Read cited wiki pages directly with `{}`. Answer with wiki path citations and keep durable new knowledge in the wiki when appropriate.",
                instance::mcp_read_tool_name(),
                instance::mcp_search_tool_name(),
                instance::mcp_read_tool_name()
            )
        }
        "wiki_ingest" => {
            let source =
                prompt_arg(arguments, "source").unwrap_or_else(|| "<raw source>".to_string());
            format!(
                "Ingest this curated source into the LLM Wiki: {source}\n\nRead `wiki/index.md` and the source material with `{}`. Update or create correctly typed wiki pages, check contradictions, update `wiki/index.md`, and append `wiki/log.md`.",
                instance::mcp_read_tool_name()
            )
        }
        "wiki_lint" => {
            let scope = prompt_arg(arguments, "scope").unwrap_or_else(|| "the wiki".to_string());
            format!(
                "Lint {scope} for contradictions, stale statuses, orphan pages, missing cross-references, and bookkeeping drift.\n\nStart from `wiki/index.md` using `{}`. Use `{}` where useful, fix issues directly, and append `wiki/log.md`.",
                instance::mcp_read_tool_name(),
                instance::mcp_search_tool_name()
            )
        }
        "wiki_research" => {
            let topic =
                prompt_arg(arguments, "topic").unwrap_or_else(|| "<research topic>".to_string());
            format!(
                "Prepare research material for later LLM Wiki ingest: {topic}\n\nUse project guidance from `wiki/index.md` and MCP resources. Store curated source material under `raw/research/` and leave ingest-ready notes with provenance."
            )
        }
        "wiki_init" => {
            let path =
                prompt_arg(arguments, "path").unwrap_or_else(|| "<project path>".to_string());
            format!(
                "Initialize or update an LLM Wiki project at {path}.\n\nUse project guidance resources and keep generated project instructions aligned with MCP tools. Preserve existing user content and update wiki bookkeeping when the project already has a wiki."
            )
        }
        _ => return None,
    };

    Some(text)
}

fn handle_resources_list(id: Value) -> Value {
    match resource_list() {
        Ok(resources) => ok_response(id, json!({ "resources": resources })),
        Err(error) => error_response(id, -32603, error.to_string()),
    }
}

fn handle_resources_read(id: Value, params: Option<Value>) -> Value {
    let params = match params {
        Some(params) => params,
        None => return error_response(id, -32602, "missing resource read params"),
    };
    let params = match serde_json::from_value::<ResourceReadParams>(params) {
        Ok(params) => params,
        Err(error) => {
            return error_response(id, -32602, format!("invalid resource read params: {error}"));
        }
    };
    // An unrecognized URI is a resource-not-found condition, not an internal
    // error: reply with the spec's -32002 code and echo the uri in `data`.
    let known_uri = resource_specs()
        .iter()
        .any(|spec| resource_uri(spec.path) == params.uri);
    if !known_uri {
        return error_response_with_data(
            id,
            -32002,
            format!("unknown llm-wiki resource uri {}", params.uri),
            json!({ "uri": params.uri }),
        );
    }
    match read_resource(&params.uri) {
        Ok(resource) => ok_response(id, json!({ "contents": [resource] })),
        Err(error) => error_response(id, -32603, error.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct ResourceReadParams {
    uri: String,
}

#[derive(Debug, Serialize)]
struct ResourceDescriptor {
    uri: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
}

#[derive(Debug, Serialize)]
struct ResourceContent {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
    text: String,
}

fn resource_list() -> Result<Vec<ResourceDescriptor>> {
    let root = discover_current_project_root()?;
    Ok(resource_specs()
        .into_iter()
        .filter(|spec| resolve_resource_path(&root, spec).is_some())
        .map(|spec| ResourceDescriptor {
            uri: resource_uri(spec.path),
            name: spec.path.to_string(),
            mime_type: spec.mime_type,
        })
        .collect())
}

fn read_resource(uri: &str) -> Result<ResourceContent> {
    let spec = resource_specs()
        .into_iter()
        .find(|spec| resource_uri(spec.path) == uri)
        .with_context(|| format!("unknown llm-wiki resource uri {uri}"))?;
    let root = discover_current_project_root()?;
    let path = resolve_resource_path(&root, &spec)
        .with_context(|| format!("resource {uri} is not available in {}", root.display()))?;
    let canonical_root = fs::canonicalize(&root)
        .with_context(|| format!("canonicalize project root {}", root.display()))?;
    let canonical_path = fs::canonicalize(&path)
        .with_context(|| format!("canonicalize resource {}", path.display()))?;
    if !canonical_path.starts_with(&canonical_root) {
        bail!("{uri} resolves outside the active project");
    }
    let text = fs::read_to_string(&canonical_path)
        .with_context(|| format!("read resource {}", canonical_path.display()))?;
    Ok(ResourceContent {
        uri: uri.to_string(),
        mime_type: spec.mime_type,
        text,
    })
}

fn resolve_resource_path(root: &Path, spec: &ResourceSpec) -> Option<PathBuf> {
    std::iter::once(spec.path)
        .chain(resource_path_aliases(spec.path).iter().copied())
        .map(|path| root.join(path))
        .find(|path| path.is_file())
}

fn resource_path_aliases(path: &str) -> &'static [&'static str] {
    match path {
        "AGENTS.md" => &["AGENTS.MD"],
        _ => &[],
    }
}

#[derive(Clone, Copy)]
struct ResourceSpec {
    path: &'static str,
    mime_type: &'static str,
}

fn resource_specs() -> Vec<ResourceSpec> {
    vec![
        ResourceSpec {
            path: "wiki/index.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/log.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/documentation-model.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/wiki-query-skill.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/wiki-ingest-skill.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/wiki-lint-skill.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/wiki-research-skill.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "wiki/specs/wiki-init-skill.spec.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "AGENTS.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "project_guidelines.md",
            mime_type: "text/markdown",
        },
        ResourceSpec {
            path: "templates/base/project_guidelines.md",
            mime_type: "text/markdown",
        },
    ]
}

fn resource_uri(path: &str) -> String {
    format!("llm-wiki://project/{path}")
}

fn handle_tool_call(id: Value, params: Option<Value>) -> Value {
    let params = match params {
        Some(params) => params,
        None => return error_response(id, -32602, "missing tool call params"),
    };
    let params = match serde_json::from_value::<ToolCallParams>(params) {
        Ok(params) => params,
        Err(error) => {
            return error_response(id, -32602, format!("invalid tool call params: {error}"));
        }
    };

    match params.name.as_str() {
        name if name == instance::mcp_read_tool_name() => call_wiki_read(id, params.arguments),
        name if name == instance::mcp_search_tool_name() => call_search(id, params.arguments),
        name if name == instance::mcp_search_all_tool_name() => {
            call_search_all(id, params.arguments)
        }
        name if name == instance::mcp_index_tool_name() => call_index(id, params.arguments),
        name if name == instance::mcp_register_tool_name() => call_register(id, params.arguments),
        name if name == instance::mcp_status_tool_name() => call_status(id, params.arguments),
        // `tools/call` is a known method; an unknown tool name is an invalid
        // parameter (-32602), not a missing method (-32601).
        name => error_response(id, -32602, format!("unknown tool {name}")),
    }
}

fn call_wiki_read(id: Value, arguments: Option<Value>) -> Value {
    let arguments = arguments.unwrap_or_else(|| json!({}));
    let request = match serde_json::from_value::<WikiReadRequest>(arguments) {
        Ok(request) => request,
        Err(error) => {
            return error_response(
                id,
                -32602,
                format!(
                    "invalid {} arguments: {error}",
                    instance::mcp_read_tool_name()
                ),
            );
        }
    };
    match read(request) {
        Ok(response) => {
            if let Some(content) = response.content.as_deref()
                && let Some(error) =
                    read_content_integrity_error(content, &response.sha256, response.byte_len)
            {
                return tool_error_response(id, error.to_string());
            }
            tool_json_response(id, &response)
        }
        Err(error) => tool_error_response(id, error.to_string()),
    }
}

fn call_status(id: Value, arguments: Option<Value>) -> Value {
    let arguments = arguments.unwrap_or_else(|| json!({}));
    let request = match serde_json::from_value::<StatusArgs>(arguments) {
        Ok(request) => request,
        Err(error) => {
            return error_response(
                id,
                -32602,
                format!(
                    "invalid {} arguments: {error}",
                    instance::mcp_status_tool_name()
                ),
            );
        }
    };
    match status_payload(request.project.as_deref()) {
        Ok(response) => tool_json_response(id, &response),
        Err(error) => tool_error_response(id, error.to_string()),
    }
}

fn call_search(id: Value, arguments: Option<Value>) -> Value {
    let args = match search_cli_args(arguments, false) {
        Ok(args) => args,
        Err(error) => return error_response(id, -32602, error.to_string()),
    };
    call_cli_json(id, args)
}

fn call_search_all(id: Value, arguments: Option<Value>) -> Value {
    let args = match search_cli_args(arguments, true) {
        Ok(args) => args,
        Err(error) => return error_response(id, -32602, error.to_string()),
    };
    call_cli_json(id, args)
}

fn call_index(id: Value, arguments: Option<Value>) -> Value {
    let args = match index_cli_args(arguments) {
        Ok(args) => args,
        Err(error) => return error_response(id, -32602, error.to_string()),
    };
    call_cli_text(id, args)
}

fn call_register(id: Value, arguments: Option<Value>) -> Value {
    let args = match register_cli_args(arguments) {
        Ok(args) => args,
        Err(error) => return error_response(id, -32602, error.to_string()),
    };
    call_cli_text(id, args)
}

fn call_cli_json(id: Value, args: Vec<String>) -> Value {
    match run_cli(args).and_then(|output| output.json()) {
        Ok(value) => tool_json_response(id, &value),
        Err(error) => tool_error_response(id, error.to_string()),
    }
}

fn call_cli_text(id: Value, args: Vec<String>) -> Value {
    match run_cli(args) {
        Ok(output) => tool_json_response(id, &output),
        Err(error) => tool_error_response(id, error.to_string()),
    }
}

#[derive(Debug, Serialize)]
struct CliToolOutput {
    stdout: String,
    stderr: String,
}

impl CliToolOutput {
    fn json(self) -> Result<Value> {
        let value = serde_json::from_str(&self.stdout)
            .with_context(|| "CLI command did not return JSON on stdout")?;
        if let Some(error) = json_payload_integrity_error(&value) {
            bail!("{error}");
        }
        Ok(value)
    }
}

/// Upper bound on the `stderr` we surface in a successful tool result. The
/// embedder (ggml/llama_model_loader) emits tens of KB of model-loader noise on
/// `index`; an agent never needs that, but a final warning/summary line is worth
/// keeping. We keep the tail and note how much was dropped. The failure path is
/// unaffected — it still reports full stderr for debugging.
const MAX_TOOL_STDERR_BYTES: usize = 2048;

fn bound_tool_stderr(stderr: String) -> String {
    if stderr.len() <= MAX_TOOL_STDERR_BYTES {
        return stderr;
    }
    let mut cut = stderr.len() - MAX_TOOL_STDERR_BYTES;
    while cut < stderr.len() && !stderr.is_char_boundary(cut) {
        cut += 1;
    }
    format!(
        "[{} bytes of earlier stderr omitted]\n{}",
        cut,
        &stderr[cut..]
    )
}

fn run_cli(args: Vec<String>) -> Result<CliToolOutput> {
    let output = Command::new(std::env::current_exe().context("resolve current executable")?)
        .args(&args)
        .output()
        .with_context(|| format!("run llm-wiki {}", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        // Bound failure output too: a failed `index`/search-runtime call can emit
        // tens of KB of ggml/llama_model_loader noise, and the actual error is at
        // the tail. Keep the tail so the agent context stays lean either way.
        bail!(
            "llm-wiki {} failed with status {}: stderr: {} | stdout: {}",
            args.join(" "),
            output.status,
            bound_tool_stderr(stderr),
            bound_tool_stderr(stdout)
        );
    }

    Ok(CliToolOutput {
        stdout,
        stderr: bound_tool_stderr(stderr),
    })
}

#[derive(Debug, Deserialize)]
struct SearchToolArgs {
    query: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    project: Option<String>,
    #[serde(default, rename = "class")]
    document_class: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    compact: bool,
    #[serde(default)]
    page_size: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
    #[serde(default)]
    allow_lexical_fallback: bool,
    #[serde(default)]
    rerank: bool,
}

fn search_cli_args(arguments: Option<Value>, all_projects: bool) -> Result<Vec<String>> {
    let input: SearchToolArgs = serde_json::from_value(arguments.unwrap_or_else(|| json!({})))
        .with_context(|| "invalid search arguments")?;
    let mut args = vec![
        if all_projects { "search-all" } else { "search" }.to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    if let Some(mode) = input.mode {
        args.extend(["--mode".to_string(), mode]);
    }
    if !all_projects && let Some(project) = input.project {
        args.extend(["--project".to_string(), project]);
    }
    if all_projects {
        for project in input.include {
            args.extend(["--include".to_string(), project]);
        }
        for project in input.exclude {
            args.extend(["--exclude".to_string(), project]);
        }
    }
    if let Some(document_class) = input.document_class {
        args.extend(["--class".to_string(), document_class]);
    }
    if let Some(status) = input.status {
        args.extend(["--status".to_string(), status]);
    }
    if let Some(limit) = input.limit {
        args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if input.compact {
        args.push("--compact".to_string());
    }
    if let Some(page_size) = input.page_size {
        args.extend(["--page-size".to_string(), page_size.to_string()]);
    }
    if let Some(offset) = input.offset {
        args.extend(["--offset".to_string(), offset.to_string()]);
    }
    if input.allow_lexical_fallback {
        args.push("--allow-lexical-fallback".to_string());
    }
    if input.rerank {
        args.push("--rerank".to_string());
    }
    // Emit the query as a positional after `--` so a leading-dash value
    // (e.g. "-foo" or "--help") is parsed as the query, not a child flag.
    args.push("--".to_string());
    args.push(input.query);
    Ok(args)
}

#[derive(Debug, Deserialize)]
struct IndexToolArgs {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    force: bool,
}

fn index_cli_args(arguments: Option<Value>) -> Result<Vec<String>> {
    let input: IndexToolArgs = serde_json::from_value(arguments.unwrap_or_else(|| json!({})))
        .with_context(|| "invalid index arguments")?;
    let mut args = vec!["index".to_string()];
    if let Some(project) = input.project {
        args.extend(["--project".to_string(), project]);
    }
    if input.force {
        args.push("--force".to_string());
    }
    Ok(args)
}

#[derive(Debug, Deserialize)]
struct RegisterToolArgs {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    update: Option<String>,
}

fn register_cli_args(arguments: Option<Value>) -> Result<Vec<String>> {
    let input: RegisterToolArgs = serde_json::from_value(arguments.unwrap_or_else(|| json!({})))
        .with_context(|| "invalid register arguments")?;
    let mut args = vec!["register".to_string()];
    if let Some(update) = input.update {
        args.extend(["--update".to_string(), update]);
    }
    if let Some(id) = input.id {
        args.extend(["--id".to_string(), id]);
    }
    if let Some(name) = input.name {
        args.extend(["--name".to_string(), name]);
    }
    if let Some(path) = input.path {
        // Positional path after `--` so a leading-dash path is not parsed as a flag.
        args.push("--".to_string());
        args.push(path);
    }
    Ok(args)
}

#[derive(Debug, Serialize)]
struct StatusPayload {
    installed: bool,
    manifest_path: String,
    managed_home: String,
    binary_stem: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<ProjectStatusPayload>,
}

#[derive(Debug, Deserialize)]
struct StatusArgs {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProjectStatusPayload {
    requested_project: Option<String>,
    registered: bool,
    project_id: Option<String>,
    project_name: Option<String>,
    project_root: Option<String>,
    index_present: bool,
    index_freshness: Option<String>,
    index_state: Option<String>,
    indexed_files: Option<usize>,
    lexical_ready: bool,
    semantic_ready: bool,
    hybrid_ready: bool,
    readiness_reason: Option<String>,
}

fn status_payload(project_id: Option<&str>) -> anyhow::Result<StatusPayload> {
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    let manifest = Manifest::read(&manifest_path)?;
    let project = project_status_payload(&paths, project_id)?;
    Ok(StatusPayload {
        installed: manifest.is_some(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        managed_home: paths.managed_home().to_string_lossy().to_string(),
        binary_stem: instance::binary_stem().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        project,
    })
}

fn project_status_payload(
    paths: &Paths,
    requested_project: Option<&str>,
) -> anyhow::Result<Option<ProjectStatusPayload>> {
    let registry = ProjectRegistry::read(&paths.project_registry())?;
    let registered = if let Some(project_id) = requested_project {
        registry.project_by_id(project_id)
    } else {
        discover_current_project_root()
            .ok()
            .and_then(|root| registry.project_by_root(&root))
    };

    let Some(project) = registered else {
        return Ok(Some(ProjectStatusPayload {
            requested_project: requested_project.map(str::to_string),
            registered: false,
            project_id: None,
            project_name: None,
            project_root: None,
            index_present: false,
            index_freshness: None,
            index_state: None,
            indexed_files: None,
            lexical_ready: false,
            semantic_ready: false,
            hybrid_ready: false,
            readiness_reason: Some("project_not_registered".to_string()),
        }));
    };

    let backend = QmdRsBackend;
    let store_path = paths.qmd_rs_store_path(&project.id);
    let status = backend.status(&project.id, &store_path, &project.wiki_root())?;
    let search_readiness = project_search_readiness(paths, project, &status)?;
    let index_freshness = match status.state {
        BackendState::Ready => Some("fresh".to_string()),
        BackendState::Stale => Some("stale".to_string()),
        _ => None,
    };

    Ok(Some(ProjectStatusPayload {
        requested_project: requested_project.map(str::to_string),
        registered: true,
        project_id: Some(project.id.clone()),
        project_name: Some(project.name.clone()),
        project_root: Some(project.root.to_string_lossy().to_string()),
        index_present: search_readiness.lexical_ready,
        index_freshness,
        index_state: Some(format!("{:?}", status.state).to_lowercase()),
        indexed_files: Some(status.indexed_files),
        lexical_ready: search_readiness.lexical_ready,
        semantic_ready: search_readiness.semantic_ready,
        hybrid_ready: search_readiness.hybrid_ready,
        readiness_reason: search_readiness.readiness_reason,
    }))
}

fn tool_json_response<T: Serialize>(id: Value, payload: &T) -> Value {
    let text = match serde_json::to_string(payload) {
        Ok(text) => text,
        Err(error) => {
            return error_response(id, -32603, format!("serialize tool response: {error}"));
        }
    };

    tool_text_response(id, text, false)
}

fn tool_error_response(id: Value, message: impl Into<String>) -> Value {
    let text = json!({
        "error": {
            "message": message.into()
        }
    })
    .to_string();

    tool_text_response(id, text, true)
}

fn tool_text_response(id: Value, text: String, is_error: bool) -> Value {
    ok_response(
        id,
        json!({
            "content": [{"type": "text", "text": text}],
            "isError": is_error
        }),
    )
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

fn ok_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

fn error_response_with_data(
    id: Value,
    code: i64,
    message: impl Into<String>,
    data: Value,
) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
            "data": data
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{MAX_TOOL_STDERR_BYTES, bound_tool_stderr};

    #[test]
    fn short_stderr_is_passed_through_unchanged() {
        let s = "indexed 10 files\n".to_string();
        assert_eq!(bound_tool_stderr(s.clone()), s);
    }

    #[test]
    fn oversized_stderr_keeps_tail_and_notes_omission() {
        // Simulate embedder noise followed by a useful trailing line.
        let noise = "llama_model_loader: loading tensor\n".repeat(2000);
        let tail = "indexed 12 files; 1 warning\n";
        let stderr = format!("{noise}{tail}");
        assert!(stderr.len() > MAX_TOOL_STDERR_BYTES);

        let bounded = bound_tool_stderr(stderr);
        assert!(bounded.len() < MAX_TOOL_STDERR_BYTES + 64);
        assert!(bounded.starts_with("[") && bounded.contains("bytes of earlier stderr omitted]"));
        // The meaningful trailing line survives.
        assert!(bounded.ends_with(tail));
    }

    #[test]
    fn truncation_respects_utf8_char_boundaries() {
        // Multi-byte chars right around the cut point must not panic or split.
        let stderr = "é".repeat(MAX_TOOL_STDERR_BYTES); // 2 bytes each
        let bounded = bound_tool_stderr(stderr);
        // Must be valid UTF-8 (String guarantees it) and contain the marker.
        assert!(bounded.contains("bytes of earlier stderr omitted]"));
    }
}
