use std::fs;

use assert_cmd::Command;
use proptest::prelude::*;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn repeated_install_has_stable_file_set(runs in 1usize..5) {
        let home = TempDir::new().expect("home");
        for _ in 0..runs {
            Command::cargo_bin("llm-wiki")
                .expect("binary")
                .env("HOME", home.path())
                .arg("install")
                .assert()
                .success();
        }
        let manifest = fs::read_to_string(home.path().join(".llm_wiki/manifest.json"))
            .expect("manifest");
        prop_assert!(manifest.contains("\"skills\""));
        prop_assert!(home.path().join(".claude/skills/knowledge-init/SKILL.md").exists());
    }
}
