# poman Is MCP-Friendly, Like llm-wiki

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-07
- Category: poman design, MCP
- Scope: How agents reach poman: every poman command is also an MCP tool with
  the same behaviour, served by poman's own MCP server and registered beside
  llm-wiki's; which parts of llm-wiki's MCP server move to `llm-wiki-core` to
  be shared; and how a tool that writes says so.
- Sources:
  - The owner, 2026-10-07: "i think poman like llm wiki should mcp friendly"
  - Issue #57, "poman · Wiki: Record that poman is MCP-friendly", which
    records what was agreed with the owner the same day
  - `src/mcp/mod.rs`, `src/mcp_config.rs`, `src/mcp_wiring.rs`,
    `src/instance.rs` and `crates/llm-wiki-core/Cargo.toml` at `cad8988`, read
    for this decision
  - `wiki/plans/poman-mcp-server.plan.md` and the PR that built it (#62): the
    recheck of what moved and the open points answered
- Related:
  - `wiki/decisions/poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`:
    poman's own model knows no tracker; GitHub is one adapter
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`: the shared
    crate, the strictest gates, and `llm-wiki install` putting poman beside
    llm-wiki
  - `wiki/decisions/mcp-onboarding-at-init-register.decision.md`: where
    llm-wiki's server is wired into hosts
  - `wiki/decisions/test-instance-namespaced-binary.decision.md`: the test
    instance's own server and tool names
  - `wiki/roadmaps/poman.roadmap.md`, PM3.1, poman's MCP server

## Decision

**Every poman command is also an MCP tool, with the same behaviour and a JSON
result, served by `poman mcp`, poman's own server, which `llm-wiki install`
registers beside llm-wiki's. The MCP plumbing is shared through
`llm-wiki-core`. A tool that writes says so, and a push, command or tool,
applies only the changeset a diff showed first.**

### One command, one tool

- **Each poman command has a tool**, named after it: `poman new deadline` is
  `poman_new_deadline`, `poman check` is `poman_check`, and so on for the
  commands each later entry adds. In Claude Code they show as
  `mcp__poman__…`.
- **Off a terminal**: a tool behaves as its command does when no terminal is
  attached: it asks nothing, and a call missing a mandatory field is refused
  with every missing argument named, as `poman new deadline` refuses off a
  terminal (`wiki/plans/poman-deadline-type.plan.md`).
- **Same behaviour**: a tool does what its command does, with the same checks,
  the same refusals and the same files written; its result is the command's
  JSON output.
- **Two interfaces to the same commands**: the CLI and the MCP server sit over
  one core, as GitHub is one adapter over poman's model
  (`poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`).

### Its own server

- **`poman mcp` serves poman's tools over stdio**, as `llm-wiki mcp serve`
  serves llm-wiki's. The two servers stay separate: each binary serves its own
  commands.
- **`llm-wiki install` registers it beside llm-wiki's**, in the same host
  configurations, since install already puts poman beside llm-wiki.

### Writes are explicit

- **A tool that writes says so** in its description and its result:
  `poman_new_deadline` says it writes a deadline file and names the file it
  wrote.
- **A push shows before it applies, command and tool alike**: `poman push`
  and `poman_push` behave the same. Both apply only the changeset `poman
  diff` wrote and showed, so nothing reaches the tracker that a diff did not
  show first, and calling push is the ask to apply it
  (`poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`, "Fetch,
  diff, push"). The tool's description says it writes to the tracker, and its
  result names what it applied.

### What moves to `llm-wiki-core`

llm-wiki's server is one module today, `src/mcp/mod.rs`, with its host
wiring in `src/mcp_config.rs` and `src/mcp_wiring.rs`. A first list of what
is general and moves to the shared crate, to be rechecked by whoever does the
work:

- the stdio transport: reading one JSON-RPC message per line with a size cap,
  skipping a malformed line, and writing each reply;
- the JSON-RPC envelope: success and error replies, and dispatching
  `initialize`, `tools/list` and `tools/call` to the server's own handlers;
- running a command as a tool: calling the binary with the command's
  arguments and JSON output, bounding a failed call's error output, and
  marking a failed call as a tool error (`run_cli` and its helpers);
- rendering and merging a host's MCP configuration for a server, given its
  name and binary (`src/mcp_config.rs`), today written for llm-wiki's name
  alone.

What stays with llm-wiki: its tools and their schemas, `llm_wiki_read`, its
resources and prompts, its server instructions, and its test-instance names.
poman's tools, schemas and instructions live in the poman crate.

Rechecked in PM3.1 (`wiki/plans/poman-mcp-server.plan.md`): the stdio
transport, the envelope and the shared dispatch moved, as
`llm_wiki_core::mcp`, a server bringing its own name, instructions, tools and
further methods through one trait. Running a command as a tool and the host
configurations stayed with llm-wiki: poman's tools call poman's own `run`
in-process with the command's arguments and `--json`, and only llm-wiki's
`install`, `init` and `register` write host configurations, which
`src/mcp_config.rs` now renders for any server, by name, binary and
arguments.

## Why

- **poman's users are mostly agents**, and agents reach tools through MCP
  first; llm-wiki already works this way.
- **MCP is one more interface**, in line with poman's core being defined
  without any interface in it: the CLI, GitHub and MCP sit around the same
  model.
- **One command, one tool, one result** means an agent and a person get the
  same answer, and a test of one proves the other.
- **Explicit writes** keep an agent from changing files or a tracker as a side
  effect of a call it read as a question.
- **Sharing the plumbing** keeps the two servers from drifting in how they
  frame messages, report failures and register themselves.

## Alternatives Considered

- **poman's tools served by llm-wiki's server.** Not chosen: it ties poman's
  tools to llm-wiki's release and to llm-wiki being installed, and blurs which
  binary owns which behaviour.
- **MCP tools only for some commands.** Not chosen: an agent would have to
  know which commands are tools and fall back to the shell for the rest.
- **Copying llm-wiki's server code into poman.** Not chosen: two copies drift.

## Consequences

- PM3.1 in `wiki/roadmaps/poman.roadmap.md` builds the server for the
  commands of PM3, and each later entry's commands ship as tools too.
- The shared plumbing brought one dependency into `llm-wiki-core`,
  `serde_json`, under the strictest gates and the deny gate.
- The shared code is held to the strictest gates when it moves, as the page
  reader was in PM2, ahead of PM8's ratchet for the rest of llm-wiki.
- Answered in PM3.1's plan: poman has no test instance of its own, and its
  registration follows llm-wiki's instance, `poman` beside `llm-wiki` and
  `poman-test` beside `llm-wiki-test`, so a test install never replaces the
  real registration; hosts are wired for poman wherever llm-wiki's server is
  (install's Codex and staged Claude configurations, and the project
  `.mcp.json` that `init` and `register` merge); and `uninstall` removes
  poman's registration with llm-wiki's. In Claude Code the tools show as
  `mcp__poman__poman_check` and `mcp__poman__poman_new_deadline`.

## What Would Revisit This

- A poman command with no behaviour off a terminal (one that cannot run
  without asking a person something mid-run).
- A host that cannot run two MCP servers side by side.
