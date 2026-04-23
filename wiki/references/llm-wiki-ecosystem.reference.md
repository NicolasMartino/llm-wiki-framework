# LLM Wiki Ecosystem Survey

- Document Class: Reference
- Status: Sourced
- Date: 2026-04-23
- Category: Ecosystem analysis
- Scope: 30+ implementations of Karpathy's LLM Wiki pattern, organized by architectural innovation and delivery model.
- Sources: raw/research/llm-wiki-ecosystem-survey.md
- Related: wiki/references/llm-wiki-pattern.reference.md, wiki/references/niharshrotri-llm-wiki.reference.md, wiki/references/qmd-search-engine.reference.md

## Context

Karpathy published the LLM Wiki pattern on 2026-04-03. Within three weeks,
30+ distinct implementations appeared on GitHub. This page catalogs
architectural innovations across the ecosystem, not individual repos.

For the base pattern definition, see wiki/references/llm-wiki-pattern.reference.md.
For the NiharShrotri reference implementation, see wiki/references/niharshrotri-llm-wiki.reference.md.

## Delivery Models

The ecosystem has produced five distinct delivery models for the same
underlying pattern:

| Model | Examples | Notes |
| --- | --- | --- |
| Agent skill | SamurAIGPT (~1965 stars), Astro-Han, Ar9av, toolboxmd | Lowest friction. Runs inside Claude Code / Codex / Cursor. Most common form. |
| CLI tool | NiharShrotri, Pratiyush, hellohejinyu, hsuanguo | Standalone commands (init, ingest, query, lint). More automation potential. |
| Desktop app | nashsu (Tauri v2, Rust + React) | Only GUI desktop app. Three-column layout. Multi-provider LLM support. |
| Web app / SaaS | lucasastorian (llmwiki.app), Apify second-brain-builder | Browser-based. lucasastorian uses MCP for Claude integration. Apify is no-code. |
| Obsidian plugin | AgriciDaniel (~1480 stars), domleca, ekadetov | Native vault integration. Background re-extraction on note save. |

The agent-skill form dominates because it requires no infrastructure — the
host agent provides the LLM, filesystem access, and context. CLI and
desktop/web forms add value through automation, persistence, and
non-developer accessibility.

## Architectural Innovations

### Two-Phase Compilation (atomicmemory)

Problem: single-pass ingest is order-dependent. The first document
processed doesn't benefit from concepts extracted from later documents.

Solution: Phase 1 extracts all concepts from all sources. Phase 2
generates pages with full concept awareness. SHA-256 hash-based change
detection enables incremental rebuilds.

Relevance to us: our current ingest processes one source at a time. If we
ever batch-ingest multiple raw files, order-independence matters.

### L1/L2 Cache Architecture (MehmetGoekce)

Problem: loading the entire wiki into context every session is wasteful.
Most sessions only need a small subset.

Solution: L1 cache (~14 files) auto-loads every session — rules, gotchas,
identity, frequently-used references. L2 (~46 pages) is queried on demand.

Relevance to us: maps closely to our CLAUDE.md (L1) + wiki/ (L2) split.
We already do this implicitly. Worth making explicit as the wiki grows.

### Parallel Multi-Agent Research (nvk)

Problem: single-agent research produces narrow, confirmation-biased results.

Solution: dispatch 5-10 specialized agents (academic, technical,
contrarian, etc.) on parallel research paths. `--mode thesis` collects
evidence for and against a claim. Counters confirmation bias by design.

Relevance to us: interesting for query operations on contested topics.
Not needed for project management wikis where facts are more settled.

### Automated Feed Monitoring (kenhuangus)

Problem: wiki only grows when a human drops files into raw/.

Solution: automated monitoring of arXiv and CVE feeds as continuous ingest
sources. Wiki grows autonomously from curated feed subscriptions.

Relevance to us: a natural extension for projects that track external
dependencies, security advisories, or research domains. Not needed for
internal project management.

### Write-Back from Queries (multiple)

Problem: good query answers are ephemeral — computed, displayed, discarded.

Solution: `--save-as` or `--save` flag promotes a query answer into a
synthesis page. The wiki grows from its own use.

Relevance to us: directly applicable. When a query produces durable
knowledge, it should become a wiki page. NiharShrotri uses `synthesis/`
subdirectory; our typed-document model would use a reference or spec type.

### Multimodal Ingest (llmrix)

Problem: diagrams, charts, and screenshots in raw sources are ignored
by text-only ingest.

