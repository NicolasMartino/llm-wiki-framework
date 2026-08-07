# Eval: MCP-First Host UX And Ordinary-Language Parity

- Document Class: Eval
- Status: Rejected
- Date: 2026-06-23
- Category: MCP host UX, no-legacy parity
- Scope: Phase 0 host capability spike and Phase 5 ordinary-language parity evidence for the MCP-first agent surface.
- Sources: `wiki/plans/mcp-first-agent-surface.plan.md`; local command transcripts from `codex --version`, `codex mcp get/list`, `codex exec --json`, `claude --version`, and `claude -p --output-format stream-json --verbose --strict-mcp-config --mcp-config <staged config>` runs captured under `target/mcp-eval/claude-*.jsonl`; a direct `prompts/list` JSON-RPC probe of `llm-wiki mcp serve`.

## Objective

Determine whether the no-legacy MCP surface can replace generated skill
projection on the two supported harnesses, Claude Code and Codex, using real
host clients rather than only in-process Rust tests.

## 2026-06-23 Repair Evidence

The merge-readiness repair pass fixed the code blockers that previously made
full-instance parity evidence unsafe to collect:

- The `test` instance now uses `llm-wiki-test` and suffixed MCP tool names, with
  production output unchanged.
- `just test-instance-live-session-proof` passes, validating the test MCP
  config, then uninstalling and comparing the real Codex config to its pre-run
  copy.
- `tools/with-repo-skills-disabled.sh rtk cargo test --test mcp` passes, proving
  the transitional no-repo-skill posture can move `.claude/skills` and
  `.codex/skills` aside and restore them.
- MCP search/read behavior remains covered by `cargo test --test mcp` and
  `cargo test --test search_commands`; test-instance MCP behavior is covered by
  `LLM_WIKI_INSTANCE=test cargo test --test mcp`.

This does not by itself move the eval to `Accepted`: a clean ordinary-language
Claude Code host run without generated/user/project skills is still required,
or the remaining blocker must be explicitly narrowed to an external host/auth
condition.

## Setup

- Built the current debug binary with `cargo build`.
- Codex CLI version: `codex-cli 0.141.0`.
- Claude Code version: `2.1.176 (Claude Code)` for the original 2026-06-23 attempt;
  `2.1.177 (Claude Code)` for the 2026-06-23 follow-up run that produced the
  Claude evidence below.
- MCP command under test:
  `/Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework_headroom_impl/target/debug/llm-wiki mcp serve`.
- Codex config override used:
  `mcp_servers.llm-wiki.command=<debug llm-wiki>`,
  `mcp_servers.llm-wiki.args=["mcp","serve"]`.
- For registered-search proof, the MCP server process used isolated
  `HOME=target/mcp-eval/home`; that home was installed with
  `llm-wiki install --skip-path-guidance --disable-llm-search`, registered as
  project `mcp-eval`, and indexed with 91 wiki files.
- Claude used `target/mcp-eval/claude-project.mcp.json` as the staged
  project MCP config. The follow-up registered-search run used
  `target/mcp-eval/claude-project-isolated.mcp.json`, which is the same config
  with an `env.HOME` pointing at the isolated registered/indexed
  `target/mcp-eval/home` so the spawned MCP server resolves the `mcp-eval`
  project (re-indexed to 92 files for this run).
- Claude follow-up runs were non-interactive headless:
  `claude -p ... --strict-mcp-config --mcp-config <config>
  --allowedTools <llm-wiki MCP tools> --disallowedTools Bash[,Skill,Read,...]
  --output-format stream-json --verbose`. `--strict-mcp-config` isolated the run
  to only the `llm-wiki` server (the operator's own session also has `headroom`
  and `serena` MCP servers, which were excluded). `--allowedTools` pre-approved
  the MCP tools so headless runs did not stall on permission prompts.
- The clean no-skill parity run additionally used an isolated
  `CLAUDE_CONFIG_DIR=target/mcp-eval/claude-clean-config` (no user-level skills)
  authenticated with a `CLAUDE_CODE_OAUTH_TOKEN` from `claude setup-token`, and
  the repo-local `.claude/skills` directory was moved aside for the duration of
  the run and restored immediately afterward. The run executed from the repo
  root so `AGENTS.md` project guidance loaded and the registered `mcp-eval`
  project resolved, but no user-level or project-level skills were present.
  `--allowedTools` deliberately still listed `Skill`, `Read`, `Grep`, and `Glob`
  so the run measured natural routing rather than forcing the MCP path; the init
  event confirmed zero `wiki`/`knowledge` slash commands were available.

