# Plan: Harness-Independent Wiki Read Tool

- Document Class: Plan
- Status: Active
- Note: The original profile-reservation and router-bypass-proof portions were
  retired in the 2026-07-07 no-legacy reduction; the shipped `llm_wiki_read` MCP
  tool and its `_test` namespacing stand.
- Date: 2026-06-22
- Category: Agent runtime, provenance safety, MCP surface
- Scope: Implement `llm_wiki_read` end-to-end per the accepted proposal,
  server-first: stand up the framework's first MCP tool surface (a Rust-native
  stdio JSON-RPC server, no SDK, single-binary preserved), then host a
  `wiki/`+`raw/`-scoped, read-only, full-file read tool on it, plus the
  `llm-wiki read <path>` CLI verb and a `_test`-namespaced sibling MCP tool
  name. The tool must be callable by an agent at the end of this plan. This plan
  must not change production names, paths, or the single-binary distribution
  invariant.
- Sources:
  - wiki/proposals/harness-independent-wiki-read-tool.proposal.md
  - wiki/references/headroom-context-compression.reference.md
- Parent: wiki/proposals/mcp-first-surface.proposal.md (delivers its increment 1)
- Related:
  - wiki/decisions/test-instance-namespaced-binary.decision.md
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md

## Deliverable

An agent on any harness can call an MCP tool `llm_wiki_read` (test instance:
`llm_wiki_read_test`) that returns the full contents of one `wiki/` or `raw/`
file under the Part 2 output schema, served by `llm-wiki mcp serve`. A local
`llm-wiki read <path>` CLI verb shares the same read core. Claude Code's
native-`Read` path and the `headroom_read` ban are unchanged.

## Open Decision (resolve in Phase 1)

**MCP transport/runtime.** The codebase is fully synchronous today (no tokio;
`reqwest` is the blocking variant). Recommendation: a **hand-rolled synchronous
stdio JSON-RPC loop** (line-delimited / `Content-Length`-framed messages read
from stdin, written to stdout), using the already-present `serde_json`, with
**no tokio and no MCP SDK**. This keeps the single-binary distribution invariant
and the SDK-refusal posture (`wiki/decisions/headroom-single-posture-mcp-first.decision.md`)
intact and avoids dragging an async runtime into a synchronous binary. The
`rmcp` crate is the fallback only if full protocol surface (beyond
`initialize` / `tools/list` / `tools/call`) is later required; adopting it would
introduce tokio and is out of scope unless Phase 1 finds the hand-rolled path
inadequate. Phase 1 must record which path was taken before later phases build
on it.

## Touchpoints

- `src/cli.rs` — add `Mcp(McpArgs)` with a `Serve` subcommand and `Read(ReadArgs)`
  to the `Command` enum; matches the existing subcommand-with-args pattern.
- `src/main.rs` — dispatch the two new commands.
- `src/mcp/` (new module) — stdio server scaffold (`serve.rs`), JSON-RPC framing,
  tool registry, and the `llm_wiki_read` tool handler.
- `src/wiki_read/` (new module, or `src/read.rs`) — the harness-independent read
  core shared by the MCP tool and the CLI verb: project resolution, path
  scoping/normalization, encoding/size handling, output-schema struct.
- `src/registry/mod.rs` — reuse `project_by_id`, `project_by_root`,
  `wiki_root()`; add a `raw_root()`/root-walk helper if not already present for
  cwd-based resolution.
- `src/instance.rs` — derive `llm_wiki_read` / `llm_wiki_read_test` through the
  MCP tool-name derivation API (`mcp_read_tool_name()` / `mcp_tool_name`); add a
  double-suffix guard test.
- Source identity lint (the test-instance scoped lint) — ensure the derived
  names are minted only via the derivation API.
- `sha2` (already a dependency) — `sha256` over raw bytes for the schema.
- `wiki/references/headroom-context-compression.reference.md` — reaffirm the
  `headroom_read` ban and add `llm_wiki_read` to the forward-reserved MCP
  surface.

## Out Of Scope

- Implementing `llm_wiki_search` / `llm_wiki_search_all` as MCP tools. They stay
  reserved names; this plan ships only the read tool (proposal "Out Of Scope").
