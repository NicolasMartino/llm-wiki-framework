//! Integration tests: the `poman` binary answers `--version` and `--help`,
//! refuses an unknown command, and runs its commands where it is started.

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
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand 'board'"));
    Ok(())
}

#[test]
fn the_binary_checks_and_writes_where_it_runs() -> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::TempDir::new()?;
    std::fs::create_dir(repo.path().join(".git"))?;
    let new = Command::new(env!("CARGO_BIN_EXE_poman"))
        .args([
            "new",
            "deadline",
            "Rent",
            "--status",
            "Todo",
            "--deadline",
            "none",
        ])
        .current_dir(repo.path())
        .stdin(std::process::Stdio::null())
        .output()?;
    assert_eq!(new.status.code(), Some(4));
    assert_eq!(
        String::from_utf8_lossy(&new.stderr),
        "poman: missing, and poman asks for them only on a terminal: --duration, --importance, --blocked-by\n"
    );
    let check = Command::new(env!("CARGO_BIN_EXE_poman"))
        .arg("check")
        .current_dir(repo.path())
        .output()?;
    assert!(check.status.success());
    assert_eq!(
        String::from_utf8_lossy(&check.stdout),
        "no wiki/ folder here, so no deadline file to check\ndeadline files checked: 0, errors: 0, warnings: 0\n"
    );
    Ok(())
}

#[test]
fn no_command_prints_the_help() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = poman::run_in(
        ["poman"],
        std::path::Path::new("/"),
        &mut std::io::empty(),
        false,
        &mut out,
        &mut err,
    );
    assert_eq!(code, poman::SUCCESS);
    assert!(String::from_utf8_lossy(&out).contains("Usage: poman"));
    assert_eq!(err, Vec::<u8>::new());
}
