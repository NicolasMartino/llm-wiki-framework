# Harness-Independent Wiki Read Tool

- Document Class: Proposal
- Status: Proposed
- Promoted: 2026-06-22 (see wiki/plans/harness-independent-wiki-read-tool.plan.md)
- Date: 2026-06-22
- Category: Agent runtime, context compression, provenance safety
- Scope: Ship a harness-independent MCP tool, `llm_wiki_read`
  (and its namespaced test sibling `llm_wiki_read_test`), that returns the full,
  provenance-exact contents of a `wiki/` or `raw/` file to the agent, served by
  `llm-wiki mcp serve`. It is the full-file read counterpart to the already
  reserved `llm_wiki_search` / `llm_wiki_search_all` names, and exists so an
  agent on a harness without a native `Read` tool can still pull a whole wiki/raw
  file into context verbatim.
- Sources:
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/10-headroom-source-content-router.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/13-headroom-source-mcp-server.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/17-headroom-config-excerpt.py
  - raw/research/2026-06-20-codex-tool-surface/research-summary.md
  - raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs
- Parent: wiki/proposals/mcp-first-surface.proposal.md (this is increment 1)
- Related:
  - wiki/references/headroom-context-compression.reference.md
  - wiki/proposals/test-instance-namespaced-binary.proposal.md
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md

## Question

Should `llm-wiki` ship a harness-independent MCP tool that hands an agent the
full, provenance-exact contents of a `wiki/` or `raw/` file, so that an agent on
a harness with no native `Read` tool can still pull a whole wiki/raw file into
context verbatim through a framework-owned surface?

## Observed Problem

`llm-wiki` has no harness-independent way to hand an agent the full, verbatim
contents of a `wiki/` or `raw/` file. On Claude Code this is a non-issue: the
native `Read` tool returns a whole wiki page directly, so nothing new is needed
there.

On the OpenAI Codex CLI it is not clean. Codex has **no native read tool**; it
reads files by running `cat`, `sed`, or `grep` through its shell-family tools
(`shell_command`, `exec_command`, `write_stdin`), all verified on
`openai/codex` `main`-tip 2026-06-20
(`sources/01-codex-handlers-shell-spec.rs` at upstream lines 210, 88, 138).
Shell-routed reads are awkward, easy to truncate, and give the agent no
provenance-exact, byte-checked artifact it can cite. Any harness without a
native `Read` tool has the same gap.

Today only the *search* side of a framework-owned tool surface is reserved:
`llm_wiki_search` and `llm_wiki_search_all` are listed as forward-compatibility
reservations. There is no counterpart for **whole-file reads**. Search returns
ranked snippets and citations; it deliberately does not return an entire page.
"Give the agent the full text of one md file, verbatim, on any harness" is a
distinct capability that the reserved search names do not cover.

This leaves a real gap: a Codex (or any non-`Read`) session that wants to pull a
complete wiki page into context has no first-class, provenance-exact way to do
it. Routing the read through `headroom_read` is not an option — the reference
explicitly **bans** it because it pipes file content through Headroom's
compress-cache-retrieve path
(`wiki/references/headroom-context-compression.reference.md`;
`sources/13-headroom-source-mcp-server.py`).

## Proposal

Ship `llm_wiki_read` as the full-file read sibling of the reserved search
tools, owned by the framework and exposed as an MCP tool through
`llm-wiki mcp serve`. It gives any harness — with or without a native `Read`
tool — a provenance-exact whole-file read of `wiki/` and `raw/`. It is one step
toward the harness-independent MCP surface the framework already opened; it is
not a new architectural concept.

### Part 1: A framework-owned, harness-independent tool name

Ship `llm_wiki_read` (and the namespaced `llm_wiki_read_test`, see Part 4) as a
framework-owned MCP tool name that is the same on every harness. A
framework-owned MCP name does not depend on a harness having a particular
built-in tool, so it is the durable mechanism that lets a non-`Read` harness
pull a whole wiki/raw file into context. It is the full-file read counterpart to
the already reserved `llm_wiki_search` / `llm_wiki_search_all` names.