## Host Capability Observations

Codex:

- `codex mcp get llm-wiki` succeeded after using the unquoted config key
  `mcp_servers.llm-wiki.*`. The earlier quoted key
  `mcp_servers."llm-wiki".*` created a literal server named `"llm-wiki"`.
- Non-interactive `codex exec` cancels MCP tool calls unless run with
  `--dangerously-bypass-approvals-and-sandbox`; with that automation flag,
  `llm_wiki_read`, `llm_wiki_search`, and `llm_wiki_status` executed.
- MCP resources are exposed. A resource probe listed
  `llm-wiki://project/wiki/index.md`.
- MCP prompts are not exposed to the model in this Codex surface. A prompt
  support probe successfully called `llm_wiki_status` but reported no prompt
  list/read capability.

Claude Code (original 2026-06-23 attempt, v2.1.176):

- `claude -p --mcp-config target/mcp-eval/claude-project.mcp.json` initialized
  with `mcp_servers` containing `llm-wiki` in `pending` state.
- The run stopped before any model or MCP tool execution because that host was
  not logged in: `apiKeySource: "none"` and result
  `authentication_failed` / `Not logged in · Please run /login`.

Claude Code (follow-up 2026-06-23 run, v2.1.177, authenticated host):

- The auth blocker was machine-specific, not a Claude Code limitation. On an
  authenticated host the headless run still reports `apiKeySource: "none"` in the
  `system/init` event, but the request succeeds via subscription OAuth
  (`result.is_error: false`). A trivial `claude -p` probe returned its expected
  literal answer, confirming headless auth works without an `ANTHROPIC_API_KEY`.
- MCP tools are exposed and execute. Tool-call events appear in `stream-json`
  output as `tool_use` entries named `mcp__llm-wiki__llm_wiki_read` and
  `mcp__llm-wiki__llm_wiki_search`. Both executed with zero `permission_denials`
  when pre-approved via `--allowedTools`.
- MCP resources are exposed. Claude surfaces the host-provided
  `ListMcpResources` and `ReadMcpResource` tools. `ListMcpResources` returned 10
  `llm-wiki://project/...` resources (`wiki/index.md`, `wiki/log.md`,
  `AGENTS.md`, `templates/base/project_guidelines.md`, and the six
  `wiki/specs/*.spec.md` operation specs); `ReadMcpResource` read
  `llm-wiki://project/wiki/index.md` successfully.
- MCP prompts are advertised by the server but were not auto-exposed by this
  Claude surface. A direct JSON-RPC `prompts/list` against `llm-wiki mcp serve`
  reports `prompts` capability and five prompts (`wiki_query`, `wiki_ingest`,
  `wiki_lint`, `wiki_research`, `wiki_init`). In the headless run, the
  `system/init` `slash_commands` list contained no `mcp__llm-wiki__*` prompt
  entries, so the prompts did not appear as routable slash commands in this
  non-interactive surface. Claude documents MCP prompts as slash commands
  (`/mcp__<server>__<prompt>`), which is an explicit-invocation surface and, like
  Codex, does not constitute ordinary-language routing.

## Transcript Summary

Codex explicit MCP read:

- Prompt explicitly required the configured `llm-wiki` MCP server and no shell.
- Transcript contained an `mcp_tool_call`:
  `server="llm-wiki"`, `tool="llm_wiki_read"`,
  `arguments={"path":"wiki/index.md"}`, `status="completed"`.
- Final answer reported: `MCP-First Agent Surface Implementation Slice —
  Active`.

Codex ordinary-language query:

- Prompt: "Query the project wiki and answer briefly..." without naming a prompt
  or tool.
- Routing primitive was mixed: the user-installed `wiki-query` skill loaded
  first, then the answer used MCP calls.
- Transcript contained `llm_wiki_read` calls for `wiki/index.md` and
  `wiki/proposals/mcp-first-surface.proposal.md`.
- Transcript attempted `llm_wiki_search`; before isolated registration it
  returned a normal tool failure explaining the current project was not
  registered. This verifies model-visible failure behavior.
- Final answer correctly reported the plan as Active and identified Phase 0 and
  Phase 5 evidence as remaining.

Codex registered search capability:

- With isolated registered/indexed MCP home, `llm_wiki_search` succeeded.
- Final answer reported top result
  `wiki/plans/mcp-first-agent-surface.plan.md`, backend ready/fresh, selected
  mode `lexical`, and no fallback or zero-result reason.

