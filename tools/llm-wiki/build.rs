use std::fs;
use std::path::{Path, PathBuf};

const SKILLS: &[&str] = &[
    "init-project",
    "knowledge-query",
    "knowledge-ingest",
    "knowledge-research",
    "knowledge-lint",
    "knowledge",
];

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");

    for skill in SKILLS {
        let path = root.join("skills").join(skill).join("SKILL.md");
        println!("cargo:rerun-if-changed={}", path.display());
        let source = read(&path);
        if let Err(err) = llm_wiki_schema::parse(&source) {
            panic!("invalid canonical skill {}: {err}", path.display());
        }

        let config = root.join("skills").join(skill).join("codex/openai.yaml");
        println!("cargo:rerun-if-changed={}", config.display());
        read(&config);
    }

    let project_guidelines = root.join("project_guidelines.template.md");
    println!("cargo:rerun-if-changed={}", project_guidelines.display());
    validate_conditional_markers(&read(&project_guidelines), &project_guidelines);

    let claude_template = root.join("tools/llm-wiki/templates/CLAUDE.md");
    println!("cargo:rerun-if-changed={}", claude_template.display());
    read(&claude_template);
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn validate_conditional_markers(input: &str, path: &Path) {
    for section in ["ML_AI", "QMD"] {
        let start = format!("<!-- SECTION:{section}");
        let end = format!("<!-- END:{section} -->");
        let starts = input.matches(&start).count();
        let ends = input.matches(&end).count();
        if starts != ends {
            panic!(
                "unbalanced conditional markers in {}: {start} count {starts}, {end} count {ends}",
                path.display()
            );
        }
    }
}
