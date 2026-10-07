# Framework V1 Roadmap

- Document Class: Roadmap
- Status: Active
- Date: 2026-04-23
- Category: Framework development
- Scope: Deliverables required to prove the project management framework works end-to-end and can spawn new self-managing projects.

## Objective

Prove that an LLM Wiki-based project management framework can:

1. manage its own development (self-referential dogfooding)
2. scale to real project complexity
3. spawn new projects where an agent is immediately productive
4. maintain knowledge integrity through ingest/query/lint cycles

## Sequencing Principles

1. Bootstrap first, refine later
2. Each deliverable must produce observable evidence
3. The framework manages its own deliverables (eat your own dogfood)
4. Portability is proven by spawning, not by theorizing

---

### D1 - Bootstrap

Status: Completed
Promise: The framework manages its own creation. Three-layer structure exists, first ingest is done, wiki is navigable.
Depends On: None
Execution Plan: Not needed (single-session bootstrap)

Included:
- reusable project guidelines template written
- CLAUDE.md written
- raw/, wiki/ structure scaffolded
- Legacy guidelines and research moved to raw/
- First ingest: research and legacy compiled into wiki pages
- Decisions recorded for key architectural choices
- Spec written for the documentation model
- index.md and log.md created
- This roadmap created

Excluded:
- Lint operation
- Automated tooling
- Multi-project testing

Proof:
- wiki/index.md catalogs all pages
- wiki/log.md records the bootstrap
- Agent can orient by reading index.md alone

Promotion Target:
- wiki/specs/documentation-model.spec.md (done)

Unlocks:
- D2, D3

---

### D2 - Ingest Cycle

Status: Completed
Promise: A new raw source is ingested end-to-end: the agent reads it, creates/updates wiki pages, checks for contradictions, updates index and log. The workflow is repeatable.
Depends On: D1
Execution Plan: Not created yet

Included:
- Ingest a non-trivial new raw source (not the bootstrap material)
- Agent follows the ingest workflow from CLAUDE.md
- Contradictions with existing content are surfaced
- Index and log are updated correctly
- Cross-references between new and existing pages are created

Excluded:
- Automated ingest triggers
- Bulk ingest of multiple sources

Proof:
- New wiki pages exist with correct metadata and source citations
- wiki/index.md includes the new pages
- wiki/log.md records the ingest
- At least one cross-reference to an existing page was created or updated

Promotion Target:
- wiki/specs/documentation-model.spec.md (update ingest as proven)

Unlocks:
- D3, D5

---

### D3 - Lint Operation

Status: Completed
Promise: The agent runs a lint pass over the wiki and finds at least one real issue (contradiction, orphan, stale status, missing cross-reference). The issue is fixed and logged.
Depends On: D1
Execution Plan: Not needed (single-session lint pass)

Included:
- Full wiki scan
- Contradiction detection
- Orphan page detection
- Stale status detection
- Missing cross-reference detection
- Direct fix of issues found
- Log entry for each fix

Excluded:
- Automated lint scheduling
- Lint tooling or scripts

Proof:
- At least one real issue found and fixed
- wiki/log.md records the lint and fixes
- wiki/index.md is consistent after lint

Promotion Target:
- wiki/specs/documentation-model.spec.md (update lint as proven)

Unlocks:
- D5

---

### D4 - Query Produces Durable Knowledge

Status: Completed
Promise: A non-trivial question is asked, the agent answers from wiki content, and the answer is valuable enough to file as a new wiki page.
Depends On: D2
Execution Plan: wiki/evals/v1-proof-run.eval.md

Included:
- A real question that requires synthesizing multiple wiki pages
- Answer with citations to wiki pages
- New wiki page created from the answer
- Index and log updated

Excluded:
- Query tooling
- Search beyond index.md navigation

Proof:
- New wiki page exists from a query answer
- Page cites multiple existing wiki pages
- wiki/index.md includes the new page
- Captured in wiki/evals/v1-proof-run.eval.md after the "what's left to do?"
  query produced durable implementation and proof knowledge.

Promotion Target:
- wiki/specs/documentation-model.spec.md (update query as proven)

Unlocks:
- D5

---

### D5 - Framework Spawns a New Project

Status: Completed
Promise: The `wiki-init` skill creates a new project end-to-end. A fresh agent in the new repo, with only the generated `project_guidelines.md`, canonical `AGENTS.md`, and `CLAUDE.md` compatibility shim, bootstraps a self-managing wiki and is immediately productive.
Depends On: D2, D3, D4
Execution Plan: wiki/evals/v1-proof-run.eval.md

Included:
- Run `wiki-init` on a real project
- Skill asks questions, generates tailored guidelines, `AGENTS.md`, and a
  `CLAUDE.md` shim
- Scaffolds raw/ + wiki/ with correct profile (ML_AI, SEARCH flags)
- Agent in the new project ingests first raw sources
- Agent answers a query about the new project using only wiki content

Excluded:
- Multi-agent coordination
- Scale testing (handled in D6)

Proof:
- New project has a navigable wiki after bootstrap
- Agent can answer a domain question from the wiki
- No human edited wiki/ directly
- Proven with `/private/tmp/llm-wiki-proof-web-product`, which was initialized
  with `llm-wiki init`, ingested its initial raw source, and answered its first
  domain question from typed wiki pages.

Promotion Target:
- wiki/specs/documentation-model.spec.md (update portability as proven)

Unlocks:
- D6, D7

---

### D6 - Scale Test

Status: Completed
Promise: The wiki handles 50+ pages across multiple document types without degrading agent navigation. Index remains usable.
Depends On: D5
Execution Plan: wiki/evals/v1-proof-run.eval.md

Included:
- A project with 50+ wiki pages
- Agent navigates via index.md to answer questions
- Measure: does the agent find the right pages on first try?
- Measure: does index.md fit in context?
- If index exceeds context: implement sub-indexes per type

Excluded:
- Hybrid search (BM25 + vector)
- Performance benchmarking

