# Documentation Model

- Document Class: Spec
- Status: Active
- Date: 2026-05-14
- Category: Framework core
- Scope: The validated documentation and execution model for projects using this framework.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md, raw/research/niharshrotri-llm-wiki-implementation.md, wiki/plans/project-registry-search-artifacts.plan.md, wiki/plans/cli-observability.plan.md, wiki/decisions/composable-project-init.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/decisions/wiki-query-search-first.decision.md, wiki/plans/init-rerun-pack-drift.plan.md, wiki/evals/v1-proof-run.eval.md, wiki/evals/natural-language-search.eval.md, wiki/plans/poman-workspace-and-strict-gates.plan.md
- Related: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/decisions/search-backend-selection.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/specs/wiki-init-skill.spec.md, wiki/specs/wiki-query-skill.spec.md, wiki/checklists/observability-contract.checklist.md

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
`custom` do not. Project init writes `.llm_wiki/init.toml` with the setup
answers needed for reruns: project name, project description, blueprint,
resolved packs, resolved folders, and framework version.

Framework distribution is binary-owned and MCP-first. The `llm-wiki` Rust binary
embeds the canonical templates, scaffolds new projects deterministically, and
serves wiki operations to hosts over the Model Context Protocol. It no longer
renders or installs generated runtime skills. `llm-wiki install` manages runtime
state under `~/.llm_wiki/`: the executable is copied or verified at
`~/.llm_wiki/bin/llm-wiki`, install ownership is recorded in
`~/.llm_wiki/manifest.json`, interrupted installs use
`~/.llm_wiki/install.partial.json`, and scoped backup snapshots live under
`~/.llm_wiki/backups/`. Install also materializes the MCP surface: it merges the
active instance into Codex `~/.codex/config.toml` and writes a staged Claude
project config at `~/.llm_wiki/mcp/claude-project.mcp.json`. Hosts spawn
`llm-wiki mcp serve` over stdio on demand; no background daemon is installed. The
MCP configs invoke the managed binary by absolute path; shell `PATH` setup is
terminal convenience only. Repo-local `.claude/skills/` sources remain the
authored skill material; they are not projected into a global runtime by install.

## Operations

Three core operations:

1. **Ingest** - compile new raw sources into wiki pages, update index and log
2. **Query** - answer questions using the wiki, file durable answers back
3. **Lint** - scan for contradictions, stale content, orphans; fix directly

Lint is an agent-owned operation. Its authored guidance lives in the repo-local
`.claude/skills/wiki-lint/` source; hosts read and search the wiki through the
`llm_wiki_*` MCP tools while performing it.

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

## Navigation: How Projects Answer Questions

- `wiki/index.md` is the mandatory first-read orientation and catalog
- `wiki/log.md` tracks all mutations chronologically
- No distributed READMEs; the index is the catalog
- For query operations, `wiki-query` uses `llm_wiki_search` for every query
  after reading the index when the MCP server is exposed and the current
  project is registered. Shell `llm-wiki search --mode auto --format json` is
  fallback only when MCP is unavailable. Search is the default project-local
  retrieval pass, not only a large-wiki fallback.
- If search is unavailable, unregistered, stale, not ready, or unhelpful, the
  agent continues from index-based navigation instead of treating search
  failure or zero results as the answer.
- `llm_wiki_search_all` performs explicit cross-project retrieval across
  registered projects when MCP is exposed; shell `llm-wiki search-all` is the
  fallback. The internal qmd-rs backend owns rebuildable search stores under
  host-local cache state; markdown files under `wiki/` remain canonical
  citations.

Validated search behavior:

- `llm-wiki search` defaults to `auto`.
- `auto` selects lexical when LLM search is disabled or no completed
  LLM-search profile exists.
- `auto` selects hybrid when LLM search is enabled and the project has a fresh
  compatible semantic index. Hybrid works out of the box: the balanced profile
  ships DEFAULT thresholds, so no calibration is required first. Calibration is
  an optional per-corpus override that replaces the shipped defaults with a
  matching scoped threshold record; it is not a gate hybrid waits on.
- Explicit `semantic` and `hybrid` modes fail closed on missing readiness unless
  the caller uses `--allow-lexical-fallback`.
