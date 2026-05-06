# qmd-rs Rust Search Crate

- Document Class: Reference
- Status: Sourced
- Date: 2026-05-06
- Category: Search infrastructure
- Scope: Rust `qmd` crate as a candidate library backend for future `llm-wiki` search commands.
- Sources: https://docs.rs/qmd/latest/qmd/ (qmd 0.3.2 docs)
- Related: wiki/references/qmd-search-engine.reference.md, wiki/proposals/project-registry-search-artifacts.proposal.md

## What It Is

`qmd` 0.3.2 is a Rust crate described as "Query Markdown Documents." It is a
local search engine for markdown files with full-text search, vector semantic
search, and LLM-powered features.

This page calls the crate **qmd-rs** to distinguish it from the Tobi Lutke
Node/Bun QMD implementation documented in
`wiki/references/qmd-search-engine.reference.md`.

## Documented Capabilities

The docs.rs page describes these capabilities:

1. Full-text search with BM25 ranking through SQLite FTS5.
2. Vector semantic search using local GGUF embedding models.
3. Hybrid search with query expansion and reciprocal rank fusion.
4. Reranking with cross-encoder models.
5. Collection management for document sets.
6. Automatic model download from Hugging Face.

The public API exports a `Store`, search result types, indexing status types,
embedding/reranking/generation engines, chunking helpers, query expansion,
snippet extraction, reciprocal rank fusion, hybrid search helpers, and model
download helpers.

## Why It Matters

D8 makes `llm-wiki` a Rust binary. A Rust search crate could let future
`llm-wiki` versions provide local markdown search without requiring users to
install Node/Bun QMD separately.

The crate appears to expose enough primitives for a future internal adapter:
project-local stores, full-text and vector retrieval, hybrid fusion, reranking,
collection metadata, and model-cache management.

## Difference From Tobi QMD

The existing QMD reference page documents Tobi Lutke's Node/Bun package,
installed as `@tobilu/qmd`. That implementation is the currently documented
scale solution for large wikis and includes an MCP server surface.

qmd-rs is a separate artifact. Feature parity with Tobi QMD is unconfirmed.
Any proposal that replaces or wraps QMD with qmd-rs must evaluate:

1. Retrieval quality on representative wiki queries.
2. Whether query expansion, reranking, and fusion behavior match the current
   QMD expectations closely enough for agent navigation.
3. Whether qmd-rs exposes or can support the command/MCP surfaces the framework
   needs.
4. Model download, cache, license, and release-packaging implications.

## Open Questions

- Is the crate API stable enough to depend on from `llm-wiki`?
- Does qmd-rs have CLI/MCP parity with Tobi QMD, or only library-level parity?
- What models does qmd-rs download by default, and are they acceptable for
  framework distribution?
- What is the practical index size, cold-start time, and query latency for a
  real LLM Wiki project?