Proof:
- Agent answers 10 questions correctly using only index navigation
- Index token count measured and recorded
- If sub-indexes needed: implemented and working
- Proven against the current 50-page project wiki. The index remained compact
  enough for first-pass navigation, so sub-indexes were not needed.

Promotion Target:
- wiki/specs/documentation-model.spec.md (update scale characteristics)

Unlocks:
- D7

---

### D7 - Self-Replicating Framework

Status: Completed
Promise: The framework is packaged so that creating a new self-managing project is a single operation: point the agent at a domain, provide initial raw sources, and the agent produces a working wiki.
Depends On: D5, D6
Execution Plan: wiki/evals/v1-proof-run.eval.md

Included:
- Template `AGENTS.md`, `CLAUDE.md` shim, and `project_guidelines.md`
- Documented bootstrap procedure (as a checklist)
- Agent can execute the bootstrap checklist without modification
- Tested on at least two distinct project domains

Excluded:
- GUI or CLI tooling
- Framework versioning or updates to spawned projects

Proof:
- Two distinct projects bootstrapped by agents using the framework
- Both projects have navigable wikis
- Neither required human editing of wiki/
- Proven with `/private/tmp/llm-wiki-proof-web-product` (`web-product`) and
  `/private/tmp/llm-wiki-proof-ml-research` (`ml-research`).

Promotion Target:
- wiki/specs/documentation-model.spec.md (framework is proven self-replicating)

Unlocks:
- D8 (faster proof path), Future: multi-agent, automated lint, hybrid search

---

### D8 - Distribution Tooling (`llm-wiki` Binary)

