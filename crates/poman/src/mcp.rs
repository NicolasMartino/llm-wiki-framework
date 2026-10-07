//! `poman mcp`: each poman command as an MCP tool, run through [`run_in`]
//! with `--json`, never as a terminal, so a tool does what its command does
//! and gives the command's JSON as its result.

use std::io;
use std::path::{Path, PathBuf};

use llm_wiki_core::mcp::{RpcError, Server, object, tool_result};
use serde_json::{Map, Value};

use crate::{FIELD_FLAGS, SUCCESS, run_in};

/// The tool that runs `poman check`.
pub const CHECK_TOOL: &str = "poman_check";

/// The tool that runs `poman new deadline`.
pub const NEW_DEADLINE_TOOL: &str = "poman_new_deadline";

/// poman's server, running each command where the server runs.
pub struct PomanServer {
    dir: PathBuf,
}

impl PomanServer {
    /// A server whose commands run from `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    fn run(&self, args: Vec<String>) -> Value {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_in(args, &self.dir, &mut io::empty(), false, &mut out, &mut err);
        tool_result(
            String::from_utf8_lossy(&out).trim_end().to_owned(),
            code != SUCCESS,
        )
    }
}

/// A tool argument's name, from its flag: `blocked_by` for `blocked-by`.
fn argument(long: &str) -> String {
    long.replace('-', "_")
}

/// The flag a tool argument gives: `blocked-by` for `blocked_by`, `slug` for
/// `slug`.
fn flag_of(name: &str) -> String {
    FIELD_FLAGS
        .iter()
        .find(|(_, long)| argument(long) == name)
        .map_or_else(|| name.to_owned(), |(_, long)| (*long).to_owned())
}

fn string_property(description: &str) -> Value {
    object([
        ("type", Value::from("string")),
        ("description", Value::from(description)),
    ])
}

/// The arguments a call gave, refusing any the tool does not take and any
/// that is not a string.
fn strings<'call>(
    tool: &str,
    arguments: Option<&'call Value>,
    known: &[String],
) -> Result<Vec<(&'call str, &'call str)>, RpcError> {
    let given = match arguments {
        None => return Ok(Vec::new()),
        Some(Value::Object(given)) => given,
        Some(_) => {
            return Err(RpcError::invalid_params(format!(
                "invalid {tool} arguments: they must be an object"
            )));
        }
    };
    given
        .iter()
        .map(|(name, value)| {
            if !known.contains(name) {
                return Err(RpcError::invalid_params(format!(
                    "invalid {tool} arguments: unknown argument `{name}`"
                )));
            }
            value
                .as_str()
                .map(|value| (name.as_str(), value))
                .ok_or_else(|| {
                    RpcError::invalid_params(format!(
                        "invalid {tool} arguments: `{name}` must be a string"
                    ))
                })
        })
        .collect()
}

impl Server for PomanServer {
    fn name(&self) -> &'static str {
        "poman"
    }

    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn instructions(&self) -> String {
        format!(
            "poman keeps a repository's deadlines as files under wiki/deadlines/. Use {CHECK_TOOL} to check every deadline file and poman.toml, and {NEW_DEADLINE_TOOL} to write a new deadline file: it writes a file, and names it. Each tool runs its poman command with --json where this server runs and gives that JSON as its result."
        )
    }

    fn tools(&self) -> Vec<Value> {
        let mut properties = Map::new();
        properties.insert(
            "title".to_owned(),
            string_property("The deadline's title, on one line."),
        );
        for (key, long) in FIELD_FLAGS {
            properties.insert(
                argument(long),
                string_property(&format!(
                    "The {key} field, written as the deadline type takes it."
                )),
            );
        }
        properties.insert(
            "slug".to_owned(),
            string_property("The file's slug; made from the title when left out."),
        );
        vec![
            object([
                ("name", Value::from(CHECK_TOOL)),
                (
                    "description",
                    Value::from(
                        "Run `poman check --json`: check every deadline file under wiki/ and poman.toml, each finding naming its file and line. Reads only; writes nothing.",
                    ),
                ),
                (
                    "inputSchema",
                    object([
                        ("type", Value::from("object")),
                        ("properties", object([])),
                        ("additionalProperties", Value::Bool(false)),
                    ]),
                ),
            ]),
            object([
                ("name", Value::from(NEW_DEADLINE_TOOL)),
                (
                    "description",
                    Value::from(
                        "Run `poman new deadline --json`: WRITES a new deadline file, wiki/deadlines/<slug>.deadline.md, and names the file written. Status, Deadline, Duration, Importance and Blocked by are mandatory: one left out is refused, naming every one missing, and nothing is written.",
                    ),
                ),
                (
                    "inputSchema",
                    object([
                        ("type", Value::from("object")),
                        ("properties", Value::Object(properties)),
                        ("required", Value::Array(vec![Value::from("title")])),
                        ("additionalProperties", Value::Bool(false)),
                    ]),
                ),
            ]),
        ]
    }

    fn call_tool(&self, name: &str, arguments: Option<&Value>) -> Result<Value, RpcError> {
        match name {
            CHECK_TOOL => {
                strings(name, arguments, &[])?;
                Ok(self.run(vec![
                    "poman".to_owned(),
                    "check".to_owned(),
                    "--json".to_owned(),
                ]))
            }
            NEW_DEADLINE_TOOL => {
                let mut known: Vec<String> =
                    FIELD_FLAGS.iter().map(|(_, long)| argument(long)).collect();
                known.extend(["title".to_owned(), "slug".to_owned()]);
                let given = strings(name, arguments, &known)?;
                let mut args = vec![
                    "poman".to_owned(),
                    "new".to_owned(),
                    "deadline".to_owned(),
                    "--json".to_owned(),
                ];
                let mut title = None;
                for (argument, value) in given {
                    if argument == "title" {
                        title = Some(value);
                    } else {
                        args.push(format!("--{}={value}", flag_of(argument)));
                    }
                }
                if let Some(title) = title {
                    args.extend(["--".to_owned(), title.to_owned()]);
                }
                Ok(self.run(args))
            }
            _ => Err(RpcError::invalid_params(format!("unknown tool {name}"))),
        }
    }
}
