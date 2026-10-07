//! The MCP plumbing both binaries' servers use.
//!
//! The stdio transport, the JSON-RPC envelope, and the dispatch every server
//! shares live here. A server brings its own name, instructions, tools and
//! any further methods through [`Server`], so llm-wiki's and poman's servers
//! frame messages and report failures one way.

use std::io::{self, BufRead, Read, Write};

use serde_json::{Map, Value};

#[cfg(test)]
mod tests;

/// The JSON-RPC version every message carries.
pub const JSONRPC_VERSION: &str = "2.0";

/// The MCP protocol version the servers speak.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The largest message read, in bytes. A longer line is drained without being
/// kept and answered with a parse error, so one unbounded line cannot exhaust
/// memory.
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

/// JSON-RPC's code for a message that is not JSON.
pub const PARSE_ERROR: i64 = -32700;
/// JSON-RPC's code for a request with no method.
pub const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC's code for a method the server does not have.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC's code for parameters the method cannot take.
pub const INVALID_PARAMS: i64 = -32602;
/// JSON-RPC's code for a failure inside the server.
pub const INTERNAL_ERROR: i64 = -32603;

/// A JSON-RPC error a method answers with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RpcError {
    /// The JSON-RPC error code.
    pub code: i64,
    /// What went wrong.
    pub message: String,
    /// More about it, when the method gives any.
    pub data: Option<Value>,
}

impl RpcError {
    /// An error with `code` and `message`, and no data.
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Parameters the method cannot take: [`INVALID_PARAMS`].
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }
}

/// What one server brings: its name and instructions, its tools, and any
/// method beyond the ones every server answers.
pub trait Server {
    /// The server's name, as `initialize` reports it.
    fn name(&self) -> &str;

    /// The server's version, as `initialize` reports it.
    fn version(&self) -> &str;

    /// What the host tells its agent about the server.
    fn instructions(&self) -> String;

    /// The capabilities `initialize` reports; tools only, unless the server
    /// says more.
    fn capabilities(&self) -> Value {
        object([("tools", object([("listChanged", Value::Bool(false))]))])
    }

    /// The tools `tools/list` lists, each with its name, description and
    /// input schema.
    fn tools(&self) -> Vec<Value>;

    /// Runs the tool `name` with its `arguments`, giving the tool's result
    /// (see [`tool_result`]).
    ///
    /// # Errors
    ///
    /// Returns an [`RpcError`] for a tool the server does not have or
    /// arguments it cannot take; a tool that ran and failed is a result with
    /// `isError` set, not an error.
    fn call_tool(&self, name: &str, arguments: Option<&Value>) -> Result<Value, RpcError>;

    /// Answers a method beyond `ping`, `initialize`, `tools/list` and
    /// `tools/call`; `None` when the server does not have it.
    fn other_method(
        &self,
        _method: &str,
        _params: Option<&Value>,
    ) -> Option<Result<Value, RpcError>> {
        None
    }
}

/// Serves `server` over `reader` and `writer`, one JSON-RPC message per line,
/// until the input ends.
///
/// A line over [`MAX_MESSAGE_BYTES`] or one that is not JSON is answered with
/// a parse error; a line that is not UTF-8, or is blank, is skipped; a
/// notification is never answered. Each reply is flushed as it is written.
///
/// # Errors
///
/// Returns the error of a read or a write that failed.
pub fn serve<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    server: &impl Server,
) -> io::Result<()> {
    loop {
        let reply = match read_frame(&mut reader)? {
            Frame::End => return Ok(()),
            Frame::TooLarge => Some(error_response(
                Value::Null,
                &RpcError::new(
                    PARSE_ERROR,
                    format!("parse error: message exceeds {MAX_MESSAGE_BYTES} byte limit"),
                ),
            )),
            Frame::Line(bytes) => String::from_utf8(bytes)
                .ok()
                .filter(|line| !line.trim().is_empty())
                .and_then(|line| match serde_json::from_str::<Value>(&line) {
                    Ok(request) => handle_request(server, &request),
                    Err(error) => Some(error_response(
                        Value::Null,
                        &RpcError::new(PARSE_ERROR, format!("parse error: {error}")),
                    )),
                }),
        };
        if let Some(reply) = reply {
            writeln!(writer, "{reply}")?;
            writer.flush()?;
        }
    }
}

enum Frame {
    Line(Vec<u8>),
    TooLarge,
    End,
}

/// Reads one line, keeping at most [`MAX_MESSAGE_BYTES`] of it.
fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Frame> {
    let mut line = Vec::new();
    let limit = u64::try_from(MAX_MESSAGE_BYTES).unwrap_or(u64::MAX) + 1;
    if reader.by_ref().take(limit).read_until(b'\n', &mut line)? == 0 {
        return Ok(Frame::End);
    }
    if line.len() > MAX_MESSAGE_BYTES && line.last() != Some(&b'\n') {
        drain_line(reader)?;
        return Ok(Frame::TooLarge);
    }
    Ok(Frame::Line(line))
}