Codex no-skill capability probe:

- Prompt explicitly forbade skills and shell commands.
- Transcript used only `llm-wiki` MCP reads and produced the correct Active
  plan status.
- This is a capability proof, not ordinary-language parity, because the prompt
  named the no-skill/no-shell constraint.

Claude Code parity attempt (original, auth-blocked):

- Transcript initialized Claude Code and loaded the staged MCP config.
- No MCP tool/resource/prompt call occurred because authentication failed before
  execution.

Claude Code explicit MCP read (`target/mcp-eval/claude-explicit-mcp-stream.jsonl`):

- Prompt explicitly required the `llm-wiki` MCP server and forbade shell.
- Transcript contained a `tool_use` for `mcp__llm-wiki__llm_wiki_read` reading
  `wiki/index.md`; result `is_error: false`.
- Final answer correctly reported the plan title `MCP-First Agent Surface
  No-Legacy Cutover` and status `Active`, and named Phase 0 and Phase 5 as the
  remaining gates.

Claude Code ordinary-language query (`target/mcp-eval/claude-ordinary-query-stream.jsonl`):

- Prompt: "Query the project wiki and answer briefly..." without naming a tool,
  skill, or prompt.
- Routing primitive was the user-installed skill: the transcript invoked the
  `Skill` tool with `skill: "wiki-query"`, then used the filesystem `Read` tool,
  and did not call any `llm_wiki_*` MCP tool.
- Final answer was correct (both phases open; plan blocked from closing), but
  this is not a fresh no-legacy project-guidance-only route. Like Codex, the
  installed skill intercepted the ordinary request before MCP tools were used.

Claude Code no-skill registered search (`target/mcp-eval/claude-registered-search-stream.jsonl`):

- Prompt forbade skills, shell, and filesystem reads (`--disallowedTools
  Bash,Skill,Read,Grep,Glob`), leaving only `llm-wiki` MCP tools.
- Transcript invoked `mcp__llm-wiki__llm_wiki_search` directly with zero
  permission denials, against the isolated registered/indexed home.
- Final answer reported backend `state: ready`, `freshness: fresh`, lexical mode
  (semantic disabled), 92 indexed files, backend `qmd-rs`, and the top result
  `wiki/proposals/mcp-first-surface.proposal.md` (the closely-named proposal
  outranked the plan, which placed fourth). This is a capability proof, not
  ordinary-language parity, because the prompt named the no-skill constraint.

Claude Code MCP resource access (`target/mcp-eval/claude-resource-stream.jsonl`):

- Prompt forbade skills, shell, and filesystem reads.
- Transcript invoked `ListMcpResources` and `ReadMcpResource`; the answer listed
  all 10 `llm-wiki://project/...` resources and read `wiki/index.md`, reporting
  its first heading `# Wiki Index`.

Claude Code clean no-skill ordinary-language query (`target/mcp-eval/claude-noskill-ordinary-stream.jsonl`):

- Decisive parity run. No user-level skills (isolated `CLAUDE_CONFIG_DIR`) and no
  project-level skills (`.claude/skills` moved aside); init confirmed zero
  `wiki`/`knowledge` slash commands. `Skill`, `Read`, `Grep`, and `Glob` were all
  allowed but had nothing skill-shaped to route to.
