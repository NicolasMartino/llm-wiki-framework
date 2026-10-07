# Plan: poman's MCP Server

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-10-07
- Category: poman development
- Scope: Carry out PM3.1 of the poman roadmap: `poman mcp` serving PM3's
  commands as MCP tools with the same behaviour and a JSON result, the MCP
  plumbing both servers use moved into `llm-wiki-core`, `llm-wiki install`
  registering poman's server beside llm-wiki's and `uninstall` removing it,
  the decision's open points answered, and a local release the owner runs and
  can revert.
- Sources:
  - `wiki/roadmaps/poman.roadmap.md`, PM3.1, and issue #60
  - `wiki/decisions/poman-is-mcp-friendly.decision.md`: one command, one
    tool; its own server; explicit writes; what moves to `llm-wiki-core`; the
    open points this plan answers
  - `wiki/plans/poman-deadline-type.plan.md`: the commands this server serves
  - `src/mcp/mod.rs`, `src/mcp_config.rs`, `src/mcp_wiring.rs`,
    `src/install.rs`, `src/uninstall.rs`, `src/instance.rs`, `src/paths.rs`,
    `src/registry/mod.rs` and the `justfile` at `51f2116`, the state this plan
    was written from, kept in git history
  - The owner, 2026-10-07: PM3 and PM3.1 are built together, in one pull
    request, to avoid back and forth; this plan is approved with that pull
    request's verdict
  - The owner, 2026-10-07: a local release the owner runs, built with
    cargo-dist, two archives per target, with a way back; workers never touch
    the riseon repository
