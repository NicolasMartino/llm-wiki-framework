# Wiki Log

## [2026-04-23] ingest | Karpathy LLM Wiki research

Compiled raw/research/llm-wiki-pattern-research.md into wiki reference page.
Source covers: original Karpathy gist, v2 extensions, Fulkerson production
deployment, wiki vs RAG tradeoffs.
Pages created: wiki/references/llm-wiki-pattern.reference.md

## [2026-04-23] ingest | Legacy project guidelines

Compiled raw/legacy/legacy-project-guidelines.md into framework decisions
and spec. Extracted key architectural choices as decision records. Merged
typed document system with LLM Wiki pattern into documentation model spec.
Pages created: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/specs/documentation-model.spec.md

## [2026-04-23] create | Framework V1 roadmap

Created roadmap for framework development with 7 deliverables: bootstrap,
ingest cycle, lint operation, query produces knowledge, spawn new project,
scale test, self-replicating framework. D1 (bootstrap) marked completed.
Pages created: wiki/roadmaps/framework-v1.roadmap.md

## [2026-04-23] create | Wiki index and log

Created wiki/index.md (master catalog) and wiki/log.md (this file).
Initial index catalogs 6 wiki pages across 4 document types.
Pages created: wiki/index.md, wiki/log.md

## [2026-04-23] ingest | QMD search engine and NiharShrotri/llm-wiki implementation

