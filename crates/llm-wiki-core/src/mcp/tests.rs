use std::io::{self, BufRead, BufReader, Cursor, Read, Write};

use serde_json::Value;

use super::{
    Frame, INVALID_PARAMS, INVALID_REQUEST, MAX_MESSAGE_BYTES, METHOD_NOT_FOUND, PARSE_ERROR,
    PROTOCOL_VERSION, RpcError, Server, drain_line, error_response, handle_request, object,
    read_frame, serve, tool_error, tool_result,
};

/// A server with one tool, `echo`, that gives back its arguments.
struct Echo;

impl Server for Echo {
    fn name(&self) -> &'static str {
        "echo-server"
    }

    fn version(&self) -> &'static str {
        "1.2.3"
    }

    fn instructions(&self) -> String {
        "Call echo.".to_owned()
    }

    fn tools(&self) -> Vec<Value> {
        vec![object([("name", Value::from("echo"))])]
    }

    fn call_tool(&self, name: &str, arguments: Option<&Value>) -> Result<Value, RpcError> {
        if name != "echo" {
            return Err(RpcError::invalid_params(format!("unknown tool {name}")));
        }
        let text = arguments.map_or_else(|| "no arguments".to_owned(), Value::to_string);
        Ok(tool_result(text, false))
    }
}

/// A server that also answers `extra/hello` and says more in `initialize`.
struct Extra;

impl Server for Extra {
    fn name(&self) -> &'static str {
        "extra"
    }

    fn version(&self) -> &'static str {
        "0"
    }

    fn instructions(&self) -> String {
        String::new()
    }

    fn capabilities(&self) -> Value {
        object([("prompts", object([]))])
    }

    fn tools(&self) -> Vec<Value> {
        Vec::new()
    }

    fn call_tool(&self, _name: &str, _arguments: Option<&Value>) -> Result<Value, RpcError> {
        Err(RpcError::new(-1, "no tools"))
    }

    fn other_method(
        &self,
        method: &str,
        params: Option<&Value>,
    ) -> Option<Result<Value, RpcError>> {
        match method {
            "extra/hello" => Some(Ok(params.cloned().unwrap_or(Value::Null))),
            "extra/fail" => Some(Err(RpcError {
                code: -32002,
                message: "not found".to_owned(),
                data: Some(object([("uri", Value::from("x"))])),
            })),
            _ => None,
        }
    }
}

/// The value at `pointer` in `value`, or null when there is none.
fn at<'value>(value: &'value Value, pointer: &str) -> &'value Value {
    static NULL: Value = Value::Null;
    value.pointer(pointer).unwrap_or(&NULL)
}

fn request(text: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(text)
}

fn reply(server: &impl Server, text: &str) -> Result<Value, serde_json::Error> {
    Ok(handle_request(server, &request(text)?).unwrap_or(Value::Null))
}

fn served(server: &impl Server, input: &[u8]) -> io::Result<Vec<Value>> {
    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output, server)?;
    String::from_utf8_lossy(&output)
        .lines()
        .map(|line| serde_json::from_str(line).map_err(io::Error::other))
        .collect()
}

