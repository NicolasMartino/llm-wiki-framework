# Plan: MCP-First Agent Surface No-Legacy Cutover

- Document Class: Plan
- Status: Active
- Date: 2026-06-22
- Updated: 2026-08-01
- Category: Internal architecture, MCP surface, harness independence, complexity reduction
- Scope: Execute `wiki/proposals/mcp-first-surface.proposal.md` end-to-end as a no-legacy migration. Deterministic operations move behind one Rust-native stdio MCP server. Procedural workflows are carried by always-on project instructions (`AGENTS.md` / harness equivalents) plus MCP tools/resources, with MCP prompts added only if host support is proven. Generated Claude/Codex skill projection is retired after ordinary-language cross-harness parity is proven instead of being preserved as permanent thin shims.
- Sources:
  - wiki/proposals/mcp-first-surface.proposal.md
  - wiki/proposals/harness-independent-wiki-read-tool.proposal.md
  - wiki/plans/harness-independent-wiki-read-tool.plan.md
  - wiki/references/headroom-context-compression.reference.md
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/05-headroom-docs-mcp.mdx
  - raw/research/2026-06-20-codex-tool-surface/research-summary.md
- Related:
  - wiki/decisions/skill-projection-template-engine.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/composable-project-init.decision.md
- Promotion Target:
  - wiki/decisions/skill-projection-template-engine.decision.md
  - wiki/references/headroom-context-compression.reference.md
  - wiki/index.md
  - wiki/log.md

## Deliverable

`llm-wiki mcp serve` becomes the primary deterministic agent surface for
framework operations. The release ships one framework-owned MCP server with
tools and resources, and optionally prompts where the host supports them.
Install/uninstall configures that server for supported harnesses. Runtime
generated skill projection is removed once ordinary-language parity is proven.

## No-Legacy Position

The target state is no permanent generated skill surface. Existing Claude and
Codex skills may remain during implementation as migration scaffolding, but
completing this plan means they no longer ship as the primary or fallback
runtime surface.

No-legacy does not mean no project instructions. `AGENTS.md`, `CLAUDE.md`, and
other harness-native project guidance remain valid always-on workflow context.
Those files are the preferred load-bearing place for role discipline,
document-model rules, and intent-routing guidance. MCP tools do deterministic
work; MCP resources expose canonical project knowledge; MCP prompts are
convenience only unless the Phase 0 spike proves they are portable.

The gate has three possible outcomes:

- **Full MCP prompt branch:** prompts are supported and useful on both harnesses;
  prompts may ship alongside tools/resources.
- **Promptless no-legacy branch:** prompts are unsupported or not auto-routed on
  at least one harness, but ordinary-language requests still route correctly via
  project instructions plus MCP tools/resources. This still counts as no-legacy
  because generated skill projection is gone.
- **Blocked branch:** ordinary-language routing cannot be preserved without
  generated skills. This plan blocks and requires a new decision; a permanent
  projected-skill shim does not close this plan.

## Scope

In scope:

- Run a Phase 0 host-UX spike before making prompts load-bearing or deleting
  skills.
- Complete the existing `llm_wiki_read` plan as increment 1.
- Add MCP tools for `llm_wiki_search`, `llm_wiki_search_all`, `llm_wiki_index`,
  `llm_wiki_register`, and `llm_wiki_status`, reusing the same deterministic
  cores as the CLI.
- Expose canonical wiki/project material as MCP resources.
- Keep procedural obligations in `AGENTS.md` / harness project guidance and
  update that guidance to route natural user requests to MCP tools/resources.
- Add MCP prompts only after the host-UX spike proves support and usefulness.
- Replace per-harness skill installation with MCP server configuration for
  supported harnesses.
- Remove generated skill projection from runtime install/embed/uninstall paths
  after the cross-harness gates pass.
- Update Headroom guidance once read and search MCP tools are live and verified.
- Promote or revise the skill-projection decision at cutover.

Out of scope:

- Changing the document model, typed document roles, `agent-owns-wiki`, or
  `wiki/`+`raw/` provenance invariants.
- Weakening the `headroom_read` ban.
- Splitting distribution into multiple binaries.
- Adopting tokio or an MCP SDK unless the read-tool plan's transport decision is
  reopened with evidence.
- Removing `AGENTS.md` or harness-native project guidance as always-on workflow
  context.
- Keeping generated Claude/Codex skill projection as a permanent compatibility
  layer.

## Touchpoints

- `src/mcp/` - server scaffold, tool registry, resource registry, optional prompt
  registry, JSON-RPC handling, and stdio lifecycle.
- `src/cli.rs` / `src/main.rs` - `mcp serve` command and shared dispatch.
- Search, registry, init, and status cores - refactor only as needed so CLI and
  MCP share behavior.
- `src/install.rs` / `src/uninstall.rs` / `src/embed.rs` - replace dual skill
  materialization with MCP configuration and remove projected skill payloads.
- `src/skill_render.rs`, `crates/llm-wiki-schema/src/projector/`, and
  `templates/skills/` - retire once the no-legacy cutover gate passes.
- `AGENTS.md`, `CLAUDE.md`, and templates that generate project guidance - keep
  workflow routing and document discipline load-bearing.
- `wiki/` - proposal, plan, reference, decision, eval, index, and log updates.

## Phases

### Phase 0 - Host UX And Prompt Support Spike

Resolve the host-platform assumptions before building toward deletion.

Tasks:

1. Research current Claude Code and Codex MCP client support for tools,
   resources, and prompts. Record whether Codex supports MCP prompts, not just
   MCP resources/tools.
2. Prototype `wiki-query` using `AGENTS.md` / harness project guidance plus MCP
   tools/resources. Add an MCP prompt only where supported.
3. On both harnesses, issue ordinary user requests such as "query the wiki",
   "ingest this raw folder", and "lint the wiki" without naming a prompt.
4. Record which primitive actually routed the workflow: skill, project
   instruction, MCP resource, MCP prompt, or explicit tool selection.
5. Decide the branch for the rest of the plan: full prompt branch, promptless
   no-legacy branch, or blocked branch.

Verification:

- A new eval records host versions, observed MCP capabilities, transcripts,
  tool/resource/prompt availability, and an intent-routing verdict.
- The eval explicitly states whether Codex supports prompts.
- Slash-command-only success does not count as ordinary-language parity.
- If the verdict is blocked, later deletion phases do not proceed.

### Phase 1 - Read Tool Foundation

Execute `wiki/plans/harness-independent-wiki-read-tool.plan.md` without changing
its scope. This creates `llm-wiki mcp serve`, the first MCP tool
(`llm_wiki_read`), and its `_test`-namespaced sibling.

Verification:

- Read-tool plan verification gates pass.
- `tools/list` exposes `llm_wiki_read`.
- `tools/call llm_wiki_read` returns the accepted content/metadata schema.
- The kept Headroom reference safety rule (do not enable `HEADROOM_MCP_READ`, do
  not call `headroom_read` against wiki/raw) still stands.

### Phase 2 - Deterministic Tool Parity

Port deterministic CLI cores to MCP tools:

- `llm_wiki_search`
- `llm_wiki_search_all`
- `llm_wiki_index`
- `llm_wiki_register`
- `llm_wiki_status`

The MCP outputs must be parseable, schema-stable, and equivalent to current CLI
JSON behavior for success, typed errors, readiness, fallback, and zero-result
states.

Verification:

- Fixture tests compare CLI JSON and MCP tool JSON for registered, unregistered,
  stale, unavailable, permission-denied, and zero-result cases.
- Tool names are centrally instance-namespaced for production/test identities.
- Codex no longer needs shell-family exclusion for `llm-wiki search` when using
  MCP tools.
- Headroom reference is updated only after read and search tools pass.

### Phase 3 - Project Guidance And MCP Resources

Move the procedural surface to confirmed primitives:

- Always-on project guidance: query, ingest, lint, research, init/update routing
  obligations in `AGENTS.md` / harness project instruction templates.
- MCP resources: `wiki/index.md`, project guidelines, operation checklists,
  document model, read-only access contract, and other canonical wiki/project
  material.
- Optional MCP prompts: only for hosts where Phase 0 proves prompt support and
  acceptable discovery.

The guidance must preserve current workflow obligations: read the index first,
use search when registered, cite wiki pages directly, update index/log on
mutations, and keep raw/wiki provenance exact.

Verification:

- Project instruction snapshots contain natural-language routing guidance for
  query, ingest, lint, research, and init/update.
- `resources/list` exposes the expected project resources.
- `prompts/list` is tested where prompts are supported; unsupported prompt
  capability is recorded and does not block if ordinary-language routing works
  through project guidance.
- MCP resources route through framework-owned read behavior and do not enable
  `headroom_read`.

### Phase 4 - Install/Uninstall Cutover

Change install/uninstall so supported harnesses get MCP server configuration
and updated project guidance instead of projected skill trees.

The cutover must remove runtime dependence on:

- `templates/skills/claude.md`
- `templates/skills/codex.md`
- `templates/skills/codex_runtime_config.yaml`
- Claude/Codex projector-specific skill output
- per-skill dual install/embed manifest entries

Verification:

- Fresh install writes MCP configuration for each supported harness.
- Fresh install writes or updates project guidance that routes ordinary user
  requests to MCP tools/resources.
- Fresh install does not write generated `.claude/skills/` or `.codex/` skill
  material as a fallback.
- Uninstall removes MCP configuration and cleans any prior managed projected
  skill output without touching unmanaged user files.
- Existing install/update fixtures are replaced with MCP-config and
  project-guidance snapshots.

### Phase 5 - Cross-Harness Ordinary-Language Parity Eval

Record an eval that runs the MCP surface on both Claude Code and Codex against
the same fixture projects. The eval starts from ordinary user requests, not
prompt names or tool names.

The eval must cover:

- Query with citations.
- Ingest or lint mutation with index/log bookkeeping.
- Search fallback and readiness reporting.
- Read-only wiki/raw access under Headroom-safe tool names.
- Failure behavior when project registration or search cache is unavailable.
- Whether prompts were used, ignored, unsupported, or unnecessary.

Verification:

- The eval records host versions, command transcripts, tool-call JSON, resulting
  wiki diffs, and a parity verdict against current skills.
- The verdict is `Accepted` before deletion begins.
- Slash-command-only success is not parity.
- Any failed ordinary-language parity case blocks this plan instead of creating
  a permanent generated-skill shim.

### Phase 6 - Delete Projection Surface

After Phase 5 is accepted, remove the legacy projection machinery from runtime
code and assets.

Status note, 2026-06-22: deletion was executed out of order by explicit user
authorization. This is not Phase 5 acceptance evidence and does not close the
Phase 0 host-UX spike, the Phase 5 cross-harness parity eval, or acceptance
criteria 1, 4, and 6.

Verification:

- Source lint fails if runtime code references generated skill install paths,
  Codex runtime YAML, or Claude/Codex skill projectors.
- Golden snapshots prove the remaining install output is MCP configuration plus
  project guidance, not generated skill trees.
- Workspace tests, clippy, and relevant release/install smoke tests pass.
- `wiki/decisions/skill-projection-template-engine.decision.md` is superseded or
  amended with the no-legacy cutover evidence.

### Phase 7 - Documentation And Release Hardening

Update durable docs and release evidence:

- Update `wiki/references/headroom-context-compression.reference.md` with the
  live read/search MCP posture and revised Codex recommendation.
- Update `wiki/index.md` and append `wiki/log.md`.
- Record the host-UX spike eval and parity eval.
- Archive or supersede any now-stale proposal/plan pages.
- Update user-facing setup docs to describe MCP configuration plus project
  guidance as the primary surface.

Verification:

