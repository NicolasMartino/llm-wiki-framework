# Codex CLI Tool Surface Research Summary

- Research Date: 2026-06-20
- Status: Ready for ingest into
  `wiki/proposals/headroom-runtime-companion.proposal.md`
- Scope: Evidence the Codex-CLI-side tool names that the Headroom Runtime
  Companion proposal needs to exclude from compression so wiki and raw
  reads, plus `llm-wiki search` results, bypass the proxy on Codex sessions.

## Executive Finding

Four OpenAI function tool names are declared at `openai/codex` `main`-tip on
2026-06-20 and are the load-bearing carve-out keys for Codex on Headroom's
proxy:

- `shell_command` — `codex-rs/core/src/tools/handlers/shell_spec.rs:210`
- `exec_command` — same file, line 88
- `write_stdin` — same file, line 138
- `apply_patch` — `codex-rs/core/src/tools/handlers/apply_patch_spec.rs:18`

A fifth tool, `request_permissions`, is also declared
(`shell_spec.rs:242`, bound by
`request_permissions.rs:30`). It is not a bulky-output tool so its
exclusion has minor practical impact, but adding it costs nothing and keeps
the exclude list a complete inventory of Codex-emitted tool names.

The proposal's two other Codex-name candidates — `shell` and `local_shell`
— do **not** appear in the files inspected. `shell` is not present in
`shell_spec.rs`, `shell.rs`, `hosted_spec.rs`, `unified_exec.rs`,
`registry.rs`, or `function_tool.rs`. `local_shell` is similarly absent and
is likely a confusion with the OpenAI Responses-API built-in tool *type*
`local_shell` (used by some hosted Codex models), not a function tool
*name*. Crucially, the captured Headroom router only extracts the routing
key from `tool_call.function.name` and `block.name`
(`sources/10-headroom-source-content-router.py:1828`, `:1838`), not from
Responses-API tool `type` fields — so even if a Codex session emits a
`type: "local_shell"` invocation, an entry `local_shell` in
`HEADROOM_EXCLUDE_TOOLS` would compare against an empty string and never
match.

## How Codex Routes File and Search Operations

Codex has no native read tool. The `handlers/` directory contains no
`read.rs` and no `Read`/`read_file` tool spec. Codex performs file reads by
running shell commands (`cat`, `sed`, `head`) through `shell_command` or
`exec_command`. It edits files via `apply_patch`.

This means:

- A user query that asks Codex to read `wiki/index.md` is, on the wire, an
  `exec_command` or `shell_command` tool_call whose `function.name` is the
  literal string `"exec_command"` or `"shell_command"`.
- The same applies to `llm-wiki search --mode auto --format json
  "<question>"` — the framework's shared `assets/skills/wiki-query/SKILL.md`
  template invokes the binary through the host's shell tool on both
  runtimes; on Claude Code that is the `Bash` tool, on Codex that is
  `shell_command` or `exec_command`.

Headroom's `DEFAULT_EXCLUDE_TOOLS` (captured at
`raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/17-headroom-config-excerpt.py`)
covers Claude's vocabulary (`Read`, `Glob`, `Grep`, `Write`, `Edit`, `Bash`,
plus lowercase variants). It does not cover any Codex name. Without the
profile additions in the Headroom Runtime Companion proposal's Part 2,
every wiki read, raw read, and search call on a Codex session is eligible
for compression on `main`-tip Headroom.

## What This Means For The Proposal

1. Drop `shell` from `HEADROOM_EXCLUDE_TOOLS`. It is not a tool name on
   current `main`. If a future Codex release reintroduces it, the install-
   time pin (Acceptance Criterion 6) will catch the change and the profile
   can be amended then.
2. Drop `local_shell` from `HEADROOM_EXCLUDE_TOOLS` — or keep it with a
   comment explicitly noting that it is dead under the captured Headroom
   router and is included only as forward compatibility for a future
   Headroom that routes by OpenAI Responses-API tool `type`. Either is
   defensible; the cleaner answer is to drop it and re-add only when a real
   Headroom release extracts `type` for routing.
3. Add `request_permissions` to `HEADROOM_EXCLUDE_TOOLS`. Verified Codex
   tool name; safe to exclude; completes the inventory.
4. Fix the proposal's citation paths: replace
   `codex-rs/core/src/openai_tools.rs` (which does not exist) with the
   per-handler files captured here.
5. Reshape Acceptance Criterion 9 so the Codex-name fixture test pins
   against `raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs`
   and `02-codex-handlers-apply-patch-spec.rs`, not against the bundled
   profile alone. Otherwise the test catches local profile edits but not
   upstream Codex renames.

## Gaps

- No older Codex release tags were inspected. The proposal's claim that
  `shell` was an earlier name could be true historically; this research did
  not check tag history.
- GitHub's authenticated code-search APIs were unavailable in this pass.
  Absence claims are scoped to the files listed above, not the full repo.
- The Headroom router's behavior with Responses-API built-in tools was
  inferred from the captured tool-name extraction code, not from a live
  test. A small dogfood run sending a `type: "local_shell"` request through
  the proxy would settle the question definitively.
