use std::fs;

use assert_cmd::Command;
use proptest::prelude::*;
use tempfile::TempDir;

proptest! {
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
        let manifest = fs::read_to_string(home.path().join(".local/share/llm-wiki/manifest.json"))
            .expect("manifest");
        prop_assert!(manifest.contains("\"files\""));
        prop_assert!(home.path().join(".claude/skills/init-project/SKILL.md").exists());
    }
}
