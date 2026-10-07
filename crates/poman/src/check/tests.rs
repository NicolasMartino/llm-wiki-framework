use std::collections::BTreeSet;
use std::fs;

use tempfile::TempDir;

use super::{Blocker, Read, blocker, check_references};
use crate::deadline::Blockers;

fn read(path: &str, blockers: &[&str]) -> Read {
    Read {
        path: path.to_owned(),
        blockers: Some(Blockers {
            line: 7,
            paths: blockers.iter().map(|&blocker| blocker.to_owned()).collect(),
        }),
    }
}

/// The folder holds `b.deadline.md` only; a volume that ignores letter case
/// would find `B.deadline.md` too, and the answer must not change.
#[test]
fn a_blocker_is_found_by_its_exact_name_only() -> std::io::Result<()> {
    let root = TempDir::new()?;
    fs::create_dir_all(root.path().join("wiki/deadlines"))?;
    fs::write(root.path().join("wiki/deadlines/b.deadline.md"), "# B\n")?;
    let deadlines = BTreeSet::from(["wiki/deadlines/b.deadline.md"]);
    let kind = |path| blocker(root.path(), &deadlines, path);
    assert_eq!(kind("wiki/deadlines/b.deadline.md"), Blocker::Deadline);
    assert_eq!(kind("wiki/deadlines/B.deadline.md"), Blocker::Missing);
    assert_eq!(kind("wiki/Deadlines/b.deadline.md"), Blocker::Missing);
    assert_eq!(kind("Wiki/deadlines"), Blocker::Missing);
    assert_eq!(
        kind("wiki/deadlines/../deadlines/b.deadline.md"),
        Blocker::Missing
    );
    assert_eq!(kind("wiki/deadlines"), Blocker::NotDeadline);
    assert_eq!(kind("wiki/deadlines/"), Blocker::NotDeadline);
    Ok(())
}

/// Nothing is on disk here, so a check that asked the file system would call
/// every blocker missing; the walk's names alone decide, and the loop is found.
#[test]
fn blockers_and_loops_come_from_the_files_the_walk_read() -> std::io::Result<()> {
    let root = TempDir::new()?;
    let files = [
        read(
            "wiki/deadlines/a.deadline.md",
            &["wiki/deadlines/b.deadline.md"],
        ),
        read(
            "wiki/deadlines/b.deadline.md",
            &["wiki/deadlines/a.deadline.md"],
        ),
        read(
            "wiki/deadlines/c.deadline.md",
            &["wiki/deadlines/A.deadline.md"],
        ),
        read("wiki/other/d.deadline.md", &[]),
        read(
            "wiki/deadlines/e.deadline.md",
            &["wiki/other/d.deadline.md"],
        ),
    ];
    let mut findings = Vec::new();
    check_references(root.path(), &files, &mut findings);
    let messages: Vec<(&str, &str)> = findings
        .iter()
        .map(|finding| (finding.path.as_str(), finding.message.as_str()))
        .collect();
    assert_eq!(
        messages,
        [
            (
                "wiki/deadlines/c.deadline.md",
                "`Blocked by` names `wiki/deadlines/A.deadline.md`, which does not exist"
            ),
            (
                "wiki/deadlines/e.deadline.md",
                "`Blocked by` names `wiki/other/d.deadline.md`, which does not exist"
            ),
            (
                "wiki/deadlines/a.deadline.md",
                "`Blocked by` forms a loop: wiki/deadlines/a.deadline.md:7, wiki/deadlines/b.deadline.md:7 wait on one another"
            ),
        ]
    );
    Ok(())
}
