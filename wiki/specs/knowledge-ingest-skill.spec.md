# Knowledge Ingest Skill

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Tooling
- Scope: The /knowledge-ingest skill for processing raw sources into wiki pages.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/init-project-skill.spec.md

## What It Does

`/knowledge-ingest <path>` processes raw source material into the project
wiki. It reads sources, extracts knowledge, creates or updates wiki pages
with the correct document types, checks for contradictions, and maintains
the index and log.

Called without arguments, it scans `raw/` for unprocessed files.

## Scope

Single project only. Ingests into the `wiki/` of the current working
directory.

## Location

Skill definition: `.claude/skills/knowledge-ingest/SKILL.md`
Global access: symlinked to `~/.claude/skills/knowledge-ingest`

## Three-Phase Pipeline

Inspired by the NiharShrotri/llm-wiki 3-pass approach:

1. **Extraction** — read source, identify facts, entities, relationships,
   decisions, open questions, status
2. **Page drafting** — for each finding, create new page or merge into
   existing page using the correct document type
3. **Bookkeeping** — update index.md, append to log.md, trigger QMD
   reindex if available

## Key Behaviors

- **Merge, don't duplicate** — checks existing pages before creating new ones
- **Correct document type** — classifies by role (spec, decision, proposal, etc.)
- **Contradiction detection** — flags conflicts with existing wiki content,
  asks user to resolve rather than silently overwriting
- **Provenance** — every wiki page cites which raw sources informed it
- **Auto-discovery** — when called without arguments, finds unprocessed
  files by comparing raw/ against log.md entries

## Source Types Supported

Markdown, text, PDF, images (multimodal), URLs (via WebFetch), conversation
transcripts, code files.

## Proven By

- Skill file exists at `.claude/skills/knowledge-ingest/SKILL.md`
- Global symlink exists at `~/.claude/skills/knowledge-ingest`
- The ingest operations performed earlier in this project (QMD, NiharShrotri,
  ecosystem survey) followed this same workflow manually

## Limitations

- No batch progress tracking (no equivalent of init.json for multi-file ingest)
- No streaming/interactive entity confirmation (unlike NiharShrotri implementation)
- No automated scheduling (manual trigger only)
