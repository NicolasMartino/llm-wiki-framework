//! poman, the project manager for projects kept with `llm-wiki`.
//!
//! The binary only calls [`run`], so everything it does is here, where the
//! tests reach it. Each command is also an MCP tool, served by `poman mcp`,
//! which runs the command through [`run_in`] with `--json`.

use std::ffi::OsString;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command};
use llm_wiki_core::mcp::object;
use llm_wiki_core::names::near_miss;
use llm_wiki_core::types::poman::{ALL, DEADLINE};
use serde_json::Value;

mod check;
mod deadline;
mod finding;
mod graph;
mod landing;
mod mcp;
mod new;
mod repo;
#[cfg(test)]
mod tests;

use finding::Severity;
use new::{Request, Terminal, new_deadline};

/// The exit code of a run that did what it was asked; for `poman check`, one
/// that found no error, warnings or not.
pub const SUCCESS: u8 = 0;

/// The exit code of a run that could not write its own output.
pub const OUTPUT_FAILED: u8 = 1;

/// The exit code of a usage error, as clap reports it.
pub const USAGE: u8 = 2;

/// The exit code of a `poman check` that found at least one error.
pub const CHECK_FAILED: u8 = 3;

/// The exit code of a `poman new` that refused and wrote nothing.
pub const REFUSED: u8 = 4;

/// The exit code of a run that could not work where it was run: outside a
/// Git repository, or a file it could not write.
pub const CANNOT_WORK: u8 = 5;

/// Each deadline field and the flag that gives it, in the type's order.
const FIELD_FLAGS: [(&str, &str); 7] = [
    ("Status", "status"),
    ("Deadline", "deadline"),
    ("Duration", "duration"),
    ("Importance", "importance"),
    ("Blocked by", "blocked-by"),
    ("Track", "track"),
    ("Who", "who"),
];

fn json_flag() -> Arg {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help("Print one JSON object instead of text; the exit code does not change")
}

// Built with clap's builder: its derive emits `allow` attributes that the
// strict crates' `forbid` lints refuse.
fn command() -> Command {
    let fields = FIELD_FLAGS.iter().map(|(key, long)| {
        Arg::new(*key)
            .long(*long)
            .value_name("VALUE")
            .help(format!("The {key} field"))
    });
    Command::new("poman")
        .version(env!("CARGO_PKG_VERSION"))
        .about("The project manager for projects kept with llm-wiki.")
        .subcommand(
            Command::new("check")
                .about("Check every deadline file under wiki/, and poman.toml")
                .arg(json_flag()),
        )
        .subcommand(
            Command::new("new")
                .about("Write a new file of a poman type: `poman new deadline \"<title>\"`")
                .arg(Arg::new("type").required(true).value_name("TYPE"))
                .arg(Arg::new("title").value_name("TITLE"))
                .args(fields)
                .arg(Arg::new("slug").long("slug").value_name("SLUG"))
                .arg(json_flag()),
        )
        .subcommand(Command::new("mcp").about("Serve poman's commands as MCP tools over stdio"))
}

/// Runs poman with `args` (the program name first) where the process runs,
/// writing what it says to `out` and its errors to `err`, and returns the
/// process's exit code.
///
/// ```
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let code = poman::run(["poman", "--version"], &mut out, &mut err);
/// assert_eq!(code, poman::SUCCESS);
/// assert_eq!(out, format!("poman {}\n", env!("CARGO_PKG_VERSION")).into_bytes());
/// ```
pub fn run<I, T>(args: I, out: &mut impl Write, err: &mut impl Write) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let dir = std::env::current_dir().unwrap_or_default();
    let stdin = io::stdin();
    let terminal = stdin.is_terminal();
    run_in(args, &dir, &mut stdin.lock(), terminal, out, err)
}

/// Runs poman as [`run`] does, from `dir`, reading from `input`, which is a
/// terminal when `terminal` is true: `poman new` asks there for what its
/// flags leave out, and `poman mcp` reads its messages from it.
///
/// ```
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let code = poman::run_in(["poman", "check"], std::path::Path::new("/"), &mut std::io::empty(), false, &mut out, &mut err);
/// assert_eq!(code, poman::CANNOT_WORK);
/// ```
pub fn run_in<I, T>(
    args: I,
    dir: &Path,
    input: &mut dyn BufRead,
    terminal: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    run_args(args, dir, input, terminal, out, err)
}

