pub struct SkillAsset {
    pub name: &'static str,
    pub skill_md: &'static str,
    pub codex_openai: &'static str,
}

pub const SKILLS: &[SkillAsset] = &[
    SkillAsset {
        name: "knowledge-init",
        skill_md: include_str!("../assets/skills/knowledge-init/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge-init/codex/openai.yaml"),
    },
    SkillAsset {
        name: "knowledge-query",
        skill_md: include_str!("../assets/skills/knowledge-query/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge-query/codex/openai.yaml"),
    },
    SkillAsset {
        name: "knowledge-ingest",
        skill_md: include_str!("../assets/skills/knowledge-ingest/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge-ingest/codex/openai.yaml"),
    },
    SkillAsset {
        name: "knowledge-research",
        skill_md: include_str!("../assets/skills/knowledge-research/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge-research/codex/openai.yaml"),
    },
    SkillAsset {
        name: "knowledge-lint",
        skill_md: include_str!("../assets/skills/knowledge-lint/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge-lint/codex/openai.yaml"),
    },
    SkillAsset {
        name: "knowledge",
        skill_md: include_str!("../assets/skills/knowledge/SKILL.md"),
        codex_openai: include_str!("../assets/skills/knowledge/codex/openai.yaml"),
    },
];

pub const PROJECT_GUIDELINES_TEMPLATE: &str =
    include_str!("../assets/templates/project_guidelines.md");
pub const CLAUDE_TEMPLATE: &str = include_str!("../assets/templates/CLAUDE.md");
