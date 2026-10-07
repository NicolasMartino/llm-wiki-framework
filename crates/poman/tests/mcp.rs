//! Integration tests: `poman mcp` lists poman's tools, and each tool gives
//! what its command gives with `--json`, writing the same file.

use std::fs;
use std::io::{self, Cursor, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use tempfile::TempDir;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn repo() -> io::Result<TempDir> {
    let repo = TempDir::new()?;
    fs::create_dir(repo.path().join(".git"))?;
    Ok(repo)
}

fn poman(dir: &Path, args: &[&str], input: &str) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut command_line = vec!["poman"];
    command_line.extend(args);
    let code = poman::run_in(
        command_line,
        dir,
        &mut Cursor::new(input),
        false,
        &mut out,
        &mut err,
    );
    (code, String::from_utf8_lossy(&out).into_owned())
}

/// The replies `poman mcp` gives to `requests`, one per line.
fn serve(dir: &Path, requests: &[Value]) -> std::result::Result<Vec<Value>, serde_json::Error> {
    let mut input = String::new();
    for request in requests {
        input.push_str(&request.to_string());
        input.push('\n');
    }
    let (code, out) = poman(dir, &["mcp"], &input);
    assert_eq!(code, poman::SUCCESS);
    out.lines().map(serde_json::from_str).collect()
}

fn call(
    dir: &Path,
    tool: &str,
    arguments: &Value,
) -> std::result::Result<Value, serde_json::Error> {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": arguments}});
    let replies = serve(dir, &[request])?;
    Ok(replies.into_iter().next().unwrap_or(Value::Null))
}

fn text(reply: &Value) -> &str {
    reply
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn is_error(reply: &Value) -> Option<bool> {
    reply.pointer("/result/isError").and_then(Value::as_bool)
}

const OK: &str = "# Fine\n\n- Status: Todo\n- Deadline: none\n- Duration: 1 day\n- Importance: low\n- Blocked by: none\n";

#[test]
fn the_server_names_itself_and_lists_both_tools() -> Result {
    let repo = repo()?;
    let replies = serve(
        repo.path(),
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        ],
    )?;
    assert_eq!(replies.len(), 2);
    let initialize = replies.first().ok_or("no reply")?;
    assert_eq!(
        initialize.pointer("/result/serverInfo/name"),
        Some(&json!("poman"))
    );
    assert_eq!(
        initialize.pointer("/result/serverInfo/version"),
        Some(&json!(env!("CARGO_PKG_VERSION")))
    );
    let instructions = initialize
        .pointer("/result/instructions")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        instructions.contains("poman_new_deadline to write a new deadline file: it writes a file"),
        "{instructions}"
    );
    let tools = replies
        .get(1)
        .and_then(|reply| reply.pointer("/result/tools"))
        .and_then(Value::as_array)
        .ok_or("no tools")?;
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    assert_eq!(names, ["poman_check", "poman_new_deadline"]);
    let new = tools.get(1).ok_or("no second tool")?;
    assert!(
        new.get("description")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("WRITES a new deadline file"))
    );
    let properties = new
        .pointer("/inputSchema/properties")
        .and_then(Value::as_object)
        .ok_or("no properties")?;
    let keys: Vec<&str> = properties.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "blocked_by",
            "deadline",
            "duration",
            "importance",
            "slug",
            "status",
            "title",
            "track",
            "who"
        ]
    );
    assert_eq!(
        new.pointer("/inputSchema/required"),
        Some(&json!(["title"]))
    );
    assert_eq!(
        properties.get("blocked_by"),
        Some(
            &json!({"type": "string", "description": "The Blocked by field, written as the deadline type takes it."})
        )
    );
    let check = tools.first().ok_or("no first tool")?;
    assert_eq!(check.pointer("/inputSchema/properties"), Some(&json!({})));
    Ok(())
}

#[test]
fn poman_check_gives_the_check_json() -> Result {
    let repo = repo()?;
    fs::create_dir_all(repo.path().join("wiki/deadlines"))?;
    fs::write(repo.path().join("wiki/deadlines/fine.deadline.md"), OK)?;
    let reply = call(repo.path(), "poman_check", &json!({}))?;
    let (code, out) = poman(repo.path(), &["check", "--json"], "");
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(text(&reply), out.trim_end());
    assert_eq!(is_error(&reply), Some(false));

    fs::write(
        repo.path().join("wiki/deadlines/broken.deadline.md"),
        "# Broken\n",
    )?;
    let reply = call(repo.path(), "poman_check", &Value::Null)?;
    let (code, out) = poman(repo.path(), &["check", "--json"], "");
    assert_eq!(code, poman::CHECK_FAILED);
    assert_eq!(text(&reply), out.trim_end());
    assert_eq!(is_error(&reply), Some(true));
    Ok(())
}

