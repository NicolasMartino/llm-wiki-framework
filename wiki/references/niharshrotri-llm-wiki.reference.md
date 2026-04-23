# NiharShrotri/llm-wiki - Reference Implementation

- Document Class: Reference
- Status: Sourced
- Date: 2026-04-23
- Category: Implementation reference
- Scope: Full working implementation of the LLM Wiki pattern with QMD search, CLI tooling, and web UI.
- Sources: raw/research/niharshrotri-llm-wiki-implementation.md
- Related: wiki/references/llm-wiki-pattern.reference.md, wiki/references/qmd-search-engine.reference.md, wiki/references/llm-wiki-ecosystem.reference.md

## What It Is

A complete, working implementation of Karpathy's LLM Wiki pattern. 100%
local, using Ollama + Qwen3-14B for reasoning and QMD for search. Includes
CLI, web UI, Obsidian integration, and automated lint.

## Architecture Choices Worth Noting

### 3-Pass Ingest (Most Significant)

Not a single LLM call. Three distinct passes:

1. **Extraction** (thinking mode ON) — structured JSON output: summary,
   takeaways, entities, concepts, tags
2. **Page drafting** (thinking mode OFF, streaming) — one call per
   entity/concept, decides create vs merge, preserves prior content
3. **Source summary** — generates audit trail page listing all wiki
   pages the source touched

This separates reasoning (pass 1) from writing (pass 2) from bookkeeping
(pass 3). Each pass has different LLM configuration.

### Wiki Subdirectories

```
wiki/
  sources/      one page per ingested file (audit trail)
  entities/     people, orgs, models, services
  concepts/     thematic/topical pages
  synthesis/    answers filed back from queries
```

Different from our typed-document approach (specs, decisions, proposals).
Their model organizes by knowledge type (entity vs concept vs synthesis).
Ours organizes by truth relationship (validated vs proposed vs executed).

### Query Scopes

Three scopes: Wiki (compiled pages only), Raw (original documents),
Hybrid (both layers). This lets users choose whether to search
pre-compiled knowledge or go back to sources.

### Lint With Auto-Fix

Detects: broken wikilinks, orphan pages, malformed frontmatter, noise
(long summaries, empty sections), contradictions (LLM-powered, optional).

Most issues fixed automatically with `--fix`. Contradictions flagged
for human review.

### Web UI

Seven-page FastAPI interface: dashboard, sources, ingest (drag-and-drop),
jobs, query (chat with streaming), lint (interactive), graph (D3
force-directed visualization).

### CLI as First-Class Interface

```
wiki init [path]          scaffold
wiki add <file> [-r]      register source
wiki ingest [source_id]   3-pass pipeline
wiki query "<q>"          search + synthesize
wiki lint [--deep] [--fix]  health check
wiki reindex              rebuild search index
wiki serve                web UI
```

## Key Differences From Our Framework

| Aspect | NiharShrotri | Our framework |
| --- | --- | --- |
| Organization | By knowledge type (entity, concept, synthesis) | By truth relationship (spec, decision, proposal) |
| Schema file | schema/AGENTS.md | CLAUDE.md |
| Ingest | 3-pass automated pipeline | Agent-driven manual operation |
| Search | QMD hybrid search always on | index.md first, QMD at scale |
| Tooling | Python CLI + FastAPI UI | No tooling yet (agent-only) |
| LLM | Ollama + Qwen3-14B (local) | Any LLM (Claude, etc.) |
| Target | Personal knowledge bases | Software project management |
| Document types | Untyped (source, entity, concept, synthesis) | 9 typed roles |
| Deliverables/roadmaps | Not applicable | First-class concept |
| Validation/promotion | No truth lifecycle | Explicit promotion rule |

## What We Could Adopt

1. **QMD as search layer** — solves our scale ceiling without cloud infra
2. **3-pass ingest** — separating extraction, drafting, and bookkeeping
   could improve ingest quality
3. **Source audit pages** — explicit record of what each raw source touched
4. **Auto-reindex after mutations** — QMD rebuilds index after ingest/lint
5. **CLI tooling** — `wiki init`, `wiki ingest`, `wiki lint` as automation
6. **Query scopes** — searching compiled wiki vs raw sources vs both
7. **Synthesis pages** — a formal document type for query answers filed back

## What We Should Not Adopt

1. **Untyped wiki pages** — our typed documents (spec, decision, proposal)
   provide structural guarantees about truth relationship that entity/concept
   categories do not
2. **Local-only LLM** — our framework should be LLM-agnostic
3. **Web UI** — premature for the framework definition stage
