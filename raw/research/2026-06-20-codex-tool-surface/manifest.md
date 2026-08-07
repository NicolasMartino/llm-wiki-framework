# Codex CLI Tool Surface Research Manifest

- Research Date: 2026-06-20
- Mode: URL research against `openai/codex` `main` branch
- Question: Which OpenAI function-tool names does the Codex CLI emit on the
  wire, and which of them appear in
  `chopratejas/headroom`'s `DEFAULT_EXCLUDE_TOOLS` set?
- Goal: Evidence the Codex-side carve-out names that
  `wiki/proposals/headroom-runtime-companion.proposal.md` needs to add to
  `HEADROOM_EXCLUDE_TOOLS` so wiki and raw reads on Codex bypass Headroom's
  compression pipeline.
- Trigger: Reviewer flagged that the proposal's Part 2 exclude list included
  Codex tool names (`shell`, `local_shell`, `shell_command`, `exec_command`,
  `write_stdin`, `apply_patch`) without any captured `raw/` source backing
  them, violating the framework's "every claim cites `raw/`" invariant.

## Source Inventory

All sources captured on 2026-06-20 from `openai/codex` at the `main`-branch
tip via `WebFetch` against `raw.githubusercontent.com/openai/codex/main/...`.
Each file was requested with a verbatim-quote prompt; the responses are
preserved here without paraphrase.

1. `sources/01-codex-handlers-shell-spec.rs` —
   `codex-rs/core/src/tools/handlers/shell_spec.rs` lines 1-257, captured
   verbatim. Declares four `ResponsesApiTool` tool specs: `exec_command`
   (line 88), `write_stdin` (line 138), `shell_command` (line 210),
   `request_permissions` (line 242). The omitted tail (lines 258-409) is
   helper functions (`unified_exec_output_schema`, `create_approval_parameters`,
   `permission_profile_schema`, `windows_shell_guidance`) and the
   `#[cfg(test)]` block — none declare additional tool names.
2. `sources/02-codex-handlers-apply-patch-spec.rs` —
   `codex-rs/core/src/tools/handlers/apply_patch_spec.rs` captured in full.
   Declares the `apply_patch` freeform tool (`FreeformTool { name:
   "apply_patch", ... }` at upstream line 18).
3. `sources/03-codex-handlers-mod.rs` —
   `codex-rs/core/src/tools/handlers/mod.rs`, only the submodule declarations
   and handler re-exports are captured (upstream lines 1-80). The omitted
   tail (lines 81-468) is helper functions and tests; the handler bindings
   that map handler structs to their `ToolName::plain(...)` strings live in
   the per-handler files, not in `mod.rs`. Captured to evidence the full
   inventory of registered handler modules.
4. `sources/04-codex-handlers-request-permissions.rs` —
   `codex-rs/core/src/tools/handlers/request_permissions.rs` lines 1-46
   captured verbatim, then the omitted `handle_call` body (upstream lines
   47-118). The captured `ToolExecutor` impl shows
   `ToolName::plain("request_permissions")` (upstream line 30), which is the
   binding to the literal tool name that arrives in the OpenAI tool_use
   envelope.

## What Was Not Found

- **`shell` as an OpenAI tool name.** Checked: `shell_spec.rs`,
  `shell.rs` (handler), `hosted_spec.rs`, `unified_exec.rs`,
  `registry.rs`, `function_tool.rs`. No `name: "shell"` literal declaration
  anywhere on current `main`. The proposal's reference to `shell` as an
  older Codex tool name could not be confirmed against `main`-tip on
  2026-06-20. It may exist on older release tags; that was not investigated.
- **`local_shell` as an OpenAI tool name.** Not found in any of the same
  files. The OpenAI Responses API has a built-in tool type `local_shell`
  (used by some hosted Codex models), but that is a `type` field, not a
  function `name`. Crucially, the captured Headroom router (see
  `raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/10-headroom-source-content-router.py`
  lines 1828, 1838) extracts the tool-name routing key only from
  `tool_call.function.name` (OpenAI) and `block.name` (Anthropic). It does
  not read OpenAI Responses-API built-in tool `type` values. An entry
  `local_shell` in `HEADROOM_EXCLUDE_TOOLS` would therefore be matched
  against a `name` that resolves to the empty string for Responses-API
  built-ins — i.e. it is dead under the captured router.
- GitHub's authenticated code search was not available to this research pass
  (`api.github.com/search/code` and `github.com/.../search` both required
  login). Absence claims above are scoped to the files listed in the
  preceding paragraph, not to the whole `openai/codex` repo.

## Local Project Context Reviewed

- `wiki/proposals/headroom-runtime-companion.proposal.md` (the proposal this
  research backs)
- `raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/10-headroom-source-content-router.py`
  (Headroom router tool-name extraction, lines 1812-1842)
- `raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/17-headroom-config-excerpt.py`
  (Headroom's `DEFAULT_EXCLUDE_TOOLS` set)
- `assets/skills/wiki-query/SKILL.md` (the framework's search-invocation
  template that both runtimes materialize)

## Retrieval And Verification Notes

- Each Rust file was fetched via `raw.githubusercontent.com/openai/codex/main/...`
  on 2026-06-20 with a prompt asking for verbatim line-by-line reproduction.
- Tool-name string literals (`name: "..."` inside `ToolSpec::Function(...)`
  or `ToolSpec::Freeform(...)`) were cross-checked against the
  `ToolName::plain("...")` binding in each handler file. For
  `request_permissions` the binding is explicit in
  `sources/04-...-request-permissions.rs:30`. For the three shell tools
  (`exec_command`, `write_stdin`, `shell_command`) the binding is implicit:
  `handlers/mod.rs` re-exports `ExecCommandHandler`, `WriteStdinHandler`,
  and `ShellCommandHandler`, and the corresponding `spec()` functions in
  `shell_spec.rs` carry the literal names.
- `openai/codex` does not have a tagged release matching the `headroom-ai
  v0.24.0` pinning style used by the parent Headroom research. The captures
  here are scoped to `main`-tip on the capture date; a follow-up plan
  should pin against a Codex release SHA at install time, the same way
  Acceptance Criterion 6 of the Headroom proposal pins the Headroom
  release.
