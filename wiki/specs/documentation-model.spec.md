# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-05-07
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/qmd-search-engine.md, raw/research/niharshrotri-llm-wiki-implementation.md, wiki/plans/project-registry-search-artifacts.plan.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-search-engine.reference.md, wiki/decisions/search-backend-selection.decision.md

## Current State

The framework uses a three-layer architecture:

1. `raw/` - immutable source material, human-curated
2. `wiki/` - compiled knowledge, agent-owned
3. `CLAUDE.md` / `AGENTS.md` - schema defining conventions and agent workflows

In this framework repository, the canonical reusable specification template is
`assets/templates/project_guidelines.md`. Generated projects get a resolved
`project_guidelines.md` derived from that embedded template.

Framework distribution is binary-owned. The `llm-wiki` Rust binary embeds the
canonical skill sources and templates, renders runtime skill variants, installs
global skills with a manifest, and scaffolds new projects deterministically.
`llm-wiki install` manages runtime state under `~/.llm_wiki/`: the executable is
copied or verified at `~/.llm_wiki/bin/llm-wiki`, install ownership is recorded
in `~/.llm_wiki/manifest.json`, interrupted installs use
`~/.llm_wiki/install.partial.json`, and scoped backup snapshots live under
`~/.llm_wiki/backups/`. Backup snapshots include changed framework skill
targets and any displaced unmanaged binary at the managed binary path.
Installed skills call the managed binary by absolute path; shell `PATH` setup
is terminal convenience only. Spawned projects do not need project-local
framework skill copies.

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
- At scale (>100 pages): `llm-wiki search` supplements `index.md` navigation
  for project-local retrieval, and `llm-wiki search-all` performs explicit
  cross-project retrieval across registered projects. The internal qmd-rs
  backend owns rebuildable search stores under host-local cache state; markdown
  files under `wiki/` remain canonical citations.

## Promotion Flow

research -> raw/ -> ingest -> proposal/reference -> roadmap -> plan -> evidence -> spec/decision -> archive

## Proven By

- `assets/templates/project_guidelines.md` exists and defines the reusable rules
- `CLAUDE.md` and `AGENTS.md` exist and define agent workflows for this repo
- `wiki/index.md` exists and catalogs all wiki content
- `wiki/log.md` records mutations
- `llm-wiki install` writes global runtime skills with managed-binary manifest
  ownership, scoped backup snapshots, and uninstall symmetry
- `llm-wiki path` prints managed-bin PATH guidance without reinstalling skills
- `llm-wiki init` produces project scaffolds from embedded templates with
  profile-specific golden tests
- `llm-wiki register`, `forget`, and `projects` manage host-local project
  registry state without writing to project files
- `llm-wiki index`, `index-all`, `search`, and `search-all` provide default-on
  qmd-rs-backed search over registered project wiki pages
- `llm-wiki build --out .` regenerates this repo's committed runtime skill
  outputs from canonical skill markdown
- This project uses the framework to manage itself

## Limitations

- Not yet tested on a second project (framework portability unproven)
- Scale beyond ~50 pages untested
- Multi-agent coordination not yet addressed
- Ingest, query, research, and lint remain agent-owned operations; the binary
  owns deterministic setup and distribution, not LLM judgment
- Semantic/hybrid model setup and answer synthesis remain future work; D9 ships
  project-local and explicit cross-project FTS retrieval first
- 3-phase ingest pipeline is documented in skills, but not binary-automated
