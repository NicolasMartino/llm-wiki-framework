# Research Manifest

- Question: How should `knowledge-research` be upgraded into a guided intake workflow across Codex and Claude?
- Goal: Produce aligned skill instructions and spec updates for a research-bundle workflow under `raw/research/`.
- Date: 2026-05-02
- Modes Used: path

## Source Inventory

1. `sources/01-knowledge-research-intake-proposal.md`
   - Original path: `wiki/proposals/knowledge-intake-command.proposal.md`
   - Method: path
   - Why included: captures the intended behavior and accepted non-goals for the research intake upgrade
2. `sources/02-knowledge-research-intake-plan.md`
   - Original path: `wiki/plans/knowledge-research-intake.plan.md`
   - Method: path
   - Why included: defines the implementation sequence, verification gates, and closure criteria
3. `sources/03-claude-knowledge-ingest-skill.md`
   - Original path: `.claude/skills/knowledge-ingest/SKILL.md`
   - Method: path
   - Why included: serves as the Claude-side structural reference for the new research skill

## Selection Notes

- Chose local path sources only for this proof run because the purpose was to
  exercise the bundle workflow shape, not to test web retrieval.
- The source set intentionally mixes proposal, plan, and existing skill
  surfaces so the resulting bundle can support both spec and implementation
  updates.

## Gaps

- This proof run does not test `url`, `site`, or `web` acquisition modes.
- It does not test shortlist interaction or multi-source web curation.