- Prompt was the same ordinary request as the skills-present run ("Query the
  project wiki and answer briefly..."), naming no tool, skill, or prompt.
- The model routed through project guidance directly to the MCP surface: the only
  primitive used was `mcp__llm-wiki__llm_wiki_read` (no `Skill`, no filesystem
  `Read`). Result `is_error: false`.
- Final answer was correct: plan `Active`, with Phase 0 and Phase 5 named as the
  two open evidence gates.
- This is ordinary-language parity for the query workflow on Claude Code: with
  the legacy skill absent, `AGENTS.md` plus the MCP read tool carried the request
  without a generated skill. The repo-local skills were restored immediately
  after the run and `git status` for `.claude` was clean.

Codex clean no-skill ordinary-language query
(`target/mcp-eval/codex-noskill-ordinary.jsonl`, Codex CLI 0.142.0):

- Matching clean run for Codex. No user-level skills (isolated
  `CODEX_HOME=target/mcp-eval/codex-clean-home` with a symlinked `auth.json` and
  no `skills/` dir) and no project-level skills (repo `.codex/skills` moved aside
  and restored after). Run was `codex exec
  --dangerously-bypass-approvals-and-sandbox --json`, which is required because
  non-interactive Codex otherwise cancels MCP tool calls.
- Same ordinary prompt as the Claude run, naming no tool, skill, or prompt.
- Routing was MCP-only: the JSONL contained exactly 10 `mcp_tool_call` events,
  all `server: "llm-wiki"`, `tool: "llm_wiki_read"` (reading
  `templates/base/project_guidelines.md`, `wiki/index.md`, the proposal, the
  plan, and this eval). There were no `exec_command`, shell, `function_call`, or
  skill invocations. The `wiki-query`/`skill` strings in the transcript appear
  only inside the content of the wiki pages Codex read, not as tool calls.
- Final answer was correct: plan `Active`, Phase 0 and Phase 5 named as the open
  gates, and it even summarized this eval's current state.
- This is ordinary-language parity for the query workflow on Codex: with skills
  absent, `~/.codex/AGENTS.md`-class project guidance plus the MCP read tool
  carried the request. The repo-local `.codex/skills` was restored and `git
  status` for `.codex` was clean.

## Verdict

Rejected for Phase 5 closure.

Both harnesses now have proven host support for MCP tools and resources when
configured correctly. Claude Code's earlier auth blocker was machine-specific:
on an authenticated host, headless `claude -p` executes `llm-wiki` MCP tools
(`llm_wiki_read`, `llm_wiki_search`) and resources (`ListMcpResources`,
`ReadMcpResource`) successfully. Codex executes the same tool/resource classes
with the automation approval flag.

Claude Code now clears the ordinary-language bar for the query workflow. With
user-installed skills present, an ordinary "query the wiki" request routes
through the `wiki-query` skill (and filesystem `Read`) before any MCP tool. But
on a clean profile with no user-level or project-level skills, the same ordinary
request routed through project guidance (`AGENTS.md`) directly to
`mcp__llm-wiki__llm_wiki_read`, with no skill and no filesystem read, and
answered correctly. This is the no-legacy project-guidance-plus-MCP route the
plan requires, proven for Claude Code's query path without forcing the MCP tool
via `--disallowedTools`.

Codex now matches. With user-installed skills present, its ordinary query routed
through the `wiki-query` skill before MCP. On a clean profile with no user-level
or project-level skills, the same ordinary request routed entirely through
`llm-wiki` `llm_wiki_read` (10 MCP reads, zero skill or shell invocations) and
answered correctly. So both harnesses prove the no-legacy project-guidance-plus-
MCP route for the query workflow on a skill-free profile.

Prompt support diverges but does not change the verdict. The server advertises
five MCP prompts. Codex did not expose prompts at all. Claude Code documents
prompts as `/mcp__<server>__<prompt>` slash commands but did not surface them in
the non-interactive run; either way, a slash command is explicit invocation, not
ordinary-language routing, so neither harness needs prompts to clear the Phase 0
parity bar.

Why this is still Rejected for Phase 5 closure: cross-harness ordinary-language
parity is now proven for the **query** workflow on clean no-skill profiles on
both harnesses, but Phase 5 acceptance requires more than query. The following
ordinary-language cases are still unrecorded on a skill-free profile: ingest or
lint **mutation** with `wiki/index.md`/`wiki/log.md` bookkeeping diffs, search
**fallback/readiness** reporting, and **failure** behavior when a project is
unregistered or the search cache is unavailable. The remaining work is now narrow
and well-scoped: the Claude auth blocker is resolved, and query parity is proven
on both harnesses; what is left is workflow breadth, not host capability or
routing.

## Required Next Action

1. Done: the authenticated Claude Code run confirmed MCP tool and resource
   execution, and the clean no-skill run proved ordinary-language query routing
   through project guidance plus the MCP read tool.
2. Done: Codex ran from a matching clean no-skill profile and routed the same
   ordinary query entirely through `llm-wiki` MCP reads. Cross-harness
   ordinary-language **query** parity is proven on skill-free profiles.
3. Remaining for Phase 5 — extend ordinary-language coverage beyond query on a
   clean no-skill profile on both harnesses: ingest or lint **mutation** in an
   isolated project (recording `wiki/index.md` / `wiki/log.md` diffs), search
   **fallback/readiness**, and **failure** behavior when a project is
   unregistered or the search cache is unavailable.
4. Accept Phase 5 only after both harnesses pass query, read/search, failure,
   and mutation workflows from ordinary-language requests on a clean no-skill
   profile.
