# Knowledge Research Intake Implementation

- Document Class: Plan
- Status: Completed
- Date: 2026-05-02
- Category: Tooling
- Scope: Execute the proposed upgrade of `knowledge-research` into a guided intake workflow with `raw/research/` bundles, `manifest.md`, and `research-summary.md`.
- Sources: wiki/archive/knowledge-intake-command.proposal.md, wiki/specs/knowledge-research-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, project_guidelines.template.md
- Related: wiki/decisions/knowledge-research-intake.decision.md, wiki/specs/knowledge-research-skill.spec.md, .codex/skills/knowledge-research/SKILL.md, .codex/skills/knowledge/SKILL.md, .claude/skills/knowledge-ingest/SKILL.md

## Deliverable

`knowledge-research` works as the framework's guided intake surface for
gathering source material. A research run can start from an underspecified user
question, ask a short scoping exchange, collect sources, and save a coherent
bundle under `raw/research/` with provenance plus one bundle-level summary.

## In Scope

- Update the `knowledge-research` spec to describe guided intake behavior.
- Update the Codex `knowledge-research` skill instructions.
- Create or update the Claude `knowledge-research` skill to mirror the same
  workflow shape used for `knowledge-ingest`.
- Update the `$knowledge` dispatcher docs if wording needs to better surface
  research as the intake entry point.
- Define the bundle structure under `raw/research/`.
- Define the minimum content of `manifest.md` and `research-summary.md`.
- Exercise the workflow once and record the resulting evidence in the wiki.

## Out Of Scope

- Automatic chaining from research into `knowledge-ingest`
- New top-level `knowledge-intake` skill or namespace command
- Automated scheduling, crawling, or feed monitoring
- Full promotion of this proposal into accepted spec truth before the workflow
  is exercised

## Steps

1. Promote the proposal direction into an implementation-ready plan and align
   terminology across proposal, skill, and spec.
2. Update `wiki/specs/knowledge-research-skill.spec.md` so research is defined
   as a guided intake workflow rather than only a source-mode selector.
3. Update `.codex/skills/knowledge-research/SKILL.md` with:
   - a short intake exchange for broad requests
   - bundle creation under `raw/research/YYYY-MM-DD-topic-slug/`
   - `manifest.md` requirements
   - `research-summary.md` requirements
   - explicit stop point before ingest unless requested
4. Create `.claude/skills/knowledge-research/SKILL.md` using
   `.claude/skills/knowledge-ingest/SKILL.md` as the invocation and structure
   reference, but with research-specific behavior and the new bundle output.
5. Review `.codex/skills/knowledge/SKILL.md` and any related invocation docs
   so research is clearly the intake path on Codex while Claude keeps its own
   direct skill surface.
6. Run one end-to-end research example that creates a bundle in `raw/research/`
   and verify the output shape and provenance.
7. If the exercised workflow is sound, update the proposal status and promote
   the validated behavior into the spec as current truth.
8. Update `wiki/index.md` and `wiki/log.md` for all resulting wiki changes.

## Verification Gates

- The research spec explicitly describes:
  - intake questioning for broad requests
  - bundle output under `raw/research/`
  - `manifest.md` as provenance and inventory
  - `research-summary.md` as bundle-level synthesis
- The Codex skill instructions match the spec.
- The Claude skill exists and matches the same workflow contract.
- The dispatcher skill remains behaviorally aligned with the research skill.
- One real research run produces the expected bundle structure.
- The workflow still keeps research and ingest as separate operations.

## Evidence To Record

- Updated spec and skill files
- New Claude research skill file under `.claude/skills/knowledge-research/`
- One example research bundle under `raw/research/`
- `wiki/log.md` entries for the documentation and workflow changes
- Any follow-up proposal, spec, or decision updates needed after exercise

## Wiki Updates When Done

- `wiki/specs/knowledge-research-skill.spec.md`
- `wiki/archive/knowledge-intake-command.proposal.md`
- `wiki/index.md`
- `wiki/log.md`

Potentially:

- `wiki/decisions/knowledge-command-namespace.decision.md`
  if the namespace wording changes materially
- `wiki/specs/knowledge-ingest-skill.spec.md`
  if Claude/Codex surface conventions need cross-reference updates

## Closure Criteria

This plan closes when:

1. the `knowledge-research` skill and spec both describe the guided intake
   workflow
2. the Claude and Codex skill definitions are both present and aligned
3. one research bundle has been created successfully under `raw/research/`
4. the bundle contains `manifest.md`, `research-summary.md`, and raw sources
5. the wiki reflects the new state without contradictions