- No stale index entry claims generated skills are the primary runtime surface.
- Headroom posture matches implemented tool behavior.
- Release or smoke proof demonstrates a clean install, ordinary-language MCP
  query, MCP search, MCP read, and uninstall cycle.

## Acceptance Criteria

1. Phase 0 records whether Codex supports MCP prompts and whether prompts are
   portable enough to use.
2. `llm-wiki mcp serve` hosts read, search, search-all, index, register, and
   status tools with schema-stable outputs; valid tool-call execution failures
   return MCP tool results with `isError: true`, while malformed params and
   unknown methods/tools remain JSON-RPC protocol errors.
3. Project guidance plus MCP tools/resources preserve all current framework
   workflows and their citation, provenance, index, and log obligations.
4. MCP prompts are optional; they ship only where the host-UX spike proves they
   are supported and useful.
5. Fresh installs automatically configure Codex MCP, stage Claude Code project
   MCP config with explicit manual wiring instructions, and do not materialize
   generated skill trees as a runtime fallback.
6. Cross-harness ordinary-language parity eval on Claude Code and Codex is
   accepted before the plan is closed.
7. The reference routes normal wiki operations through the framework-owned
   read/search MCP tools and keeps the harness-neutral safety rule (no
   `HEADROOM_MCP_READ=on`; no `headroom_read` against wiki/raw) intact.
8. Runtime skill projection code, templates, Codex runtime YAML generation, and
   dual install/embed/uninstall paths are removed or made test-only historical
   evidence.
9. `wiki/decisions/skill-projection-template-engine.decision.md` is superseded or
   amended with the accepted cutover evidence.
10. `wiki/index.md` and `wiki/log.md` reflect the cutover.

## Implementation Progress

### 2026-06-22 - MCP Foundation And Deterministic Tool Slice

Implemented the first Rust-native stdio MCP server slice in code:

- Added `llm-wiki mcp serve` with synchronous JSON-RPC handling for
  `initialize`, `tools/list`, `tools/call`, `resources/list`, and
  `resources/read`.
- Added `llm-wiki read <path> [--project <id>]` and shared
  `llm_wiki_read` read core for `wiki/` and `raw/` full-file reads.
- `llm_wiki_read` returns canonical project metadata, `tree`, `byte_len`,
  `sha256`, UTF-8 content when inlineable, and explicit omission reasons for
  binary or over-1 MiB content.
- Added MCP tool entries and handlers for `llm_wiki_search`,
  `llm_wiki_search_all`, `llm_wiki_index`, `llm_wiki_register`, and
  `llm_wiki_status`; search/search-all return the existing CLI JSON envelope.
- Added allowlisted MCP resources for canonical project material such as
  `wiki/index.md`, `wiki/log.md`, `AGENTS.md`, and project-guideline files.
- Updated `AGENTS.md` and `templates/base/project_guidelines.md` to route
  normal wiki reads/search through framework-owned MCP tools when available,
  while keeping the `headroom_read` ban intact.
- Reserved `llm_wiki_read` in the test-instance suffixing path alongside
  `llm_wiki_search` and `llm_wiki_search_all`.

Verification evidence:

- `cargo test --test mcp` — 11 passed.
- `cargo test --test identity_lint` — 2 passed.
- `cargo check` — passed.
- `cargo test` — 303 passed, 2 ignored.

Still pending:

- Phase 0 host-UX and prompt-support spike.
- Cross-harness ordinary-language parity eval.
- Install/uninstall MCP configuration cutover.
- Runtime skill projection deletion and decision supersession/amendment.
- Headroom reference update after the final live read/search MCP posture is
  accepted.

### 2026-06-22 - MCP Install Configuration Slice

Implemented the first install/config migration slice:

- Fresh `llm-wiki install` now writes Codex MCP configuration to
  `~/.codex/config.toml` under `[mcp_servers."llm-wiki"]`, preserving existing
  top-level keys and other MCP servers.
- Install materializes a managed Claude project-scope MCP config artifact at
  `~/.llm_wiki/mcp/claude-project.mcp.json`, matching Claude Code's
  `.mcp.json` `mcpServers` shape.
