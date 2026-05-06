mod embed;

fn main() {
    let embedded_bytes = embed::SKILLS
        .iter()
        .map(|skill| skill.name.len() + skill.skill_md.len() + skill.codex_openai.len())
        .sum::<usize>()
        + embed::PROJECT_GUIDELINES_TEMPLATE.len()
        + embed::CLAUDE_TEMPLATE.len();
    let _ = embedded_bytes;
    println!("llm-wiki {}", env!("CARGO_PKG_VERSION"));
}
