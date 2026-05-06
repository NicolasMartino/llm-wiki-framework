# Knowledge Research Intake

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-06
- Category: Tooling
- Scope: Use `knowledge-research` as the guided intake surface for source gathering instead of adding a separate `knowledge-intake` command.
- Sources: wiki/archive/knowledge-intake-command.proposal.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md
- Related: wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md

## Choice

Do not add a separate `knowledge-intake` command.

Instead, `knowledge-research` is the guided intake surface for source
gathering:

1. ask a short scoping exchange when the research task is underspecified
2. gather material from local paths, explicit URLs, one site, or the web
3. save the collected source set as a coherent bundle under `raw/research/`
4. write `manifest.md` for provenance and inventory
5. write one bundle-level `research-summary.md`
6. stop before ingest unless the user explicitly asks for `knowledge-ingest`

## Why

The framework's boundary is:

- `knowledge-research` gathers or snapshots source material into `raw/`
- `knowledge-ingest` compiles explicit `raw/` sources into `wiki/`

Adding `knowledge-intake` would create a second pre-ingest front door with
substantial overlap. Strengthening `knowledge-research` keeps the command
surface smaller while preserving the source-acquisition vs wiki-compilation
split.

## Consequences

- `$knowledge research` is the namespace path for guided source intake.
- `$knowledge ingest` remains an explicit separate step.
- Research bundles are the handoff artifact between acquisition and ingest.
- The earlier accepted proposal is archived because the accepted choice now
  lives in this decision and the active behavior lives in the research spec.

## Revisit When

- Users still struggle to discover `knowledge-research`.
- The framework needs an orchestrator that intentionally chains research and
  ingest.
- Research bundle structure changes materially from the current
  `raw/research/YYYY-MM-DD-topic-slug/` convention.