### Part 2: Define the tool contract

**Inputs.** `llm_wiki_read` takes a project-relative `path` and an optional
explicit `project` id. The `project` field disambiguates a global MCP server (see
"Project resolution" below).

**Output schema.** The result separates content from metadata so an agent never
has to parse provenance out of the body:

```
{
  "project_id": "<resolved registered project id>",
  "resolved_path": "<project-relative path under wiki/ or raw/>",
  "tree": "wiki" | "raw",
  "byte_len": <integer>,
  "sha256": "<hex digest of the raw bytes>",
  "encoding": "utf-8" | "binary",
  "content": "<verbatim UTF-8 text>",      // present only when encoding == "utf-8"
  "content_omitted": "<reason>"            // present only when content is withheld
}
```

`sha256` and `byte_len` are computed over the raw on-disk bytes regardless of
encoding, so an agent can cite an exact, verifiable artifact even when the body
is withheld.

**Encoding and size behavior.** `wiki/` is text markdown and is the primary
target. `raw/`, however, is explicitly allowed to hold binary artifacts —
`templates/base/project_guidelines.md` lists screenshots, data samples, and
design mockups under Layer 1. The tool therefore must define non-text and
oversized behavior rather than assume UTF-8:

- **Valid UTF-8 within the size cap:** return `encoding: "utf-8"` with `content`
  set to the exact decoded text (no normalization, no trailing-newline fixups).
- **Non-UTF-8 / binary:** return `encoding: "binary"`, omit `content`, set
  `content_omitted: "binary (non-utf-8) file; not returned inline"`, and still
  return `byte_len` and `sha256`. The tool does not base64-inline binaries — a
  whole-file binary in the model's context is rarely what is wanted, and an agent
  that genuinely needs the bytes can read the path through the harness's own
  image/file path. (A future revision may add an opt-in base64 mode if a concrete
  need appears; out of scope here.)
- **Oversized (over a configured byte cap, default proposed at 1 MiB):** omit
  `content`, set `content_omitted: "exceeds N-byte inline cap"`, and return
  `byte_len`/`sha256`. The agent can then fall back to a ranged/host read. The
  cap protects against a single read flooding context, not against compression.

**Non-negotiable contract properties**, so the tool cannot become a hole in the
invariants it exists to protect:

1. **Read scope is restricted to `wiki/` and `raw/`.** The tool refuses any path
   that does not resolve, after symlink and `..` normalization, under the
   resolved project's `wiki/` or `raw/` tree. It is not a general file reader; it
   is the provenance-safe surface for the `wiki/` and `raw/` trees.
2. **Verbatim.** When `content` is returned it is the exact decoded file text,
   with no sampling, truncation, or summarization of the tool's own. The tool's
   job is to return the file's bytes faithfully.
3. **Read-only.** No mutation surface. Consistent with
   `wiki/decisions/agent-owns-wiki.decision.md` — the agent owns `wiki/` through
   the normal write path, not through this tool.
4. **Distinct from `headroom_read`.** `headroom_read` routes content *through*
   Headroom's compress-cache-retrieve path and is banned. `llm_wiki_read`
   returns file content directly and verbatim. The names must not be confused;
   the reference's `headroom_read` ban stays in force unchanged.

**Project resolution.** A harness-independent MCP server may be global, with
multiple registered projects, so a bare relative path is ambiguous. Resolution
order:

1. If the `project` input is set, resolve it against the registry; error
   (`unknown project`) if it is not registered.
2. Otherwise, resolve the project from the server's working directory by walking
   up to the nearest registered project root.
3. If neither yields exactly one project — no `project` given and the cwd is not
   inside a registered project, or is ambiguous — the tool errors
   (`ambiguous project; pass an explicit project id`) rather than guessing.

A path is always interpreted relative to the *resolved project's* root, never the
server cwd, so a read can never silently cross from one project into another.
Cross-project, missing-project, and wrong-project cases are part of the test
matrix (see Acceptance Criteria).

### Part 3: Optional CLI verb for humans and scripts

