# Knowledge Command Namespace

- Document Class: Decision
- Status: Superseded
- Date: 2026-04-23
- Superseded: 2026-05-08
- Category: Tooling
- Scope: Use `$knowledge` as the primary explicit command-like namespace for LLM Wiki operations in Codex. Superseded by D11; the current namespace is `$wiki`.
- Sources: archived skill assets prior to D11 rename
- Related: wiki/proposals/project-and-skill-rename.proposal.md, wiki/plans/project-and-skill-rename.plan.md, wiki/specs/wiki-init-skill.spec.md, wiki/specs/wiki-query-skill.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/specs/wiki-research-skill.spec.md, wiki/specs/wiki-lint-skill.spec.md

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
- Existing direct skill names remain acceptable aliases alongside the
  dispatcher namespace.

## Revisit When

- Codex adds first-class custom command registration that changes the best
  invocation surface.
- The framework gains enough operations that `$knowledge` needs nested
  namespaces or a different command shape.