#[test]
fn initialize_names_the_server() -> Result<(), serde_json::Error> {
    let reply = reply(&Echo, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)?;
    assert_eq!(at(&reply, "/jsonrpc"), "2.0");
    assert_eq!(at(&reply, "/id"), 1);
    let result = at(&reply, "/result");
    assert_eq!(at(result, "/protocolVersion"), PROTOCOL_VERSION);
    assert_eq!(at(result, "/serverInfo/name"), "echo-server");
    assert_eq!(at(result, "/serverInfo/version"), "1.2.3");
    assert_eq!(at(result, "/instructions"), "Call echo.");
    assert_eq!(at(result, "/capabilities/tools/listChanged"), false);
    let extra = self::reply(&Extra, r#"{"id":"a","method":"initialize"}"#)?;
    assert_eq!(at(&extra, "/id"), "a");
    assert_eq!(
        at(&extra, "/result/capabilities"),
        &object([("prompts", object([]))])
    );
    Ok(())
}

#[test]
fn ping_and_tools_list_answer() -> Result<(), serde_json::Error> {
    assert_eq!(
        at(&reply(&Echo, r#"{"id":2,"method":"ping"}"#)?, "/result"),
        &object([])
    );
    let tools = reply(&Echo, r#"{"id":3,"method":"tools/list"}"#)?;
    assert_eq!(at(&tools, "/result/tools/0/name"), "echo");
    Ok(())
}

#[test]
fn tools_call_passes_the_name_and_arguments() -> Result<(), serde_json::Error> {
    let called = reply(
        &Echo,
        r#"{"id":4,"method":"tools/call","params":{"name":"echo","arguments":{"a":1}}}"#,
    )?;
    assert_eq!(at(&called, "/result/content/0/text"), r#"{"a":1}"#);
    assert_eq!(at(&called, "/result/isError"), false);
    for params in [r#"{"name":"echo"}"#, r#"{"name":"echo","arguments":null}"#] {
        let called = reply(
            &Echo,
            &format!(r#"{{"id":5,"method":"tools/call","params":{params}}}"#),
        )?;
        assert_eq!(at(&called, "/result/content/0/text"), "no arguments");
    }
    Ok(())
}

#[test]
fn tools_call_refuses_bad_params() -> Result<(), serde_json::Error> {
    let cases = [
        (
            r#"{"id":6,"method":"tools/call"}"#,
            "missing tool call params",
        ),
        (
            r#"{"id":6,"method":"tools/call","params":{}}"#,
            "invalid tool call params: missing field `name`",
        ),
        (
            r#"{"id":6,"method":"tools/call","params":{"name":7}}"#,
            "invalid tool call params: `name` must be a string",
        ),
        (
            r#"{"id":6,"method":"tools/call","params":{"name":"nope"}}"#,
            "unknown tool nope",
        ),
    ];
    for (text, message) in cases {
        let reply = reply(&Echo, text)?;
        assert_eq!(at(&reply, "/error/code"), INVALID_PARAMS, "{text}");
        assert_eq!(at(&reply, "/error/message"), message, "{text}");
        assert_eq!(at(&reply, "/id"), 6);
    }
    Ok(())
}

#[test]
fn other_methods_go_to_the_server() -> Result<(), serde_json::Error> {
    let hello = reply(
        &Extra,
        r#"{"id":7,"method":"extra/hello","params":{"x":1}}"#,
    )?;
    assert_eq!(at(&hello, "/result"), &object([("x", Value::from(1))]));
    let failed = reply(&Extra, r#"{"id":8,"method":"extra/fail"}"#)?;
    assert_eq!(at(&failed, "/error/code"), -32002);
    assert_eq!(at(&failed, "/error/data/uri"), "x");
    for server_reply in [
        reply(&Extra, r#"{"id":9,"method":"extra/other"}"#)?,
        reply(&Echo, r#"{"id":9,"method":"extra/hello"}"#)?,
    ] {
        assert_eq!(at(&server_reply, "/error/code"), METHOD_NOT_FOUND);
        assert!(
            at(&server_reply, "/error/message")
                .as_str()
                .is_some_and(|message| message.starts_with("unknown method extra/"))
        );
        assert_eq!(at(&server_reply, "/error").get("data"), None);
    }
    Ok(())
}

#[test]
fn a_request_with_no_method_is_invalid() -> Result<(), serde_json::Error> {
    let reply = reply(&Echo, r#"{"id":10}"#)?;
    assert_eq!(at(&reply, "/error/code"), INVALID_REQUEST);
    assert_eq!(at(&reply, "/error/message"), "missing method");
    assert_eq!(at(&reply, "/id"), 10);
    let no_id = self::reply(&Echo, r#"{"method":7}"#)?;
    assert_eq!(at(&no_id, "/id"), &Value::Null);
    Ok(())
}

#[test]
fn notifications_are_never_answered() -> Result<(), serde_json::Error> {
    for text in [
        r#"{"method":"tools/list"}"#,
        r#"{"id":11,"method":"notifications/initialized"}"#,
        r#"{"method":"notifications/cancelled"}"#,
    ] {
        assert_eq!(handle_request(&Echo, &request(text)?), None, "{text}");
    }
    Ok(())
}

#[test]
fn serve_answers_each_line_and_skips_what_is_not_a_message() -> io::Result<()> {
    let mut input = Vec::new();
    input.extend_from_slice(b"{\"id\":1,\"method\":\"ping\"}\n");
    input.extend_from_slice(b"\n   \n");
    input.extend_from_slice(b"\xff\xfe\n");
    input.extend_from_slice(b"{\"method\":\"notifications/initialized\"}\n");
    input.extend_from_slice(b"not json\n");
    input.extend_from_slice(b"{\"id\":2,\"method\":\"tools/list\"}");
    let replies = served(&Echo, &input)?;
    assert_eq!(replies.len(), 3, "{replies:?}");
    assert_eq!(
        replies.first().map(|reply| at(reply, "/id")),
        Some(&Value::from(1))
    );
    let parse = replies.get(1).map(|reply| at(reply, "/error"));
    assert_eq!(
        parse.map(|error| at(error, "/code")),
        Some(&Value::from(PARSE_ERROR))
    );
    assert!(
        parse
            .and_then(|error| at(error, "/message").as_str())
            .is_some_and(|message| message.starts_with("parse error: "))
    );
    assert_eq!(
        replies.get(2).map(|reply| at(reply, "/id")),
        Some(&Value::from(2))
    );
    Ok(())
}

#[test]
fn serve_refuses_a_message_over_the_limit_and_goes_on() -> io::Result<()> {
    let mut input = vec![b'x'; MAX_MESSAGE_BYTES + 10];
    input.extend_from_slice(b"\n{\"id\":3,\"method\":\"ping\"}\n");
    let replies = served(&Echo, &input)?;
    assert_eq!(replies.len(), 2, "{replies:?}");
    let first = replies.first().map(|reply| at(reply, "/error"));
    assert_eq!(
        first.map(|error| at(error, "/code")),
        Some(&Value::from(PARSE_ERROR))
    );
    assert_eq!(
        first.map(|error| at(error, "/message")),
        Some(&Value::from(format!(
            "parse error: message exceeds {MAX_MESSAGE_BYTES} byte limit"
        )))
    );
    assert_eq!(
        replies.get(1).map(|reply| at(reply, "/id")),
        Some(&Value::from(3))
    );
    Ok(())
}

#[test]
fn a_frame_at_the_limit_is_read_whole() -> io::Result<()> {
    let mut with_newline = vec![b'a'; MAX_MESSAGE_BYTES];
    with_newline.push(b'\n');
    let mut reader = Cursor::new(with_newline);
    assert!(
        matches!(read_frame(&mut reader)?, Frame::Line(line) if line.len() == MAX_MESSAGE_BYTES + 1)
    );
    assert!(matches!(read_frame(&mut reader)?, Frame::End));

    let mut at_end = Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES]);
    assert!(
        matches!(read_frame(&mut at_end)?, Frame::Line(line) if line.len() == MAX_MESSAGE_BYTES)
    );

    let mut over = Cursor::new(vec![b'a'; MAX_MESSAGE_BYTES + 1]);
    assert!(matches!(read_frame(&mut over)?, Frame::TooLarge));
    assert!(matches!(read_frame(&mut over)?, Frame::End));
    Ok(())
}

#[test]
fn draining_stops_just_after_the_newline() -> io::Result<()> {
    let mut reader = BufReader::with_capacity(4, Cursor::new(b"0123456789\nnext\n".to_vec()));
    drain_line(&mut reader)?;
    let mut rest = String::new();
    reader.read_to_string(&mut rest)?;
    assert_eq!(rest, "next\n");

    let mut no_newline = BufReader::with_capacity(4, Cursor::new(b"0123456789".to_vec()));
    drain_line(&mut no_newline)?;
    assert_eq!(no_newline.fill_buf()?, b"");
    Ok(())
}

/// A writer that refuses every write, or every flush, as a closed pipe does.
struct Closed {
    on_flush: bool,
}

impl Write for Closed {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.on_flush {
            Ok(buf.len())
        } else {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.on_flush {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        } else {
            Ok(())
        }
    }
}

/// A reader whose every read fails.
struct Broken;

impl Read for Broken {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("broken"))
    }
}

#[test]
fn a_failed_read_or_write_ends_serving_with_its_error() {
    let ping = b"{\"id\":1,\"method\":\"ping\"}\n";
    for on_flush in [false, true] {
        let error = serve(Cursor::new(ping), Closed { on_flush }, &Echo).err();
        assert_eq!(
            error.map(|error| error.kind()),
            Some(io::ErrorKind::BrokenPipe)
        );
    }
    let error = serve(BufReader::new(Broken), Vec::new(), &Echo).err();
    assert_eq!(
        error.map(|error| error.to_string()),
        Some("broken".to_owned())
    );

    let mut over = vec![b'x'; MAX_MESSAGE_BYTES + 1];
    over.extend_from_slice(b"rest");
    let chained = Cursor::new(over).chain(Broken);
    let error = serve(BufReader::new(chained), Vec::new(), &Echo).err();
    assert_eq!(
        error.map(|error| error.to_string()),
        Some("broken".to_owned())
    );
}

#[test]
fn replies_carry_their_parts() {
    let error = RpcError::new(-5, "bad");
    assert_eq!(error.data, None);
    let reply = error_response(Value::from(1), &error);
    assert_eq!(at(&reply, "/error/message"), "bad");
    assert_eq!(at(&reply, "/jsonrpc"), "2.0");
    let failed = tool_error("no such project");
    assert_eq!(at(&failed, "/isError"), true);
    assert_eq!(
        at(&failed, "/content/0/text"),
        r#"{"error":{"message":"no such project"}}"#
    );
    assert_eq!(at(&failed, "/content/0/type"), "text");
}

#[test]
fn the_limit_and_the_codes_are_json_rpc_s() {
    assert_eq!(MAX_MESSAGE_BYTES, 16_777_216);
    assert_eq!(
        [
            PARSE_ERROR,
            INVALID_REQUEST,
            METHOD_NOT_FOUND,
            INVALID_PARAMS,
            super::INTERNAL_ERROR
        ],
        [-32700, -32600, -32601, -32602, -32603]
    );
}
