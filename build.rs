use std::fs;
use std::path::{Path, PathBuf};

const SKILLS: &[&str] = &[
    "wiki-init",
    "wiki-query",
    "wiki-ingest",
    "wiki-research",
    "wiki-lint",
    "wiki",
];

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    println!("cargo:rerun-if-changed=templates");

    for skill in SKILLS {
        let path = root.join("assets/skills").join(skill).join("SKILL.md");
        println!("cargo:rerun-if-changed={}", path.display());
        let source = read(&path);
        if let Err(err) = llm_wiki_schema::parse(&source) {
            panic!("invalid canonical skill {}: {err}", path.display());
        }

        let config = root
            .join("assets/skills")
            .join(skill)
            .join("codex/openai.yaml");
        println!("cargo:rerun-if-changed={}", config.display());
        read(&config);
    }

    validate_templates(&root.join("templates"));
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn validate_templates(path: &Path) {
    if path.is_dir() {
        for entry in fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        {
            let entry = entry.expect("template directory entry");
            validate_templates(&entry.path());
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
        println!("cargo:rerun-if-changed={}", path.display());
        read(path);
    }
}
