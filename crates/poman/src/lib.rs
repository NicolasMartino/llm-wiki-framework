//! poman, the project manager for projects kept with `llm-wiki`.
//!
//! The binary only calls [`run`], so everything it does is here, where the
//! tests reach it.

use std::ffi::OsString;
use std::io::Write;

use clap::Command;
use clap::error::ErrorKind;

#[cfg(test)]
mod tests;

/// The exit code of a run that did what it was asked.
pub const SUCCESS: u8 = 0;

/// The exit code of a run that could not write its own output.
pub const OUTPUT_FAILED: u8 = 1;

// Built with clap's builder: its derive emits `allow` attributes that the
// strict crates' `forbid` lints refuse.
fn command() -> Command {
    Command::new("poman")
        .version(env!("CARGO_PKG_VERSION"))
        .about("The project manager for projects kept with llm-wiki.")
}

/// Runs poman with `args` (the program name first), writing what it says to
/// `out` and its errors to `err`, and returns the process's exit code.
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
    let written = match command().try_get_matches_from(args) {
        Ok(_) => {
            let help = command().render_help();
            write!(out, "{help}").map(|()| SUCCESS)
        }
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
