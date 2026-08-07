# Headroom Context Compression

- Document Class: Reference
- Status: Sourced
- Date: 2026-06-21 (rewritten 2026-06-30; updated 2026-07-21 for Option A)
- Category: Agent runtime, context compression, provenance safety
- Scope: What Headroom is, why `llm-wiki` treats it as an optional, unmanaged
  external tool rather than a framework-integrated companion, and the single
  posture an opt-in user follows to keep `wiki/` and `raw/` provenance exact. The
  framework does not ship, vendor, configure, or depend on Headroom.
- Sources:
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/research-summary.md
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/03-headroom-docs-architecture.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/04-headroom-docs-ccr.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/05-headroom-docs-mcp.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/07-headroom-docs-limitations.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/13-headroom-source-mcp-server.py
  - Installed Headroom source read 2026-07-20 for `headroom-ai 0.32.0`
    (`headroom/proxy/handlers/openai.py`,
    `headroom/transforms/compression_units.py`)
- Related:
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md (the governing decision)
  - wiki/plans/headroom-wrap-command.plan.md (original launch convenience + no-legacy reduction)
  - wiki/plans/headroom-passthrough-launcher.plan.md (current passthrough process model)
  - wiki/decisions/three-layer-architecture.decision.md
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md

> **History.** This page previously documented a four-mode Headroom integration
> (A off / B MCP-only / C proxy carve-out / D proxy optimize-off) with a vetted
> profile, a managed `run-proxy.sh` launcher, a tool-name exclude list, a version
> pin, `install --with-headroom`, and doctor proxy checks. All of that was retired
> on 2026-06-30 and removed from the codebase; a five-run field test (Codex
> carve-out impossible, Anthropic carve-out fragile) drove the change. See
> `wiki/decisions/headroom-single-posture-mcp-first.decision.md`.

## What Headroom Is

Headroom is a third-party context-compression layer that sits between an agent
and the LLM API. It implements Compress-Cache-Retrieve (CCR): bulky tool outputs
are compressed before they reach the model, cached under a retrieval key, and the
model can retrieve the full content on demand
(`sources/04-headroom-docs-ccr.mdx`). It ships two surfaces: an HTTP **proxy**
(reached via `ANTHROPIC_BASE_URL`) that transparently compresses traffic, and an
**MCP server** (`headroom mcp serve`) exposing explicit `headroom_compress` /
`headroom_retrieve` tools (`sources/05-headroom-docs-mcp.mdx`,
`sources/13-headroom-source-mcp-server.py`). It is distributed as the PyPI
package `headroom-ai` and requires Python; it is not a linkable Rust crate.

Headroom never intercepts local subprocesses. The `llm-wiki` Rust binary's own
shell-outs (eval harness, doctor checks, registry reads) never traverse it.
Framework correctness therefore never depends on Headroom behaving any particular
way (`raw/research/2026-06-11-headroom-llm-wiki-comparison/research-summary.md`).

## Why It Is Not A Framework Integration

Headroom decides whether to compress a message from the **tool name** that
produced it, plus content-type heuristics and length thresholds. It has **no
path-based filtering**, so it cannot see that a tool output contains `wiki/` or
`raw/` content. Making a transparent proxy safe for provenance therefore depends
on carving tools out by name. That is fragile and version-dependent: inspected
`headroom-ai 0.24.0` ignored the tool-name exclude list on the OpenAI-Responses
path used by Codex, while installed `headroom-ai 0.32.0` source appears to build
a Responses call-id to function-name map and protect excluded tool outputs. A
safety mechanism controlled by an optional external proxy is still not a
provenance boundary. The framework's founding principle — single static binary,
no vendoring, no Python/Node dependency, correctness independent of Headroom —
is only literally true when Headroom behavior is not part of the correctness
claim.

So `llm-wiki` ships no Headroom configuration, launcher, profile, exclude list,
version pin, or doctor proxy check — with one narrow, temporary exception: a
`llm-wiki headroom` passthrough convenience that injects `HEADROOM_MCP_READ=off`
plus a best-effort exclude list (covering the production and test MCP namespaces)
and then execs the real `headroom` binary with forwarded args. It guarantees
only `HEADROOM_MCP_READ=off`; the exclude list is not a provenance boundary.
Otherwise Headroom is an optional external tool a user may run, exactly like any
other MCP server.

