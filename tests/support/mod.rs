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

/// `llm-wiki`, hard-linked (or copied) into a folder of its own, with the
/// stand-in poman beside it.
///
/// The folder is named after the build of `llm-wiki` it holds, and shared by
/// every test binary of that build. Folders of earlier builds are removed,
/// because their hard links would keep those binaries on disk.
pub fn llm_wiki_bin() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let built = assert_cmd::cargo::cargo_bin("llm-wiki");
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("llm-wiki-with-poman");
        let build = build_key(&built);
        remove_other_builds(&root, &build);
        let dir = root.join(&build);
        fs::create_dir_all(&dir).expect("create the folder for llm-wiki and poman");
        let bin = dir.join("llm-wiki");
        if !bin.exists() {
            link_or_copy(&built, &bin);
        }
        let poman = dir.join("poman");
        if fs::read_to_string(&poman).ok().as_deref() != Some(POMAN_SCRIPT) {
            let staged = dir.join(format!(".poman-{}", std::process::id()));
            write_executable(&staged, POMAN_SCRIPT);
            fs::rename(&staged, &poman).expect("put the stand-in poman in place");
        }
        bin
    })
    .clone()
}

/// The build's size and modification time: a rebuild changes them, and
/// hashing a debug binary would take seconds.
fn build_key(built: &Path) -> String {
    let metadata = fs::metadata(built).expect("the built llm-wiki");
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    format!("{}-{modified}", metadata.len())
}

fn remove_other_builds(root: &Path, build: &str) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name() != build {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Hard-links `from` to `to`, or copies it where linking is not possible,
/// through a staged name so a test binary running at the same time never
/// sees a half-written file.
fn link_or_copy(from: &Path, to: &Path) {
    let staged = to.with_file_name(format!(".llm-wiki-{}", std::process::id()));
    let _ = fs::remove_file(&staged);
    if fs::hard_link(from, &staged).is_err() {
        fs::copy(from, &staged).expect("copy llm-wiki");
    }
    fs::rename(&staged, to).expect("put llm-wiki in place");
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