#[test]
fn poman_new_deadline_writes_what_the_command_writes() -> Result {
    let by_tool = repo()?;
    let by_command = repo()?;
    for repo in [&by_tool, &by_command] {
        fs::create_dir_all(repo.path().join("wiki/deadlines"))?;
        fs::write(repo.path().join("wiki/deadlines/a.deadline.md"), OK)?;
    }
    let reply = call(
        by_tool.path(),
        "poman_new_deadline",
        &json!({
            "title": "-Renew The Domain",
            "status": "Todo",
            "deadline": "2026-12-01",
            "duration": "2 days",
            "importance": "medium",
            "blocked_by": "wiki/deadlines/a.deadline.md",
            "track": "-Company",
            "who": "Ana",
            "slug": "renew-the-domain",
        }),
    )?;
    let (code, out) = poman(
        by_command.path(),
        &[
            "new",
            "deadline",
            "--json",
            "--status=Todo",
            "--deadline=2026-12-01",
            "--duration=2 days",
            "--importance=medium",
            "--blocked-by=wiki/deadlines/a.deadline.md",
            "--track=-Company",
            "--who=Ana",
            "--slug=renew-the-domain",
            "--",
            "-Renew The Domain",
        ],
        "",
    );
    assert_eq!(code, poman::SUCCESS, "{out}");
    assert_eq!(is_error(&reply), Some(false));
    assert_eq!(text(&reply), out.trim_end());
    let path = "wiki/deadlines/renew-the-domain.deadline.md";
    let written = fs::read(by_tool.path().join(path))?;
    assert_eq!(written, fs::read(by_command.path().join(path))?);
    assert!(String::from_utf8(written)?.starts_with("# -Renew The Domain\n"));
    assert_eq!(
        serde_json::from_str::<Value>(text(&reply))?.get("written"),
        Some(&json!(path))
    );
    Ok(())
}

#[test]
fn a_tool_refuses_as_its_command_does() -> Result {
    let repo = repo()?;
    let reply = call(
        repo.path(),
        "poman_new_deadline",
        &json!({"importance": "low"}),
    )?;
    assert_eq!(is_error(&reply), Some(true));
    let (code, out) = poman(
        repo.path(),
        &["new", "deadline", "--json", "--importance=low"],
        "",
    );
    assert_eq!(code, poman::REFUSED);
    assert_eq!(text(&reply), out.trim_end());
    assert_eq!(
        serde_json::from_str::<Value>(text(&reply))?.pointer("/error/message"),
        Some(&json!(
            "missing, and poman asks for them only on a terminal: the title, --status, --deadline, --duration, --blocked-by"
        ))
    );
    assert!(!repo.path().join("wiki").exists());
    Ok(())
}

#[test]
fn bad_arguments_are_invalid_params() -> Result {
    let repo = repo()?;
    let cases = [
        (
            "poman_check",
            json!({"path": "x"}),
            "invalid poman_check arguments: unknown argument `path`",
        ),
        (
            "poman_check",
            json!([]),
            "invalid poman_check arguments: they must be an object",
        ),
        (
            "poman_new_deadline",
            json!({"title": 7}),
            "invalid poman_new_deadline arguments: `title` must be a string",
        ),
        (
            "poman_new_deadline",
            json!({"owner": "Ana"}),
            "invalid poman_new_deadline arguments: unknown argument `owner`",
        ),
        ("poman_list", json!({}), "unknown tool poman_list"),
    ];
    for (tool, arguments, message) in cases {
        let reply = call(repo.path(), tool, &arguments)?;
        assert_eq!(reply.pointer("/error/code"), Some(&json!(-32602)), "{tool}");
        assert_eq!(
            reply.pointer("/error/message"),
            Some(&json!(message)),
            "{tool}"
        );
    }
    Ok(())
}

#[test]
fn the_binary_serves_over_stdio() -> Result {
    let repo = repo()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_poman"))
        .arg("mcp")
        .current_dir(repo.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("no stdin")?
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"poman_check\"}}\n")?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    let reply: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        text(&reply),
        r#"{"errors":0,"files_checked":0,"findings":[],"warnings":0}"#
    );
    Ok(())
}