- Added `src/mcp_config.rs` renderer tests and `tests/mcp_install.rs`
  integration coverage.

Verification evidence:

- `cargo test --test mcp_install` — 2 passed.
- `cargo test --test mcp` — 11 passed.
- `cargo clippy` — passed with no issues.

## 2026-06-22 - No Generated Skill Install Output Slice

Runtime `llm-wiki install` no longer materializes generated `.claude/skills/*`
or `.codex/skills/*` trees. The install manifest keeps `skills` empty for new
installs and records install-owned host integration through the `mcp-config`
managed asset. Full uninstall now
validates and removes both managed skills and managed assets, while preserving
user-authored legacy skill files that are not manifest-owned.

This is an installation-surface cutover only. Historical skill-rendering code,
build-time projection snapshots, and the legacy projection decision remain in
place until the cross-harness parity and deletion phases close.

Verification evidence:

- `cargo test --test install` - 36 passed.
- `cargo test --test status_doctor` - 19 passed.
- `cargo test --test post_install` - 3 passed.
- `cargo test --test properties` - 1 passed.
- `cargo test --test mcp_install` - 2 passed.
- `cargo test --test mcp` - 11 passed.
- `cargo test` - 306 passed, 2 ignored.
- `cargo clippy` - passed with no issues.

## 2026-06-22 - Codex MCP Config Uninstall Cleanup Slice

Full uninstall now removes the install-owned Codex MCP server entry from
`~/.codex/config.toml`. If that file contained only the managed `llm-wiki`
server, uninstall removes the file; if user-authored settings or other MCP
servers exist, uninstall preserves them and removes only `mcp_servers.llm-wiki`.
The cleanup uses TOML parsing and serialization instead of text matching.

Verification evidence:

- `cargo test --test mcp_install` - 4 passed.
- `cargo test --test install` - 36 passed.
- `cargo test --test status_doctor` - 19 passed.
- `cargo test --test post_install` - 3 passed.
- `cargo clippy` - passed with no issues.

## 2026-06-22 - Advisory MCP Prompt Support Slice

The MCP server now advertises prompt capability and handles `prompts/list` plus
`prompts/get` for advisory operation prompts: `wiki_query`, `wiki_ingest`,
`wiki_lint`, `wiki_research`, and `wiki_init`. Prompt text routes agents back
to project guidance, `wiki/index.md`, and deterministic MCP tools such as
`llm_wiki_read` and `llm_wiki_search`.

This does not close Phase 0 or Phase 5 by itself. Host discovery and
ordinary-language parity still require cross-harness evidence; prompts remain
optional and non-load-bearing until that evidence is accepted.

Verification evidence:

- `cargo test --test mcp` - 12 passed.
- `cargo clippy` - passed with no issues.

## 2026-06-22 - Canonical Operation Spec Resources Slice

MCP resources now include the durable operation and documentation specs when
present in a project: `wiki/specs/documentation-model.spec.md`,
`wiki/specs/wiki-query-skill.spec.md`, `wiki/specs/wiki-ingest-skill.spec.md`,
`wiki/specs/wiki-lint-skill.spec.md`, `wiki/specs/wiki-research-skill.spec.md`,
and `wiki/specs/wiki-init-skill.spec.md`. These are allowlisted alongside
`wiki/index.md`, `wiki/log.md`, `AGENTS.md`, and project guidelines so hosts
can discover canonical workflow rules without generated skills.

Verification evidence:

- `cargo test --test mcp` - 13 passed.
- `cargo clippy` - passed with no issues.

## 2026-06-22 - MCP Initialize Instructions Slice

The MCP server now returns concise server-level `instructions` from
`initialize`, directing hosts to read `wiki/index.md` with `llm_wiki_read`, use
`llm_wiki_search` for registered-project queries, inspect canonical MCP
resources, and use optional prompts where supported.

Verification evidence:

