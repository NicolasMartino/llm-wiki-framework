# LLM Wiki Pattern

- Document Class: Reference
- Status: Sourced
- Date: 2026-04-23
- Category: Knowledge management architecture
- Scope: Synthesis of Karpathy's LLM Wiki pattern and extensions, applied to software project management.
- Sources: raw/research/llm-wiki-pattern-research.md
- Related: wiki/references/qmd-rs-search-crate.reference.md, wiki/references/niharshrotri-llm-wiki.reference.md, wiki/references/llm-wiki-ecosystem.reference.md

## Origin

Andrej Karpathy published the LLM Wiki pattern on 2026-04-03 as a GitHub
Gist. It describes a personal knowledge system where an LLM compiles raw
sources into a structured, interlinked markdown wiki rather than using
traditional RAG.

## Core Insight

RAG re-derives knowledge on every query. LLM Wiki compiles once at ingest,
then queries against pre-organized content. Knowledge compounds instead of
being discarded.

Key quote: "The tedious part of maintaining a knowledge base is not the
reading or the thinking - it's the bookkeeping."

## Three-Layer Architecture

1. **Raw sources** - immutable curated documents (human-owned)
2. **Wiki** - LLM-generated markdown with summaries, entity pages, synthesis (agent-owned)
3. **Schema** - configuration document specifying structure and workflows

## Three Operations

1. **Ingest** - process new source, write/update pages, update index, log
2. **Query** - read index, read relevant pages, synthesize answer with citations
3. **Lint** - scan for contradictions, stale claims, orphans, missing cross-references

## Navigation

- `index.md` - content catalog, agent reads first, fits in one context window
- `log.md` - append-only chronological mutation record

## Scale Characteristics

- Works reliably to ~100 articles / ~400,000 words / ~50,000-100,000 index tokens
- Beyond that: hybrid search (BM25 + vector + graph traversal with reciprocal rank fusion)
- Queries get cheaper over time (synthesis already done at ingest)

## V2 Extensions (rohitg00)

- Confidence scoring per fact (source count, recency, contradictions)
- Supersession (new info explicitly replaces old with audit trail)
- Forgetting curves (relevance decay unless reinforced)
- Consolidation tiers: working memory -> episodic -> semantic -> procedural
- Knowledge graph: typed entities and relationships
- Event-driven hooks (auto-ingest, session compression, periodic lint)
- Self-healing lint (fixes issues, not just flags)
- Multi-agent mesh sync with shared/private scoping

## Production Lessons (Fulkerson)

Deployed with 14 MCP servers pulling live data. Key findings:

- Post-compact hooks needed to re-inject context after LLM context compression
- Learning graduation loops: daily capture -> weekly review -> permanent rules
- Skill routing: 26 workflows that read from AND write back to the wiki
- Provenance in YAML frontmatter on every page
- Structural gaps hidden by incremental building only revealed by external comparison

## Wiki vs RAG Tradeoffs

| Dimension | Wiki | RAG |
| --- | --- | --- |
| Interpretability | High (human-readable) | Low (opaque retrieval) |
| Updates | Edit markdown | Re-embed chunks |
| Debugging | Trace to page | Reverse-engineer retrieval |
| Scale | Hundreds of files | Thousands to millions |
| Infrastructure | Git + filesystem | Vector DB + embeddings |
| Query cost trend | Decreasing | Constant |

Hybrid approach viable: wiki for architecture/specs, RAG for code search.
