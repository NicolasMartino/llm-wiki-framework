use std::io::{self, Write};

use std::path::Path;

use super::new::flag;
use super::{CANNOT_WORK, FIELD_FLAGS, OUTPUT_FAILED, REFUSED, SUCCESS, run, run_in};
use llm_wiki_core::types::poman::DEADLINE;

/// A writer that refuses every write, as a closed pipe does.
struct ClosedPipe;

impl Write for ClosedPipe {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn run_with(args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(args.iter().copied(), &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn version_names_the_workspace_version() {
    let (code, out, err) = run_with(&["poman", "--version"]);
    assert_eq!(code, SUCCESS);
    assert_eq!(out, format!("poman {}\n", env!("CARGO_PKG_VERSION")));
    assert_eq!(err, "");
}

#[test]
fn help_goes_to_standard_output() {
    let (code, out, err) = run_with(&["poman", "--help"]);
    assert_eq!(code, SUCCESS);
    assert!(out.contains("Usage: poman"), "{out}");
    assert_eq!(err, "");
}

#[test]
fn no_arguments_prints_the_help() {
    let (code, out, err) = run_with(&["poman"]);
    assert_eq!(code, SUCCESS);
    assert!(out.contains("Usage: poman"), "{out}");
    assert_eq!(err, "");
}

#[test]
fn an_unknown_command_is_refused_on_standard_error() {
    let (code, out, err) = run_with(&["poman", "board"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.contains("unrecognized subcommand 'board'"), "{err}");
}

#[test]
fn output_that_cannot_be_written_fails_the_run() {
    let mut err = Vec::new();
    assert_eq!(
        run(["poman", "--version"], &mut ClosedPipe, &mut err),
        OUTPUT_FAILED
    );
    assert_eq!(run(["poman"], &mut ClosedPipe, &mut err), OUTPUT_FAILED);
    let mut out = Vec::new();
    assert_eq!(
        run(["poman", "board"], &mut out, &mut ClosedPipe),
        OUTPUT_FAILED
    );
}

#[test]
fn each_field_has_the_flag_its_key_gives() {
    let keys: Vec<&str> = DEADLINE.fields.iter().map(|field| field.key).collect();
    let flagged: Vec<&str> = FIELD_FLAGS.iter().map(|(key, _)| *key).collect();
    assert_eq!(keys, flagged);
    for (key, long) in FIELD_FLAGS {
        assert_eq!(flag(key), format!("--{long}"));
    }
}

fn run_failing(args: &[&str], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    run_in(
        args.iter().copied(),
        Path::new("/"),
        &mut io::empty(),
        false,
        out,
        err,
    )
}

#[test]
fn a_command_whose_output_cannot_be_written_fails_the_run() {
    let mut sink = Vec::new();
    for args in [
        &["poman", "check"][..],
        &["poman", "check", "--json"],
        &["poman", "new", "task"],
        &["poman", "new", "task", "--json"],
    ] {
        assert_eq!(
            run_failing(args, &mut ClosedPipe, &mut ClosedPipe),
            OUTPUT_FAILED,
            "{args:?}"
        );
    }
    let mut other = Vec::new();
    assert_eq!(
        run_failing(&["poman", "check"], &mut sink, &mut other),
        CANNOT_WORK
    );
    assert_eq!(
        run_failing(&["poman", "new", "task"], &mut ClosedPipe, &mut sink),
        REFUSED
    );
}

/// A writer that takes `writes` writes, then refuses every one after.
struct FailAfter {
    writes: usize,
}

impl Write for FailAfter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.writes == 0 {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        self.writes -= 1;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_check_whose_lines_cannot_all_be_written_fails_the_run() -> io::Result<()> {
    let repo = tempfile::TempDir::new()?;
    std::fs::create_dir(repo.path().join(".git"))?;
    for writes in [0, 1] {
        let mut out = FailAfter { writes };
        let mut err = Vec::new();
        let code = run_in(
            ["poman", "check"],
            repo.path(),
            &mut io::empty(),
            false,
            &mut out,
            &mut err,
        );
        assert_eq!(code, OUTPUT_FAILED, "{writes}");
    }
    Ok(())
}

#[test]
fn a_terminal_that_cannot_be_written_to_fails_the_run() -> io::Result<()> {
    let repo = tempfile::TempDir::new()?;
    std::fs::create_dir(repo.path().join(".git"))?;
    let mut out = Vec::new();
    let mut input = io::Cursor::new("Rent\n");
    let code = run_in(
        ["poman", "new", "deadline"],
        repo.path(),
        &mut input,
        true,
        &mut out,
        &mut ClosedPipe,
    );
    assert_eq!(code, OUTPUT_FAILED);
    let mut err = FailAfter { writes: 1 };
    let mut input = io::Cursor::new("Rent\n");
    let code = run_in(
        ["poman", "new", "deadline", "--json"],
        repo.path(),
        &mut input,
        true,
        &mut out,
        &mut err,
    );
    assert_eq!(code, OUTPUT_FAILED);
    let refused: serde_json::Value = serde_json::from_slice(&out).map_err(io::Error::other)?;
    assert_eq!(
        refused
            .pointer("/error/message")
            .and_then(serde_json::Value::as_str),
        Some("cannot ask on the terminal: broken pipe")
    );
    Ok(())
}
