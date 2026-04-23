# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/qmd-search-engine.md, raw/research/niharshrotri-llm-wiki-implementation.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-search-engine.reference.md

## Current State

The framework uses a three-layer architecture:

1. `raw/` - immutable source material, human-curated
2. `wiki/` - compiled knowledge, agent-owned
3. `CLAUDE.md` - schema defining conventions and agent workflows

The canonical specification is `project_guidelines.md` at the repository root.

## Operations

Three core operations:

1. **Ingest** - compile new raw sources into wiki pages, update index and log
2. **Query** - answer questions using the wiki, file durable answers back
3. **Lint** - scan for contradictions, stale content, orphans; fix directly

## Document Types

Nine typed document roles, each with distinct truth relationship:

- Spec (validated truth), Decision (durable choice)
- Proposal (unvalidated direction)
- Roadmap (deliverable coordination), Plan (tactical execution)
- Experiment (investigation evidence), Eval (measured performance)
- Checklist (repeatable procedure), Reference (external evidence)

## Navigation

- `wiki/index.md` is the sole agent entry point for small wikis (<100 pages)
- `wiki/log.md` tracks all mutations chronologically
- No distributed READMEs; the index is the catalog
- At scale (>100 pages): QMD hybrid search (BM25 + vector + LLM re-ranking)
  supplements index.md navigation. QMD indexes wiki/ as a collection and
  exposes search via MCP server. See wiki/references/qmd-search-engine.reference.md

## Promotion Flow

raw/ -> ingest -> proposal/reference -> roadmap -> plan -> evidence -> spec/decision -> archive

## Proven By

- `project_guidelines.md` exists and defines all rules
- `CLAUDE.md` exists and defines agent workflows
- `wiki/index.md` exists and catalogs all wiki content
- `wiki/log.md` records mutations
- This project uses the framework to manage itself

## Limitations

- Not yet tested on a second project (framework portability unproven)
- Lint operation not yet exercised
- Scale beyond ~50 pages untested
- Multi-agent coordination not yet addressed
- No automated tooling for ingest or lint (manual agent operations only)
- QMD integration identified as the scale solution but not yet implemented
- 3-pass ingest pipeline (extraction → drafting → bookkeeping) not yet adopted
