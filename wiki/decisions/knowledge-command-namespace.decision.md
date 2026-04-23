# Knowledge Command Namespace

- Document Class: Decision
- Status: Accepted
- Date: 2026-04-23
- Category: Tooling
- Scope: Use `$knowledge` as the primary explicit command-like namespace for LLM Wiki operations in Codex.
- Sources: .codex/skills/knowledge/SKILL.md, .codex/skills/init-project/SKILL.md, .codex/skills/knowledge-query/SKILL.md, .codex/skills/knowledge-ingest/SKILL.md, .codex/skills/knowledge-research/SKILL.md, .codex/skills/knowledge-lint/SKILL.md
- Related: wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md, wiki/decisions/project-local-codex-skills.decision.md

## Choice

Expose framework operations through a single explicit command-like namespace in
Codex:

- `$knowledge init`
- `$knowledge query`
- `$knowledge ingest`
- `$knowledge research`
- `$knowledge lint`

The namespace is an alias layer on top of normal language and existing direct
skill names. It does not depend on product-level slash command registration.

## Why

A single namespace makes the framework easier to remember and easier to teach.
Users do not need to recall four separate skill names before they can operate
the wiki, and they do not need unsupported custom slash commands.

The namespace also clarifies the boundary between acquisition and compilation:
`$knowledge research` gathers sources into `raw/`, while `$knowledge ingest`
compiles explicit raw sources into `wiki/`.

## Consequences

- `.codex/skills/knowledge/` becomes the dispatcher entry point for explicit
  skill invocation.
- Existing skill docs must stay behaviorally aligned with the namespace.
- Natural language requests remain first-class and should continue to work.
- Legacy direct commands such as `/init-project` and direct skill names remain
  acceptable aliases.

## Revisit When

- Codex adds first-class custom command registration that changes the best
  invocation surface.
- The framework gains enough operations that `$knowledge` needs nested
  namespaces or a different command shape.