/// Discards the rest of a line, up to and including its newline, without
/// keeping it.
fn drain_line<R: BufRead>(reader: &mut R) -> io::Result<()> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        let (used, found) = available
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or((available.len(), false), |newline| (newline + 1, true));
        reader.consume(used);
        if found {
            return Ok(());
        }
    }
}

/// The reply to one request, or `None` for a notification, which is never
/// answered: a message with no `id`, or a method under `notifications/`.
#[must_use]
pub fn handle_request(server: &impl Server, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(error_response(
            id,
            &RpcError::new(INVALID_REQUEST, "missing method"),
        ));
    };
    if request.get("id").is_none() || method.starts_with("notifications/") {
        return None;
    }
    let params = request.get("params");
    let result = match method {
        "ping" => Ok(Value::Object(Map::new())),
        "initialize" => Ok(initialize_result(server)),
        "tools/list" => Ok(object([("tools", Value::Array(server.tools()))])),
        "tools/call" => call_tool(server, params),
        _ => server.other_method(method, params).unwrap_or_else(|| {
            Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("unknown method {method}"),
            ))
        }),
    };
    Some(match result {
        Ok(result) => ok_response(id, result),
        Err(error) => error_response(id, &error),
    })
}

fn initialize_result(server: &impl Server) -> Value {
    object([
        ("protocolVersion", Value::from(PROTOCOL_VERSION)),
        ("capabilities", server.capabilities()),
        (
            "serverInfo",
            object([
                ("name", Value::from(server.name())),
                ("version", Value::from(server.version())),
            ]),
        ),
        ("instructions", Value::from(server.instructions())),
    ])
}

fn call_tool(server: &impl Server, params: Option<&Value>) -> Result<Value, RpcError> {
    let params = params.ok_or_else(|| RpcError::invalid_params("missing tool call params"))?;
    let name = match params.get("name") {
        Some(Value::String(name)) => name,
        Some(_) => {
            return Err(RpcError::invalid_params(
                "invalid tool call params: `name` must be a string",
            ));
        }
        None => {
            return Err(RpcError::invalid_params(
                "invalid tool call params: missing field `name`",
            ));
        }
    };
    server.call_tool(
        name,
        params.get("arguments").filter(|value| !value.is_null()),
    )
}

/// A successful JSON-RPC reply.
///
/// ```
/// use llm_wiki_core::mcp::ok_response;
/// use serde_json::Value;
///
/// let reply = ok_response(Value::from(7), Value::from("done"));
/// assert_eq!(reply.to_string(), r#"{"id":7,"jsonrpc":"2.0","result":"done"}"#);
/// ```
#[must_use]
pub fn ok_response(id: Value, result: Value) -> Value {
    object([
        ("jsonrpc", Value::from(JSONRPC_VERSION)),
        ("id", id),
        ("result", result),
    ])
}

/// A JSON-RPC error reply, with the error's data when it has any.
///
/// ```
/// use llm_wiki_core::mcp::{RpcError, error_response};
/// use serde_json::Value;
///
/// let reply = error_response(Value::Null, &RpcError::new(-32601, "unknown method x"));
/// assert_eq!(reply["error"]["code"], -32601);
/// assert_eq!(reply["error"].get("data"), None);
/// ```
#[must_use]
pub fn error_response(id: Value, error: &RpcError) -> Value {
    let mut body = Map::new();
    body.insert("code".to_owned(), Value::from(error.code));
    body.insert("message".to_owned(), Value::from(error.message.as_str()));
    if let Some(data) = &error.data {
        body.insert("data".to_owned(), data.clone());
    }
    object([
        ("jsonrpc", Value::from(JSONRPC_VERSION)),
        ("id", id),
        ("error", Value::Object(body)),
    ])
}

/// A tool's result: `text` as its one content item, and whether the tool
/// failed.
///
/// ```
/// use llm_wiki_core::mcp::tool_result;
///
/// let result = tool_result("{}".to_owned(), true);
/// assert_eq!(result.to_string(), r#"{"content":[{"text":"{}","type":"text"}],"isError":true}"#);
/// ```
#[must_use]
pub fn tool_result(text: String, is_error: bool) -> Value {
    object([
        (
            "content",
            Value::Array(vec![object([
                ("type", Value::from("text")),
                ("text", Value::from(text)),
            ])]),
        ),
        ("isError", Value::Bool(is_error)),
    ])
}

/// A tool's failure, as `{"error": {"message": …}}` with `isError` set.
///
/// ```
/// use llm_wiki_core::mcp::tool_error;
///
/// let result = tool_error("no such project");
/// assert_eq!(result["content"][0]["text"], r#"{"error":{"message":"no such project"}}"#);
/// ```
#[must_use]
pub fn tool_error(message: impl Into<String>) -> Value {
    let error = object([("error", object([("message", Value::from(message.into()))]))]);
    tool_result(error.to_string(), true)
}

/// A JSON object of these keys and values.
///
/// ```
/// use llm_wiki_core::mcp::object;
/// use serde_json::Value;
///
/// assert_eq!(object([("a", Value::from(1))]).to_string(), r#"{"a":1}"#);
/// ```
#[must_use]
pub fn object<const N: usize>(entries: [(&str, Value); N]) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}
