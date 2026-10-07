//! Integration tests: `poman check` on the fixtures, each broken file with
//! its exact message and line, the walk's edges, `poman.toml`, and this
//! repository's own wiki.

use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// A Git repository holding a copy of the fixture `name`.
fn fixture_repo(name: &str) -> io::Result<TempDir> {
    let repo = TempDir::new()?;
    copy_tree(&fixtures().join(name), repo.path())?;
    fs::create_dir(repo.path().join(".git"))?;
    Ok(repo)
}

fn empty_repo() -> io::Result<TempDir> {
    let repo = TempDir::new()?;
    fs::create_dir(repo.path().join(".git"))?;
    Ok(repo)
}

fn write(root: &Path, path: &str, text: impl AsRef<[u8]>) -> io::Result<()> {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn check(dir: &Path, args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut command_line = vec!["poman", "check"];
    command_line.extend(args);
    let code = poman::run_in(
        command_line,
        dir,
        &mut io::empty(),
        false,
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

const OK: &str = "# Fine\n\n- Status: Todo\n- Deadline: none\n- Duration: 1 day\n- Importance: low\n- Blocked by: none\n";

#[test]
fn the_valid_fixture_passes_with_no_finding() -> Result {
    let repo = fixture_repo("valid")?;
    let (code, out, err) = check(repo.path(), &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(out, fs::read_to_string(fixtures().join("valid.check.txt"))?);
    assert_eq!(out, "deadline files checked: 5, errors: 0, warnings: 0\n");
    assert_eq!(err, "");
    Ok(())
}

#[test]
fn a_check_from_a_folder_below_the_root_checks_the_whole_repository() -> Result {
    let repo = fixture_repo("valid")?;
    let (code, out, _) = check(&repo.path().join("wiki/deadlines"), &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(out, "deadline files checked: 5, errors: 0, warnings: 0\n");
    Ok(())
}

#[test]
fn each_broken_file_gets_its_message_and_line() -> Result {
    let repo = fixture_repo("broken")?;
    let (code, out, err) = check(repo.path(), &[]);
    assert_eq!(
        out,
        fs::read_to_string(fixtures().join("broken.check.txt"))?
    );
    assert_eq!(code, poman::CHECK_FAILED);
    assert_eq!(err, "");
    Ok(())
}

#[test]
fn json_gives_the_same_findings() -> Result {
    let repo = fixture_repo("broken")?;
    let (code, out, _) = check(repo.path(), &["--json"]);
    assert_eq!(code, poman::CHECK_FAILED);
    let report: Value = serde_json::from_str(&out)?;
    assert_eq!(report.get("files_checked"), Some(&Value::from(23)));
    assert_eq!(report.get("errors"), Some(&Value::from(42)));
    assert_eq!(report.get("warnings"), Some(&Value::from(6)));
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or("no findings")?;
    let (_, text, _) = check(repo.path(), &[]);
    let lines: Vec<String> = findings
        .iter()
        .map(|finding| {
            format!(
                "{}:{}: {}: {}",
                finding
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                finding
                    .get("line")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
                finding
                    .get("severity")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                finding
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )
        })
        .collect();
    let text_lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len() + 1, text_lines.len());
    for (line, text_line) in lines.iter().zip(text_lines) {
        assert_eq!(line, text_line);
    }
    Ok(())
}

#[test]
fn a_repository_with_no_wiki_checks_poman_toml_only() -> Result {
    let repo = empty_repo()?;
    let (code, out, _) = check(repo.path(), &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(
        out,
        "no wiki/ folder here, so no deadline file to check\ndeadline files checked: 0, errors: 0, warnings: 0\n"
    );
    write(repo.path(), "poman.toml", "branch = \"x\"\n")?;
    let (code, out, _) = check(repo.path(), &[]);
    assert_eq!(code, poman::CHECK_FAILED);
    assert!(
        out.starts_with("poman.toml:1: error: `branch` is not a key poman knows"),
        "{out}"
    );
    Ok(())
}

#[test]
fn outside_a_repository_it_refuses() -> Result {
    let folder = TempDir::new()?;
    let (code, out, err) = check(folder.path(), &[]);
    assert_eq!(code, poman::CANNOT_WORK);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "poman: not inside a Git repository; poman works in the nearest folder up that holds .git\n"
    );
    let (code, out, err) = check(folder.path(), &["--json"]);
    assert_eq!(code, poman::CANNOT_WORK);
    assert_eq!(err, "");
    let error: Value = serde_json::from_str(&out)?;
    assert_eq!(error.pointer("/error/code"), Some(&Value::from(5)));
    Ok(())
}

#[test]
fn a_git_file_marks_a_worktree_root() -> Result {
    let repo = TempDir::new()?;
    write(repo.path(), ".git", "gitdir: elsewhere\n")?;
    write(repo.path(), "wiki/deadlines/fine.deadline.md", OK)?;
    let (code, out, _) = check(repo.path(), &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(out, "deadline files checked: 1, errors: 0, warnings: 0\n");
    Ok(())
}

#[test]
fn links_are_followed_only_to_files_inside_the_repository() -> Result {
    let repo = empty_repo()?;
    let outside = TempDir::new()?;
    let root = repo.path();
    write(root, "wiki/deadlines/fine.deadline.md", OK)?;
    write(outside.path(), "far.deadline.md", OK)?;
    let deadlines = root.join("wiki/deadlines");
    symlink(
        deadlines.join("fine.deadline.md"),
        deadlines.join("linked.deadline.md"),
    )?;
    symlink(
        outside.path().join("far.deadline.md"),
        deadlines.join("outside.deadline.md"),
    )?;
    symlink(outside.path(), deadlines.join("folder.deadline.md"))?;
    symlink(root.join("nowhere"), deadlines.join("dangling.deadline.md"))?;
    symlink(&deadlines, root.join("wiki/also-deadlines"))?;
    symlink(
        outside.path().join("far.deadline.md"),
        root.join("wiki/far.md"),
    )?;
    let (code, out, _) = check(root, &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(
        out,
        "wiki/deadlines/dangling.deadline.md:1: warning: is a link to nothing; poman does not follow it, so it is not checked\n\
         wiki/deadlines/folder.deadline.md:1: warning: links to a folder; poman does not follow it, so it is not checked\n\
         wiki/deadlines/outside.deadline.md:1: warning: links outside the repository; poman does not follow it, so it is not checked\n\
         deadline files checked: 2, errors: 0, warnings: 3\n"
    );
    Ok(())
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_at_line_one() -> Result {
    let repo = empty_repo()?;
    let root = repo.path();
    write(root, "wiki/deadlines/latin.deadline.md", b"# Caf\xe9\n")?;
    write(root, "wiki/deadlines/locked.deadline.md", OK)?;
    write(root, "wiki/closed/inside.md", "# Inside\n")?;
    let name = std::ffi::OsStr::from_bytes(b"\xff.deadline.md");
    fs::write(root.join("wiki/deadlines").join(name), OK)?;
    let locked = root.join("wiki/deadlines/locked.deadline.md");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))?;
    let closed = root.join("wiki/closed");
    fs::set_permissions(&closed, fs::Permissions::from_mode(0o000))?;
    let (code, out, _) = check(root, &[]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644))?;
    fs::set_permissions(&closed, fs::Permissions::from_mode(0o755))?;
    assert_eq!(code, poman::CHECK_FAILED);
    assert_eq!(
        out,
        "wiki/closed:1: error: this folder cannot be read: Permission denied (os error 13)\n\
         wiki/deadlines/latin.deadline.md:1: error: is not UTF-8\n\
         wiki/deadlines/locked.deadline.md:1: error: cannot be read: Permission denied (os error 13)\n\
         wiki/deadlines/\u{FFFD}.deadline.md:1: error: the slug `\u{FFFD}` is not lowercase ASCII letters and digits in groups joined by single hyphens\n\
         deadline files checked: 3, errors: 4, warnings: 0\n"
    );
    Ok(())
}

#[test]
fn a_broken_poman_toml_names_its_line() -> Result {
    let cases: [(&[u8], &str); 4] = [
        (
            b"\n\nlanding-branch = \n",
            "poman.toml:3: error: is not TOML: ",
        ),
        (
            b"landing-branch = \"\xff\"\n",
            "poman.toml:1: error: is not UTF-8\n",
        ),
        (
            b"landing-branch = 7\n",
            "poman.toml:1: error: `landing-branch` must be a branch name",
        ),
        (
            b"[landing-branch]\n",
            "poman.toml:1: error: `landing-branch` must be a branch name",
        ),
    ];
    for (text, start) in cases {
        let repo = empty_repo()?;
        write(repo.path(), "poman.toml", text)?;
        let (code, out, _) = check(repo.path(), &[]);
        assert_eq!(code, poman::CHECK_FAILED, "{out}");
        assert!(out.starts_with(start), "{out}");
    }
    let repo = empty_repo()?;
    fs::create_dir(repo.path().join("poman.toml"))?;
    let (code, out, _) = check(repo.path(), &[]);
    assert_eq!(code, poman::CHECK_FAILED);
    assert!(
        out.starts_with("poman.toml:1: error: cannot be read: "),
        "{out}"
    );
    Ok(())
}

#[test]
fn a_poman_toml_without_the_key_passes() -> Result {
    let repo = empty_repo()?;
    write(repo.path(), "poman.toml", "# nothing set\n")?;
    let (code, _, _) = check(repo.path(), &[]);
    assert_eq!(code, poman::SUCCESS);
    Ok(())
}

/// This repository's own wiki, copied, so the test runs where `.git` is not
/// part of the checkout (a mutation run's copy of the tree).
#[test]
fn this_repository_has_no_deadline_file_and_no_warning() -> Result {
    let repo = empty_repo()?;
    let wiki = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../wiki");
    copy_tree(&wiki, &repo.path().join("wiki"))?;
    let (code, out, err) = check(repo.path(), &[]);
    assert_eq!(code, poman::SUCCESS);
    assert_eq!(out, "deadline files checked: 0, errors: 0, warnings: 0\n");
    assert_eq!(err, "");
    Ok(())
}
