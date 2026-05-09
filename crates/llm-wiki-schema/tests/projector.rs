use llm_wiki_schema::{
    ClaudeProjector, CodexInterfaceConfig, CodexProjector, CodexRuntimeConfig, ProjectError,
    Projector, Runtime, TargetRuntime, parse,
};

fn canonical() -> &'static str {
    r#"---
name: wiki-query
description: "{verb} wiki knowledge in {runtime}."
runtimes: [claude, codex]
operations: [query]
arguments:
  - name: question
    required: true
    description: Question to answer.
---
# Wiki Query

## Purpose

Answer from the wiki.

## Behavior

1. Read `wiki/index.md`.
2. Answer with citations.

## Invocation

`<wiki-query> what is D8?`

## Notes

Do not speculate.
"#
}

#[test]
fn claude_projection_uses_slash_idiom_and_no_runtime_config() {
    let doc = parse(canonical()).expect("canonical parses");
    let rendered = ClaudeProjector.project(&doc).expect("project");
    assert!(rendered.skill_md.contains("# /wiki-query"));
    assert!(
        rendered
            .skill_md
            .contains("invoke wiki knowledge in Claude Code")
    );
    assert!(rendered.skill_md.contains("`/wiki-query what is D8?`"));
    assert!(rendered.runtime_config.is_none());
}

#[test]
fn codex_projection_uses_namespace_idiom_and_runtime_config() {
    let doc = parse(canonical()).expect("canonical parses");
    let rendered = CodexProjector::with_runtime_config(CodexRuntimeConfig {
        interface: CodexInterfaceConfig {
            display_name: "wiki-query".to_string(),
            short_description: "Query wiki knowledge.".to_string(),
            default_prompt: "Ask the wiki.".to_string(),
        },
    })
    .project(&doc)
    .expect("project");
    assert!(rendered.skill_md.contains("# Wiki Query"));
    assert!(rendered.skill_md.contains("use wiki knowledge in Codex"));
    assert!(rendered.skill_md.contains("`$wiki-query what is D8?`"));
    assert!(rendered.skill_md.contains("- `$wiki query`"));
    assert_eq!(
        rendered.runtime_config.as_deref(),
        Some(
            "interface:\n  display_name: \"wiki-query\"\n  short_description: \"Query wiki knowledge.\"\n  default_prompt: \"Ask the wiki.\"\n"
        )
    );
}

#[test]
fn codex_projection_has_default_runtime_config() {
    let doc = parse(canonical()).expect("canonical parses");
    let rendered = CodexProjector::new().project(&doc).expect("project");
    assert_eq!(
        rendered.runtime_config.as_deref(),
        Some(
            "interface:\n  display_name: \"Wiki Query\"\n  short_description: \"use wiki knowledge in Codex.\"\n  default_prompt: \"Use $wiki-query in Codex.\"\n"
        )
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
            skill: "wiki-query".to_string(),
            runtime: TargetRuntime::Codex
        }
    );
}
