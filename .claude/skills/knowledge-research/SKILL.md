---
name: knowledge-research
description: Gather source material into raw/research/ through a guided intake flow. Use when Claude Code is asked what to research, wants web or site research, provides local files or URLs to collect, or wants to prepare a research bundle for later ingest.
---

# /knowledge-research

## Purpose

Gather source material into `raw/research/` so it can be ingested later. This
skill is the framework's guided intake surface for pre-ingest research. It
does research and source acquisition; it does not update `wiki/` directly.

## Behavior

1. Require `wiki/index.md` and `raw/` in the current working directory. If
   they are missing, tell the user to initialize the project first.
2. If the request is broad or the source mode is unclear, ask one short intake
   exchange to determine the research question, goal, source constraints, and
   whether to shortlist or auto-select sources.
3. Before gathering sources, read `wiki/index.md`,
   `project_guidelines.md` if needed, and the one to three most relevant wiki
   pages when the request is topical enough to identify them.
4. Create or extend one bundle at `raw/research/YYYY-MM-DD-topic-slug/` with
   `manifest.md`, `research-summary.md`, and numbered files under `sources/`.
5. Path mode: copy the provided file or directory into `sources/`, record the
   original path and acquisition method in `manifest.md`, and include it in
   `research-summary.md`.
6. URL mode: fetch each explicit URL, save it as markdown under `sources/`,
   prefix source metadata, record it in `manifest.md`, and summarize it.
7. Site mode: search only within the specified domain, shortlist relevant
   pages, ask which to save unless delegated, then save approved pages with
   domain/query metadata.
8. Web mode: search broadly, prefer primary sources when source quality
   matters, shortlist relevant pages, ask which to save unless delegated, then
   save approved pages with query metadata.
9. `manifest.md` records the research question, goal, date, source modes,
   inventory, retrieval date when applicable, selection rationale, and gaps.
10. `research-summary.md` captures the question, scope, source set reviewed,
    key findings, disagreements, caveats, open questions, and ingest readiness.
11. Do not ingest automatically unless the user explicitly asks for ingest as a
    follow-up step.

## Invocation

Use normal language or an explicit skill invocation:

- `/knowledge-research on path ./notes`
- `/knowledge-research on url https://example.com/spec`
- `/knowledge-research site docs.example.com auth tokens`
- `/knowledge-research web vector database benchmarks`

## Notes

Report the research mode used, candidate source count when applicable, bundle
path, saved source files, manifest and summary files, notable gaps, and
whether the material is ready for ingest.