- Related:
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`: the shared
    crate and the strictest gates
  - `wiki/decisions/mcp-onboarding-at-init-register.decision.md`: where
    llm-wiki's server is wired into hosts
  - `wiki/decisions/test-instance-namespaced-binary.decision.md`: the test
    instance's own server names

## What This Proves

An agent reaches poman as it reaches llm-wiki: through MCP, with the same
answers a person gets from the command line. Each tool is its command run with
`--json`, so a test of one proves the other. Both servers frame messages and
report failures one way, because that code is written once, in the shared
crate, under the strictest gates. One `llm-wiki install` puts both binaries in
place and registers both servers, and `uninstall` takes both away.

## Where It Stands (2026-10-07, after PM3.1)

- **`llm_wiki_core::mcp`** holds the stdio transport, the JSON-RPC envelope
  and the shared dispatch; llm-wiki's server (`src/mcp/mod.rs`) and poman's
  (`crates/poman/src/mcp.rs`) each implement its `Server` trait.
- **`poman mcp` serves `poman_check` and `poman_new_deadline`**, each running
  its command with `--json` through `poman::run_in`, never as a terminal.
- **`src/mcp_config.rs` renders any server's entry** (`McpServer`), and
  `install`, `init`, `register` and `uninstall` handle llm-wiki's and poman's
  together.
- **`just local-release` and `just local-release-revert`** run
  `tools/local-release.sh`, proved under a temporary `HOME`.

The state this plan was written from, at `51f2116`, is in git history.

## Target

### What moves to `llm-wiki-core`

The decision's first list, rechecked:

- **Moves:** the stdio transport (one JSON-RPC message per line, capped at
  16 MiB, an oversized line drained and answered with a parse error, a line
  that is not UTF-8 or is blank skipped, a reply flushed per message); the
  JSON-RPC envelope (success, error, error with data); the dispatch every
  server shares (`ping`, `initialize`, `tools/list`, `tools/call`,
  notifications never answered, a missing or unknown method refused), with
  the rest of the methods handed to the server; and a tool's result (its JSON
  as text, `isError` set on failure). The module is `llm_wiki_core::mcp`; a
  server implements one trait for its name, instructions, tools and calls.
- **Stays with llm-wiki, after the recheck:**
  - `run_cli` and its bounded error output: poman calls its own `run`
    in-process with the command's arguments, so nothing else would use it;
  - rendering and merging host configurations: only `llm-wiki install`,
    `init` and `register` write them, never poman. They are made general
    over the server (its name, binary and arguments) in `src/mcp_config.rs`,
    so llm-wiki writes both servers' entries with one code path.
- **The dependency it brings:** `serde_json` only, already in the workspace
  and in the lock file, parsed without serde's derive.
- **llm-wiki's server reads through it**: its resources, prompts, tools and
  instructions stay in `src/mcp/mod.rs`, and its MCP tests pass unchanged.

### `poman mcp` and its tools

- **`poman mcp` serves over stdio** until its input ends, through the shared
  transport; server name `poman`, its version, and instructions naming its two
  tools and that `poman_new_deadline` writes.
- **`poman_check`** takes no argument and runs `poman check --json` where the
  server runs, the repository found as the command finds it.
- **`poman_new_deadline`** takes `title` (required) and `status`,
  `deadline`, `duration`, `importance`, `blocked_by`, `track`, `who` and
  `slug`, all strings, and runs `poman new deadline --json` with the matching
  flags. Its description says it writes a deadline file; its result names the
  file written.
- **Same behaviour:** each tool calls poman's `run` with the command's
  arguments, never as a terminal, so a mandatory field left out is refused
  naming every flag missing, as the command off a terminal is. The tool's
  result is the command's JSON output; `isError` is set when the command's
  exit code is not 0. An argument of the wrong type is an invalid-params
  error, as llm-wiki's tools give.

### JSON output of PM3's commands

- **`--json` on `poman check` and `poman new deadline`** prints one JSON
  object on standard output instead of the text; the exit codes do not change.
- **`poman check --json`:** `files_checked`, `errors`, `warnings`, and
  `findings`, each with `path`, `line`, `severity` (`error` or `warning`)
  and `message`, in the text output's order.
- **`poman new deadline --json`:** `written`, the file's path from the
  repository root, and `landing_branch`.
- **A refusal or failure under `--json`** prints
  `{"error": {"code": <exit code>, "message": "…"}}`.

### Registration

- **Wherever llm-wiki's server is wired, poman's is wired beside it:** in the
  Codex config and the staged Claude config `install` writes, and in the
  project `.mcp.json` that `init` and `register` merge, one write per file.
  The entry runs the managed poman (`~/.llm_wiki/bin/poman`) with `mcp`.
- **The server's name is `poman`**, so Claude Code shows its tools as
  `mcp__poman__poman_check` and `mcp__poman__poman_new_deadline`; under the
  test instance it is `poman-test`, beside `llm-wiki-test`.
- **`uninstall` removes poman's entry from the Codex config** with
  llm-wiki's, and the staged Claude config holding both goes with the
  manifest's assets.

### The decision's open points, answered

1. **A test instance for poman: none of its own.** poman's registration
   follows llm-wiki's instance, `poman-test` beside `llm-wiki-test`, so a test
   install never replaces the real registration; its binary and tool names
   do not change. Not chosen: a `poman-test` binary, which `install` would
   have to build and find beside a test llm-wiki.
2. **Where hosts are wired: everywhere llm-wiki's server is** (install,
   `init`, `register`), above. Not chosen: install's configurations only,
   which leaves Claude Code without poman, since it reads the project's
   `.mcp.json`.
3. **Uninstall: removes poman's registration with llm-wiki's**, above.

### The local release and its revert

The owner's decision, 2026-10-07: the owner tries a build on this machine
before any real release.

- **`just local-release`** builds the release archives with cargo-dist for
  this machine's target (`dist build --artifacts=local`, no tag, nothing
  published), two archives, llm-wiki's and poman's; unpacks both into one
  temporary folder; saves what is installed now; and runs that folder's
  `llm-wiki install`, which puts both binaries in `~/.llm_wiki/bin` and
  registers both servers.
- **What it saves first**, in a dated folder under
  `~/.llm_wiki/local-release/`: every file of the managed home except the
  models and indexes, every file the manifest lists, and the Codex config with
  its backup, with a list of which existed.
- **`just local-release-revert`** restores the newest saved state exactly:
  each saved file put back, and each file the release added (one the saved
  list did not hold, among the same places and the files the release's
  manifest lists) removed.
- **Both commands are in `README.md`.** Neither is ever run here against the
  real `~/.llm_wiki`, which serves every session on this machine; they are
  proved under a temporary `HOME`.
- **The owner's test follows:** with the local release installed, the owner
  uses poman on the riseon repository; no worker touches that repository.

## Phases

1. **The shared plumbing:** `llm_wiki_core::mcp` with its tests, llm-wiki's
   server reading through it; `just strict` and llm-wiki's MCP tests pass.
2. **`poman mcp`:** the `--json` output of PM3's commands, the server and its
   two tools, their tests.
3. **Registration:** the config writers made general over the server,
   `install`, `init`, `register` and `uninstall` wiring both servers, their
   tests.
4. **The local release:** the two recipes, proved under a temporary `HOME`,
   and `README.md`.

## Done When

- **The gates:** `just strict` reports every gate run and passed for both
  strict crates, the shared plumbing and `poman mcp` included, 0 skipped and
  0 failed; `just verify` passes with nothing skipped.
- **`poman mcp`:** `tools/list` lists both tools; each tool's result matches
  its command's `--json` output on the same fixture, and `poman_new_deadline`
  writes the file `poman new deadline` writes, byte for byte.
- **llm-wiki's server:** its MCP tests pass unchanged through the shared
  plumbing.
- **Registration:** an install into a temporary `HOME` registers both servers
  in the Codex config and the staged Claude config, `register` adds both to a
  project's `.mcp.json`, and `uninstall` removes poman's entry with
  llm-wiki's.
- **The local release:** under a temporary `HOME`, `just local-release`
  installs both binaries from the archives and registers both servers, and
  `just local-release-revert` brings back the saved state exactly, compared
  file by file.

### Evidence Recorded

In the PR: `just strict`'s summary, `poman mcp`'s tool list and a call of each
tool, the registered configs from a temporary `HOME`, and the local release
and its revert run there.

### Wiki Pages To Update When Done

- this plan, "Where It Stands";
- `wiki/decisions/poman-is-mcp-friendly.decision.md`: the open points
  answered, and the recheck of what moved;
- `wiki/roadmaps/poman.roadmap.md`, PM3.1.

### What The Work Touches

`crates/llm-wiki-core` (the `mcp` module, its tests, its manifest), `crates/poman`
(the `mcp` subcommand and `--json`), `src/mcp/mod.rs`, `src/mcp_config.rs`,
`src/mcp_wiring.rs`, `src/install.rs`, `src/uninstall.rs`, their tests, the
lock file, the `justfile`, `tools/local-release.sh` (the coordinator's yes,
2026-10-07) and `README.md`.

### What Closes This Plan

The owner's PASS on the PR that meets "Done When", merged into `develop`.

## Open For The Owner

Decided with the pull request's verdict; the Target above follows each
recommendation.

1. **`run_cli` and the config writers stay with llm-wiki**, made general over
   the server, as "What moves" says. Not chosen: moving them into the shared
   crate, where poman would never call them.
2. **The three open points** as answered above.

## Out Of Scope

- The tools of later entries' commands; each entry ships its own.
- `doctor` checking poman's wiring (it checks llm-wiki's today).
- A real release, tags and publishing; the release workflow's runner (#16).
- Any file in the riseon repository.
