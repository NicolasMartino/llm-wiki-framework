# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/qmd-search-engine.md, raw/research/niharshrotri-llm-wiki-implementation.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-search-engine.reference.md

## Current State

The framework uses a three-layer architecture:

1. `raw/` - immutable source material, human-curated
2. `wiki/` - compiled knowledge, agent-owned
3. `CLAUDE.md` / `AGENTS.md` - schema defining conventions and agent workflows

In this framework repository, the canonical reusable specification template is
`project_guidelines.template.md` at the repository root. Generated projects get
a resolved `project_guidelines.md` derived from that template.

## Operations

Three core operations:

1. **Ingest** - compile new raw sources into wiki pages, update index and log
2. **Query** - answer questions using the wiki, file durable answers back
3. **Lint** - scan for contradictions, stale content, orphans; fix directly

In Codex, lint can be exposed through a dedicated skill (`knowledge-lint`) or
through the dispatcher alias `$knowledge lint`.

Research is a supporting acquisition step, not a fourth core mutation
operation. Research gathers candidate material into `raw/`, then ingest
compiles those explicit raw sources into `wiki/`.

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

research -> raw/ -> ingest -> proposal/reference -> roadmap -> plan -> evidence -> spec/decision -> archive

## Proven By

- `project_guidelines.template.md` exists and defines the reusable rules
- `CLAUDE.md` and `AGENTS.md` exist and define agent workflows for this repo
- `wiki/index.md` exists and catalogs all wiki content
- `wiki/log.md` records mutations
- This project uses the framework to manage itself

## Limitations

- Not yet tested on a second project (framework portability unproven)
- Scale beyond ~50 pages untested
- Multi-agent coordination not yet addressed
- No automated tooling for ingest or lint (manual agent operations only)
- QMD integration identified as the scale solution but not yet implemented
- 3-phase ingest pipeline is documented in skills, but not automated
