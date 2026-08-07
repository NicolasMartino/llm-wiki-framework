//! Source lint against hard-coded instance identities (decision part 5).
//!
//! Scope note: the production managed-home literal `.llm_wiki` is intentionally
//! *overloaded* — it is also the per-project marker dir (`<project>/.llm_wiki/
//! init.toml`), which is deliberately NOT namespaced (decision part 6, shared
//! project tree). A broad grep for the production literal would therefore flag
//! legitimate project-tree code. Production-identity drift is instead pinned by
//! the golden snapshot in `tests/instance_snapshot.rs` and the path assertions
//! in `src/paths.rs`.
//!
//! This lint enforces the one invariant that is unambiguous and high-value:
//! across the *runtime source under `src/`*, the *suffixed* instance identities
//! must be minted only by the derivation API in `src/instance.rs`. Any other
//! `src/` file spelling a `-test` identity as a literal is special-casing the
//! test instance instead of calling the API — exactly the regression part 5
//! exists to catch. See wiki/plans/test-instance-namespaced-binary.plan.md
//! Phase 3.
//!
//! Out of scope by construction:
//! - The cargo build script (`build.rs`, repo root) necessarily duplicates the
//!   `llm-wiki-test` stem because it runs *before* the crate and cannot call
//!   `instance::`. It lives outside `src/` and is intentionally not scanned.
//! - Rendered skill payloads (e.g. `wiki-init-test`) are derived output, not
//!   source. Their namespace correctness is pinned by the production snapshot in
//!   `tests/instance_snapshot.rs`.

use std::fs;
use std::path::{Path, PathBuf};

/// Suffixed instance identities that must never be hard-coded outside the
/// derivation module. Spelled as exact substrings so ordinary test scaffolding
/// (`#[test]`, `fn foo_test`, `is_test`) is not matched.
const FORBIDDEN: &[&str] = &[
    "llm-wiki-test",
    "llm_wiki-test",
    "llm_wiki_read_test",
    "llm_wiki_search_test",
    "llm_wiki_search_all_test",
    "llm_wiki_index_test",
    "llm_wiki_register_test",
    "llm_wiki_status_test",
];

/// The only file allowed to spell suffixed identities (it derives them).
/// Matched by exact path, not by file name, so a stray `foo/instance.rs`
/// elsewhere in the tree cannot silently inherit the exemption.
const ALLOWLIST_REL: &str = "instance.rs";

fn first_violation(line: &str) -> Option<&'static str> {
    FORBIDDEN
        .iter()
        .copied()
        .find(|needle| line.contains(needle))
}

#[test]
fn no_hardcoded_instance_suffix_outside_derivation_api() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed = src.join(ALLOWLIST_REL);
    let mut rust_files = Vec::new();
    collect_rs(&src, &mut rust_files);

    let mut violations = Vec::new();
    for file in rust_files {
        if file == allowed {
            continue;
        }
        let contents = fs::read_to_string(&file).unwrap_or_else(|err| {
            panic!("identity lint: failed to read {}: {err}", file.display())
        });
        for (idx, line) in contents.lines().enumerate() {
            if let Some(needle) = first_violation(line) {
                violations.push(format!("{}:{}: {needle}", file.display(), idx + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "hard-coded instance identity outside the derivation API \
         (src/instance.rs); mint it via the instance:: helpers instead:\n{}",
        violations.join("\n")
    );
}

/// Negative test: the detector must flag a reintroduced literal at a minting
/// site and leave benign test scaffolding alone.
#[test]
fn lint_detects_reintroduced_literal() {
    assert_eq!(
        first_violation(r#"let home = root.join(".llm_wiki-test");"#),
        Some("llm_wiki-test")
    );
    assert_eq!(
        first_violation(r#"const BIN: &str = "llm-wiki-test";"#),
        Some("llm-wiki-test")
    );
    assert_eq!(
        first_violation(r#"exclude.push("llm_wiki_search_all_test");"#),
        Some("llm_wiki_search_all_test")
    );
    assert_eq!(
        first_violation(r#"tools.push("llm_wiki_register_test");"#),
        Some("llm_wiki_register_test")
    );
    // Benign: ordinary test scaffolding must not trip the lint.
    assert_eq!(first_violation("    #[test]"), None);
    assert_eq!(first_violation("fn parse_manifest_test() {"), None);
    assert_eq!(
        first_violation("    let is_test = instance::is_test();"),
        None
    );
}

fn collect_rs(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("read src dir") {
            collect_rs(&entry.expect("dir entry").path(), out);
        }
    } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
        out.push(path.to_path_buf());
    }
}

#[test]
fn no_legacy_generated_skill_projection_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root.join("assets/skills").exists(),
        "legacy generated-skill source assets must not exist"
    );

    let mut rust_files = Vec::new();
    collect_rs(&root.join("src"), &mut rust_files);
    collect_rs(&root.join("crates/llm-wiki-schema/src"), &mut rust_files);
    collect_rs(&root.join("tests"), &mut rust_files);

    let this_file = root.join("tests/identity_lint.rs");
    let legacy_cleanup_file = root.join("src/legacy_skills.rs");
    let legacy_cleanup_test = root.join("tests/install.rs");
    let forbidden = [
        ".claude/skills",
        ".codex/skills",
        "assets/skills",
        "templates/skills",
        "agents/openai.yaml",
        "skill_render",
        "ClaudeProjector",
        "CodexProjector",
        "CodexRuntimeConfig",
        "Command::Build",
        "BuildArgs",
    ];

    let mut violations = Vec::new();
    for file in rust_files {
        if file == this_file || file == legacy_cleanup_file || file == legacy_cleanup_test {
            continue;
        }
        let contents = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("legacy lint: failed to read {}: {err}", file.display()));
        for (idx, line) in contents.lines().enumerate() {
            if let Some(needle) = forbidden.iter().find(|needle| line.contains(*needle)) {
                violations.push(format!("{}:{}: {needle}", file.display(), idx + 1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "legacy generated-skill projection references remain:\n{}",
        violations.join("\n")
    );
}