- A directory/listing/glob MCP tool.
- Base64-inlining binary `raw/` artifacts.
- Adopting tokio or an MCP SDK unless Phase 1 proves the hand-rolled path
  inadequate (Open Decision).

## Phases

### Phase 1 — Stdio MCP Server Scaffold

Stand up the first MCP surface with no domain tools yet.

1. Add `llm-wiki mcp serve` (`Mcp` command, `Serve` subcommand) to `src/cli.rs`
   and dispatch in `src/main.rs`.
2. Create `src/mcp/` with a synchronous stdio JSON-RPC loop implementing
   `initialize`, `tools/list` (empty or a trivial registry), and `tools/call`
   (returns method-not-found for unknown tools), per the recommended hand-rolled
   path in the Open Decision. Record the chosen transport in this plan's
   Implementation Progress.
3. Clean shutdown on stdin EOF; errors are JSON-RPC error objects, not panics.
   No stdout pollution outside the protocol (diagnostics go to stderr, matching
   `CliContext`/tracing).

Verification:

- A scripted `initialize` + `tools/list` round-trip over stdio returns a
  well-formed response (integration test driving the binary via `assert_cmd`).
- `tools/call` for an unknown tool returns a typed JSON-RPC error.
- Server respects the active instance binary stem.

Closes: prerequisite for AC 3.

### Phase 2 — Project Resolution Core

Shared, registry-based resolution for a possibly-global server.

1. Implement resolution order from the proposal: explicit `project` id → registry
   lookup; else nearest registered project root walking up from cwd; else a typed
   `ambiguous project` error. Unknown explicit id → typed `unknown project`.
2. Paths are always rooted at the resolved project, never the server cwd.

Verification:

- Unit tests: unknown explicit id errors; cwd inside a registered project
  resolves with no explicit id; no-cwd-match and ambiguous cases error rather
  than guess; a relative path cannot cross into another registered project.

Closes: AC 5.

### Phase 3 — Scoped Read Core And Output Schema

The provenance-safe read, independent of transport.

1. Normalize the requested path (resolve `..` and symlinks) and refuse anything
   not under the resolved project's `wiki/` or `raw/` tree with a typed error.
2. Detect encoding: valid UTF-8 within the byte cap → `encoding: "utf-8"` with
   verbatim `content`; non-UTF-8 → `encoding: "binary"`, `content` omitted,
   `content_omitted` set; over the cap (default 1 MiB) → `content` omitted,
   `content_omitted` naming the cap. Always compute `byte_len` and `sha256` over
   raw bytes; set `tree` to `wiki`/`raw`.
3. Define the serde output struct; never normalize or trim returned text.

Verification:

- Unit tests for: UTF-8 happy path (verbatim, correct metadata), `..`-escape
  refusal, symlink-escape refusal, non-UTF-8 binary case, oversized case, and
  `sha256` correctness across all encodings.

Closes: AC 3 (read behavior), AC 4.

### Phase 4 — Wire `llm_wiki_read` Tool And CLI Verb

Expose the read core through both surfaces.

1. Register `llm_wiki_read` (instance-derived name) in the MCP `tools/list` with
   a JSON schema for inputs (`path`, optional `project`) and dispatch
   `tools/call` to the Phase 3 core via Phase 2 resolution.
2. Add `llm-wiki read <path> [--project <id>]` reusing the identical core and
   resolution, printing the schema as JSON (and a human view without `-v`
   noise on stdout).

Verification:

- End-to-end MCP test: `tools/call llm_wiki_read` returns the schema for a real
  `wiki/` page and a typed error for an out-of-tree path.
- CLI scoping test mirrors the MCP refusal cases (shared core, no divergence).

Closes: AC 3, AC 4, AC 6 (read-only — no mutation surface exposed).

### Phase 5 — `_test` Namespacing Of The MCP Tool Name

Namespace the MCP tool name correctly for the test instance.

1. Derive `llm_wiki_read` → `llm_wiki_read_test` for the test instance through
   the `src/instance.rs` MCP tool-name derivation API (`mcp_read_tool_name()`).
   `llm_wiki_read` has no prefix collision with the search names, but add a guard
   test asserting no double-suffix and no accidental rewrite of unrelated tokens.
