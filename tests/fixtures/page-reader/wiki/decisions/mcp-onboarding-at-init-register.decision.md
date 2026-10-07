# MCP Onboarding Belongs At init/register, Not Only install

- Document Class: Decision
- Status: Accepted
- Date: 2026-06-29 (accepted 2026-07-31; amended 2026-07-18)
- Category: Tooling, MCP onboarding, init/register UX
- Scope: Where the framework wires its own `llm-wiki` MCP server into the host
  agent's configuration. Before this decision, only `install` (a global,
  project-agnostic command) touched MCP config: it auto-wired the global Codex
  config but could only *stage* the per-project Claude config. This decision
  moves per-project MCP wiring into `init` and `register`, which are the
  project-scoped commands, so a Claude Code user gets the same automatic
  onboarding a Codex user already gets. It does not change what the MCP server
  exposes.
- Sources:
  - src/install.rs (`materialize_mcp_configs`)
  - src/mcp_config.rs (`render_claude_project_mcp_config`,
    `merge_claude_project_mcp_config`, `merge_codex_config`)
  - src/mcp_wiring.rs (`wire_project_mcp`,
    `ensure_claude_project_mcp_config`, `ensure_codex_mcp_config`)
  - src/paths.rs (`claude_project_mcp_config`)
  - src/init/command.rs, src/registry/mod.rs
- Related:
  - wiki/decisions/composable-project-init.decision.md
  - wiki/references/headroom-context-compression.reference.md
  - wiki/plans/headroom-passthrough-launcher.plan.md
  - wiki/plans/mcp-onboarding-init-register.plan.md

## Acceptance Update (2026-07-31)

Accepted after the production `llm-wiki 0.2.14` MCP field test exposed the
remaining Claude onboarding gap in ordinary use: Codex sees the global MCP
config, while Claude only sees a project `.mcp.json` from the directory where it
is launched. The `keto_diet` project still had a `.mcp.json` wired to the
`llm-wiki-test` instance, and this framework repo had no project `.mcp.json` at
all, even though production `llm-wiki 0.2.14` was installed and staged a Claude
template under `~/.llm_wiki/mcp/`.

The accepted product behavior is: `llm-wiki init` and `llm-wiki register` are
the normal repair/onboarding commands. They should add or update the llm-wiki
MCP server entry in project-local `.mcp.json` non-destructively, reconcile a
stale command path for the active server key, ensure Codex global wiring
idempotently, and leave manual copying of the staged Claude template as a
fallback only. Only the *active* instance's server entry is touched: a
different-instance entry (`llm-wiki` vs `llm-wiki-test`) is deliberately left
alone so the test and production servers coexist in one project.

## Choice

`llm-wiki init` and `llm-wiki register` shall wire the project's host-agent MCP
configuration as part of registering the project, so that the `llm-wiki` MCP
server is connected without a manual copy step. Specifically:

1. For **Claude Code**, write (or non-destructively merge into) a project-local
   `.mcp.json` in the project root, with the `llm-wiki[-test]` stdio server
   entry that `render_claude_project_mcp_config` already produces. This is the
   step a user formerly did by hand (`cp ~/.llm_wiki*/mcp/claude-project.mcp.json
   <project>/.mcp.json`).
2. For **Codex**, keep the existing global wiring (`merge_codex_config` into
   `~/.codex/config.toml`); `init`/`register` should ensure it is present rather
   than assume `install` already ran.
3. Make it **non-destructive and opt-outable**: merge into an existing
   `.mcp.json` (never clobber a user's other servers), no-op when the entry is
   already correct, and gate behind a flag if the project should stay unwired
   (e.g. `--no-mcp`, mirroring the existing `--no-register`).

`install` keeps staging the Claude template (for users who wire manually or use
a non-standard project layout), but it is no longer the *only* path, and the
staged file stops being a required manual step for the normal flow.

## Why

The asymmetry is not an oversight; it falls out of where each host keeps its
config, and the fix follows from the same fact:

- **Codex MCP config is global** — one file, `~/.codex/config.toml`. A global
  command (`install`) is the right owner, and it auto-wires it once.
- **Claude MCP config is per-project** — a `.mcp.json` in each project
  directory. A global command *cannot* know which projects exist, so the most
  `install` can honestly do is stage a template and point users toward
  project-scoped wiring. The former manual-copy path was exactly Blocker B1
  from the 2026-06-29 Claude field test: the test MCP server was never
  connected, so Phases A/B had no tools to exercise.
- **`init` and `register` are project-scoped** — they already operate on a
  specific project root (creating it, or registering an existing path). They
  are therefore the natural and correct owner of a per-project `.mcp.json`.
  Before this decision, they touched no MCP config at all, which is why the
  manual copy survived.

So the field-test friction — and the broader first-run experience for any Claude
Code user installing llm-wiki — is a missing hook, not a missing capability. The
renderer (`render_claude_project_mcp_config`) and the path
(`claude_project_mcp_config`) already existed; only the project-scoped call site
was absent. Moving the wiring to `init`/`register` makes Claude onboarding match
Codex's "it just works," with no change to the single-static-binary distribution
invariant and no new dependency: this wires *llm-wiki's own* MCP server, not
Headroom.

## The retired proxy-launch concern (no longer in scope)

An earlier draft of this decision set aside a *second*, separate concern: how a
user would launch the Headroom **proxy** alongside
onboarding. That entire posture — the four-mode proxy model, a materialized
`run-proxy.sh`, and the carve-out profile — was retired on 2026-06-30 (see
`wiki/decisions/headroom-single-posture-mcp-first.decision.md`). The framework
now ships no proxy launcher, and the only Headroom launch convenience is the
in-binary passthrough `llm-wiki headroom [--] <headroom args...>` (for example,
`llm-wiki headroom -- wrap codex`; see
`wiki/plans/headroom-passthrough-launcher.plan.md`), which is orthogonal to
onboarding. This decision is therefore solely about wiring *llm-wiki's own* MCP
server at `init`/`register`.

## Consequences

- `init`/`register` gain MCP side effects. They must be non-destructive (merge,
  never clobber), idempotent (no-op when already wired), and opt-outable
  (`--no-mcp`), and should emit a clear line about what they wrote and where.
- Writing into a user's project directory (`<project>/.mcp.json`) is a new
  outward effect for these commands. It is the user's own project, but the merge
  must preserve any non-llm-wiki servers already configured there.
- `register --update` and re-runs should reconcile the active server entry (e.g.
  when the managed binary path changes) in place rather than duplicate it. It is
  keyed by the active instance's server name, so switching instances adds the
  other instance's entry without disturbing the first — the two are meant to
  coexist.
- `doctor` should grow a check that, for a registered project, the host MCP
  config actually references the `llm-wiki[-test]` server — turning the silent
  B1 failure into a surfaced one.
- The staged template under `~/.llm_wiki*/mcp/claude-project.mcp.json` remains
  for manual/edge use; it is demoted from "required step" to "fallback."

## Resolved Implementation Notes

- **Project root vs. parent for `.mcp.json`.** Claude Code reads `.mcp.json`
  from the directory it is launched in. The implementation should document and
  test the project-root path: launch Claude from the project root. User-level
  `claude mcp add` support is a future option, not part of this decision.
- **Which command is authoritative.** Both `init` and `register` are
  authoritative project-scoped onboarding commands. They should call one shared
  wiring core so behavior cannot drift.
- **Codex re-wiring scope.** `init` and `register` should ensure the global
  Codex config idempotently as well as the Claude project config, so projects
  registered on machines with older installs still end up wired.
