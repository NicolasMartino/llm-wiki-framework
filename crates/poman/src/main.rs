//! The `poman` binary: everything it does is in the library's `run`.

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(poman::run(
        std::env::args_os(),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    ))
}
