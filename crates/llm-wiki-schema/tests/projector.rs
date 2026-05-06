use llm_wiki_schema::{
    ClaudeProjector, CodexProjector, ProjectError, Projector, Runtime, TargetRuntime, parse,
};

fn canonical() -> &'static str {
    r#"---
name: knowledge-query
description: "{verb} wiki knowledge in {runtime}."
runtimes: [claude, codex]
operations: [query]
arguments:
  - name: question
    required: true
    description: Question to answer.
---
# Knowledge Query

## Purpose

Answer from the wiki.

## Behavior

1. Read `wiki/index.md`.
2. Answer with citations.

## Invocation

`<knowledge-query> what is D8?`

## Notes

Do not speculate.
"#
}

#[test]
fn claude_projection_uses_slash_idiom_and_no_runtime_config() {
    let doc = parse(canonical()).expect("canonical parses");
    let rendered = ClaudeProjector.project(&doc).expect("project");
    assert!(rendered.skill_md.contains("# /knowledge-query"));
    assert!(
        rendered
            .skill_md
            .contains("invoke wiki knowledge in Claude Code")
    );
    assert!(rendered.skill_md.contains("`/knowledge-query what is D8?`"));
    assert!(rendered.runtime_config.is_none());
}

#[test]
fn codex_projection_uses_namespace_idiom_and_runtime_config() {
    let doc = parse(canonical()).expect("canonical parses");
    let rendered = CodexProjector::with_runtime_config_template(
        "interface:\n  display_name: \"{skill_name}\"\n",
    )
    .project(&doc)
    .expect("project");
    assert!(rendered.skill_md.contains("# Knowledge Query"));
    assert!(rendered.skill_md.contains("use wiki knowledge in Codex"));
    assert!(rendered.skill_md.contains("`$knowledge-query what is D8?`"));
    assert!(rendered.skill_md.contains("- `$knowledge query`"));
    assert_eq!(
        rendered.runtime_config.as_deref(),
        Some("interface:\n  display_name: \"knowledge-query\"\n")
    );
}

#[test]
fn runtime_restrictions_are_enforced() {
    let mut doc = parse(canonical()).expect("canonical parses");
    doc.frontmatter.runtimes = vec![Runtime::Claude];
    let err = CodexProjector::new()
        .project(&doc)
        .expect_err("unsupported");
    assert_eq!(
        err,
        ProjectError::UnsupportedRuntime {
            skill: "knowledge-query".to_string(),
            runtime: TargetRuntime::Codex
        }
    );
}
