# Source Snapshot

- Title: NiharShrotri/llm-wiki README
- URL: https://github.com/NiharShrotri/llm-wiki
- Retrieved: 2026-04-23
- Method: web
- Query: llm wiki 3 phase ingest pipeline

## Summary

This README is the clearest implementation-level description of a three-pass
LLM Wiki ingest pipeline that I found in primary sources.

Relevant points:

- Each source goes through three LLM passes: extraction, page drafting, and
  source summary.
- Extraction runs with "thinking mode" on and produces structured output such
  as summary, takeaways, entities, concepts, and tags.
- Page drafting runs with thinking mode off and streams one call per
  entity/concept to decide create vs merge.
- Source summary writes a `sources/<slug>.md` audit page listing every wiki
  page touched by the source.
- After the three passes, `index.md` is rebuilt, `log.md` is appended, and the
  QMD search index is updated automatically.

## Relevant Details

The README frames the three-pass design as a division of labor rather than one
large prompt:

1. Extraction interprets the raw source into a structured intermediate form.
2. Page drafting applies those findings to durable wiki pages while preserving
   prior content and provenance.
3. Source summary and post-ingest automation make the ingest traceable and
   immediately queryable.

The README also shows an interactive ingest example where the system presents
the extracted entities and concepts before filing them. That example implies a
useful review boundary between extraction and mutation.

## Why This Source Matters

This source turns Karpathy's general ingest workflow into a concrete
three-pass implementation with distinct responsibilities, outputs, and
post-ingest maintenance steps.