- Hybrid is the promoted natural-language path; semantic-only mode is
  diagnostic and not the promotion surface.
- Thresholds are scoped by project/corpus, profile, embedding artifact,
  dimensions, qmd-rs adapter, qmd-rs version, and chunking strategy. Any label,
  corpus, retrieval, model, qmd-rs, or chunking change requires fresh eval and
  calibration before threshold promotion.
- Completed qmd-rs cache reads are sandbox-safe: read-only search, search-all,
  doctor, and project status use immutable completed-store reads, report
  `permission_denied` and `transient` distinctly from corruption, and keep JSON
  output parseable on cache access failures.

## Promotion Flow: How Accepted Proposals Become Plans

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
- Long-running byte-oriented install operations provide unconditional progress
  on stderr. TTY output is terminal-aware and redraws in place; non-TTY output
  is bounded and append-only. Progress never contaminates normal or JSON
  stdout and does not introduce new interactivity.
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
- `llm-wiki install` materializes the MCP surface (merged Codex
  `config.toml`, staged Claude `claude-project.mcp.json`) with managed-binary
  manifest ownership, scoped backup snapshots, and uninstall symmetry; it writes
  zero runtime skills. It installs `poman` from beside its own binary into the
  managed bin folder and records it (manifest schema 3), refuses when there is
  no `poman` of its version beside it and none recorded, and `llm-wiki
  uninstall` removes everything install wrote, both binaries included
- `llm-wiki path` prints managed-bin PATH guidance
- `llm-wiki mcp serve` runs the stdio MCP server that hosts spawn on demand,
  exposing `llm_wiki_read`, `llm_wiki_search`, `llm_wiki_search_all`,
  `llm_wiki_index`, `llm_wiki_register`, and `llm_wiki_status`
- `llm-wiki read` reads a scoped `wiki/`/`raw/` file through the framework
- `llm-wiki init` produces project scaffolds from embedded templates with
  blueprint/pack golden tests
- `llm-wiki register` manages host-local project registry state and, unless
  `--no-mcp` is set, wires project-local host config by merging the active
  server into `<project>/.mcp.json`; `forget` and `projects` remain registry-only
  and do not write project files
- `llm-wiki index`, `index-all`, `search`, and `search-all` provide default-on
  qmd-rs-backed search over registered project wiki pages
- `llm-wiki search --mode auto` and explicit lexical/semantic/hybrid modes
  implement the accepted semantic/hybrid mode decision, including readiness
  metadata, zero-result metadata, scoped thresholds, and exact-identifier
  preservation
- Completed qmd-rs stores are proved immutable-readable before live promotion;
  read commands use `read_only_immutable` status paths and do not require cache
  write permission.
- `wiki-query` uses `llm_wiki_search` for every registered-project query after
  index orientation when MCP is exposed, falls back to shell search only when
  MCP is unavailable, then reads and cites the returned wiki pages directly
- Semantic/hybrid runtime readiness includes accepted model-license records,
  fresh project-scoped semantic indexes, and compatible scoped thresholds;
  `search-all` reports these outcomes per project and skips unready projects
  without hiding the readiness reason
- Every `llm-wiki` binary command accepts global `-v` / `--verbose` and emits
  command-specific diagnostics to stderr while preserving normal stdout and
  JSON result contracts
- `llm-wiki init` produces project scaffolds from Askama-compiled templates
  using `--blueprint` and repeatable `--pack`, with golden tests for generic,
  custom-pack, `research`, `web-product`, `cli-tool`, `ml-research`, and
  `ops-infra` outputs
- Rerunning `llm-wiki init` against a project with `.llm_wiki/init.toml`
  preloads current setup answers, can create newly selected pack folders, and
  preserves existing `wiki/index.md` and `wiki/log.md`; when the resolved pack
  set or resolved folder composition changes, rerun appends structured
  schema-drift evidence to `wiki/log.md` and refreshes only the generated
  schema-drift section in `wiki/index.md`
- This project uses the framework to manage itself

## Limitations

- Multi-agent coordination is not yet addressed.
- The committed test suite covers deterministic scaffolding, MCP serving,
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
