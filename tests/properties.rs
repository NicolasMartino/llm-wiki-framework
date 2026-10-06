use std::fs;

use assert_cmd::Command;
use proptest::prelude::*;
use tempfile::TempDir;

mod support;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn repeated_install_has_stable_file_set(runs in 1usize..5) {
        let home = TempDir::new().expect("home");
        for _ in 0..runs {
            Command::new(support::llm_wiki_bin())
                .env("HOME", home.path())
                .env_remove("RUST_LOG")
                .env_remove("XDG_CACHE_HOME")
                .env_remove("XDG_DATA_HOME")
                .args(["install", "--disable-llm-search", "--skip-path-guidance"])
                .assert()
                .success();
        }
        let manifest = fs::read_to_string(home.path().join(".llm_wiki/manifest.json"))
            .expect("manifest");
        prop_assert!(manifest.contains("\"skills\""));
        prop_assert!(
            home.path()
                .join(".llm_wiki/mcp/claude-project.mcp.json")
                .exists()
        );
    }
}
