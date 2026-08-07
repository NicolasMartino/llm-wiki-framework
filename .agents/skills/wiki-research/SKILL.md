---
name: wiki-research
description: Gather source material into raw/research/ through a guided intake flow. Use when Codex is asked what to research, wants web or site research, provides local files or URLs to collect, or wants to prepare a research bundle for later ingest.
---

# /wiki-research

## Purpose

Gather source material into `raw/research/` so it can be ingested later. This
skill is the framework's guided intake surface for pre-ingest research. It
does research and source acquisition; it does not update `wiki/` directly.

## MCP Routing

When the host exposes the LLM Wiki MCP server, use the framework-owned MCP
tools for wiki/raw orientation: `llm_wiki_read` for full `wiki/` or `raw/`
files, `llm_wiki_search` for current-project retrieval, and
`llm_wiki_search_all` only for cross-project retrieval. If a follow-up ingest or
lint mutation changes wiki content, refresh with `llm_wiki_index`. Shell
`llm-wiki ...` commands and direct wiki/raw file reads are fallback only when
MCP is unavailable. Research acquisition itself may still use web, URL, or
local path tools because there is no deterministic `llm_wiki_research` MCP tool.

## Behavior

1. Require `wiki/index.md` and `raw/` in the current working directory. If
   they are missing, tell the user to initialize the project first.
2. If the request is broad or the source mode is unclear, ask one short intake
   exchange to determine the research question, goal, source constraints, and
   whether to shortlist or auto-select sources.
3. Before gathering sources, read `wiki/index.md` and the one to three most
   relevant wiki pages via `llm_wiki_read` when MCP is exposed. Use
   `llm_wiki_search` to identify relevant pages when the request is topical
   enough and the project is registered. Read `project_guidelines.md` from an
   MCP resource if exposed, otherwise directly.
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

- `/wiki-research on path ./notes`
- `/wiki-research on url https://example.com/spec`
- `/wiki-research site docs.example.com auth tokens`
- `/wiki-research web vector database benchmarks`

## Notes

Report the research mode used, candidate source count when applicable, bundle
path, saved source files, manifest and summary files, notable gaps, and
whether the material is ready for ingest.
