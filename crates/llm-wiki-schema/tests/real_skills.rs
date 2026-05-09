use llm_wiki_schema::{
    ClaudeProjector, CodexProjector, CodexRuntimeConfig, Projector, Runtime, parse,
};

struct SkillFixture {
    name: &'static str,
    skill_md: &'static str,
    codex_config: Option<&'static str>,
}

const SKILLS: &[SkillFixture] = &[
    SkillFixture {
        name: "wiki-init",
        skill_md: include_str!("../../../assets/skills/wiki-init/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki-init/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "wiki-query",
        skill_md: include_str!("../../../assets/skills/wiki-query/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki-query/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "wiki-ingest",
        skill_md: include_str!("../../../assets/skills/wiki-ingest/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki-ingest/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "wiki-research",
        skill_md: include_str!("../../../assets/skills/wiki-research/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki-research/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "wiki-lint",
        skill_md: include_str!("../../../assets/skills/wiki-lint/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki-lint/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "wiki",
        skill_md: include_str!("../../../assets/skills/wiki/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../assets/skills/wiki/codex/openai.yaml"
        )),
    },
];

#[test]
fn real_skills_project_to_snapshots() {
    for fixture in SKILLS {
        let skill_md = fixture.skill_md.replace("{llm_wiki_binary}", "llm-wiki");
        let doc = parse(&skill_md).expect(fixture.name);
        assert_eq!(doc.frontmatter.name, fixture.name);

        if doc.frontmatter.runtimes.contains(&Runtime::Claude) {
            let rendered = ClaudeProjector.project(&doc).expect("claude projection");
            insta::assert_snapshot!(format!("{}_claude", fixture.name), rendered.skill_md);
        }

        if doc.frontmatter.runtimes.contains(&Runtime::Codex) {
            let runtime_config =
                CodexRuntimeConfig::from_yaml(fixture.codex_config.expect("codex config"))
                    .expect("codex config parses");
            let rendered = CodexProjector::with_runtime_config(runtime_config)
                .project(&doc)
                .expect("codex projection");
            let mut snapshot = rendered.skill_md;
            snapshot.push_str("\n--- agents/openai.yaml ---\n");
            snapshot.push_str(rendered.runtime_config.as_deref().expect("runtime config"));
            insta::assert_snapshot!(format!("{}_codex", fixture.name), snapshot);
        }
    }
}