// Not generic, so one compiled copy serves every caller and the coverage gate
// sees each of its lines run in one place.
fn run_args(
    args: Vec<OsString>,
    dir: &Path,
    input: &mut dyn BufRead,
    terminal: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let written = match command().try_get_matches_from(args) {
        Ok(matches) => match matches.subcommand() {
            Some(("check", options)) => run_check(dir, options.get_flag("json"), out, err),
            Some(("new", options)) => run_new(dir, options, input, terminal, out, err),
            Some(("mcp", _)) => {
                llm_wiki_core::mcp::serve(input, out, &mcp::PomanServer::new(dir)).map(|()| SUCCESS)
            }
            _ => write!(out, "{}", command().render_help()).map(|()| SUCCESS),
        },
        Err(error) => {
            let code = u8::try_from(error.exit_code()).unwrap_or(OUTPUT_FAILED);
            let text = error.render();
            match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => write!(out, "{text}"),
                _ => write!(err, "{text}"),
            }
            .map(|()| code)
        }
    };
    written.unwrap_or(OUTPUT_FAILED)
}

/// A refusal, as text on standard error or as JSON on standard output.
fn refuse(
    code: u8,
    message: &str,
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    if json {
        let error = object([(
            "error",
            object([
                ("code", Value::from(code)),
                ("message", Value::from(message)),
            ]),
        )]);
        writeln!(out, "{error}")?;
    } else {
        writeln!(err, "poman: {message}")?;
    }
    Ok(code)
}

fn run_check(dir: &Path, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let Some(root) = repo::root_from(dir) else {
        return refuse(
            CANNOT_WORK,
            "not inside a Git repository; poman works in the nearest folder up that holds .git",
            json,
            out,
            err,
        );
    };
    let report = check::check(&root);
    let errors = report.count(Severity::Error);
    let warnings = report.count(Severity::Warning);
    if json {
        let findings = report
            .findings
            .iter()
            .map(finding::Finding::to_json)
            .collect();
        let result = object([
            ("files_checked", Value::from(report.files_checked)),
            ("errors", Value::from(errors)),
            ("warnings", Value::from(warnings)),
            ("findings", Value::Array(findings)),
        ]);
        writeln!(out, "{result}")?;
    } else {
        for finding in &report.findings {
            writeln!(out, "{finding}")?;
        }
        if !report.has_wiki {
            let note = format!(
                "no {}/ folder here, so no deadline file to check",
                repo::WIKI
            );
            writeln!(out, "{note}")?;
        }
        let checked = report.files_checked;
        let summary =
            format!("deadline files checked: {checked}, errors: {errors}, warnings: {warnings}");
        writeln!(out, "{summary}")?;
    }
    Ok(if errors == 0 { SUCCESS } else { CHECK_FAILED })
}

fn run_new(
    dir: &Path,
    options: &ArgMatches,
    input: &mut dyn BufRead,
    terminal: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let json = options.get_flag("json");
    let doc_type = options
        .get_one::<String>("type")
        .map(String::as_str)
        .unwrap_or_default();
    let known = ALL
        .iter()
        .filter_map(|known| known.suffix.strip_suffix(".md"));
    if Some(doc_type) != DEADLINE.suffix.strip_suffix(".md") {
        let names: Vec<String> = known.clone().map(|name| format!("`{name}`")).collect();
        let hint = near_miss(doc_type, known)
            .map(|close| format!("; did you mean `{close}`?"))
            .unwrap_or_default();
        let message = format!(
            "`{doc_type}` is not a type poman knows; it knows {}{hint}",
            names.join(", ")
        );
        return refuse(REFUSED, &message, json, out, err);
    }
    let request = Request {
        title: options.get_one::<String>("title").cloned(),
        slug: options.get_one::<String>("slug").cloned(),
        values: FIELD_FLAGS
            .iter()
            .filter_map(|(key, _)| {
                options
                    .get_one::<String>(key)
                    .map(|value| (*key, value.clone()))
            })
            .collect(),
    };
    let asking = terminal.then_some(Terminal {
        input,
        output: &mut *err,
    });
    match new_deadline(dir, request, asking) {
        Ok(written) if json => {
            let result = object([
                ("written", Value::from(written.path)),
                ("landing_branch", Value::from(written.branch)),
            ]);
            writeln!(out, "{result}").map(|()| SUCCESS)
        }
        Ok(written) => writeln!(
            out,
            "wrote {}; it lands on {}",
            written.path, written.branch
        )
        .map(|()| SUCCESS),
        Err(refusal) => refuse(refusal.code, &refusal.message, json, out, err),
    }
}