Ingested two new raw sources:
- raw/research/qmd-search-engine.md (Tobi Lutke's on-device markdown search)
- raw/research/niharshrotri-llm-wiki-implementation.md (full LLM Wiki implementation)

Key findings:
- QMD solves our scale ceiling (>100 pages) with local hybrid search via MCP
- 3-pass ingest pipeline (extraction → drafting → bookkeeping) improves quality
- Source audit pages provide provenance tracking
- Our typed-document approach (spec/decision/proposal) is differentiated from
  the entity/concept/synthesis model — both valid, ours better for software projects

Pages created: wiki/references/qmd-search-engine.reference.md, wiki/references/niharshrotri-llm-wiki.reference.md
Pages updated: wiki/specs/documentation-model.spec.md (navigation, limitations, sources), wiki/references/llm-wiki-pattern.reference.md (cross-references), wiki/index.md

## [2026-04-23] create | Init project skill and template

Created /init-project skill for spawning new projects from the framework.
Renamed project_guidelines.md to project_guidelines.template.md with
conditional section markers (ML_AI, QMD). Skill asks 6 questions to
determine project profile, generates tailored guidelines and CLAUDE.md,
scaffolds folder structure, optionally ingests initial sources. Supports
create and update modes.
Pages created: .claude/skills/init-project.md, wiki/specs/init-project-skill.spec.md
Files modified: project_guidelines.template.md (renamed, added conditional markers)
Pages updated: wiki/index.md

## [2026-04-23] ingest | LLM Wiki ecosystem survey

Ingested raw/research/llm-wiki-ecosystem-survey.md — a GitHub-wide survey
of 30+ implementations of Karpathy's LLM Wiki pattern, three weeks after
publication.

Key findings organized by architectural innovation, not by repo:
- Five delivery models: agent skill, CLI, desktop app, web app, Obsidian plugin
- Agent-skill form dominates (lowest friction, no infra needed)
- Notable innovations: two-phase compilation (order-independent ingest),
  L1/L2 cache architecture, parallel multi-agent research, automated feed
  monitoring, write-back from queries, multimodal ingest, anti-repetition
  memory, codebase-as-source, wiki-as-training-data
- Top adoption candidates for our framework: write-back from queries,
  codebase as source, L1/L2 cache formalization, two-phase compilation

Pages created: wiki/references/llm-wiki-ecosystem.reference.md
Pages updated: wiki/references/llm-wiki-pattern.reference.md (cross-ref), wiki/references/niharshrotri-llm-wiki.reference.md (cross-ref), wiki/index.md

## [2026-04-23] create | Knowledge query skill

Created /knowledge-query skill for querying the project wiki with citations
and optional save-back. Single-project scope. Reads index.md to orient,
identifies relevant pages, synthesizes answer with citations, flags gaps
and contradictions, offers to save durable answers as new wiki pages.
Optionally uses QMD if available. Symlinked to global skills.
Pages created: .claude/skills/knowledge-query/SKILL.md, wiki/specs/knowledge-query-skill.spec.md
Pages updated: wiki/index.md

## [2026-04-23] create | Knowledge ingest skill

Created /knowledge-ingest skill for processing raw sources into wiki pages.
Three-phase pipeline: extraction, page drafting, bookkeeping. Handles merge
vs create, contradiction detection, provenance tracking, auto-discovery of
unprocessed files. Supports markdown, text, PDF, images, URLs, transcripts,
code. Symlinked to global skills.
Pages created: .claude/skills/knowledge-ingest/SKILL.md, wiki/specs/knowledge-ingest-skill.spec.md
Pages updated: wiki/index.md

## [2026-04-23] create | Codex skill translations

Translated the three Claude framework skills into Codex skill folders under
`.codex/skills/` and exposed them globally through symlinks in
`~/.codex/skills/`.
Pages created: .codex/skills/init-project/SKILL.md, .codex/skills/knowledge-query/SKILL.md, .codex/skills/knowledge-ingest/SKILL.md, wiki/decisions/project-local-codex-skills.decision.md
Pages updated: wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/index.md

## [2026-04-23] create | Knowledge namespace and research skill

Added a Codex-only `/knowledge` dispatcher skill so framework operations share
one slash-style command surface. Added `knowledge-research` as the pre-ingest
acquisition step for gathering source material into `raw/` from local paths,
explicit URLs, one site, or broader web search.

Refined `knowledge-ingest` to operate on explicit raw sources only, with web
or site discovery routed through research first. Updated the framework docs to
record the `/knowledge` namespace and the new raw acquisition workflow.

Pages created: .codex/skills/knowledge/SKILL.md, .codex/skills/knowledge/agents/openai.yaml, .codex/skills/knowledge-research/SKILL.md, .codex/skills/knowledge-research/agents/openai.yaml, wiki/decisions/knowledge-command-namespace.decision.md, wiki/specs/knowledge-research-skill.spec.md
Pages updated: .codex/skills/init-project/SKILL.md, .codex/skills/knowledge-query/SKILL.md, .codex/skills/knowledge-ingest/SKILL.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/documentation-model.spec.md, wiki/decisions/project-local-codex-skills.decision.md, wiki/index.md

## [2026-04-23] correct | Knowledge invocation surface

Corrected the Codex command surface after discovering that custom `/...`
strings are intercepted by the product and produce an unrecognized command
error before skills can run. Updated the framework to use explicit skill
invocation with `$knowledge`, `$knowledge-research`, and existing direct skill
names instead of claiming support for custom slash commands.

Pages updated: .codex/skills/knowledge/SKILL.md, .codex/skills/knowledge/agents/openai.yaml, .codex/skills/knowledge-research/SKILL.md, .codex/skills/knowledge-research/agents/openai.yaml, .codex/skills/init-project/SKILL.md, .codex/skills/knowledge-query/SKILL.md, .codex/skills/knowledge-ingest/SKILL.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/index.md

## [2026-04-23] create | Knowledge lint skill

Promoted lint into a dedicated Codex skill so `$knowledge lint` routes to a
real skill instead of an inline dispatcher note. Added `knowledge-lint`
metadata, updated the dispatcher to call it directly, and documented the new
skill in the framework wiki.

Pages created: .codex/skills/knowledge-lint/SKILL.md, .codex/skills/knowledge-lint/agents/openai.yaml, wiki/specs/knowledge-lint-skill.spec.md
Pages updated: .codex/skills/knowledge/SKILL.md, wiki/specs/documentation-model.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/decisions/project-local-codex-skills.decision.md, wiki/index.md

## [2026-04-23] lint | wiki consistency pass

Scanned the wiki for contradictions, stale claims, orphan pages, and missing
cross-references.
Issues found: orphaned reference page `wiki/references/llm-wiki-ecosystem.reference.md` missing from `wiki/index.md`; stale claim that lint had not yet been exercised; stale namespace language claiming `/init-project` remained an acceptable alias.
Pages updated: wiki/index.md, wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-lint-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/decisions/project-local-codex-skills.decision.md, wiki/specs/init-project-skill.spec.md, wiki/roadmaps/framework-v1.roadmap.md
Outstanding questions: none

## [2026-04-23] ingest | Web sources on three-phase ingest pipeline

Saved two primary web source snapshots under `raw/web/`:
- `raw/web/gist.github.com/2026-04-23-karpathy-llm-wiki.md`
- `raw/web/github.com/2026-04-23-niharshrotri-llm-wiki-readme.md`

Synthesized a new reference page that explains the three-phase ingest pipeline
in detail using web sources only. The page distinguishes the conceptual ingest
responsibilities in Karpathy's gist from the explicit three-pass implementation
described in NiharShrotri's README.

Pages created: wiki/references/three-phase-ingest-pipeline.reference.md
Pages updated: wiki/index.md

## [2026-05-02] create | Knowledge intake command proposal

Recorded a proposal for a possible `knowledge-intake` skill and
`$knowledge intake` namespace entry. The proposal keeps the accepted boundary
between source acquisition (`knowledge-research`) and wiki compilation
(`knowledge-ingest`), and defines intake as a user-facing orchestration layer
above those two operations.

Pages created: wiki/proposals/knowledge-intake-command.proposal.md
Pages updated: wiki/index.md

## [2026-05-02] update | Knowledge research intake direction

Refined the earlier intake proposal after clarifying the intended behavior.
The recommendation now is to improve `knowledge-research` itself into a guided
research-intake flow that asks what to research, gathers material from local
and web sources, saves coherent bundles under `raw/research/`, and leaves
`knowledge-ingest` as a separate explicit step.

Pages updated: wiki/proposals/knowledge-intake-command.proposal.md, wiki/index.md

## [2026-05-02] update | Per-source research summaries

Extended the knowledge research intake proposal to require one summary file
per collected source inside each research bundle. This makes the research
output more legible and creates a cleaner handoff into later ingest work while
preserving raw-source provenance.

Pages updated: wiki/proposals/knowledge-intake-command.proposal.md

## [2026-05-02] update | Bundle-level research summary

Revised the proposal again to use one `research-summary.md` per research
bundle instead of one summary per source. The bundle now centers on three
artifacts: raw source files, a manifest for provenance and inventory, and a
single synthesis file for the overall research run.

Pages updated: wiki/proposals/knowledge-intake-command.proposal.md

## [2026-05-02] create | Knowledge research intake implementation plan

Created a plan for implementing the proposed `knowledge-research` upgrade.
The plan covers spec and skill updates, bundle structure under
`raw/research/`, dispatcher wording review, one exercised research run, and
the verification needed before treating the new behavior as accepted truth.

Pages created: wiki/plans/knowledge-research-intake.plan.md
Pages updated: wiki/index.md

## [2026-05-02] update | Claude research skill added to plan scope

Expanded the implementation plan after confirming that `knowledge-research`
exists only on the Codex side today. The plan now explicitly includes creating
the missing Claude `knowledge-research` skill and aligning its invocation and
workflow shape with the existing Claude `knowledge-ingest` skill.

Pages updated: wiki/plans/knowledge-research-intake.plan.md

## [2026-05-02] create | Knowledge research skill implementation

Implemented the guided research-intake workflow across the framework surfaces.
Updated the research spec, rewrote the Codex `knowledge-research` skill around
research bundles, created the missing Claude `knowledge-research` skill, and
refined the Codex `$knowledge` dispatcher wording so research is clearly the
pre-ingest intake path.

Exercised the workflow with a local path-based proof run and created:
- `raw/research/2026-05-02-knowledge-research-intake-proof/manifest.md`
- `raw/research/2026-05-02-knowledge-research-intake-proof/research-summary.md`
- `raw/research/2026-05-02-knowledge-research-intake-proof/sources/...`

Pages updated: wiki/specs/knowledge-research-skill.spec.md, wiki/proposals/knowledge-intake-command.proposal.md, wiki/plans/knowledge-research-intake.plan.md, wiki/index.md, wiki/log.md
Files created: .claude/skills/knowledge-research/SKILL.md, raw/research/2026-05-02-knowledge-research-intake-proof/manifest.md, raw/research/2026-05-02-knowledge-research-intake-proof/research-summary.md
Files updated: .codex/skills/knowledge-research/SKILL.md, .codex/skills/knowledge/SKILL.md

## [2026-05-02] update | Research spec verification correction

Corrected the `knowledge-research` spec after verification to remove a Claude
global symlink claim that was not actually present on disk. The spec now only
asserts the repo-local Claude skill, the existing Codex global symlink, and
the proof bundle artifact that were verified directly.

Pages updated: wiki/specs/knowledge-research-skill.spec.md, wiki/log.md

## [2026-05-06] update | Rename path and skill consistency repair

Fixed framework rename drift after the project moved from
`software_project_management` to `llm_wiki_framework`.

Key changes:
- Updated Codex and Claude `init-project` skills to resolve the framework root
  from the skill file location instead of a hardcoded absolute path.
- Added a durable decision for framework path resolution.
- Rebuilt broken global framework symlinks for Codex and Claude to point at
  this repository.
- Made this framework repo consistently treat `project_guidelines.template.md`
  as its canonical reusable schema while generated projects still receive
  `project_guidelines.md`.
- Aligned Claude ingest with the research-first URL/site/web acquisition
  boundary.
- Promoted the accepted research-intake choice into a decision and archived
  the old proposal.

Pages created: wiki/decisions/framework-path-resolution.decision.md, wiki/decisions/knowledge-research-intake.decision.md
Pages moved: wiki/proposals/knowledge-intake-command.proposal.md -> wiki/archive/knowledge-intake-command.proposal.md
Pages updated: wiki/index.md, wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/decisions/project-local-codex-skills.decision.md, wiki/decisions/typed-documents.decision.md, wiki/roadmaps/framework-v1.roadmap.md, wiki/plans/knowledge-research-intake.plan.md, wiki/log.md
Files updated: .codex/skills/init-project/SKILL.md, .claude/skills/init-project/SKILL.md, .claude/skills/knowledge-ingest/SKILL.md, .claude/skills/knowledge-research/SKILL.md, CLAUDE.md, AGENTS.MD
Contradictions found: stale path and canonical-file claims resolved

## [2026-05-06] update | Single-source skill consolidation

Implemented the skill consolidation plan.

Key changes:
- Created canonical skill sources under `skills/`.
- Added `legacy skill render script` to render `.claude/skills/` and `.codex/skills/`
  outputs from canonical sources.
- Moved the root `plan.md` into `wiki/plans/single-source-skills.plan.md`
  and marked it Active pending runtime smoke tests.
- Added the accepted single-source skills decision.
- Generated Claude and Codex variants for shared skills.
- Added Claude `knowledge-lint` and its global symlink.
- Kept the Codex-only `$knowledge` dispatcher generated only for Codex.
- Updated skill specs to point at canonical `skills/` sources and generated
  runtime outputs.
- Superseded the older project-local Codex-only skills decision.
- Rebuilt canonical skill sources from the full pre-consolidation runtime
  baselines before rendering, avoiding instruction loss from shortened drafts.

Pages created: wiki/decisions/single-source-skills.decision.md
Pages moved: plan.md -> wiki/plans/single-source-skills.plan.md
Pages updated: wiki/index.md, wiki/log.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md, wiki/decisions/project-local-codex-skills.decision.md
Files created: skills/README.md, legacy skill render script, skills/init-project/SKILL.md, skills/knowledge-ingest/SKILL.md, skills/knowledge-query/SKILL.md, skills/knowledge-research/SKILL.md, skills/knowledge-lint/SKILL.md, skills/knowledge/SKILL.md, skills/*/codex/openai.yaml, .claude/skills/knowledge-lint/SKILL.md
Files regenerated: .claude/skills/*/SKILL.md, .codex/skills/*/SKILL.md, .codex/skills/*/agents/openai.yaml
Verification: `bash legacy skill render script` is idempotent; no skill body contains an absolute `/Users/...` path; URL routing is research-first in both runtimes; every indexed skill has a canonical source under `skills/`. Runtime smoke tests in Claude Code and Codex remain pending.

## [2026-05-06] create | LLM Wiki framework binary proposal

Proposed packaging the framework as a single Rust binary (`llm-wiki`) with
embedded canonical skill content, `clap`-driven subcommands (install, build,
init, status, doctor, uninstall), and direct global writes to
`~/.claude/skills/` and `~/.codex/skills/` instead of symlinks. Folds in the
agent-vs-binary split for `init-project` (agent owns intake, binary owns
file generation), the `--env local|global` target flag for self-dogfooding,
`cargo-dist` multi-arch distribution, and a `git`-style backward-compat
versioning model (no per-project pinning).

Rationale: closes review.md §9.2 (broken global symlinks) by construction,
eliminates per-runtime skill drift via a typed projector with `insta`
snapshot tests, and is the operational form of D7 (self-replicating
framework) — `llm-wiki init <path>` becomes the single bootstrap operation.

If accepted, this proposal supersedes the legacy shell renderer in
`wiki/plans/single-source-skills.plan.md` and the symlink-based installation
model recorded in `wiki/decisions/project-local-codex-skills.decision.md`
and `wiki/decisions/framework-path-resolution.decision.md`.

Pages created: wiki/proposals/llm-wiki-binary.proposal.md
Pages updated: wiki/index.md

## [2026-05-06] update | Revise llm-wiki binary proposal per external review

Applied reviewer-blocking fixes and high-value improvements to
`wiki/proposals/llm-wiki-binary.proposal.md`:

- Added Roadmap Position section: proposal now explicitly recommends adding
  D8 to the roadmap rather than reframing D7. Resolves the contradiction
  with `framework-v1.roadmap.md:246` "GUI or CLI tooling" exclusion.
- Added Install State And Manifest section: defines manifest schema
  (`~/.local/share/llm-wiki/manifest.json`), collision policy table
  (path-absent, manifest-owned/match, manifest-owned/drift, user-authored,
  symlink), and `--force` backup-and-overwrite behavior.
- Narrowed `init` to Create mode only; Update mode marked out of scope and
  remains agent-owned. Binary refuses `init` against a non-empty `wiki/`.
- Removed staged-delivery row from risk table (V1 ships only install+build,
  init in V1.1) which contradicted Acceptance Criterion requiring init
  golden-file fixtures. `init` is now non-negotiable for V1.
- Replaced placeholder canonical-schema example with a real schema:
  required/optional frontmatter fields with types, fixed body section
  shape, projection rules, schema snapshot test commitment.
- Defined "wiki shape" precisely (document type suffixes, metadata block
  fields, folder layout, status vocabulary, three-layer architecture
  invariant) and committed to compat fixtures under `tests/fixtures/wikis/`.
- Replaced the muddy E2E test claim ("claude --version discovers skills")
  with concrete post-install file-and-manifest verification. Manual smoke
  testing documented but not automated; no runtime exposes a stable
  skill-listing API.
- Cited measured line-loss deltas (init-project Claude 369 -> 155;
  knowledge-research Codex 200 -> 124) instead of "~400 lines" handwave.
- Reworded "no network dependency" to clarify it applies to operations
  after install, not to distribution acquisition.
- Softened "per-runtime skill drift eliminated" claim: projector
  eliminates cross-runtime drift on the same canonical content, not
  authoring drift in the canonical itself; golden-file tests are the
  discipline against the second class.
- Added two new risk rows: user-authored skill collision; manifest-vs-FS
  desync. Mitigations documented (refuse default, --force backup, atomic
  manifest writes via temp-file + rename).
- Acceptance Criteria reorganized into Functional / Test / Distribution /
  Roadmap groupings; expanded from 12 items to 16; manifest, collision
  matrix, compat fixture, and roadmap update added as explicit gates.

Pages updated: wiki/proposals/llm-wiki-binary.proposal.md, wiki/log.md

## [2026-05-06] update | Address second-round review of llm-wiki binary proposal

Fixed five reviewer findings on `wiki/proposals/llm-wiki-binary.proposal.md`:

1. Compat fixture / agent-driven contradiction: rewrote the fixture
   contract so it asserts only what the binary can actually verify
   (parseability, metadata extraction, template compatibility, skill
   availability). Operations-level compatibility (ingest/query/lint
   succeed) moved to a separate agent-driven smoke-test checklist at
   `wiki/checklists/v1-fixture-smoke.checklist.md` (to be created when
   the fixture lands), run manually as part of release gating. Dropped
   the round-trip identity gate because the binary does not write into
   `wiki/` content.
2. Bad citations corrected: `plan.md` -> `wiki/plans/single-source-skills.plan.md`
   (Sources, Implementation Outline x2). Imaginary `review.md §11`
   replaced with the real `wiki/log.md` 2026-05-06 entry plus `review.md
   §10` for the audit wording. Sources field updated.
3. D7 proof overcompression: reworded "the proof becomes" to "the setup
   step becomes" and added explicit text noting that `init` produces
   scaffolding, not a navigable self-managing wiki — the latter requires
   the agent to run end-to-end against the scaffolded project.
4. Local-dev manifest collision edge case eliminated by removing
   `install --env local` entirely. New surface: `install` is global-only
   and manifest-tracked; `build [--target] [--out]` covers all local
   rendering (snapshot tests, self-dogfooding) without a manifest.
   Self-dogfooding now uses `llm-wiki build --out .`. Updated the
   subcommand surface, the install/build/uninstall descriptions, the
   self-referential dev workflow section, the test sections, the
   non-goals, the revisit-when triggers, and the implementation outline.
5. Fixed `<path>.bak` collision risk: switched to timestamped backup
   suffix `<path>.bak.<UTC-ISO8601>` (e.g. `<path>.bak.20260506T123456Z`),
   so repeated `--force` runs never overwrite an earlier backup. Updated
   the manifest collision-policy table, the risk table, and the
   acceptance criteria.

Pages updated: wiki/proposals/llm-wiki-binary.proposal.md, wiki/log.md

## [2026-05-06] promote | Accept llm-wiki binary proposal; add D8; supersede predecessors

Formal acceptance pass per the framework promotion rule
(`project_guidelines.template.md:344-356`). Status flips, decision page,
roadmap addition, and supersession bookkeeping for the binary proposal.

Created:
- `wiki/decisions/llm-wiki-binary-distribution.decision.md` — records the
  choice, six rejected alternatives (status quo, legacy shell renderer +
  conditional blocks, Python script, per-project install, MCP, agent-only
  scaffolding), consequences, and the bounded backward-compatibility
  promise.

Status changes:
- `wiki/proposals/llm-wiki-binary.proposal.md`: Proposed -> Accepted; added
  Promoted To pointing at the new decision and roadmap D8.
- `wiki/decisions/single-source-skills.decision.md`: Accepted -> Superseded
  (Superseded By: binary distribution). Bash renderer + conditional-block
  approach is replaced by the binary's typed projector.
- `wiki/decisions/framework-path-resolution.decision.md`: Accepted ->
  Superseded. Binary embeds canonical content, eliminating the framework-
  root path-resolution problem entirely.
- `wiki/decisions/project-local-codex-skills.decision.md`: already
  Superseded by single-source-skills; now transitively superseded by binary
  distribution. Chronological chain preserved.
- `wiki/plans/single-source-skills.plan.md`: Active -> Superseded. Repo-
  local consolidation work landed but the smoke-test gates from §6 will be
  discharged by the D8 implementation rather than by completing this plan.

Roadmap:
- `wiki/roadmaps/framework-v1.roadmap.md`: D8 added with full Promise,
  Included, Excluded, Proof, Promotion Target, Unlocks, Closes sections.
  D7 unchanged; D7's Unlocks updated to note D8 as the faster proof path.

Index:
- `wiki/index.md`: Stage line updated to include D8 (Draft); proposal
  status flipped to Accepted; new decision listed; three predecessor
  decisions and one plan marked Superseded with reasons.

No skill files, canonical sources, or repo-local skill outputs were
modified by this pass. The legacy shell renderer continues to operate the repo's
self-dogfooding workflow until D8 ships.

Next step: write `wiki/plans/llm-wiki-binary.plan.md` to execute D8.

Pages created: wiki/decisions/llm-wiki-binary-distribution.decision.md
Pages updated: wiki/proposals/llm-wiki-binary.proposal.md, wiki/decisions/single-source-skills.decision.md, wiki/decisions/framework-path-resolution.decision.md, wiki/plans/single-source-skills.plan.md, wiki/roadmaps/framework-v1.roadmap.md, wiki/index.md, wiki/log.md

## [2026-05-06] update | Normalize acceptance pass per third-round review

Reviewer caught five issues in the prior acceptance pass. All addressed.

**Premature supersession (high).** Predecessor decisions were marked
`Superseded` while the binary that supersedes them does not yet exist.
The new decision's own consequences section even said the legacy shell renderer
remains in operation. Reverted statuses to reflect what is actually
running:

- `wiki/decisions/single-source-skills.decision.md`: Superseded -> Accepted.
  Added forward-pointing `Successor field:` field.
- `wiki/decisions/framework-path-resolution.decision.md`: Superseded ->
  Accepted. Added `Successor field:` field.
- `wiki/plans/single-source-skills.plan.md`: Superseded -> Active. Added
  `Successor field:` field. Smoke-test gates from §6 will be folded into D8
  rather than discharged separately.
- `wiki/decisions/project-local-codex-skills.decision.md`: unchanged
  (already Superseded by single-source; chain stays chronologically
  accurate; will collapse forward when D8 ships).
- Index lines updated to match the reverted statuses with explicit
  "in operation now, will be superseded by binary distribution when D8
  ships" annotations.

The binary-distribution decision's metadata changed `Supersedes:` to
`deferred supersession field:`. The `Consequences` section was split
into **Immediately** (forward-pointing annotations only) and **On D8
completion (planned, not yet effected)** (the actual flips, file
removals, and spec updates). This makes the deferred-supersession model
explicit instead of implied.

**Invalid AC #11 (high).** Compat-fixture acceptance criterion still
required "framework operations succeed" and "round-trip is byte-identical"
even though the proposal's Compat Fixtures section had already dropped
both claims (binary does not own ingest/query/lint; binary does not
write into `wiki/` content). Rewrote AC #11 to match: parseability,
metadata extraction against hand-coded structs, template compatibility
via `init` re-render, skill-availability cross-check. Operations-level
compatibility is the agent's checklist responsibility.

**Install upgrade semantics underspecified (high).** The collision-policy
table used a two-way comparison (current file vs manifest) that
conflated "no-op" with "upgrade." Rewrote the table as a three-way hash
comparison: `current` (file on disk) vs `manifest` (last installed) vs
`bundled` (this binary). Eight cases now distinguished:
- absent + no manifest entry = fresh install
- absent + manifest entry = recovery
- match + match = no-op
- match + differ = upgrade (file untouched, framework moved on)
- differ + match = user edited the framework file (refuse default)
- differ + differ = user edited AND framework moved on
- non-manifest path = collision (refuse default)
- symlink = pre-binary residue (refuse default; suggest doctor)

**`init` non-empty-dir contradiction (medium).** Earlier wording
("refuses against a non-empty `wiki/`") contradicted the existing
`init-project` spec's IS_EXISTING profile. Rewrote: the binary's `init`
collision check is on framework artifacts only (`wiki/`, `raw/`,
`CLAUDE.md`, `project_guidelines.md`), not on directory emptiness.
IS_EXISTING happy path preserved: existing source code, configs, and
tests are left untouched. D8 roadmap entry updated to match.

**"Seven deliverables" stale text (low).** Index line updated to "Eight
deliverables: bootstrap through self-replicating framework, plus D8
distribution tooling."

**Untracked decision file note.** `wiki/decisions/llm-wiki-binary-
distribution.decision.md` remains untracked in git. Not added in this
pass — staging and commit are the user's call. Flagging here so it is
not lost in the next commit.

Knowledge base now coherent: the operating model is the legacy shell renderer +
canonical `skills/` source; the binary is an accepted future direction
recorded as D8; predecessor decisions show their forward path without
misrepresenting current state.

Next step (unchanged): write `wiki/plans/llm-wiki-binary.plan.md` to
execute D8.

Pages updated: wiki/proposals/llm-wiki-binary.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/single-source-skills.decision.md, wiki/decisions/framework-path-resolution.decision.md, wiki/plans/single-source-skills.plan.md, wiki/roadmaps/framework-v1.roadmap.md, wiki/index.md, wiki/log.md

## [2026-05-06] create | D8 implementation plan for the llm-wiki binary

Wrote `wiki/plans/llm-wiki-binary.plan.md` to execute the accepted D8
deliverable. Plan is `Status: Draft` until stage 5.1 begins.

Structure:
- 13 sequenced stages (5.1 scaffold + CI -> 5.13 cleanup and supersession
  bookkeeping)
- Crate lives at `tools/llm-wiki/` with explicit module layout (schema,
  projector, manifest, install/build/init/status/doctor/uninstall)
- 16 verification gates mapped one-to-one to the proposal's acceptance
  criteria
- Tightest-first ordering inside the implementation phase: schema ->
  projector -> canonical migration (the bug-prevention spine that would
  have caught the legacy shell renderer's content-loss class), then manifest,
  init, diagnostics
- Stage 5.4 (canonical migration) is explicitly a content-completeness
  audit with PR review of every diff between today's rendered outputs
  and the new projector output. The bug we just hit is the headline
  reason this stage exists.
- Stage 5.13 collapses the deferred-supersession wording recorded
  earlier today into effective supersession when D8 ships
- Implementation risks separated from design risks (design risks live
  in the proposal); seven implementation-specific risks documented
- Four open implementation questions flagged as sequencing details, not
  design forks: crate name for crates.io, workspace vs standalone
  Cargo.toml, dirs crate vs hand-rolled HOME resolution, git2 for
  .gitignore awareness in IS_EXISTING profile

Index updated to list the new plan.

Pages created: wiki/plans/llm-wiki-binary.plan.md
Pages updated: wiki/index.md, wiki/log.md

## [2026-05-06] update | Revise D8 implementation plan per fourth-round review

Fixed five reviewer findings on `wiki/plans/llm-wiki-binary.plan.md`:

1. **Release-before-skill-migration ordering bug (high).** Earlier draft
   tagged v0.1.0 in stage 5.11, then migrated `init-project` skill in
   5.12. Since canonical skills are embedded at compile time (5.5), v0.1.0
   would have shipped with the legacy markdown-driven skill, not the
   wrapper. Reordered: 5.11 is now the wrapper migration (must precede
   release); 5.12 is `cargo-dist` and v0.1.0; 5.13 is cleanup. Added an
   explicit "Critical reordering note" in §10 documenting the constraint.

2. **Stage 5.3 unverifiable as written (high).** Earlier draft said
   stage-5.3 golden tests use canonicals "after stage 5.4," contradicting
   the per-stage checkpoint commit rule. Split: 5.3 now uses hand-coded
   `SkillDoc` values and tiny inline canonicals (decoupled from real skill
   content) to verify projector logic; the "snapshot tests against real
   skills" gate moved to 5.4 where the migrated canonicals exist.

3. **`build.rs` cannot depend on the package being built (medium).** A
   Rust build script cannot use items from the crate it is building.
   Restructured into a workspace with two crates: `crates/llm-wiki-schema/`
   (pure library — parsing + projection) and `tools/llm-wiki/` (binary).
   The binary depends on `llm-wiki-schema` as both a normal dep and a
   `[build-dependencies]` entry, so the binary's `build.rs` can validate
   embedded canonicals at compile time. Added a CI negative test asserting
   that a malformed canonical fails the build.

4. **Initial-source handling agent/binary split (medium).** Earlier draft
   accepted `--initial-sources` without specifying behavior. Clarified:
   the binary copies sources into `<path>/raw/initial/` deterministically
   and writes a manifest; it does not invoke ingest. The agent's wrapper
   skill handles the conversational handoff to `knowledge-ingest`. This
   preserves the principle: deterministic file ops in the binary,
   LLM-driven judgment in the agent.

5. **Fixture path inconsistency (medium).** Earlier draft alternated
   between `tests/fixtures/wikis/v1/` and `tools/llm-wiki/tests/fixtures/
   wikis/v1/`. Standardized on the qualified path everywhere, since
   fixtures live next to the tests that use them per Rust convention.

6. **Crate naming open question resolved (low).** Specified explicitly:
   package name `llm-wiki-framework` (matches `cargo install llm-wiki-
   framework` in the distribution gate); binary name `llm-wiki` (`[[bin]]
   name = "llm-wiki"` in Cargo.toml). Open question struck through and
   marked resolved.

Also updated the §10 sequencing diagram to reflect new stage numbers,
fixed two stragglers in §11 (risk references to old stage numbers), and
fixed one straggler in stage 5.8 (cross-reference to wrapper migration
updated from 5.12 to 5.11).

Pages updated: wiki/plans/llm-wiki-binary.plan.md, wiki/log.md

## [2026-05-06] update | D8 plan: nuclear no-legacy discipline

User directive: D8 ships clean-slate; no dead code, no old features
lingering, no main-branch state where legacy shell renderer and binary coexist.

Added §2a "No-Legacy Discipline (Load-Bearing)" to
`wiki/plans/llm-wiki-binary.plan.md`. Seven principles, non-negotiable:

1. Single source of truth at any moment. No coexistence of legacy shell renderer
   and binary on main.
2. Dead code is a release blocker. No legacy parsing paths in the binary.
3. Old prose is deleted, not migrated. The current `init-project` skill
   prose is `git rm`-ed when the wrapper lands.
4. Spec rewrites, not spec patches. The binary-relevant specs are
   rewritten from scratch.
5. Superseded predecessors move to `wiki/archive/`. Per the framework's
   own rule (project_guidelines.template.md:327), they leave the active
   index.
6. Open questions resolve before stage 5.1. No "we'll figure it out
   during implementation."
7. Backward compatibility is bounded — external project wikis preserved,
   internal bash-renderer canonical format is dead code.

Stage updates to enforce the discipline:

- **5.4** (canonical migration): explicit "no `<!-- TAG -->` markers
  anywhere," CI grep gate added that fails the build on any reintroduction.
  Removed the "if approached cleanly" hedge.
- **5.11** (init-project wrapper): explicit `git rm` of the old prose;
  fresh canonical under 60 lines; verification asserts `git log -p` shows
  delete-then-rewrite, not an accumulating patch.
- **5.13** (cleanup): reframed as a single-merge operation. Predecessors
  archived (`git mv` to `wiki/archive/`), not just status-flipped.
  Specs rewritten (delete-then-rewrite), not edited. Deferred-supersession
  framing dropped from the binary-distribution decision once D8 lands.
  `Successor field:` annotations removed entirely.

New §6 verification gates (No-Legacy Audit, gates 17-22):

- 17: `cargo +nightly udeps` reports zero unused dependencies.
- 18: `cargo clippy --all-targets -- -D warnings -D dead_code` passes;
  no unjustified `#[allow(dead_code)]`.
- 19: CI grep gate — no `legacy Claude runtime marker`/`legacy Codex runtime marker`/`legacy end marker`
  anywhere in the working tree.
- 20: CI grep gate — no legacy shell rendering references in
  active wiki docs (allowed only in `wiki/archive/` and `wiki/log.md`).
- 21: No `parse_legacy()` / `SchemaVersion` enum / fallback paths in the
  binary. The codebase handles only the current schema.
- 22: No `Successor field:` fields remain on active documents.

§12 reframed from "open questions" to "implementation decisions
(pre-stage-5.1)." All four resolved:

1. Package: `llm-wiki-framework`; binary: `llm-wiki`. Reserve crates.io name.
2. Workspace layout (mandated by `build.rs` build-dependency requirement).
3. Hand-rolled `HOME`/XDG path resolution, not `dirs` crate (avoids the
   macOS `~/Library/Application Support/` mismatch).
4. IS_EXISTING profile operates on literal paths only; no `git2`, no
   implicit `.gitignore` walking. Predictability over convenience.

Pages updated: wiki/plans/llm-wiki-binary.plan.md, wiki/log.md
## [2026-05-06] update | D8 binary implementation started

Started implementation of `wiki/plans/llm-wiki-binary.plan.md` on branch
`d8-llm-wiki-binary`. Added the Rust workspace scaffold with the
`llm-wiki-schema` library crate and `llm-wiki-framework` binary crate,
baseline CI workflow, and coverage configuration. The D8 implementation plan
is now Active.

Pages updated: wiki/plans/llm-wiki-binary.plan.md, wiki/index.md, wiki/log.md

## [2026-05-06] create | Project registry and search artifacts proposal

Created a post-D8 proposal for adding project registration, centralized
per-project QMD search artifacts, and explicit cross-project search to the
`llm-wiki` binary.

The proposal keeps D8 unchanged and treats the active
`wiki/plans/llm-wiki-binary.plan.md` as the baseline implementation. It
recommends a later D9-style deliverable with a visible command split:
`llm-wiki search` for one project and `llm-wiki search-all` for explicitly
registered projects. Search artifacts are rebuildable caches under
`~/.cache/llm-wiki/`; `wiki/` remains the canonical knowledge source.

The proposal records the Rust `qmd` crate as the preferred candidate backend
because its docs expose Store, SQLite FTS5/BM25 search, local GGUF embeddings,
hybrid search, reranking, collection helpers, and model download support. It
requires a search-quality eval before accepting the backend as the framework's
search engine.

Also corrected D8 status in `wiki/index.md` and
`wiki/roadmaps/framework-v1.roadmap.md` from Draft to Active, matching the
existing D8 implementation-start log entry.

Pages created: wiki/proposals/project-registry-search-artifacts.proposal.md
Pages updated: wiki/index.md, wiki/roadmaps/framework-v1.roadmap.md, wiki/log.md

## [2026-05-06] update | D8 binary implementation completed

Completed the D8 `llm-wiki` binary implementation on
`d8-llm-wiki-binary`.

Implemented:

- Rust workspace with `llm-wiki-schema` and `llm-wiki-framework`
- typed canonical skill parser and runtime projectors
- compile-time embedded canonical validation
- `build`, `install`, `uninstall`, `init`, `status`, and `doctor`
- manifest ownership with hash-based drift/collision handling
- deterministic Create-mode scaffolding with profile snapshots
- v1 compatibility fixture and fixture smoke checklist
- release and post-install verification workflows

Cleanup:

- removed the legacy skill render script
- rewrote `init-project` as a thin wrapper over `llm-wiki init`
- archived predecessor decisions and plan under `wiki/archive/`
- rewrote the affected skill specs under the binary distribution model
- marked D8 completed on the roadmap and index

Verification run locally:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- no legacy runtime marker strings remain in `skills/`

External release gates still require human/repository action: reserve/publish
the crates.io package, tag `v0.1.0`, run the generated release workflow, and
complete manual Claude/Codex smoke checks.

Pages updated: wiki/index.md, wiki/log.md, wiki/roadmaps/framework-v1.roadmap.md,
wiki/plans/llm-wiki-binary.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md,
wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md,
wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md,
wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md,
wiki/archive/single-source-skills.decision.md, wiki/archive/framework-path-resolution.decision.md,
wiki/archive/project-local-codex-skills.decision.md, wiki/archive/single-source-skills.plan.md

## [2026-05-06] update | D8 review verification gaps closed

Addressed review findings against the D8 implementation. CI now runs the
`cargo +nightly udeps --workspace` unused-dependency audit. The tag-triggered
post-install workflow now runs a concrete redirected-HOME integration test
that installs the binary output, reads the manifest, verifies the expected
file count, verifies every manifest path exists, verifies each SHA-256 hash,
and checks that installed skill files are manifest-owned.

The reported coverage concern was reviewed against the actual local
`cargo llvm-cov --workspace --fail-under-lines 80` gate: line coverage is
above the configured threshold, so no status rollback was needed.

Pages updated: .github/workflows/ci.yml, .github/workflows/post-install.yml,
tools/llm-wiki/tests/post_install.rs, justfile, wiki/log.md

## [2026-05-06] create | D8 product layout addendum

Created a post-D8 layout addendum. D8 remains completed behaviorally; the
addendum corrects repository shape so the binary is the root product crate and
embedded product assets live under `assets/` instead of root `skills/` or root
`project_guidelines.template.md`.

Target layout:

- root `Cargo.toml` is both workspace root and `llm-wiki-framework` package
- root `src/` contains the `llm-wiki` binary
- root `build.rs` validates embedded assets
- `assets/skills/` contains canonical skill assets
- `assets/templates/` contains scaffold templates
- `crates/llm-wiki-schema/` remains the pure parser/projector library
- generated `.claude/skills/` and `.codex/skills/` outputs remain convenience
  outputs

Pages created: wiki/plans/llm-wiki-product-layout-addendum.plan.md
Pages updated: wiki/plans/llm-wiki-binary.plan.md, wiki/index.md, wiki/log.md
