//! Integration tests: the `poman` binary answers `--version` and `--help` and
//! refuses an unknown command.

use std::io;
use std::process::Command;

fn poman(args: &[&str]) -> io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_poman"))
        .args(args)
        .output()
}

#[test]
fn the_binary_answers_version() -> io::Result<()> {
    let output = poman(&["--version"])?;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("poman {}\n", env!("CARGO_PKG_VERSION"))
    );
    Ok(())
}

#[test]
fn the_binary_answers_help() -> io::Result<()> {
    let output = poman(&["--help"])?;
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: poman"));
    Ok(())
}

#[test]
fn the_binary_refuses_an_unknown_command() -> io::Result<()> {
    let output = poman(&["board"])?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument 'board'"));
    Ok(())
}
