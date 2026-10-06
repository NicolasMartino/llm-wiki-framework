use std::io::{self, Write};

use super::{OUTPUT_FAILED, SUCCESS, run};

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
    assert!(err.contains("unexpected argument 'board'"), "{err}");
}

#[test]
fn output_that_cannot_be_written_fails_the_run() {
    let mut err = Vec::new();
    assert_eq!(run(["poman", "--version"], &mut ClosedPipe, &mut err), OUTPUT_FAILED);
    assert_eq!(run(["poman"], &mut ClosedPipe, &mut err), OUTPUT_FAILED);
    let mut out = Vec::new();
    assert_eq!(run(["poman", "board"], &mut out, &mut ClosedPipe), OUTPUT_FAILED);
}
