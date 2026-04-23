# QMD - On-Device Hybrid Search for Markdown

- Document Class: Reference
- Status: Sourced
- Date: 2026-04-23
- Category: Search infrastructure
- Scope: QMD as the search layer for wiki navigation beyond the index.md scale ceiling.
- Sources: raw/research/qmd-search-engine.md
- Related: wiki/references/llm-wiki-pattern.reference.md, wiki/specs/documentation-model.spec.md

## What It Is

QMD is a local search engine for markdown created by Tobi Lutke. It combines
BM25 full-text search, vector semantic search, and LLM re-ranking into a
single hybrid pipeline. All processing runs on-device. No cloud, no API keys.

## Why It Matters For This Framework

Our framework's index.md approach works up to ~50,000 tokens (~100-200 pages).
Beyond that, the agent cannot fit the index in context. QMD solves this
without requiring cloud infrastructure or a vector database:

1. The wiki pages remain human-readable markdown (no format changes)
2. QMD indexes them locally in SQLite
3. The agent searches via QMD when the index is insufficient
4. Results are ranked by BM25 + vector similarity + LLM re-ranking

This means the three-layer architecture holds at any scale. The agent
reads index.md for small wikis, uses QMD for large ones.

## Search Pipeline

1. **Query expansion** — LLM generates query variations
2. **Parallel retrieval** — each variant hits both FTS and vector indexes
3. **Reciprocal rank fusion** — score = sum(1/(k+rank+1)), k=60
4. **Re-ranking** — LLM evaluates top candidates (yes/no + confidence)
5. **Position-aware blending** — final scores blend retrieval + reranker

Three modes: `search` (BM25 only, fast), `vsearch` (vector only),
`query` (full hybrid, highest quality).

## Smart Chunking

~900-token chunks with 15% overlap. Break points scored by structure:
- Headings (H1=100pts, decreasing), code blocks (80pts), paragraphs (20pts)
- Code files: tree-sitter AST parsing at function/class boundaries

This means wiki pages are chunked intelligently — specs, decisions,
and plans break at their natural section boundaries, not arbitrary
token counts.

## Models (~2GB total)

- EmbeddingGemma-300M for vectors
- Qwen3-Reranker-0.6B for relevance scoring
- QMD-Query-Expansion-1.7B for query variants

All auto-downloaded and cached locally.

## MCP Integration

QMD exposes an MCP server. The agent can use it directly:

```bash
qmd mcp           # stdio transport
qmd mcp --http    # HTTP on localhost:8181
```

Exposed tools: `query`, `get`, `multi_get`, `status`.

This means the agent can search the wiki programmatically without
reading every file. The MCP server becomes part of the CLAUDE.md
schema: "for wikis with >100 pages, use qmd query before reading
wiki pages."

## TypeScript API

Also usable programmatically for tooling:

```typescript
import { createStore } from '@tobilu/qmd'
const store = await createStore({ dbPath: './index.sqlite', ... })
const results = await store.search({ query: "auth flow" })
```

## Installation

```bash
npm install -g @tobilu/qmd
```

Requires Node.js >= 22. ~2GB model download on first run.
