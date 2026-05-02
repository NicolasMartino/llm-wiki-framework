# Source Snapshot

- Title: LLM Wiki
- URL: https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f
- Retrieved: 2026-04-23
- Method: web
- Query: llm wiki 3 phase ingest pipeline

## Summary

Karpathy's gist defines the core compile-on-ingest pattern that the later
three-phase pipeline makes concrete.

Relevant points:

- The wiki is a persistent, compounding layer between raw sources and queries.
- Ingest is where the LLM reads a new source, extracts key information, and
  integrates it into the existing wiki instead of waiting for query-time
  retrieval.
- A single ingest can update many pages, not just create one summary.
- `index.md` is the content catalog and is updated on every ingest.
- `log.md` is the chronological activity record and is appended on every ingest.
- Contradictions and cross-references are part of ingest maintenance, not an
  afterthought.

## Relevant Details

Karpathy describes ingest as a workflow where the agent reads the source,
discusses takeaways, writes a summary page, updates relevant entity and concept
pages, updates the index, and appends to the log. He also notes that humans may
prefer one-source-at-a-time ingest with supervision, but batch ingest is also
possible.

The gist does not define an explicit "three-phase pipeline", but it clearly
separates three kinds of work that later implementations formalize:

1. understanding the source
2. mutating the wiki pages
3. maintaining navigation and audit files

## Why This Source Matters

This source provides the underlying ingest responsibilities and explains why
bookkeeping belongs inside ingest. It is the conceptual basis for later
implementations that break ingest into explicit passes.