Status: Completed
Promise: A single Rust binary (`llm-wiki`) installs framework skills globally for both Claude Code and Codex with one command, scaffolds new projects deterministically, and projects canonical skill markdown into per-runtime variants. No symlinks, no working tree dependency, no manual configuration. Spawned projects need no skill files of their own.
Depends On: None (D8 ships independently; it makes D5 cheaper and D7's proof faster but is not blocked by either)
Execution Plan: wiki/plans/llm-wiki-binary.plan.md

Included:
- `llm-wiki install`: writes skills directly to `~/.claude/skills/` and `~/.codex/skills/` with an ownership manifest at `~/.local/share/llm-wiki/manifest.json`. Idempotent; refuses user-authored collisions; `--force` backs up to `<path>.bak.<UTC-ISO8601>` before overwriting.
- `llm-wiki build [--target] [--out]`: renders canonical skills to a chosen directory without touching global state or writing a manifest. Used for self-dogfooding this repo (`build --out .`) and for CI snapshot tests.
- `llm-wiki init <path>`: Create-mode scaffolding from embedded templates with conditional-section resolution (ML_AI, SEARCH, and the later-retired IS_EXISTING profile). Collision check is on framework artifacts only (`wiki/`, `raw/`, `.llm_wiki/`, `AGENTS.md`, `CLAUDE.md`, `project_guidelines.md`), not on directory emptiness. Update mode is explicitly out of scope and remains agent-owned.
- `llm-wiki status`, `doctor`, `uninstall`: state diagnostics and clean removal.
- Canonical skill schema: clean markdown with typed YAML frontmatter (no `legacy tag marker` blocks). Embedded into the binary at compile time via `include_str!`.
- Typed Rust projector with golden-file (`insta`) snapshot tests for all skill × runtime projections and all `init` profile outputs.
- Compat fixtures under `tests/fixtures/wikis/v1/` exercising parseability, metadata extraction, template compatibility, and skill availability.
- `cargo-dist` multi-arch release pipeline (macOS arm64, macOS x86_64, Linux x86_64, Linux arm64).
- Full test strategy: unit, integration (with `tempfile::TempDir` and `HOME` redirection), golden-file, property (idempotency, totality), post-install verification. ≥ 80% line coverage gate via `cargo-llvm-cov`.

Excluded:
- `init` Update mode (remains agent-owned).
- Automated runtime skill-discovery tests (no stable runtime API exists).
- Auto-update (`llm-wiki self-update`).
- Per-project skill overrides.
- Runtime support beyond Claude and Codex (Cursor/Aider/Amp deferred to V2).
- MCP-based skill exposure.
- Network fetching of skills (all content embedded at compile time; only distribution acquisition is network-dependent).

Proof:
- `llm-wiki install` against a redirected `HOME` writes correct files with a manifest matching every entry; second run is a no-op.
- `llm-wiki uninstall` removes only manifest-owned files, verified by integration test against a tempdir containing both framework and user-authored content.
- `llm-wiki init <path>` produces the same project structure as the initialization skill for equivalent answers (golden-file fixture).
- `cargo insta test --check` passes in CI for all skill × runtime projections and all `init` profiles.
- Compat fixture v1 wiki passes parseability, metadata, template, and skill-availability checks.
- Binary builds on all four target platforms via `cargo-dist`.
- End-to-end install path (`curl ... | sh && llm-wiki install`) verified by post-install file-and-manifest assertions in a release-gate workflow.

Promotion Target:
- wiki/specs/documentation-model.spec.md (distribution model recorded as proven)
- wiki/specs/wiki-init-skill.spec.md (binary as the authority for scaffolding; agent retains intake)
- wiki/specs/wiki-*-skill.spec.md (canonical source location updated to `skills/<name>/SKILL.md`)
- wiki/decisions/llm-wiki-binary-distribution.decision.md (already accepted; status confirmed as proven)

Unlocks:
- Faster D5 (`llm-wiki init` replaces today's manual scaffolding skill flow).
- Cheaper D7 proof (two distinct projects scaffolded via `llm-wiki init`; agents bootstrap each one's wiki without manual symlink-and-copy work).
- Future: V2 runtime targets (Cursor, Aider, Amp) via the projector trait.

Closes:
- review.md §9.1 (hardcoded skill paths) by construction — binary embeds its own content.
- review.md §9.2 (broken global symlinks) by construction — no symlinks at all.
- The pre-binary renderer's content-loss class of bug — golden-file snapshot tests are the explicit defense.

---

### D8.1 - Managed Runtime Bootstrap

Status: Completed
Promise: `llm-wiki install` makes installed skills work even when the user runs
a manually downloaded binary from outside `PATH`. Skills invoke a managed binary
absolute path under `~/.llm_wiki/bin/llm-wiki`; PATH setup is only terminal
convenience.
Depends On: D8
Execution Plan: wiki/plans/binary-path-bootstrap.plan.md

Included:
- Managed runtime home at `~/.llm_wiki/` on Unix-like systems and future
  `%LOCALAPPDATA%\llm_wiki\` on Windows.
- Managed binary copied or verified at `~/.llm_wiki/bin/llm-wiki`.
- Manifest v2 at `~/.llm_wiki/manifest.json` with `sha256` hashes and explicit
  binary, skill, and backup entries.
- Pre-release correction from the D8 manifest path
  `~/.local/share/llm-wiki/manifest.json`; no public migration required.
- `install.partial.json` transaction marker for interrupted-install recovery.
- Self-install behavior that skips copy-over-self and verifies in place.
- Installed skills render managed absolute binary paths instead of bare
  `llm-wiki`.
- Scoped backup snapshots under `~/.llm_wiki/backups/`.
- `llm-wiki path` for PATH guidance without reinstalling skills.
- `doctor` reports managed binary drift, stale transaction state, and PATH
  visibility as convenience status.
- `wiki-init` is the initialization skill name before public release (renamed
  from `knowledge-init` by D11).

Excluded:
- Silent shell profile edits.
- Windows release artifacts.
- Self-update.
- Automated rollback command.

Proof:
- End-to-end test runs a binary from outside `PATH`, installs skills without
  shell profile edits, invokes an installed skill or skill-equivalent stub, and
  confirms the managed binary path executes.
- Integration tests cover manifest v2, backup snapshots, managed binary
  executability, outside-`PATH` execution, `llm-wiki path`, doctor drift
  reporting, and uninstall behavior.

Promotion Target:
- wiki/specs/documentation-model.spec.md (managed runtime home and manifest v2
  recorded as validated distribution behavior)
- wiki/specs/wiki-init-skill.spec.md (replaced or superseded once implementation lands)
- wiki/specs/wiki-*-skill.spec.md if dispatcher or installed-path behavior
  changes

Unlocks:
- Manual-download installation path that works without shell profile edits.
- Cleaner Windows compatibility path before Windows release artifacts ship.

---

### D9 - Project Registry and Search Artifacts

Status: Completed
Promise: `llm-wiki` owns a host-local project registry, rebuildable search
artifacts, project-local search, and explicit cross-project `search-all`
retrieval while keeping markdown wiki pages canonical.
Depends On: D8, D8.1, Search Backend Selection
Execution Plan: wiki/plans/project-registry-search-artifacts.plan.md

Included:
- Host-local `projects.json` registry under the framework data home.
- `register`, `forget`, and `projects` commands.
- `index` and `index-all` commands that build rebuildable qmd-rs search stores
  from `wiki/**/*.md`.
- `search` command for project-local retrieval with class/status filters and
  text/JSON output.
- `search-all` command for explicit cross-project retrieval across registered
  projects, with project labels, include/exclude filters, and RRF fusion.
- `init --no-register` and default auto-registration after successful project
  scaffolding.
- `doctor` registry/search diagnostics.

Excluded:
- Answer synthesis or a `query` command.
- Automatic `search-all` use by `wiki-query`.
- Indexing `raw/` by default.
- Automatic semantic model downloads.

Proof:
- Registry commands operate against redirected `HOME` fixtures without writing
  outside temp state.
- `init` auto-registers successful scaffolds by default and supports
  `--no-register`; recoverable registry-write failure is tested.
- Default builds include qmd-rs and exercise real index/search behavior.
- Two-project fixtures prove `search-all` labels results, honors include/exclude
  filters, and keeps cross-project retrieval explicit.
- `doctor` reports install, registry, current-project, search-index, and
  semantic-model sections separately.
- `just verify` and `cargo test --workspace` pass.

Release Notes:
- New commands: `register`, `forget`, `projects`, `index`, `index-all`,
  `search`, and `search-all`.
- New default behavior: qmd-rs is always compiled in normal builds, and the
  first `llm-wiki index --project <id>` creates the per-project search store.
- New framework paths: `~/.cache/llm-wiki/` for rebuildable indexes/model cache
  and `~/.local/share/llm-wiki/projects.json` for the host-local project
  registry.
- Migration: no user action is required. Existing projects remain valid; search
  state appears when each project is first indexed.

Promotion Target:
- wiki/specs/documentation-model.spec.md (scale search becomes
  `llm-wiki search` / `search-all`)
- wiki/evals/search-backend-selection.eval.md (user-visible command replay if
  behavior changes)

Unlocks:
- Framework-owned search at scale without asking agents to manage separate
  search indexes manually.
- Later explicit `wiki-query` scope expansion for registered
  cross-project search.

---

### D10 - Composable Project Init

Status: Completed
Promise: `llm-wiki init` produces a tailored canonical `AGENTS.md` and
`project_guidelines.md` from a chosen blueprint (or `custom`) plus a selected
set of opt-in packs, rendered through a compile-time template engine. Claude
compatibility is preserved by writing a tiny `CLAUDE.md` shim pointing at
`AGENTS.md`. The current static template gated by `<!-- SECTION:ML_AI -->` /
`<!-- SECTION:SEARCH -->` is retired in the same change set. Each project
gains a `.llm_wiki/init.toml` recording setup answers for manifest-backed
reruns and a future `upgrade` command.
Depends On: D8.1
Execution Plan: wiki/plans/composable-project-init.plan.md

Included:
- A compile-time template engine (`askama`, per the decision) wired into the
  binary as the single rendering path for init.
- Markdown templates stored as `.md` files under `templates/`, rendered through
  Askama with `escape = "none"`.
- Migration of the existing init template onto the chosen engine, byte-stable
  against current snapshots except for the intentional `AGENTS.md` canonical
  file plus `CLAUDE.md` shim transition.
- Rust `Pack` and `Blueprint` enums in `src/init/` with accessor methods —
  the pack catalog is a Rust API surface, not a TOML schema.
- Initial pack catalog plus post-D10 code pack: `api`, `frontend`, `library`,
  `ml`, `data`, `ops`, `ops-lite`, `security`, `research`, `qmd-rs-scale`,
  `code`.
- Initial blueprint catalog: `generic`, `web-product`, `library-sdk`,
  `cli-tool`, `ml-research`, `ops-infra`, `security`, `research`, `custom`.
- Two-step interactive flow with `inquire`: blueprint `Select`, then pack
  `MultiSelect` with the blueprint's defaults pre-checked.
- Non-interactive flag mapping: `--blueprint <name>` and repeatable
  `--pack <name>`; the old `--type` and `--scale` init flags are retired with
  the static profile model.
- Per-project `.llm_wiki/` folder with `init.toml` recording project name,
  project description, chosen blueprint, resolved pack list, and framework
  version.
- Manifest-backed rerun mode: interactive reruns prefill current setup answers,
  newly selected pack folders are created, and existing `wiki/index.md` /
  `wiki/log.md` content is preserved.
- Blueprint changes during rerun use the new blueprint's pack defaults, and
  auto-registration updates the existing same-root registry entry while
  preserving its project id.

Excluded:
- Skill projection migration onto the same engine (sibling proposal:
  `wiki/proposals/skills-template-engine.proposal.md`).
- A full `upgrade` migration command. Rerun init can edit setup answers, but it
  does not perform arbitrary project migrations.
- Project-local pack overrides under `.llm_wiki/`.
- User-defined packs (defining a pack means writing Rust).

Proof:
- `cargo test --workspace` is green, including snapshot tests for every named
  blueprint at default pack selection and every pack rendered standalone.
- Two integration tests through `assert_cmd` — `--blueprint ml-research` and
  `--blueprint ops-infra` non-interactive runs — produce green wikis with the
  expected folders, doc types, and status vocabulary.
- The migrated existing init template's content is byte-identical to the
  pre-migration output for at least one fixed input set, except for the
  intentional `AGENTS.md` canonical file plus `CLAUDE.md` shim transition.
- `.llm_wiki/init.toml` round-trips: `init` writes it, a follow-up read parses
  it back into the same enum values.

Promotion Target:
- wiki/specs/wiki-init-skill.spec.md (new flow, two-step prompts, flag
  surface).
- wiki/specs/documentation-model.spec.md (composable init replacing the static
  template description, if the spec currently leans on it).

Unlocks:
- D7 self-replicating proof against two genuinely different project shapes
  rather than two copies of the same template.
- The skill-projection follow-on (`wiki/proposals/skills-template-engine.proposal.md`)
  built on a proven template engine.

---

### D11 - Project and Skill Rename

Status: Completed
Promise: The framework's user-facing product surface is renamed coherently:
the Cargo package becomes `llm-wiki-rs`, canonical skills and installed runtime
surfaces move from `knowledge*` to `wiki-*`, active documentation matches the
new names, and the few remaining pre-rename local repos are handled through a
one-off legacy migration that does not become framework behavior.
Depends On: D8.1, D10
Execution Plan: wiki/plans/project-and-skill-rename.plan.md

Included:
- Rename Cargo package metadata from `llm-wiki-framework` to `llm-wiki-rs`.
- Rename canonical embedded skills from `knowledge*` to `wiki-*`.
- Rename the dispatcher from `knowledge` to `wiki`.
- Regenerate `.claude/` and `.codex/` runtime mirrors from the renamed
  canonicals.
- Sweep active specs, decisions, plans, templates, README, index, and log for
  current-truth references to the old names.
- Prove the renamed install/discovery/invocation/uninstall surface in both
  Claude and Codex.
- Execute the one-off local legacy migration for the few repos that still
  depend on home-level skill symlinks pointing into in-repo mirrors.

Excluded:
- Product-level compatibility aliases for `knowledge*`.
- Automatic legacy handling in `llm-wiki install`.
- Changes to `~/.llm_wiki/` managed runtime paths.
- Changes to `knlg` or the separate `es_llm_wiki` repo.

Proof:
- `llm-wiki install` writes only `wiki*` skill names.
- Claude and Codex expose and invoke the renamed commands successfully.
- Active documentation no longer describes `knowledge*` or
  `llm-wiki-framework` as current truth.
- The one-off local legacy migration is recorded in `wiki/log.md`, and the
  affected home-level `knowledge*` symlinks now target frozen
  `.claude.legacy/` / `.codex.legacy/` trees instead of this repo's live
  `.claude/skills/` / `.codex/skills/` trees.

Promotion Target:
- wiki/archive/knowledge-command-namespace.decision.md is now archived as
  superseded; the active namespace is documented in the renamed `wiki-*`
  skill specs and the dispatcher canonical at `assets/skills/wiki/SKILL.md`.

Unlocks:
- Clean post-rename baseline for future public-facing documentation and any
  post-D10 template-retirement follow-on.

---

## Post-V1 Backlog

### P1 - Semantic and Hybrid Search

Status: Completed
Promise: Natural-language project questions retrieve useful wiki pages through
semantic/hybrid search without weakening exact lexical search.
Depends On: D9; CLI verbose diagnostics completed in a separate worktree
Execution Plan: wiki/plans/semantic-hybrid-search.plan.md
Completed: 2026-05-12. Validated outcome is recorded in
wiki/decisions/semantic-hybrid-search-mode.decision.md,
wiki/specs/documentation-model.spec.md, wiki/specs/wiki-query-skill.spec.md,
and wiki/evals/natural-language-search.eval.md.

Included:
- interactive LLM search profile setup
- managed `~/.llm_wiki` model/index state
- `auto`, `lexical`, `semantic`, and `hybrid` search modes
- semantic index freshness and model readiness checks
- relevance floors, rank fusion, and optional reranking
- rank-merged `search-all`
- `wiki-query` metadata consumption

Excluded:
- hosted LLM retrieval
- answer synthesis inside the binary
- silent model downloads
- mandatory reranking

Proof:
- natural-language eval records hybrid improvement without exact lexical
  regressions
- JSON and verbose diagnostics distinguish readiness failures from honest
  zero-result outcomes
- existing lexical evals and command contracts remain green

Promotion Target:
- wiki/specs/documentation-model.spec.md
- wiki/specs/wiki-query-skill.spec.md
- a durable search-mode decision after validation

Unlocks:
- direct CLI search for realistic project questions
- search-first `wiki-query` retrieval for registered-project queries, with
  index-based fallback when search is unavailable or unhelpful

### P2 - Sandbox-Safe Search Cache Reads

Status: Completed
Promise: Read-only search, search-all, doctor, and project-status checks can
inspect completed managed qmd-rs caches from sandboxed agents without cache
write permission and without misclassifying access failures as corruption.
Depends On: P1, D9
Execution Plan: wiki/plans/sandbox-safe-search-cache-reads.plan.md
Proposal: wiki/proposals/sandbox-safe-search-cache-reads.proposal.md
Completed: 2026-05-23. Validated outcome is recorded in
wiki/plans/sandbox-safe-search-cache-reads.plan.md,
wiki/decisions/search-backend-selection.decision.md, and
wiki/decisions/semantic-hybrid-search-mode.decision.md.

Included:
- immutable completed-store read path for lexical/status reads
- writer-side completed-store proof before qmd-rs live promotion
- metadata-first validation for completed stores
- retryable transient state for mixed sqlite/metadata promotion observations
- `permission_denied` state distinct from corrupt/schema-mismatch/stale/missing
- `search-all` per-project JSON readiness and warning behavior for cache access
  failures
- backend-status JSON `open_mode`

Excluded:
- moving indexes into project roots
- implicit rebuilds from read commands
- model/license/materialization changes
- Metal, query-expansion, embedding, or reranker runtime diagnostics
- replacing qmd-rs

Proof:
- lexical JSON search succeeds against a managed qmd-rs cache with read access
  but no write access
- doctor/projects report read-only caches as ready or stale and true access
  failures distinctly
- permission_denied never routes to force-reindex guidance
- search-all JSON stays parseable and reports per-project access failures while
  returning other ready results
- writer promotion verifies immutable readability before publication and treats
  races as completed old/new or retryable transient states
- existing qmd-rs query parity tests remain green

Promotion Target:
- wiki/decisions/search-backend-selection.decision.md
- wiki/decisions/semantic-hybrid-search-mode.decision.md
- wiki/specs/documentation-model.spec.md
- wiki/specs/wiki-query-skill.spec.md

Unlocks:
- reliable search-first `wiki-query` in sandboxed Codex sessions
- safer project-update `--reindex` follow-on
- cleaner Windows/E2E proof later because read/write cache scopes are explicit

### P3 - Cross-Platform Release E2E And Windows Support

Status: Active
Promise: Supported release artifacts prove the documented `llm-wiki` product
story on their target platforms, with Linux Docker simulation, native host
proof, Windows host proof, and GGUF CPU proof separated by evidence type.
Depends On: P1, P2, D8-D11
Execution Roadmap: wiki/roadmaps/cross-platform-release-e2e.roadmap.md
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md
Proposal: wiki/proposals/full-windows-support.proposal.md

Included:
- release E2E runner and report contract
- Linux Docker archive simulation and native Linux proof
- native macOS archive proof
- Windows runtime path, PowerShell, artifact, and host E2E proof
- real GGUF CPU release proof where semantic/hybrid support is claimed

Excluded:
- treating Docker as macOS or Windows proof
- claiming Windows support before Windows-specific path and host gates pass
- claiming semantic/hybrid platform support from deterministic hooks or
  no-model readiness checks

Proof:
- target-platform release E2E reports exist for each supported artifact
- report metadata distinguishes native, virtualized, and emulated proof
- Windows reports cover Known Folder paths, path with spaces, PowerShell
  invocation, `.exe` runnability, JSON path escaping, search, and uninstall
- GGUF reports prove forced-CPU semantic, hybrid, and `search-all` from
  managed models

Promotion Target:
- wiki/specs/documentation-model.spec.md
- future release E2E checklist or eval
- Windows support decision/spec after validation

Unlocks:
- honest platform support claims
- release-gate evidence for future artifacts

### P4 - Coordinated Development Workflow

Status: Completed (develop)
Promise: This repository is worked on the way the owner decided on
2026-10-06: the owner decides, a coordinating session keeps the roadmaps, the
plans and the board true and starts the workers, every PR gets one blind
feedback review and then the owner's verdict, and the repository is the record.
Depends On: None
Execution Plan: wiki/plans/development-workflow-setup.plan.md

Included:
- the coordinator's checklists, the operations skills and the worker briefs
  (committed 2026-10-06 as `70bc5a6`)
- the board (GitHub project 5) with the plan statuses as its columns, its ids
  in `wiki/checklists/operation-manager.checklist.md`
- wiki search for this repository and for each worker's worktree (`orca.yaml`,
  `tools/wiki-worktree.sh`)
- the stale branch `impl/sandbox-safe-search-cache-reads`, whose work reached
  master in another form, kept as the tag
  `archive/sandbox-safe-search-cache-reads` and removed as a branch (the
  owner, 2026-10-06)
- the first work, each its own entry below: poman's decisions and roadmap (P5),
  the four plans' statuses (P6), the install download progress branch (P7),
  the `build-skills` recipes (P8)

Excluded:
- GitHub branch rules and merge settings (none yet, the owner, 2026-10-06)
- releases, which stay the owner's
- a tool drawing the board (poman's roadmap, from P5)

Proof:
- `just branch-status` runs clean
- the setup plan is `Completed (master)` and `wiki/log.md` records the setup
- the first worker PRs land through the blind review and the owner's PASS

Promotion Target:
- wiki/decisions/work-is-recorded-in-the-repository.decision.md
- wiki/checklists/operation-manager.checklist.md

Unlocks:
- P5 to P8, and poman's roadmap

### P5 - poman's Decisions And Roadmap

Status: Completed (develop)
Promise: The owner's settled decisions for poman, the project-management
binary, are wiki decision pages, and their order is a poman roadmap the owner
has approved, so no poman work starts from a conversation.
Depends On: P4
Execution Plan: Not needed (wiki-only work)
Source: the owner's decisions of 2026-10-06

Included:
- decision pages for the settled areas: where poman lives and the shared
  crate; syncing with a tracker the way Git syncs a remote; file types and the
  deadline type
- the owner's answers of 2026-10-06 to the questions still open: a
  duration in whole working days (Monday to Friday), three importance levels,
  `poman tree` and `poman list` as their own deliverable after the deadline
  commands, no "Part of" field for now, poman drawing this repository's own
  board later, and the strict gates reached by a ratchet
- GitHub as one interface among others (the owner, 2026-10-06: "gh is just a
  ui for poman so we should think this so as to be able to adapt it to other
  UIs if needed"): poman's data and its fetch, diff and push are defined
  without GitHub in them, and GitHub is the first adapter, so another tracker
  or board can be added without changing the core
- `wiki/roadmaps/poman.roadmap.md`, Draft, for the owner to approve

Excluded:
- code, crates, and the plans of poman's deliverables (each comes once the
  roadmap is approved)
- the source material, which stays outside this repository (the owner,
  2026-10-06)

Proof:
- the decision pages and the roadmap are on master, listed in `wiki/index.md`
- the owner has approved the poman roadmap

Unlocks:
- poman's first plan: the workspace and the strict gates

### P6 - Plan Statuses In The Vocabulary

Status: Completed (develop)
Promise: Every plan's `- Status:` line uses the plan vocabulary (Draft,
Active, Blocked, Completed, Superseded), with what is still pending written in
its body, so `just branch-status` can pass.
Depends On: P4
Execution Plan: Not needed (wiki-only work)

Included:
- `wiki/plans/headroom-mcp-merge-readiness-repair.plan.md` and
  `wiki/plans/macos-installed-binary-codesign-repair.plan.md`, whose metadata
  block has no bullets, so no `- Status:` line is found
- `wiki/plans/headroom-passthrough-launcher.plan.md` and
  `wiki/plans/headroom-wrap-command.plan.md`, whose status says
  "Implemented…"
- their entries in `wiki/index.md`

Excluded:
- the pending proofs themselves (a host parity gate, a native release archive
  proof)

Proof:
- `just branch-status` reports no STALE line

### P7 - Install Model Download Progress

Status: Draft
Promise: `llm-wiki install` with LLM search enabled reports progress on stderr
while it hashes, downloads and verifies the model (a bar on a terminal, a line
every 25 % otherwise), and leaves stdout and JSON output unchanged.
Depends On: None
Execution Plan: wiki/plans/install-download-progress.plan.md (on the branch
`impl/install-download-progress`, pushed 2026-10-06, until its PR merges)

Included:
- the plan's scope, already written on the branch
- landing the branch through a draft PR, its blind review and the owner's PASS

Excluded:
- what the plan excludes

Proof:
- the plan's proof, which includes a real install with the model enabled

### P8 - Working build-skills Recipes

Status: Completed (develop)
Promise: `just build-skills` and `just build-skills-to <out>` do what their
names say, or are gone, instead of failing with "unrecognized subcommand
'build'".
Depends On: None
Execution Plan: `wiki/plans/build-skills-recipes.plan.md`

Included:
- the two justfile recipes, which call `cargo run -- build`, a subcommand the
  CLI no longer has
- the pages that tell people to run them

Proof:
- each recipe runs and produces its output, or it no longer exists and no page
  tells anyone to run it (pages that record the failure as history stay)

### P9 - Search Ranks A Page's Own Title Low

Status: Completed (develop)
Promise: A lexical search for a page's own title finds that page first, or the
reason it cannot is known and written down.
Depends On: None
Execution Plan: Not needed (an investigation first; any fix gets its own plan)

Included:
- the case found on 2026-10-06 in this repository: `llm-wiki search "operation
  manager" --mode lexical` ranks the page titled "Operation Manager"
  (`wiki/checklists/operation-manager.checklist.md`) fourth, with every result
  scoring 0.000, and "operation manager checklist" leaves it out of the top
  three
- why the scores collapse to zero, and whether a page's title weighs in the
  ranking at all

Excluded:
- semantic and hybrid search, unless the cause is shared

Proof:
- a comment on its issue giving the cause, or every cause ruled out, each
  number with the command that produced it

### P10 - A Fast Check On Develop, The Full CI On The Way To Master

Status: Completed (develop)
Promise: Work lands on `develop`. A PR into `develop`, and its merge there,
run one fast Linux check; only the PR from `develop` into master, and master
itself, run the full CI. Both are green.
Depends On: None
Execution Plan: wiki/plans/develop-and-master-ci.plan.md

Included:
- the `develop` branch, cut from master on 2026-10-06 and made GitHub's
  default branch (the owner, 2026-10-06)
- the CI split by target branch, and the three reasons master's CI is red
- `just branch-status` taking `develop` as the base

Excluded:
- releases, tags and the post-install workflow
- the rules and pages that still say "master" (P11)

Proof:
- the plan's Done When

### P11 - The Way Of Working Points At Develop

Status: Completed (develop)
Promise: Every rule, brief and skill of the way of working says where work
lands now: work PRs into `develop`, and master only through a PR from
`develop` with the full CI and the owner's PASS.
Depends On: None
Execution Plan: Not needed (wiki-only work)

Included:
- the three workflow decisions, the two checklists, the worker base texts, the
  spec template and the operations skills
- what "Completed (master)" becomes, where the log entry is written (per merge
  into `develop`, and for the PR into master), and when `develop` goes to master
- the setup plan's and AGENTS.MD's lines, proposed for the coordinator to apply

Excluded:
- CI and `just branch-status` (P10)

Proof:
- no page of the way of working tells a worker or the coordinator to branch
  from, target or merge into master, except for the PR from `develop`

### P12 - The Release Workflow Off The Retired Runner

Status: Draft
Promise: A tag release runs again: the release workflow's jobs ask for a
runner image GitHub still provides, instead of the retired `ubuntu-20.04`
that cargo-dist 0.28 asks for by default.
Depends On: None
Execution Plan: Not created yet (a short plan comes before its worker)

Included:
- the release workflow's runners, through cargo-dist's own configuration
  (`github-custom-runners`), with `release.yml` regenerated, not hand-edited
- whether the release plan job comes back on PRs into master

Excluded:
- cutting a release, which stays the owner's

Proof:
- a dry run of the release workflow gets a runner and passes its plan job

Starts only when the owner says so (the owner, 2026-10-06): releases are the
owner's.

### P13 - Run The Tests Once And Share One CI Cache

Status: Completed (develop)
Promise: `just verify`, and the full CI that runs it, run the test suite once
instead of twice, and the CI cache is saved only from `develop` and master, so
every PR starts from `develop`'s cache.
Depends On: P10
Execution Plan: wiki/plans/tests-once-shared-ci-cache.plan.md

Included:
- the `verify` recipe and the full CI's jobs
- the cache's save rule, and stable job names

Excluded:
- dropping any test or check, coverage included

Proof:
- the plan's Done When

### P14 - The Search Eval Test Runs Against A Frozen Wiki

Status: Completed (develop)
Promise: `search::qmd_rs::tests::fixed_eval_queries_keep_expected_targets_in_top_two`
checks search quality against a frozen copy of the wiki, so a page added to the
live wiki cannot turn the fast check red.
Depends On: None
Execution Plan: wiki/plans/search-eval-test-frozen-wiki.plan.md

Included:
- the test and the fixture it reads

Excluded:
- changing how search ranks pages (P9 investigates that)

Proof:
- the plan's Done When

### P15 - Compact Search Honours Its Limit

Status: Completed (develop)
Promise: `llm_wiki_search` with `compact` set returns as many results as its
`limit` asks for, or says plainly how to get the rest, instead of a first page
of three that reads as the limit being ignored.
Depends On: None
Execution Plan: Not needed (an investigation first; any fix gets its own plan)

Included:
- the case met on 2026-10-06: `compact=true` with `limit=12` returned a page
  of 3 results (`page_size` 3), and `limit=5` gave pages of 3 in the
  coordinator's own searches the same day
- how `limit`, `page_size` and `offset` relate in compact mode, and what the
  reply says about further pages

Excluded:
- ranking (P9)

Proof:
- a comment on its issue giving the cause and the fix to make, each claim with
  the command that showed it

### P16 - Lexical Search Weights Titles And File Names, And Falls Back To Phrases

Status: Draft
Promise: A lexical search for a page's own title or file name finds that page
at the top, and a query whose words no single page holds all of still returns
the pages that hold its phrases.
Depends On: P9
Execution Plan: `wiki/plans/search-ranking-weights-and-phrase-fallback.plan.md`

Included:
- the fix the investigation of P9 measured (issue #7, comment of 2026-10-06):
  weight the file path and title columns in the lexical ranking (10, 10, 1
  put "Operation Manager" first), and fall back from the all-words query to
  each hyphenated name kept as a phrase, joined by OR, when it returns too few
- a regression test on a frozen wiki where the title's words are in more than
  half the pages

Excluded:
- semantic and hybrid search
- compact search's limit (P15)

Proof:
- the plan's Done When

### P17 - Compact Search Pages Follow The Limit

Status: Draft
Promise: `llm_wiki_search` in compact mode returns as many results as its
`limit` asks for by default, and says plainly when more exist.
Depends On: P15
Execution Plan: `wiki/plans/compact-search-pages-follow-the-limit.plan.md`
(its PR marked P15 Completed, the answer on #24 having landed)

Included:
- the fix the investigation of P15 named (issue #24, comment of 2026-10-06):
  the compact page size defaults to `limit` instead of the constant 3
  (`src/search/commands.rs`, `DEFAULT_COMPACT_SEARCH_PAGE_SIZE`), the reply
  says when more results exist, both documented in the MCP tool's schema and
  the spec, with a test

Excluded:
- ranking (P16)

Proof:
- the plan's Done When

### P18 - A Plan's Status Lands In The PR That Does The Work

Status: Completed (develop)
Promise: A plan's status changes in the PR that does its work: the worker sets
it Active with its Branch line in its first push and Completed before the PR
leaves draft, and the merge makes it true. The coordinator commits no status,
the log PR carries only the log (log PRs replaced by P24 on 2026-10-07), and `just branch-status` reads each branch's
plans from its own `origin/<branch>`.
Depends On: None
Execution Plan: Not needed (the rules change is wiki-only; the recipe change is
tooling, each its own issue)

Included:
- the owner's decision of 2026-10-06: "the status change should be part of the
  PR that contains the work that is suppose to achieve this status change, so
  no separate commit or PR"
- AGENTS.MD, both checklists, the two workflow decisions, the operations
  skills, the worker briefs' shared rules and a plan-status part for code,
  tooling and wiki workers
- `just branch-status` reading Branch lines from each branch's own ref

Excluded:
- statuses already on `develop`, which stay until their work merges

Proof:
- no page of the way of working tells the coordinator to commit a status, and
  every brief for code, tooling and wiki work carries the plan-status part
- `just branch-status` lists each pushed branch with the plan its own ref names

### P19 - The Operations Setup Ships With llm-wiki

Status: Active
Promise: Any project can set up the coordinated way of working from llm-wiki
itself: an `operation-manager` init pack (beside the existing `ops` pack;
named by the owner on 2026-10-06) holds the kit's files, and an
`operations_setup` MCP prompt holds the steps, so a host like Claude Code runs
it as a slash command (`/mcp__llm-wiki__operations_setup`) next to the wiki
prompts.
Depends On: P18
Execution Plan: `wiki/plans/operations-setup-in-llm-wiki.plan.md` (Active),
from the accepted proposal
`wiki/proposals/operations-setup-in-llm-wiki.proposal.md`

Included:
- the owner's decision of 2026-10-06 to propose it ("I think we should propose
  to include inside llm wiki; I think the pack and mcp make sense")
- a proposal built from the owner's global operations setup skill, which looks
  at a repository, asks the standard questions, writes the fitted kit
  uncommitted, tests it and has it blind-reviewed
- how it fits llm-wiki's no-shipped-skills posture
  (`wiki/decisions/skill-projection-template-engine.decision.md` is
  superseded) and the existing MCP prompts (`wiki_query`, `wiki_ingest`,
  `wiki_lint`, `wiki_research`, `wiki_init`)
- the owner accepting the proposal and answering its open choices (when to
  build, where the kit's AGENTS sections go, the pack's name): the first
  step, not the end of P19
- a marked block in `AGENTS.md` that init owns and refreshes, leaving the
  rest of the file to the project (the owner's answer of 2026-10-06), built
  first by P19's plan
- the pack, the prompt and its built-in resource, built by P19's plan

Excluded:
- building it before the proposal is accepted and the plan is written

Proof:
- on a fresh project, `llm-wiki init` and then the `operations_setup` prompt
  in an interactive Claude Code session write the fitted kit, which passes the
  setup's tests and comes back from its blind review with no P1 or P2
- a rerun of `llm-wiki init` leaves every kit file, and every byte outside
  init's block, as it was

### P20 - Search After Edits In A Worktree

Status: Completed (develop)
Promise: A worker that edits wiki pages in its worktree and then searches gets
answers from the pages as they are, or a plain instruction it can follow,
instead of results from the index built when the worktree was made.
Depends On: None
Execution Plan: Not needed (an investigation first; any fix gets its own plan)

Included:
- the case met twice on 2026-10-06: after editing pages in a fresh worker
  worktree, `llm_wiki_search` answered from the stale index with only a
  warning
- whether search should rebuild a small word-match index itself when it is
  stale, or say exactly which command to run, and what that costs

Excluded:
- the meaning-based index of the main checkout

Proof:
- a comment on its issue giving the cause and the fix to make, each claim with
  the command that showed it

### P21 - Search Says When Its Index Is Stale, And Rebuilds A Small One

Status: Draft
Promise: After pages change, search says plainly, in the CLI and in the MCP
reply, that its index is stale and the exact command that rebuilds it, and a
small word-match index rebuilds itself before answering when that is safe.
Depends On: P20
Execution Plan: `wiki/plans/search-stale-index-warning-and-rebuild.plan.md`
(its PR marked P20 Completed, the answer on #34 having landed)

Included:
- the fix the investigation of P20 named (issue #34, comment of 2026-10-06):
  the stale warning carries the exact `llm-wiki index --project <id>` command,
  set in the JSON reply's warnings list as well as its single warning field
- a self-rebuild of a word-match index when the lock is free and the cache is
  writable (about 0.65 s and 4.9 MB for a worktree's index), falling back to
  the warning when the lock is held or the cache is read-only

Excluded:
- rebuilding the main checkout's meaning-based index

Proof:
- the plan's Done When

### P22 - The Dependency Gate Covers Dev-Dependencies

Status: Completed (develop)
Promise: The strict gates' dependency check (`cargo deny check`) covers the
strict crates' dev-dependencies in all its checks, so a banned, duplicated,
unlicensed or advised-against crate pulled in only by tests cannot slip past
it, each shown by a recorded slip, and the gate says so. The one exception is
`getrandom` 0.3.4, let through beside 0.4.2 because both come from inside
proptest.
Depends On: PM1 (poman roadmap)
Execution Plan: `wiki/plans/dependency-gate-covers-dev-dependencies.plan.md`

Included:
- the leftover the blind review of PR #28 named (2026-10-06), read from `cargo
  deny list`, which does not show dev-dependencies. `cargo deny check`, which
  the gate runs, does walk them for bans, advisories and sources (PR #59's
  review), but not for duplicates or licences unless `deny.toml` turns on
  `multiple-versions-include-dev` and `include-dev` (PR #64's review); both
  keys, the one duplicate they show, and a slip recorded for each check

Excluded:
- llm-wiki's own modules (PM8)

Proof:
- the plan's Done When

### P23 - The Search Eval Runs In A Temporary Home

Status: Draft
Promise: The hand-run search eval runs on any machine through one `just`
recipe that registers and indexes its project in a temporary home, without
touching the machine's real llm-wiki registry.
Depends On: None
Execution Plan: `wiki/plans/search-eval-in-a-temporary-home.plan.md`

Included:
- the gap met in poman's PM1 and PM2 (2026-10-06 and 2026-10-07): the ignored
  `tests/natural_language_search_eval.rs` needs the project id
  `llm-wiki-framework-semantic-search` registered with its managed models, and
  nothing says how
- a recipe, and the eval page naming it

Excluded:
- what the eval measures, and its floors

Proof:
- the recipe runs the eval with that project unregistered on the machine, and
  the real registry is unchanged afterwards

### P24 - Each PR Carries Its Own Log Entry And Backlog Lines

Status: Completed (develop)
Promise: Each PR into `develop` carries its own log entry and any waiting
backlog roadmap entries in one bookkeeping commit, which the coordinator adds
just before the PR goes ready, or just after the owner's PASS when it touches
only `wiki/log.md` and roadmap entries. Log PRs stop, and no roadmap entry is
committed straight to `develop`.
Depends On: P18
Execution Plan: Not needed (the rules change is wiki-only, with the operations
skills)

Included:
- the owner's decision of 2026-10-07: "just before the merge you add a commit
  with the backlog and all the llm wiki logs", and "it's ok just after the pass
  only if it touches logs/backlog and no code"
- AGENTS.MD, the operation manager checklist, the worker briefs checklist, the
  review-surface and work-in-flight decisions, the operations skills and the
  workers' base texts (issue #51)

Excluded:
- `wiki/log.md` entries and past records, which stay as written
- the blind review and the owner's verdict rules themselves

Proof:
- no page of the way of working tells the coordinator to open a log PR, except
  as history, and the landing steps describe the bookkeeping commit and both
  moments