2. Add/extend an instance render test asserting the production instance advertises
   `llm_wiki_read` and the test instance advertises `llm_wiki_read_test`.
3. Ensure the scoped source identity lint covers the derived names (minted only
   via the derivation API).

Verification:

- Instance render test proves correct suffixing and no double-suffix.
- Identity lint fails on a hardcoded `llm_wiki_read_test` literal (negative test).

Closes: AC 1, AC 8.

### Phase 6 — Docs And Bookkeeping

1. Update `wiki/references/headroom-context-compression.reference.md`: reaffirm
   the `headroom_read` ban and add `llm_wiki_read` to the forward-reserved /
   "What Headroom Does Not Provide" surface.
2. Update `wiki/index.md` (status, scope) and append a `wiki/log.md` entry.
3. On completion, promote a short decision or fold into the Headroom companion
   decision noting the MCP surface now exists and the read tool is live.

Verification:

- `wiki/index.md` links the plan and the reference change is present.
- The `headroom_read` ban in the reference is intact.

Closes: AC 9, AC 10.

## Implementation Progress

### 2026-06-22 - Read Tool And MCP Foundation Implemented

Implemented the read-tool foundation in code:

- Added `llm-wiki mcp serve` with synchronous stdio JSON-RPC handling.
- Added `llm-wiki read <path> [--project <id>]`.
- Added shared `wiki_read` core used by both CLI and MCP.
- `llm_wiki_read` scopes reads to resolved project `wiki/` or `raw/`,
  normalizes `..` and symlinks before tree checks, returns canonical metadata,
  computes `sha256`, preserves verbatim UTF-8 text, and omits binary or
  over-1 MiB content with explicit reasons.
- Added test-instance suffix derivation for `llm_wiki_read`.

Verification evidence:

- `cargo test --test mcp` — 11 passed, including CLI read, MCP read, binary,
  oversize, parent escape, symlink escape, resources, register/index/search,
  and status coverage.
- `cargo test --test identity_lint` — 2 passed.
- `cargo test` — 303 passed, 2 ignored.

Pending before closing this plan as Completed:

- Decide whether additional registry-ambiguity tests are required for
  same-root/current-directory resolution.

### 2026-06-23 - Test-Instance MCP Identity Repair

The MCP merge-readiness repair pass extended the test-instance
reservation from `llm_wiki_read_test` to the full MCP tool identity set used by
the compiled `test` instance. Production still advertises `llm_wiki_read`;
`LLM_WIKI_INSTANCE=test` advertises `llm_wiki_read_test` and the corresponding
suffixed search/index/register/status tools.

Verification: `LLM_WIKI_INSTANCE=test cargo test --test mcp`,
`LLM_WIKI_INSTANCE=test cargo test --test mcp_install`, and
`just test-instance-live-session-proof` pass.

## Risks And Mitigations

- **Introducing an MCP server is new surface area.** Mitigation: hand-rolled
  synchronous stdio, no tokio/SDK (Open Decision); minimal method set
  (`initialize`/`tools/list`/`tools/call`); strict stdout discipline so the
  protocol stream stays clean.
- **Scope refusal must be airtight.** Mitigation: normalize `..` and symlinks
  before the tree check; cover both escape vectors with explicit tests; root all
  paths at the resolved project so cross-project reads are impossible.
- **`_test` rename collisions.** Mitigation: `llm_wiki_read` has no prefix
  overlap with the search names, but a double-suffix/`no-stray-rewrite` guard
  test is required anyway.

## Verification Gates

The plan closes when: `llm-wiki mcp serve` answers `initialize`/`tools/list`/
`tools/call`; `llm_wiki_read` returns the schema for a `wiki/`/`raw/` file and
refuses out-of-tree, binary, and oversized cases per tests; project resolution
tests pass including cross/missing/ambiguous; the `llm-wiki read` CLI verb shares
the core; the production instance advertises `llm_wiki_read` and the `test`
instance advertises `llm_wiki_read_test` with the identity lint enforcing
derivation-only minting; and the reference, index, and log are updated with the
`headroom_read` ban intact.
