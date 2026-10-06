//! The `llm-wiki` under test, with a poman beside it.
//!
//! `llm-wiki install` refuses without a poman of its own version beside it,
//! and `target/debug/poman` is there only after a build that included poman
//! (`cargo test --test post_install` alone does not). So the tests run
//! `llm-wiki` from a folder of their own, with a stand-in poman beside it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The stand-in poman: it answers `--version` as poman does.
pub const POMAN_SCRIPT: &str = concat!(
    "#!/bin/sh\necho \"poman ",
    env!("CARGO_PKG_VERSION"),
    "\"\n"
);

/// `llm-wiki`, hard-linked (or copied) into a folder of this test binary's
/// own, with the stand-in poman beside it.
pub fn llm_wiki_bin() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("llm-wiki-with-poman")
            .join(format!(
                "{}-{}",
                env!("CARGO_CRATE_NAME"),
                std::process::id()
            ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the folder for llm-wiki and poman");
        let bin = dir.join("llm-wiki");
        let built = assert_cmd::cargo::cargo_bin("llm-wiki");
        if fs::hard_link(&built, &bin).is_err() {
            fs::copy(&built, &bin).expect("copy llm-wiki");
        }
        write_executable(&dir.join("poman"), POMAN_SCRIPT);
        bin
    })
    .clone()
}

/// Writes `contents` to `path` as an executable file.
pub fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod executable");
    }
}
