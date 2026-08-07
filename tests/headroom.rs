use assert_cmd::Command;
use predicates::prelude::*;

#[cfg(unix)]
use std::collections::HashSet;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};

fn llm_wiki() -> Command {
    Command::cargo_bin("llm-wiki").expect("binary")
}

#[cfg(unix)]
#[test]
fn passthrough_forwards_args_and_sets_headroom_env() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let fake = write_fake_headroom(temp.path());

    let assert = llm_wiki()
        .env_remove("HEADROOM_MCP_READ")
        .arg("headroom")
        .arg("--headroom-bin")
        .arg(&fake)
        .args(["--", "wrap", "codex"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.lines().any(|line| line == "ARGV=wrap|codex"));
    assert_eq!(env_value(&stdout, "HEADROOM_MCP_READ"), Some("off"));
    let exclude_tools = env_value(&stdout, "HEADROOM_EXCLUDE_TOOLS").expect("exclude tools env");
    assert_exclude_tools_cover_prod_and_test(exclude_tools);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("best-effort only"));
    assert!(stderr.contains("not a provenance boundary"));
}

#[cfg(unix)]
#[test]
fn unsafe_mcp_read_removes_parent_value_from_headroom_env() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let fake = write_fake_headroom(temp.path());

    let assert = llm_wiki()
        .env("HEADROOM_MCP_READ", "on")
        .arg("headroom")
        .arg("--headroom-bin")
        .arg(&fake)
        .args(["--unsafe-mcp-read", "--", "proxy"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(
        env_value(&stdout, "HEADROOM_MCP_READ"),
        Some("<absent>"),
        "HEADROOM_MCP_READ leaked into Headroom env:\n{stdout}"
    );
    let exclude_tools = env_value(&stdout, "HEADROOM_EXCLUDE_TOOLS").expect("exclude tools env");
    assert_exclude_tools_cover_prod_and_test(exclude_tools);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("WARNING"));
    assert!(stderr.contains("will be absent"));
}

#[cfg(unix)]
#[test]
fn passthrough_accepts_separator_and_direct_forms_with_hyphenated_args() {
    let expected = "ARGV=wrap|codex|--model|x|--port|8790";

    for forwarded in [
        vec!["--", "wrap", "codex", "--model", "x", "--port", "8790"],
        vec!["wrap", "codex", "--model", "x", "--port", "8790"],
    ] {
        let temp = tempfile::TempDir::new().expect("tempdir");
        let fake = write_fake_headroom(temp.path());

        let assert = llm_wiki()
            .arg("headroom")
            .arg("--headroom-bin")
            .arg(&fake)
            .args(forwarded)
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.lines().any(|line| line == expected),
            "forwarded argv was mangled:\n{stdout}"
        );
    }
}

#[cfg(unix)]
#[test]
fn passthrough_resolves_headroom_from_path() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let path_dir = temp.path().join("bin");
    fs::create_dir(&path_dir).expect("path dir");
    write_fake_headroom(&path_dir);

    let assert = llm_wiki()
        .env("PATH", &path_dir)
        .args(["headroom", "--", "wrap", "codex"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.lines().any(|line| line == "ARGV=wrap|codex"));
}

#[test]
fn passthrough_reports_missing_headroom_binary() {
    llm_wiki()
        .env("PATH", "")
        .args(["headroom", "--", "wrap", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--headroom-bin"));
}

#[cfg(unix)]
#[test]
fn passthrough_forwards_headroom_exit_code() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let fake = write_fake_headroom(temp.path());

    llm_wiki()
        .env("FAKE_HEADROOM_EXIT", "7")
        .arg("headroom")
        .arg("--headroom-bin")
        .arg(&fake)
        .args(["--", "wrap", "codex"])
        .assert()
        .code(7);
}

#[test]
fn passthrough_rejects_empty_headroom_args() {
    llm_wiki()
        .args(["headroom"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("HEADROOM_ARGS")));
}

#[cfg(unix)]
#[test]
fn passthrough_merges_preexisting_exclude_tools() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let fake = write_fake_headroom(temp.path());

    let assert = llm_wiki()
        .env_remove("HEADROOM_MCP_READ")
        .env(
            "HEADROOM_EXCLUDE_TOOLS",
            "custom_tool_a,custom_tool_b,llm_wiki_read",
        )
        .arg("headroom")
        .arg("--headroom-bin")
        .arg(&fake)
        .args(["--", "wrap", "codex"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let exclude_tools = env_value(&stdout, "HEADROOM_EXCLUDE_TOOLS").expect("exclude tools env");
    let values: Vec<_> = exclude_tools.split(',').collect();
    let entries: HashSet<_> = values.iter().copied().collect();
    assert!(
        entries.contains("custom_tool_a"),
        "pre-existing exclusion dropped: {exclude_tools}"
    );
    assert!(
        entries.contains("custom_tool_b"),
        "pre-existing exclusion dropped: {exclude_tools}"
    );
    assert_eq!(
        values.len(),
        entries.len(),
        "duplicate entries: {exclude_tools}"
    );
    assert_exclude_tools_cover_prod_and_test(exclude_tools);
}

#[cfg(unix)]
fn write_fake_headroom(directory: &Path) -> PathBuf {
    let path = directory.join("headroom");
    fs::write(
        &path,
        r#"#!/bin/sh
printf 'ARGV='
sep=''
for arg do
  printf '%s%s' "$sep" "$arg"
  sep='|'
done
printf '\n'
if [ "${HEADROOM_MCP_READ+x}" = x ]; then
  printf 'HEADROOM_MCP_READ=%s\n' "$HEADROOM_MCP_READ"
else
  printf 'HEADROOM_MCP_READ=<absent>\n'
fi
printf 'HEADROOM_EXCLUDE_TOOLS=%s\n' "$HEADROOM_EXCLUDE_TOOLS"
exit "${FAKE_HEADROOM_EXIT:-0}"
"#,
    )
    .expect("write fake headroom");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

#[cfg(unix)]
fn env_value<'a>(stdout: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    stdout.lines().find_map(|line| line.strip_prefix(&prefix))
}

#[cfg(unix)]
fn assert_exclude_tools_cover_prod_and_test(value: &str) {
    let entries: HashSet<_> = value.split(',').collect();
    assert!(
        entries.contains("*llm_wiki*"),
        "missing broad llm-wiki glob: {value}"
    );
    let bases = [
        "llm_wiki_read",
        "llm_wiki_search",
        "llm_wiki_search_all",
        "llm_wiki_index",
        "llm_wiki_register",
        "llm_wiki_status",
    ];

    for base in bases {
        assert!(entries.contains(base), "missing production tool {base}");
        assert!(
            entries.contains(format!("mcp__llm-wiki__{base}").as_str()),
            "missing hyphenated production route for {base}"
        );
        assert!(
            entries.contains(format!("mcp__llm_wiki__{base}").as_str()),
            "missing underscored production route for {base}"
        );

        let test_tool = format!("{base}_test");
        assert!(
            entries.contains(test_tool.as_str()),
            "missing test tool {test_tool}"
        );
        assert!(
            entries.contains(format!("mcp__llm-wiki-test__{test_tool}").as_str()),
            "missing hyphenated test route for {test_tool}"
        );
        assert!(
            entries.contains(format!("mcp__llm_wiki_test__{test_tool}").as_str()),
            "missing underscored test route for {test_tool}"
        );
    }
}
