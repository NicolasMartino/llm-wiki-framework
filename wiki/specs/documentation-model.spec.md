# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-05-11
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/niharshrotri-llm-wiki-implementation.md, wiki/plans/project-registry-search-artifacts.plan.md, wiki/plans/cli-observability.plan.md, wiki/decisions/composable-project-init.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/evals/v1-proof-run.eval.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/decisions/search-backend-selection.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/specs/wiki-init-skill.spec.md, wiki/checklists/observability-contract.checklist.md

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
Root code/deploy folders (`src/`, `tests/`, `scripts/`, `infra/`) are created
only when the resolved pack set includes the `code` pack. Software-shaped
blueprints, including `cli-tool`, default to `code`; `generic`, `research`, and
`custom` do not.

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

Promotion is role-based, not a filename move. Accepted proposals record an
approved direction, but they do not automatically become validated truth. If
the direction requires execution, the roadmap names and coordinates the
deliverable, and a plan owns the tactical work, proof gates, and verification
commands. Only after plan execution produces evidence should the durable
behavior or lasting choice be promoted into a spec or decision. Superseded
proposal and plan pages may then be archived once their validated content has
been carried forward.

## CLI Observability Contract

Observability is part of the framework's core user experience. Any future work
that adds or changes `llm-wiki` binary commands, command decisions, registry
state transitions, install/runtime state, indexing, search, or other
user-visible operational paths must preserve and extend the CLI observability
surface.

Required behavior:

- `CliContext` remains the command-owned diagnostic boundary.
- Verbose diagnostics flow through the CLI's `tracing` subscriber to stderr;
  command modules do not add ad-hoc verbose `eprintln!` paths.
- Normal stdout remains stable, and JSON stdout remains parseable and free of
  diagnostic text or ANSI sequences.
- `-v` / `--verbose` explains real command decisions and state, such as inputs,
  selected paths, registry/project selection, backend state, filters, counts,
  skipped work, safety refusals, and recovery guidance.
- Diagnostic facts come from the same state and decisions used by the command;
  formatters must not re-resolve paths or duplicate command logic.
- `--verbose` does not alter success, failure, or exit-code semantics.
- Tests for new CLI behavior include a nearby verbose assertion and protect
  stderr expectations from inherited `RUST_LOG`.

Use `wiki/checklists/observability-contract.checklist.md` as the review gate for
future CLI implementation plans and code reviews.

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
- Every `llm-wiki` binary command accepts global `-v` / `--verbose` and emits
  command-specific diagnostics to stderr while preserving normal stdout and
  JSON result contracts
- `llm-wiki init` produces project scaffolds from Askama-compiled templates
  using `--blueprint` and repeatable `--pack`, with golden tests for generic,
  custom-pack, `research`, `web-product`, `cli-tool`, `ml-research`, and
  `ops-infra` outputs
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
