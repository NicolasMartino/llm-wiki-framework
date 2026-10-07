# Knowledge Research Intake

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-06
- Category: Tooling
- Scope: Use `wiki-research` as the guided intake surface for source gathering instead of adding a separate `wiki-intake` command.
- Sources: wiki/archive/knowledge-intake-command.proposal.md, wiki/specs/wiki-research-skill.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/archive/knowledge-command-namespace.decision.md
- Related: wiki/specs/wiki-research-skill.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/archive/knowledge-command-namespace.decision.md

## Choice

Do not add a separate `wiki-intake` command.

Instead, `wiki-research` is the guided intake surface for source
gathering:

1. ask a short scoping exchange when the research task is underspecified
2. gather material from local paths, explicit URLs, one site, or the web
3. save the collected source set as a coherent bundle under `raw/research/`
4. write `manifest.md` for provenance and inventory
5. write one bundle-level `research-summary.md`
6. stop before ingest unless the user explicitly asks for `wiki-ingest`

## Why

The framework's boundary is:

- `wiki-research` gathers or snapshots source material into `raw/`
- `wiki-ingest` compiles explicit `raw/` sources into `wiki/`

Adding `wiki-intake` would create a second pre-ingest front door with
substantial overlap. Strengthening `wiki-research` keeps the command
surface smaller while preserving the source-acquisition vs wiki-compilation
split.

## Consequences

- `$wiki research` is the namespace path for guided source intake.
- `$wiki ingest` remains an explicit separate step.
- Research bundles are the handoff artifact between acquisition and ingest.
- The earlier accepted proposal is archived because the accepted choice now
  lives in this decision and the active behavior lives in the research spec.

## Revisit When

- Users still struggle to discover `wiki-research`.
- The framework needs an orchestrator that intentionally chains research and
  ingest.
- Research bundle structure changes materially from the current
  `raw/research/YYYY-MM-DD-topic-slug/` convention.
