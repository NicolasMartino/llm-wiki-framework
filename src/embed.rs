pub struct SkillAsset {
    pub name: &'static str,
    pub skill_md: &'static str,
    pub codex_openai: &'static str,
}

pub const SKILLS: &[SkillAsset] = &[
    SkillAsset {
        name: "wiki-init",
        skill_md: include_str!("../assets/skills/wiki-init/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki-init/codex/openai.yaml"),
    },
    SkillAsset {
        name: "wiki-query",
        skill_md: include_str!("../assets/skills/wiki-query/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki-query/codex/openai.yaml"),
    },
    SkillAsset {
        name: "wiki-ingest",
        skill_md: include_str!("../assets/skills/wiki-ingest/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki-ingest/codex/openai.yaml"),
    },
    SkillAsset {
        name: "wiki-research",
        skill_md: include_str!("../assets/skills/wiki-research/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki-research/codex/openai.yaml"),
    },
    SkillAsset {
        name: "wiki-lint",
        skill_md: include_str!("../assets/skills/wiki-lint/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki-lint/codex/openai.yaml"),
    },
    SkillAsset {
        name: "wiki",
        skill_md: include_str!("../assets/skills/wiki/SKILL.md"),
        codex_openai: include_str!("../assets/skills/wiki/codex/openai.yaml"),
    },
];