- `cargo test --test mcp` - 13 passed.
- `cargo clippy` - passed with no issues.

## 2026-06-22 - Post-Slice Verification

After the prompt, resource, initialize-instructions, install-output, and MCP
config uninstall slices, the full workspace test suite passed.

Verification evidence:

- `cargo test` - 310 passed, 2 ignored.

## 2026-06-22 - No-Legacy Projection Deletion Slice

The user accepted the no-legacy branch and directed removal of the remaining
legacy surface. The implementation deletes the generated skill projection build
surface (`llm-wiki build`, `src/build.rs`, `src/skill_render.rs`), generated
skill source assets, schema Claude/Codex projectors, Askama skill projection
templates, projector tests/snapshots, generated-skill path helpers, and
doctor/status tests that only existed for generated skill residue. The schema
crate remains as parser/validation support only.

The guard `no_legacy_generated_skill_projection_surface` now fails if production
code or tests reintroduce generated skill install paths, runtime YAML paths,
Claude/Codex projector names, or the deleted build command surface.

Verification evidence recorded before the final full-suite run:

- `cargo test --test identity_lint` - 3 passed.
- `cargo check` - passed.
- `cargo test --test install` - 34 passed.
- `cargo test --test post_install` - 3 passed.
- `cargo test --test status_doctor` - 17 passed.
- `cargo test --test mcp` - 13 passed.
- `cargo test --test compat` - 2 passed.
- `cargo test -p llm-wiki-schema` - 9 passed.
- `cargo test --test mcp_install` - 4 passed.
- `cargo clippy` - passed with no issues.
- `cargo test` - 297 passed, 2 ignored.

## 2026-06-22 - Review Hardening Follow-Up

The MCP resource layer now resolves canonical `llm-wiki://project/AGENTS.md`
against either `AGENTS.md` or `AGENTS.MD`, matching registry project-root
validation and preventing Linux/case-sensitive hosts from dropping the project
guidance resource.

Tool execution failures for `llm_wiki_read`, `llm_wiki_status`, and
CLI-backed tools now return normal MCP tool results with `isError: true` and a
JSON text payload shaped as `{ "error": { "message": ... } }`. Malformed
tool params and unknown tools remain JSON-RPC protocol errors. Tests cover
read-path rejection and a subprocess-backed `llm_wiki_search` failure.

Install diagnostics now distinguish host behavior: Codex MCP config is merged
into `~/.codex/config.toml`, while Claude Code receives a staged
`~/.llm_wiki/mcp/claude-project.mcp.json` file that must be copied to a project
`.mcp.json` or registered with Claude Code per project until the host-UX spike
proves an automatic registration path.

Evidence correction: the no-legacy deletion slice was user-authorized, not an
accepted Phase 5 parity eval. Phase 0 and Phase 5 remain open evidence gates.

Verification evidence:

- `cargo fmt` / `cargo fmt --check` - passed.
- `cargo check` - passed.
- `cargo test --test mcp` - 15 passed.
- `cargo test --test install` - 34 passed.
- `cargo test --test identity_lint` - 3 passed.

## 2026-06-23 - MCP Startup Contract

MCP startup is intentionally host-managed stdio, not a background daemon. Host
MCP configuration starts the managed binary on demand as
`llm-wiki mcp serve`; the server reads JSON-RPC over stdio and exits with its
host process.

The startup command is now centralized in `src/mcp_config.rs` so Codex config,
Claude project config, install diagnostics, and status output share one
contract. `llm-wiki install --verbose` reports that hosts spawn the server on
demand and that no daemon is installed. `llm-wiki status` reports
`mcp server startup: host-managed stdio (...)` using the managed binary path
from the manifest.

Verification evidence:

- `cargo test --test install verbose_install_emits_command_diagnostics` -
  passed.
- `cargo test --test status_doctor status_reports_installed_files` - passed.
- `cargo fmt` - passed.
- `cargo test --test mcp_install` - 4 passed.
- `cargo test --test status_doctor` - 17 passed.
- `cargo test --test install` - 34 passed.

