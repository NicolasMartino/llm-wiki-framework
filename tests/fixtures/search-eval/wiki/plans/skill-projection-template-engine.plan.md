# Plan: Skill Projection Template Engine

- Document Class: Plan
- Status: Completed
- Date: 2026-05-09
- Category: Tooling, skill projection, template engine adoption
- Scope: Execute the accepted skill projection template-engine decision by migrating Claude/Codex skill markdown and Codex runtime config rendering onto Askama templates.
- Sources: wiki/proposals/skills-template-engine.proposal.md, wiki/decisions/skill-projection-template-engine.decision.md, templates/skills/, crates/llm-wiki-schema/src/projector/{claude,codex,format,types}.rs
- Related: wiki/decisions/composable-project-init.decision.md, wiki/checklists/v1-fixture-smoke.checklist.md

## Deliverable

Skill projection uses the shared compile-time template engine:

1. `templates/skills/base.md` defines the common projected skill structure.
2. `templates/skills/claude.md` and `templates/skills/codex.md` define runtime
   headings over the shared base.
3. `templates/skills/codex_runtime_config.yaml` renders typed Codex interface
   metadata.
4. The schema crate owns the Askama-backed projector contexts and no longer
   builds projected markdown through section-by-section string concatenation.

## Implementation Notes

- Added `crates/llm-wiki-schema/askama.toml` so the schema crate derives can
  use the repository-level `templates/` root.
- Preserved runtime support checks and invocation idiom rewriting in Rust.
- Replaced `CodexProjector::with_runtime_config_template` with
  `CodexProjector::with_runtime_config(CodexRuntimeConfig)`.
- Preserved existing Codex YAML quoting and final newline shape to keep
  snapshot output byte-stable.

## Verification

- `cargo test -p llm-wiki-schema`
- `cargo test --workspace`
- V1 fixture smoke in `/private/tmp/llm-wiki-v1-smoke-20260509`

## Fixture Smoke Result

The smoke copied `tests/fixtures/wikis/v1/` into a temp project, added
`raw/smoke/runtime-template-note.md`, ingested it into a new typed reference
page, queried the fixture spec and decision, and ran a lint pass.

The lint pass found and fixed one temp-project bookkeeping issue: the copied
index's `Updated` date needed to advance after ingest. No unresolved
contradictions or orphan pages remained in the temp fixture copy.
