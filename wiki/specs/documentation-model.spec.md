# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-05-09
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/niharshrotri-llm-wiki-implementation.md, wiki/plans/project-registry-search-artifacts.plan.md, wiki/decisions/composable-project-init.decision.md, wiki/evals/v1-proof-run.eval.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/decisions/search-backend-selection.decision.md, wiki/specs/wiki-init-skill.spec.md

## Current State

The framework uses a three-layer architecture:

1. `raw/` - immutable source material, human-curated
2. `wiki/` - compiled knowledge, agent-owned
3. `AGENTS.md` - canonical schema defining conventions and agent workflows,
   with `CLAUDE.md` as a compatibility shim in generated projects

In this framework repository, canonical reusable init templates live under
`templates/base/` and `templates/packs/`. Generated projects get a resolved
`project_guidelines.md` and canonical `AGENTS.md` from a blueprint plus pack
selection. `CLAUDE.md` is generated as `See @AGENTS.md.` for compatibility.

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

In Codex, lint can be exposed through a dedicated skill (`wiki-lint`) or
through the dispatcher alias `$wiki lint`.

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

- `templates/base/project_guidelines.md` and `templates/base/agents.md` define
  the reusable spine
- `templates/packs/` defines opt-in domain fragments for generated projects
- `CLAUDE.md` and `AGENTS.md` exist and define agent workflows for this repo
- `wiki/index.md` exists and catalogs all wiki content
- `wiki/log.md` records mutations
- `llm-wiki install` writes global runtime skills with managed-binary manifest
  ownership, scoped backup snapshots, and uninstall symmetry
- `llm-wiki path` prints managed-bin PATH guidance without reinstalling skills
- `llm-wiki init` produces project scaffolds from embedded templates with
  blueprint/pack golden tests
- `llm-wiki register`, `forget`, and `projects` manage host-local project
  registry state without writing to project files
- `llm-wiki index`, `index-all`, `search`, and `search-all` provide default-on
  qmd-rs-backed search over registered project wiki pages
- `llm-wiki init` produces project scaffolds from Askama-compiled templates
  using `--blueprint` and repeatable `--pack`, with golden tests for generic,
  custom-pack, `ml-research`, and `ops-infra` outputs
- `llm-wiki build --out .` regenerates this repo's committed runtime skill
  outputs from canonical skill markdown
- This project uses the framework to manage itself

## Limitations

- Multi-agent coordination is not yet addressed.
- The committed test suite covers deterministic scaffolding, projection,
  registry, indexing, and search; proof projects for end-to-end agent
  bootstrap currently live under `/private/tmp` rather than as committed
  fixtures.
- Ingest, query, research, and lint remain agent-owned operations; the binary
  owns deterministic setup, distribution, registry, and retrieval, not LLM
  judgment.
- Semantic/hybrid model setup and answer synthesis remain future work; D9 ships
  project-local and explicit cross-project FTS retrieval first.
- Generated projects can opt into qmd-rs guidance with the `qmd-rs-scale` pack, while
  the binary's own default search path uses qmd-rs-backed FTS.
- The 3-phase ingest pipeline is documented in skills, but not binary-automated.
