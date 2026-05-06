# Research Summary

- Question: How should `knowledge-research` be upgraded into a guided intake workflow across Codex and Claude?
- Scope: Framework-local implementation details for the research skill, not a broader product redesign.
- Constraints: Keep research and ingest separate. Do not introduce a new top-level intake command.

## Sources Reviewed

- `sources/01-knowledge-research-intake-proposal.md`
- `sources/02-knowledge-research-intake-plan.md`
- `sources/03-claude-knowledge-ingest-skill.md`

## Key Findings

1. `knowledge-research` should be the intake surface, rather than adding a
   separate `knowledge-intake` command.
2. The research output should be one bundle under `raw/research/` containing
   raw source copies or snapshots, `manifest.md`, and one
   `research-summary.md`.
3. Claude needs its own `knowledge-research` skill file because that surface
   was missing, while Codex already had both the direct skill and the
   `$knowledge research` dispatcher path.
4. The Claude `knowledge-ingest` skill is the correct structural reference for
   usage and step formatting on the Claude side.

## Caveats

- This proof bundle only validates the `path` acquisition mode.
- The workflow still needs broader exercise against `url`, `site`, and `web`
  requests to validate shortlist behavior and provenance formatting at scale.

## Ingest Readiness

This bundle is ready for ingest as a proof artifact of the research workflow.
Its main value is demonstrating the bundle shape and the cross-surface skill
alignment process.
