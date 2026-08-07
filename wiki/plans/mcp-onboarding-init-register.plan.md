# Plan: Automatic MCP Onboarding At init/register

- Document Class: Plan
- Status: Active
- Date: 2026-06-29 (decision accepted 2026-07-31)
- Category: Tooling, MCP onboarding, init/register UX, host parity
- Scope: Make the `llm-wiki` MCP server connect automatically for Claude Code
  the way it already does for Codex, by moving per-project `.mcp.json` wiring
  into the project-scoped `init` and `register` commands. Extract a shared,
  non-destructive, idempotent wiring core; call it from both commands; add a
  `doctor` check that a registered project is actually wired; demote `install`'s
  Claude staging from "required manual step" to "fallback"; and update docs and
  the field-test checklists to drop the manual `cp …/.mcp.json` step.
- Sources:
  - wiki/decisions/mcp-onboarding-at-init-register.decision.md
  - src/install.rs (`materialize_mcp_configs`)
  - src/mcp_config.rs (`render_claude_project_mcp_config`,
    `merge_claude_project_mcp_config`, `merge_codex_config`)
  - src/mcp_wiring.rs (`wire_project_mcp`,
    `ensure_claude_project_mcp_config`, `ensure_codex_mcp_config`)
  - src/paths.rs (`claude_project_mcp_config`)
  - src/registry/mod.rs (`register`)
  - src/init/command.rs (`run`)
  - src/cli.rs (`RegisterArgs`, init registration flags)
- Related:
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md (the posture this
    onboarding serves: MCP tools are the wiki/raw surface, so wiring them per
    project is what makes that posture real)
  - wiki/decisions/composable-project-init.decision.md
  - wiki/references/headroom-context-compression.reference.md
  - wiki/checklists/mcp-field-test.checklist.md
  - wiki/plans/mcp-first-agent-guidance.plan.md (sibling gap: this plan *wires*
    the MCP server; that plan makes generated AGENTS.md *route* agents to it —
    both are needed before an agent actually uses the tools)
  - wiki/plans/init-rerun-pack-drift.plan.md (Completed: the rerun mechanism
    that refreshes an existing project's framework-owned AGENTS.md after the
    templates change)

## Decision Status

The governing decision,
`wiki/decisions/mcp-onboarding-at-init-register.decision.md`, was accepted on
2026-07-31 after the production `llm-wiki 0.2.14` MCP field test and the
follow-up Claude wiring check showed the same root problem in current product
use: production is installed and Codex is globally wired, but Claude depends on
project-local `.mcp.json` that `install` can only stage. This plan is now the
implementation plan of record for that accepted behavior.

## Problem

Before this plan, MCP wiring lived only in `install`
(`materialize_mcp_configs`), a global, project-agnostic command. `init` and
`register` touched no MCP config at all. Because of where each host stores
config:

- **Codex config is global** (`~/.codex/config.toml`), so `install` auto-wires
  it once via `merge_codex_config`.
- **Claude config is per-project** (`<project>/.mcp.json`), so a global command
  cannot know which projects exist. The most `install` can do is stage a
  template at `~/.llm_wiki*/mcp/claude-project.mcp.json` and print "copy this
  into a project `.mcp.json`".

That staged-only step is Blocker B1 from the 2026-06-29 Claude field test: the
test MCP server was never connected, so the proxy run could not even exercise
the tools. It is also the first-run experience for every Claude Code user. The
fix is to wire MCP from the project-scoped commands that already own a project
root — `init` and `register` — per
`wiki/decisions/mcp-onboarding-at-init-register.decision.md`.

## Deliverable

After this plan:

- `llm-wiki register <path>` and `llm-wiki init` write/merge a project-local
  `.mcp.json` containing the `llm-wiki[-test]` stdio server, non-destructively
  and idempotently, and ensure the global Codex config is present.
- A `--no-mcp` flag (mirroring `--no-register`) opts out.
- `llm-wiki doctor` reports when a registered project's host MCP config does not
  reference the server, with a one-line fix hint.
- `install` still stages the Claude template, but it is documented as a fallback,
  and its diagnostic points at `register`/`init` instead of a manual copy.
- The two field-test checklists no longer require the manual `cp` step; the
  reference/proposal/guidance reflect automatic onboarding.

## Non-Goals

- No change to what the MCP server exposes (tools, resources, schemas).
- No new runtime dependency and no change to the single-static-binary
  distribution invariant. This wires llm-wiki's own MCP server, not Headroom.