## 2026-06-23 - Host UX And Ordinary-Language Parity Eval

Recorded `wiki/evals/mcp-first-host-parity.eval.md` as a rejected Phase 0/5
host eval. Codex CLI 0.141.0 can execute `llm-wiki` MCP tools/resources when
configured with `mcp_servers.llm-wiki.*` and run non-interactively with MCP
approval bypass; `llm_wiki_read`, `llm_wiki_search`, and `llm_wiki_status`
were observed. Codex MCP prompts were not exposed. The ordinary Codex wiki
query routed through the user-installed `wiki-query` skill before using MCP,
so it does not prove a fresh project-guidance-only no-legacy path.

Claude Code 2.1.176 loaded the staged MCP config and listed `llm-wiki` as a
pending MCP server during initialization, but the run stopped with
`authentication_failed` / `Not logged in`, so Claude parity could not be
evaluated on this machine.

Phase 5 remains open and cannot be accepted from this evidence.

## 2026-06-23 - Merge-Readiness Repair Hardening

`wiki/plans/headroom-mcp-merge-readiness-repair.plan.md` executed the code
repair slice required before more live parity evidence can be trusted:

- MCP server/tool identity is instance-derived. Production remains byte-stable
  at `llm-wiki` / `llm_wiki_*`; the `test` instance emits `llm-wiki-test` and
  suffixed MCP tool names.
- Install/uninstall now reconciles old generated-skill manifests during
  MCP-first upgrades: unchanged manifest-owned old skills are removed,
  user-edited skills are preserved with warnings, and manifest-less old skill
  directories are reported.
- Repo-local skills use a transitional dogfood posture through
  `tools/with-repo-skills-disabled.sh`.
- `just test-instance-live-session-proof` now verifies `llm-wiki-test install`
  without leaving production Codex MCP config changed after uninstall.

Verification: `cargo check --tests`, focused MCP/install/status/identity/search
suites, `LLM_WIKI_INSTANCE=test` MCP/MCP-install suites, no-repo-skill MCP test
helper, live test-instance proof, `cargo test`, and `cargo clippy` pass.

The plan remains active until `wiki/evals/mcp-first-host-parity.eval.md` records
accepted clean cross-harness ordinary-language parity or narrows the remaining
blocker to an explicit external host condition.

## 2026-08-01 - Production MCP Field-Test Review

The installed production MCP surface (`llm-wiki 0.2.15`, unsuffixed
`llm_wiki_*` tools) passed both Codex and Claude field-test checklists against
the external `keto-diet` project in an existing ready-index posture. The runs
validate the deterministic tool layer in ordinary host wiring:
status/register/index, scoped wiki/raw reads with hashes and path-safety
errors, lexical/auto/semantic/hybrid search with default thresholds,
class/status filters, rerank diagnostics, and cross-project search degradation
with include/exclude filters all returned parseable payloads.

The only apparent blocking `BUG` row was a checklist wording mismatch:
`search-all` returns per-project readiness in `projects[]` and globally ranked
flat `results[]` rows with project attribution, rather than grouped result
arrays. The checklist and host evals now record that as the current contract.
Claude also found one minor API-validation follow-up: obsolete
`include_projects` / `exclude_projects` keys are silently ignored, while the
documented `include` / `exclude` keys work.

This does not close Phase 5. The remaining acceptance gap is still clean
no-skill ordinary-language workflow breadth across Claude Code and Codex,
including mutation/bookkeeping and failure/readiness cases.

## Failure Handling

Failure to reach ordinary-language parity blocks the plan. The default next
action is to repair project guidance, MCP tools/resources, prompt support, or
harness configuration and rerun the eval. A permanent generated-skill shim is
not an accepted completion state for this plan.

If a harness lacks MCP prompts but ordinary-language routing works through
project guidance plus MCP tools/resources, the plan can still complete via the
promptless no-legacy branch. If ordinary-language routing cannot be preserved
without generated skills, a new decision is required before continuing.
