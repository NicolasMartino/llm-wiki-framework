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
`<!-- SECTION:SEARCH -->` is retired in the same change set. Each new project
gains a `.llm_wiki/init.toml` recording the choices for a future `upgrade`
command.
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
- Initial pack catalog: `api`, `frontend`, `library`, `ml`, `data`, `ops`,
  `ops-lite`, `security`, `research`, `qmd-rs-scale`.
- Initial blueprint catalog: `generic`, `web-product`, `library-sdk`,
  `ml-research`, `ops-infra`, `security`, `research`, `custom`.
- Two-step interactive flow with `inquire`: blueprint `Select`, then pack
  `MultiSelect` with the blueprint's defaults pre-checked.
- Non-interactive flag mapping: `--blueprint <name>` and repeatable
  `--pack <name>`; the old `--type` and `--scale` init flags are retired with
  the static profile model.
- Per-project `.llm_wiki/` folder with `init.toml` recording chosen blueprint,
  resolved pack list, and framework version.

Excluded:
- Skill projection migration onto the same engine (sibling proposal:
  `wiki/proposals/skills-template-engine.proposal.md`).
- An `upgrade` command. `.llm_wiki/init.toml` is written for that future, not
  this one.
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