- Not editing a user's Claude config destructively: existing non-llm-wiki
  servers in a project `.mcp.json` are preserved.
- Nothing about the Headroom proxy. The four-mode proxy posture was retired on
  2026-06-30 (`wiki/decisions/headroom-single-posture-mcp-first.decision.md`), so
  this plan no longer has any proxy-launch cleanup phase.

## Phase 0 — Shared, non-destructive MCP wiring core

Extract project-scoped MCP wiring into one reusable function so `init`,
`register`, and (optionally) `install` cannot drift. Likely home: `src/mcp_config.rs`
(pure rendering/merge logic) plus a thin orchestrator in `src/registry/mod.rs`
that both commands call.

- Add `merge_claude_project_mcp_config(existing: Option<&str>, binary: &Path) -> Result<String>`
  alongside the existing `render_claude_project_mcp_config`,
  matching the merge semantics already used for Codex in `merge_codex_config`:
  parse existing JSON, insert/replace only the `llm-wiki[-test]` server
  entry under `mcpServers`, preserve all other keys and servers, and pin the
  managed binary path. No-op (byte-identical output) when the active server
  entry is already structurally correct, even if the existing `.mcp.json` uses
  compact or custom formatting.
- Add an orchestrator `wire_project_mcp(project_root, paths, binary, opts)` that:
  1. merges/writes `<project_root>/.mcp.json` for Claude,
  2. ensures the global Codex config via `merge_codex_config` idempotently,
  3. returns a structured summary (what changed, what was already correct) for
     the caller to print.
- Reconcile stale entries: when the managed binary path for the active server
  key differs from what is on disk, the merge replaces that entry in place rather
  than duplicating it (covers `register --update` and re-runs). Only the active
  instance's key is touched — a different-instance entry (`llm-wiki` vs
  `llm-wiki-test`) is left intact so the two servers coexist in one project; the
  tool never writes underscore name variants, so there is nothing to migrate.

Verification:
- Unit tests in `src/mcp_config.rs` / `src/mcp_wiring.rs`: (a) merge into an existing `.mcp.json` with
  an unrelated server preserves that server; (b) merge is idempotent (second run
  byte-identical); (c) a stale binary path is reconciled, not duplicated; (d)
  empty/absent input produces the same content as
  `render_claude_project_mcp_config`; (e) a compact/custom-formatted but already
  structurally correct `.mcp.json` is not rewritten.
- `cargo test --bin llm-wiki mcp_config` green.

## Phase 1 — Wire into `register`

`register` (`src/registry/mod.rs`) already owns a concrete project root
(`RegisterArgs.path`). After the project is recorded:

- Call `wire_project_mcp` with the project root.
- Honor a new `--no-mcp` flag (add to `RegisterArgs`); when set, skip wiring and
  print a one-line pointer to the fallback template path, `install`, and
  `doctor`.
- Print exactly what was written and where (`<root>/.mcp.json` created/updated/
  already-correct; Codex config ensured), so the side effect is visible.

Verification:
- Integration test (extend `tests/mcp.rs` or a new `tests/mcp_onboarding.rs`):
  `register` on a temp project writes a `.mcp.json` whose `mcpServers` contains
  the instance-correct `llm-wiki[-test]` entry pointing at the managed binary;
  re-running is idempotent; `--no-mcp` writes nothing.
- Instance-aware: under `LLM_WIKI_INSTANCE=test`, the server key and binary are
  the `-test` variants.

## Phase 2 — Wire into `init`

`init` (`src/init/command.rs`) scaffolds and (unless `--no-register`) registers a
new project. After scaffolding/registration:

- Call the same `wire_project_mcp` core on the freshly created project root,
  guarded by the same `--no-mcp` opt-out and skipped when `--no-register` is set
  (no registration ⇒ respect the user's "don't touch hosts" intent; still print
  the staged-template pointer).
- Keep the existing `.llm_wiki/init.toml` answer recording (composable-init
  decision) unaffected; MCP wiring is a side effect, not a recorded answer,
  unless we later decide to persist the `--no-mcp` choice there.

Verification:
- Snapshot/integration: `init` of a fresh project yields a project-root
  `.mcp.json` with the correct entry; `init --no-register` does not write it;
  existing init snapshots (`tests/snapshots/init__*`) updated only where output
  text changed, via `cargo insta`.

## Phase 3 — `doctor` connectivity check

