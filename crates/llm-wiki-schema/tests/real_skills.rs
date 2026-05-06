use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};

struct SkillFixture {
    name: &'static str,
    skill_md: &'static str,
    codex_config: Option<&'static str>,
}

const SKILLS: &[SkillFixture] = &[
    SkillFixture {
        name: "init-project",
        skill_md: include_str!("../../../skills/init-project/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../skills/init-project/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "knowledge-query",
        skill_md: include_str!("../../../skills/knowledge-query/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../skills/knowledge-query/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "knowledge-ingest",
        skill_md: include_str!("../../../skills/knowledge-ingest/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../skills/knowledge-ingest/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "knowledge-research",
        skill_md: include_str!("../../../skills/knowledge-research/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../skills/knowledge-research/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "knowledge-lint",
        skill_md: include_str!("../../../skills/knowledge-lint/SKILL.md"),
        codex_config: Some(include_str!(
            "../../../skills/knowledge-lint/codex/openai.yaml"
        )),
    },
    SkillFixture {
        name: "knowledge",
        skill_md: include_str!("../../../skills/knowledge/SKILL.md"),
        codex_config: Some(include_str!("../../../skills/knowledge/codex/openai.yaml")),
    },
];

#[test]
fn real_skills_project_to_snapshots() {
    for fixture in SKILLS {
        let doc = parse(fixture.skill_md).expect(fixture.name);
        assert_eq!(doc.frontmatter.name, fixture.name);

        if doc.frontmatter.runtimes.contains(&Runtime::Claude) {
            let rendered = ClaudeProjector.project(&doc).expect("claude projection");
            insta::assert_snapshot!(format!("{}_claude", fixture.name), rendered.skill_md);
        }

        if doc.frontmatter.runtimes.contains(&Runtime::Codex) {
            let rendered = CodexProjector::with_runtime_config_template(
                fixture.codex_config.expect("codex config"),
            )
            .project(&doc)
            .expect("codex projection");
            let mut snapshot = rendered.skill_md;
            snapshot.push_str("\n--- agents/openai.yaml ---\n");
            snapshot.push_str(rendered.runtime_config.as_deref().expect("runtime config"));
            insta::assert_snapshot!(format!("{}_codex", fixture.name), snapshot);
        }
    }
}
