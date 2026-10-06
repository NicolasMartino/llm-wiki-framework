# Knowledge Research Intake

- Document Class: Proposal
- Status: Archived
- Date: 2026-05-06
- Category: Tooling
- Scope: Refine `knowledge-research` into a guided intake flow that asks what to research, gathers material from local and web sources, and saves the collected source set under `raw/research/`.
- Sources: wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/specs/documentation-model.spec.md
- Related: wiki/decisions/knowledge-research-intake.decision.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md
- Superseded By: wiki/decisions/knowledge-research-intake.decision.md

## Question

Should the framework add a separate intake command, or should
`knowledge-research` itself become the guided intake surface for gathering new
source material?

## Decision

Do not add a separate `knowledge-intake` command right now.

Instead, strengthen `knowledge-research` so it becomes the intake surface for
source gathering:

1. ask the user what they want researched when the topic is underspecified
2. clarify scope, source preferences, and output expectations in one short
   intake exchange
3. gather material from local files, explicit URLs, one site, or the web
4. save the resulting source bundle under `raw/research/`
5. create one summary file for the whole research run
6. stop there unless the user explicitly asks for `knowledge-ingest`

## Why

The current framework already has the correct internal boundary:

- `knowledge-research` gathers or snapshots source material into `raw/`
- `knowledge-ingest` compiles explicit `raw/` sources into `wiki/`

That split should remain the product truth.

Your clarified use case is not "auto-route into ingest." It is "help me define
the research task, collect the relevant material, and store it cleanly for
later processing." That is research behavior, just with a better intake
experience.

A bundle-level research summary is the better default handoff artifact. The
raw snapshots preserve provenance, while the research summary captures the
overall question, the most important findings across sources, tensions or
gaps, and what appears ready for ingest. That keeps the workflow lightweight
without losing the synthesis step.

Adding `knowledge-intake` as a second front door would mostly duplicate the
role of `knowledge-research` and make the command surface harder to teach.
Improving the existing research skill is the cleaner design.

## Non-Goals

- Do not collapse `research` and `ingest` into one implementation.
- Do not write research findings directly into `wiki/`.
- Do not make the user choose between two nearly identical pre-ingest
  commands.

## Proposed Behavior

Minimal viable behavior:

1. If the request is broad, ask a short intake question set: topic, goal,
   preferred source types, and any scope limits.
2. Create a dated research bundle under `raw/research/`.
3. Save all gathered inputs for that research run inside the bundle, including:
   copied local material, fetched URLs, shortlisted web pages, and a small
   manifest describing the question and retrieval context.
4. Write one `research-summary.md` for the whole bundle that captures:
   the research question, scope, source set reviewed, key findings, important
   disagreements or caveats, and ingest recommendations.
5. Preserve provenance for each saved source in the manifest and raw files.
6. Report the saved bundle path, summary file, and whether the material is
   ready for ingest.

Suggested bundle shape:

```text
raw/research/YYYY-MM-DD-topic-slug/
  manifest.md
  research-summary.md
  sources/
    01-source-snapshot.md
    02-source-snapshot.md
```

## Risks

- If the intake questioning is too heavy, simple research requests become slow.
- If everything is forced into one folder shape, source-specific organization
  may get messy without a manifest convention.
- If the skill saves too much low-quality material, later ingest becomes noisy.
- If the research summary is too shallow, ingest may still need to re-open
  many sources to recover context.

## Acceptance Criteria

- `knowledge-research` is explicitly documented as the guided intake surface
  for source gathering.
- Broad research requests trigger a short scoping exchange before collection.
- Research outputs are saved as coherent bundles under `raw/research/`.
- Each research bundle produces one `research-summary.md`.
- `knowledge-ingest` remains a separate explicit step.

## Revisit When

- Users still struggle to discover `knowledge-research` even after the intake
  behavior is improved.
- The framework later needs a true ingestion orchestrator that chains research
  and ingest intentionally.
- We settle on a different raw folder convention than `raw/research/`.