## The Single Posture

**Own our outputs; route reads and search through our MCP tools.** The largest
token sinks in a wiki workflow are the `wiki/`/`raw/` reads and the search
payloads — and `llm-wiki` owns those in Rust. The MCP server emits right-sized
outputs, and the agent reads/searches through the framework-owned tools:

- `llm_wiki_read` for `wiki/`/`raw/` files (not `cat`/`sed`/`grep`, and not
  Headroom's `headroom_read`).
- `llm_wiki_search` and `llm_wiki_search_all` for retrieval.

Read this way, those outputs are produced by `llm-wiki` and are exact at the MCP
server boundary. That is not the same as a guarantee that an optional Headroom
proxy will leave the host-delivered tool-output item unchanged. In installed
`headroom-ai 0.24.0`, Codex/OpenAI-Responses tool outputs were compressed without
consulting `HEADROOM_EXCLUDE_TOOLS`; in installed `headroom-ai 0.32.0`, source
inspection shows Responses exclusion support appears implemented and needs live
field-test confirmation. A Headroom-launched Codex session must therefore record
the exact Headroom version/path, prove whether excludes are honored for the
actual llm-wiki tool names, then either rely on full excluded-tool payloads, use
compression-aware small/paginated llm-wiki outputs, or treat CCR markers and
omitted payloads as a failed provenance check. On a developer test instance the
tools carry a `_test` suffix (e.g. `llm_wiki_search_test`).

## The One Safety Rule (if you run Headroom anyway)

If you choose to run Headroom (MCP or proxy) in front of the model, one rule
keeps provenance exact:

> Do not enable `HEADROOM_MCP_READ=on`, and do not call the `headroom_read` MCP
> tool against any `wiki/` or `raw/` path.

`headroom_read` ships off by default (`sources/13-headroom-source-mcp-server.py`)
and is described as a replacement for the host read tool; if enabled against
wiki/raw it routes that content through CCR with retrieval markers, defeating
exact provenance. Everything else follows from using the `llm_wiki_*` MCP tools
as the read/search surface.

Running Headroom's proxy transparently in front of the model is unsupported by
the framework: it is not configured, not blessed, and its behavior is external
and version-dependent. On Codex with `headroom-ai 0.24.0`, it could not protect
full wiki/raw tool outputs through `HEADROOM_EXCLUDE_TOOLS`; on inspected
`headroom-ai 0.32.0`, the relevant Responses exclude path appears present but
requires a live field test. The launcher now emits `*llm_wiki*` plus explicit
production/test MCP route-key entries as a broader best-effort exclude set. If
you run it anyway, the `llm_wiki_*` MCP tools
remain the required read/search path, but a compressed or metadata-stripped
result is a failed delivery check, not exact content. The chosen 2026-07-21
product posture is Option A: compact search is supported for discovery under
Headroom, while exact large `wiki/` and `raw/` reads should be performed outside
Headroom. The framework does not add read pagination for Headroom in the current
posture. A future exact-read-under-Headroom design would need a separate
decision and acceptance criteria.

The optional launch convenience is:

```bash
llm-wiki headroom -- wrap codex
```

That command exports the guard env and then runs `headroom wrap codex`; `llm-wiki`
does not start or supervise the proxy itself.

## What Headroom Does Not Provide

As captured at `chopratejas/headroom` on 2026-06-15:

- No path-based include/exclude. The router keys on tool name plus content-type
  heuristics; it cannot see filesystem paths inside tool outputs.
- No per-content-block opt-out from inside a tool result.
- No path-based wiki/raw exclusion. Current Headroom versions may honor
  tool-name excludes on more provider paths than 0.24.0 did, but the router still
  cannot decide from a filesystem path inside the payload that a result is
  wiki/raw provenance content.

A future Headroom release that added path-based (or content-marker) exclusion
could change this calculus; until then, the single MCP-first posture stands.
