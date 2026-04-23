# Knowledge Ingest Skill

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Tooling
- Scope: The `knowledge-ingest` skill for processing raw sources into wiki pages.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/init-project-skill.spec.md

## What It Does

The knowledge-ingest skill processes raw source material into the project wiki.
It reads sources, extracts knowledge, creates or updates wiki pages with the
correct document types, checks for contradictions, and maintains the index and
log. In Codex, it triggers from normal requests to ingest, process, or compile
new raw material, and it also supports explicit invocation through
`$knowledge-ingest` and the namespace alias `$knowledge ingest`.

Called without arguments, it scans `raw/` for unprocessed files.

## Scope

Single project only. Ingests into the `wiki/` of the current working
directory.

## Location

Claude skill definition: `.claude/skills/knowledge-ingest/SKILL.md`
Codex skill definition: `.codex/skills/knowledge-ingest/SKILL.md`
Claude global access: symlinked to `~/.claude/skills/knowledge-ingest`
Codex global access: symlinked to `~/.codex/skills/knowledge-ingest`
Codex namespace alias: `$knowledge ingest`

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
- **Raw-only contract** — web or site discovery should use
  `knowledge-research` first so explicit source snapshots exist in `raw/`

## Source Types Supported

Markdown, text, PDF, images (multimodal), conversation transcripts, code
files. URL or web discovery belongs to `knowledge-research`.

## Proven By

- Claude skill file exists at `.claude/skills/knowledge-ingest/SKILL.md`
- Codex skill file exists at `.codex/skills/knowledge-ingest/SKILL.md`
- Codex global symlink exists at `~/.codex/skills/knowledge-ingest`
- Codex dispatcher skill exists at `.codex/skills/knowledge/SKILL.md`
- Claude global symlink exists at `~/.claude/skills/knowledge-ingest`
- The ingest operations performed earlier in this project (QMD, NiharShrotri,
  ecosystem survey) followed this same workflow manually

## Limitations

- No batch progress tracking (no equivalent of init.json for multi-file ingest)
- No streaming/interactive entity confirmation (unlike NiharShrotri implementation)
- No automated scheduling (manual trigger only)
