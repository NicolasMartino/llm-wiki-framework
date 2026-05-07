use std::fs;
use std::path::{Path, PathBuf};

const SKILLS: &[&str] = &[
    "knowledge-init",
    "knowledge-query",
    "knowledge-ingest",
    "knowledge-research",
    "knowledge-lint",
    "knowledge",
];

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));

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

    let project_guidelines = root.join("templates/base/project_guidelines.md");
    println!("cargo:rerun-if-changed={}", project_guidelines.display());
    read(&project_guidelines);

    let agents_template = root.join("templates/base/agents.md");
    println!("cargo:rerun-if-changed={}", agents_template.display());
    read(&agents_template);
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
