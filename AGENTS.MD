# AGENTS.md - Project Schema

This project defines and dogfoods a software project management framework
based on the LLM Wiki pattern. The framework is the product. The wiki
manages its own development.

## Agent Role

You own `wiki/`. You write, update, cross-link, and maintain all wiki content.
Humans curate `raw/` and make judgment calls. You handle the bookkeeping.

## How To Orient

1. Read `templates/base/project_guidelines.md` for the documentation model and rules.
2. Read `wiki/index.md` for the catalog of all project knowledge.
3. Read specific wiki pages identified from the index.
4. Read `raw/` sources only when wiki content is insufficient.

Never browse the filesystem to find information. `wiki/index.md` is your
entry point.

## Operations

### Ingest

When new material appears in `raw/`:

1. Read the raw source fully.
2. Identify facts, entities, relationships, decisions.
3. Write or update wiki pages (use the correct document type).
4. Check for contradictions with existing wiki content.
5. Update `wiki/index.md`.
6. Append to `wiki/log.md`.

### Query

When answering questions:

1. Read `wiki/index.md` to find relevant pages.
2. Read those pages.
3. Synthesize an answer with citations.
4. If the answer is durable new knowledge, file it as a wiki page.

### Lint

Periodically or on request:

1. Scan for contradictions between pages.
2. Find stale statuses or outdated claims.
3. Identify orphan pages not linked from index.
4. Check for missing cross-references.
5. Fix issues directly.
6. Log all changes in `wiki/log.md`.

## Conventions

- Document types: spec, decision, proposal, roadmap, plan, experiment,
  eval, checklist, reference.
- Use the type by role, not convenience. See `templates/base/project_guidelines.md`.
- Every wiki page has a metadata block (Document Class, Status, Date,
  Category, Scope, Sources).
- Filenames: `[slug].type.md` or `[index]-[slug].type.md`.
- Archived documents go to `wiki/archive/`.
- `wiki/log.md` uses format: `## [YYYY-MM-DD] operation | subject`.

## This Project Specifically

This project is self-referential. The framework is both the product and
the management system. When working here:

- Specs in `wiki/specs/` define the framework's own rules.
- Decisions in `wiki/decisions/` record why the framework works this way.
- The roadmap in `wiki/roadmaps/` coordinates the framework's deliverables.
- Every pain point is a bug in the product.
- The ultimate proof: a fresh agent in a fresh project, using only
  these specs, can bootstrap and maintain a self-managing wiki.

## Read-Only Access Contract For `wiki/` And `raw/`

These rules keep `wiki/` and `raw/` provenance exact, and keep them safe if a
context-compression tool such as Headroom is ever run in front of the model. See
`wiki/references/headroom-context-compression.reference.md` and
`wiki/decisions/headroom-single-posture-mcp-first.decision.md` for the full
posture.

**Route wiki/raw through the MCP tools.** When the host exposes the LLM Wiki MCP
server, read and search `wiki/` and `raw/` through the framework-owned tools —
`llm_wiki_read`, `llm_wiki_search`, and `llm_wiki_search_all` — rather than
shell-family file reads/search (`cat`, `sed`, `grep`). On Claude Code the native
`Read` tool is an acceptable fallback for a single wiki/raw file; on Codex (no
native read tool) the MCP tools are the primary surface. These outputs are
produced by llm-wiki itself, so read this way they are exact at the MCP server
boundary; if an optional host/proxy later replaces content with CCR markers,
compression envelopes, or omitted payload fields, treat that as a failed
delivery check rather than exact content.

**`headroom_read` ban.** If you run Headroom, do not enable `HEADROOM_MCP_READ=on`
and do not call the `headroom_read` MCP tool against any `wiki/` or `raw/` path.
That tool routes file content through CCR with retrieval markers, which would
defeat exact provenance. Headroom is an optional, unmanaged external runtime; the
framework does not bundle or configure Headroom itself (the four-mode proxy
companion was retired 2026-06-30). It does ship one launch convenience,
`llm-wiki headroom [--headroom-bin <PATH>] [--unsafe-mcp-read] [--] <headroom args...>`,
which execs the real `headroom` binary with `HEADROOM_MCP_READ=off` and the
`llm_wiki_*` MCP tools added to `HEADROOM_EXCLUDE_TOOLS`. Exact wiki/raw
provenance still comes from routing reads/search through the `llm_wiki_*` MCP
tools; `HEADROOM_EXCLUDE_TOOLS` is best-effort only and is not a provenance
boundary. Known `headroom-ai 0.24.0` Codex/OpenAI-Responses runs did not consult
that exclude list for normal tool-output compression; inspected
`headroom-ai 0.32.0` source appears to honor Responses excludes, so live
Headroom runs must record the exact version/path and treat CCR markers,
compression envelopes, or omitted payload fields as failed delivery checks.