Add a `doctor` check (`src/doctor.rs`) that, for each registered project with
an existing root, reports whether its host MCP config actually references the
`llm-wiki[-test]` server:

- Claude: look for `<project_root>/.mcp.json` with the expected `mcpServers`
  entry; missing/mismatched ⇒ a warning (not a failure) with the fix hint "run
  `llm-wiki register <path>` (or `init`) to wire it, or run `llm-wiki install`
  to materialize the fallback Claude template for manual wiring".
- Codex: confirm the global config references the server.
- This turns the silent B1 failure into a surfaced one. It must not fail
  `doctor` (a user may intentionally run unwired) and must not require Headroom.

Verification:
- `tests/status_doctor.rs`: a wired project passes the check; an unwired
  registered project emits the warning; the check never flips `doctor`'s exit
  status. Keep the existing 21/21+ doctor assertions green.

## Phase 4 — Demote install staging to fallback

- `materialize_mcp_configs` keeps staging the Claude
  template (for manual/edge layouts) but its diagnostic changes from
  "copy {} to a project .mcp.json or register it" to "Claude MCP is wired per
  project by `llm-wiki init`/`register`; this staged copy at {} is a fallback for
  manual setups." Codex global wiring is unchanged.
- No behavior regression for existing installs; this is messaging + the staged
  file's role.

Verification:
- Install tests updated for the new diagnostic string; staged template still
  produced.

## Phase 5 — Docs, checklists, and decision promotion

- Update the field-test checklist
  (`wiki/checklists/mcp-field-test.checklist.md`) to drop the manual
  `cp …/claude-project.mcp.json <project>/.mcp.json` step and instead state
  "register the project (`llm-wiki register`/`init`) wires `.mcp.json`
  automatically; launch Claude Code from the project root".
- Update `wiki/references/headroom-context-compression.reference.md` onboarding
  notes, and
  `templates/base/agents.md` / `templates/base/project_guidelines.md` if they
  describe MCP setup.
- Resolve the decision's open questions in the docs:
  1. **Launch dir.** `.mcp.json` is read from Claude's launch directory; document
     "launch Claude from the project root." (User-level `claude mcp add` support
     is deferred; note it as a future option, not built here.)
  2. **Authoritative command.** Shared core lives in one place (Phase 0) and both
     `init` and `register` call it.
  3. **Codex re-wiring.** `init`/`register` ensure the global Codex config
     idempotently, so a project registered on a machine whose `install` predates
     the MCP server still ends up wired.
- Confirm/update the accepted decision's closure evidence and update its
  `wiki/index.md` line if implementation evidence changes.

Verification:
- `git grep -n "cp .*claude-project.mcp.json"` over `wiki/` returns nothing in
  the live checklists (only historical eval text may retain it).
- A manual fresh-project proof: `llm-wiki init` (or `register`) a throwaway
  project, launch Claude from its root, confirm `llm_wiki_*` MCP tools are
  present without any manual copy.

## Phase 6 (removed) — proxy-launch cleanup

This plan originally carried an optional final phase to bless a `run-proxy.sh`
proxy launch over the `set -a; … headroom wrap` dance. It is **removed**: the
Headroom proxy posture (and `run-proxy.sh` itself) was retired on 2026-06-30 by
`wiki/decisions/headroom-single-posture-mcp-first.decision.md`, so there is no
proxy launch to bless. Nothing in this plan touches Headroom.

## Acceptance Criteria

1. `merge_claude_project_mcp_config` exists with non-destructive, idempotent,
   reconciling merge semantics, and the wiring core preserves byte-identical
   compact/custom-formatted configs that are already structurally correct
   (Phase 0).
2. `register` and `init` both write/merge the project `.mcp.json` and ensure the
   Codex global config, via one shared core, honoring `--no-mcp` and printing
   what changed (Phases 1–2).
3. `doctor` surfaces an unwired registered project as a warning with a fix hint,
   without changing exit status (Phase 3).
4. `install` still stages the Claude template but its diagnostic frames it as a
   fallback to `init`/`register` (Phase 4).
5. The field-test checklists and onboarding docs no longer require a manual
   `.mcp.json` copy; the accepted decision carries current closure evidence
   (Phase 5).
6. Instance-aware throughout: under `LLM_WIKI_INSTANCE=test` all wiring uses the
   `-test` server key and managed binary.
7. `cargo test`, `cargo clippy`, and `cargo insta` are green; the manual
   fresh-project proof connects the MCP tools with no manual copy.