Ship `llm-wiki read <path>` as the local CLI counterpart for human and script
use. This is convenience only and carries no agent-runtime guarantee: a CLI read
invoked through Claude's `Bash` is already covered by the `Bash` exclusion, and
through Codex's shell tools it inherits whatever that harness's exclusion is. The
agent-facing guarantee lives entirely in the **MCP** form (Part 2). The CLI verb
should reuse the same path-scoping logic as the MCP tool so the two cannot
diverge.

### Part 4: Namespacing for the test instance

The build-time `test` scope
(`wiki/proposals/test-instance-namespaced-binary.proposal.md`) namespaces every
identity-bearing name. The read tool follows the same rule the search tools
follow: the production MCP name is `llm_wiki_read`, the test-instance name is
`llm_wiki_read_test`, and both are minted only through the central instance
derivation API (`src/instance.rs`) rather than hardcoded.

## What This Does Not Change

- The wiki/raw provenance invariants are unchanged. This tool is a new,
  narrower, safer path to content that agents can already read; it does not
  widen what is readable.
- Claude Code behavior is unchanged. `Read` remains the recommended path there;
  `llm_wiki_read` is redundant on Claude and is not required to be used there.
- The `headroom_read` ban is unchanged and still applies.
- The Read-only access contract in `AGENTS.MD` and
  `templates/base/project_guidelines.md` is unchanged; this proposal adds an
  alternative excluded surface, it does not relax any existing rule.
- The framework still does not ship, vendor, or depend on Headroom.

## Out Of Scope

- General-purpose file reading outside `wiki/` and `raw/`.
- Migrating Claude Code sessions off the native `Read` tool.
- Re-enabling or weakening the `headroom_read` ban.
- Install-time verification of the installed Codex CLI tool surface (already a
  separate follow-up in the Headroom Runtime Companion plan).
- A directory/listing or glob MCP tool. If a harness-independent listing surface
  is wanted, it is a separate reservation, not this one.

## Acceptance Criteria

1. `llm-wiki mcp serve` advertises `llm_wiki_read` (production) and
   `llm_wiki_read_test` (test instance) as a framework-owned, harness-independent
   MCP tool name, and a test asserts each instance advertises its correct name.
2. *(Retired in the 2026-07-07 no-legacy reduction: the synthetic
   router-bypass/exclude-set proof was removed with the Headroom proxy
   apparatus. Numbering is preserved so existing cross-references stay valid.)*
3. The MCP `llm_wiki_read` tool returns the Part 2 output schema. For a UTF-8
   file under `wiki/` or `raw/` it returns `encoding: "utf-8"` with verbatim
   `content`, plus `resolved_path`, `project_id`, `tree`, `byte_len`, and
   `sha256`. It refuses, with a typed error, any path that resolves outside those
   trees after normalization (tests include a `..`-escape and a symlink-escape
   case).
4. **Encoding and size behavior is covered.** Tests assert: a non-UTF-8 `raw/`
   artifact returns `encoding: "binary"` with `content` omitted but `byte_len`
   and `sha256` present; a file over the inline byte cap returns `content`
   omitted with `content_omitted` naming the cap; and both still return a correct
   `sha256` over the raw bytes.
5. **Project resolution is covered.** Tests assert: an explicit unregistered
   `project` errors; a cwd inside a registered project resolves without an
   explicit id; an ambiguous/none-resolved case errors rather than guessing; and
   a path is always rooted at the resolved project so a cross-project read is
   impossible.
6. The tool is read-only and exposes no mutation surface.
7. The optional `llm-wiki read <path>` CLI verb shares the path-scoping and
   project-resolution logic with the MCP tool and is covered by a scoping test.
8. The namespaced `_test` names are minted only through the central instance
   derivation API, never hardcoded, and the source identity lint enforces this.
9. The `headroom_read` ban in
   `wiki/references/headroom-context-compression.reference.md` is reaffirmed,
   and the reference's "What Headroom Does Not Provide" / forward-reserved
   surface is updated to list `llm_wiki_read` alongside the search names.
10. No change to Claude Code's recommended `Read`-based access path.
