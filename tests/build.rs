use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn build_out_writes_both_runtime_trees() {
    let temp = TempDir::new().expect("tempdir");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .args(["build", "--out"])
        .arg(temp.path())
        .assert()
        .success();

    assert!(
        temp.path()
            .join(".claude/skills/wiki-init/SKILL.md")
            .exists()
    );
    assert!(
        temp.path()
            .join(".codex/skills/wiki-init/SKILL.md")
            .exists()
    );
    assert!(
        temp.path()
            .join(".codex/skills/wiki-init/agents/openai.yaml")
            .exists()
    );
    let skill =
        fs::read_to_string(temp.path().join(".claude/skills/wiki-init/SKILL.md")).expect("skill");
    assert!(skill.contains("`llm-wiki init "));
    assert!(!skill.contains("{llm_wiki_binary}"));
    assert!(!temp.path().join(".claude/skills/wiki/SKILL.md").exists());
    assert!(temp.path().join(".codex/skills/wiki/SKILL.md").exists());
}

#[test]
fn build_target_claude_skips_codex_output() {
    let temp = TempDir::new().expect("tempdir");

    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .args(["build", "--target", "claude", "--out"])
        .arg(temp.path())
        .assert()
        .success();

    assert!(
        temp.path()
            .join(".claude/skills/wiki-init/SKILL.md")
            .exists()
    );
    assert!(!temp.path().join(".codex").exists());
}

#[test]
fn invalid_target_is_rejected() {
    Command::cargo_bin("llm-wiki")
        .expect("binary")
        .args(["build", "--target", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}