Solution: uses Claude's multimodal capability for semantic comprehension
of visual content during ingest — not OCR, but understanding.

Relevance to us: applicable when raw sources contain architecture diagrams,
flowcharts, or UI screenshots. Requires a multimodal-capable LLM.

### Anti-Repetition Memory (skyllwt/OmegaWiki)

Problem: agents re-attempt failed approaches because they don't remember
what didn't work.

Solution: failed experiments become negative knowledge that prevents
dead-end re-exploration. Part of a full research lifecycle (paper ingestion
through peer review response).

Relevance to us: the concept of recording what was tried and failed is
valuable for any project wiki. Could manifest as a "rejected" status on
proposals or experiment documents.

### Wiki-as-Training-Data (louiswang524)

Problem: the LLM doesn't actually "know" the wiki contents — it reads
them into context every time.

Solution: use the wiki as fine-tuning data to create a model that
genuinely knows the knowledge base without context injection.

Relevance to us: speculative but worth tracking. Would eliminate the
context-window bottleneck entirely.

### Codebase Wiki (ussumant, toolboxmd)

Problem: the LLM Wiki pattern assumes human-written documents as sources.
Codebases are also knowledge.

Solution: compile wiki from source code repos — architecture, API
contracts, decision records, deployment configs. Reduces context costs
by ~90%.

Relevance to us: directly relevant. Our framework manages software
projects. Ingesting the codebase itself (not just documents about it)
could auto-generate architecture references and API documentation.

### ReAct Agent Queries (hellohejinyu)

Problem: wiki pages may summarize too aggressively, losing detail needed
for deep queries.

Solution: when a wiki page cites a source, the ReAct agent reads the
original source for deeper detail. Multi-language output (answers in the
language of the question).

Relevance to us: a natural fallback for our query operation. If wiki
content is insufficient, the agent already knows to read raw/ sources
(per CLAUDE.md). This formalizes that fallback as a ReAct loop.

### Multi-Agent Namespacing (kenhuangus)

Problem: multiple agents or team members writing to the same wiki create
conflicts.

Solution: shared team wiki with per-agent write isolation. Each agent
writes to its own namespace; a merge process reconciles.

Relevance to us: not needed while we have a single agent, but critical
if the framework ever supports team use.

## Ecosystem Infrastructure

### Search Engines

| Approach | Implementations |
| --- | --- |
| Index.md only (no search infra) | Most agent skills |
| QMD (BM25 + vector + rerank) | NiharShrotri, rarce |
| LanceDB | nashsu |
| Three-layer (grep + BM25 + embeddings) | MauricioPerera |
| Graphthulhu (Datalog over knowledge graph) | skridlevsky (MCP server for Logseq/Obsidian) |

### Visualization

| Tool | Implementations |
| --- | --- |
| Obsidian graph view | AgriciDaniel, domleca, ekadetov, kytmanov |
| D3 force-directed | NiharShrotri (web UI) |
| Louvain community detection | nashsu (desktop app) |
| Interactive knowledge graph | ussumant |

### Curated Resources

- tjiahen/awesome-llm-wiki — curated list of tools, schemas, implementations
- ScrapingArt/Karpathy-LLM-Wiki-Stack — build-ready blueprint document
- redmizt gist — 18 architectural extensions for multi-agent production

## Scale Data Points

- Astro-Han: 94 articles, 99 sources, maintained daily (production use)
- MehmetGoekce: L1 ~14 files, L2 ~46 pages (personal knowledge base)
- Our base pattern ceiling: ~100 articles / ~400K words without search infra

## What This Means For Our Framework

Innovations worth adopting (ordered by relevance):

1. **Write-back from queries** — query answers that produce durable
   knowledge should become wiki pages. Low effort, high value.
2. **Codebase as source** — ingesting source code, not just documents,
   to auto-generate architecture and API references.
3. **L1/L2 cache formalization** — make the CLAUDE.md / wiki split
   explicit as a caching strategy as the wiki grows.
4. **Two-phase compilation** — for batch ingests of multiple raw files.
5. **Anti-repetition memory** — recording failed approaches as negative
   knowledge (maps to rejected proposals / failed experiments).

Innovations to watch but not adopt yet:

- Parallel multi-agent research (overkill for project management)
- Automated feed monitoring (requires external feed sources)
- Multimodal ingest (requires multimodal-capable LLM at ingest time)
- Wiki-as-training-data (speculative, no proven workflow)
- Multi-agent namespacing (single-agent for now)
