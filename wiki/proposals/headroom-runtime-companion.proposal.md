# Headroom Runtime Companion

- Document Class: Proposal
- Status: Accepted
- Date: 2026-06-15
- Promoted: 2026-06-20 (see wiki/plans/headroom-runtime-companion.plan.md)
- Category: Agent runtime, context compression, provenance safety
- Scope: Make Headroom (context-compression layer) an optional, opt-in runtime
  companion for `llm-wiki` projects without letting it touch the durable
  knowledge layer in `wiki/` or the immutable source layer in `raw/`.
- Sources:
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/manifest.md
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/research-summary.md
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/03-headroom-docs-architecture.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/04-headroom-docs-ccr.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/05-headroom-docs-mcp.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/07-headroom-docs-limitations.mdx
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/10-headroom-source-content-router.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/12-headroom-source-compression-store.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/13-headroom-source-mcp-server.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/14-headroom-source-proxy-server.py
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/17-headroom-config-excerpt.py
  - raw/research/2026-06-20-codex-tool-surface/manifest.md
  - raw/research/2026-06-20-codex-tool-surface/research-summary.md
  - raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs
  - raw/research/2026-06-20-codex-tool-surface/sources/02-codex-handlers-apply-patch-spec.rs
  - raw/research/2026-06-20-codex-tool-surface/sources/03-codex-handlers-mod.rs
  - raw/research/2026-06-20-codex-tool-surface/sources/04-codex-handlers-request-permissions.rs
- Related:
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/three-layer-architecture.decision.md
  - wiki/decisions/wiki-query-search-first.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md
  - wiki/decisions/binary-path-bootstrap.decision.md
  - wiki/specs/wiki-query-skill.spec.md
  - AGENTS.MD (current RTK / `headroom:rtk-instructions` section)

## Source Capture

This proposal builds on the headroom-vs-llm-wiki research bundle ingested under
`raw/research/2026-06-11-headroom-llm-wiki-comparison/`. The bundle's
`research-summary.md` concluded that Headroom should be treated as an optional
agent-runtime accelerator and not as a replacement for any wiki layer, and
flagged a follow-up to produce a reference page and (optionally) a proposal for
a runtime companion. This is that proposal. It only commits the framework to
configuration and documentation; it does not commit to vendoring Headroom code
or adding a runtime dependency on the Headroom Python or TypeScript SDK.

The proposal also extends the research summary by surfacing the concrete
safety mechanism that the summary did not cover: Headroom's
`DEFAULT_EXCLUDE_TOOLS` set. The original `raw/` bundle only contained
circumstantial evidence about its contents (an import statement, an in-router
log label "(Read/Glob)", and a "protects ~5 Reads" comment). A subsequent
partial capture of `headroom/config.py` from `chopratejas/headroom` at
`main`-tip on 2026-06-15 was taken to evidence the literal set. That capture
lives at `raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/17-headroom-config-excerpt.py`.

The verified set as of capture is:

```
{"Read", "Glob", "Grep", "Write", "Edit", "Bash",
 "read", "glob", "grep", "write", "edit", "bash"}
```

The capture also exposed an upstream consistency bug: the comment block at
`headroom/config.py:202-210` includes a line that reads
"Bash is NOT excluded -- its outputs (build logs, test output) are ideal
compression targets" while the immediately following frozenset literal at
`headroom/config.py:211-227` clearly contains `"Bash"`. Runtime behavior is
governed by the literal: agents using the host's `Bash` tool currently see
their tool_results pass through uncompressed. A future upstream commit that
reconciles the literal to match the comment would silently remove that
protection. This proposal treats that as a real instability risk and
defensively re-adds the entire default set into `HEADROOM_EXCLUDE_TOOLS` in
the vetted profile, so the framework's guarantees do not depend on
`DEFAULT_EXCLUDE_TOOLS` continuing to look the way it does today.

The `sources/17` capture is contemporaneous with this proposal (both dated
2026-06-15); it is a snapshot, not a moving record. The captured excerpt is
therefore not a defense against upstream drift on its own. The actual
defense is the install-time release pin in Acceptance Criterion 6, which
refuses to enable Mode C unless the installed Headroom release's
`headroom/config.py` still agrees with `sources/17`. Without that pin, the
proposal's guarantees decay the moment upstream lands its next config
change.

A second source capture, `raw/research/2026-06-20-codex-tool-surface/`,
was taken on 2026-06-20 to evidence the OpenAI Codex CLI tool names that
the Part 2 profile must exclude. The earlier revision of this proposal
asserted six Codex tool names (`shell`, `local_shell`, `shell_command`,
`exec_command`, `write_stdin`, `apply_patch`) and cited
`codex-rs/core/src/openai_tools.rs` — a file that does not exist at
`openai/codex` `main`-tip on 2026-06-20. The new bundle replaces that
broken citation with verbatim captures of
`codex-rs/core/src/tools/handlers/shell_spec.rs`,
`apply_patch_spec.rs`, `mod.rs`, and `request_permissions.rs`. The bundle's
`research-summary.md` records what verified (`shell_command`,
`exec_command`, `write_stdin`, `apply_patch`, plus the additional
`request_permissions`) and what did not (`shell`, `local_shell`). Per that
research, this proposal now drops `shell` and `local_shell` from the
profile, adds `request_permissions`, and reshapes Acceptance Criterion 10
to pin the Codex exclude entries against the new raw capture so an
upstream Codex rename is caught by the fixture, not only by a local
profile edit.

## Question

Should `llm-wiki` document a supported, opt-in runtime configuration that lets
projects use Headroom for compression of incidental agent tool output, while
guaranteeing that no content from `wiki/` or `raw/` is ever passed through
Headroom's compression pipeline before reaching the model?

## Observed Problem

The framework's correctness rests on two invariants:

1. Every claim in `wiki/` is cited to a source under `raw/` or another
   `wiki/` page.
2. `raw/` is immutable and `wiki/index.md` is the agent's only sanctioned
   entry point into the project's knowledge.

Headroom is a context-compression layer (Compress-Cache-Retrieve, with a
SmartCrusher sampler for JSON arrays and a text/code pipeline behind safety
gates) sitting in front of the LLM API. Its proxy mode is an HTTP interceptor
reached by setting `ANTHROPIC_BASE_URL=http://127.0.0.1:8787`; its MCP mode is
an in-host tool server (`headroom mcp serve`). Neither mode intercepts local
subprocesses — the `llm-wiki` Rust binary's own `std::process::Command`
shell-outs (eval harness in `src/eval.rs`, doctor checks, registry status
reads) never traverse the proxy. Headroom only sees what the agent's tool
calls return on their way back to the model.

If Headroom is enabled naively as a transparent HTTP proxy with no carve-out,
three failure modes become reachable.

**Failure mode 1: sampled wiki and raw content reaching the model as truth.**
When wiki or raw content arrives in the conversation through a tool whose name
is not on Headroom's exclude list, Headroom's ContentRouter and SmartCrusher
may apply text compression or JSON-array sampling before the model sees the
payload on the next turn. The agent can then synthesize wiki pages that cite
specific lines, links, or items that were not in the compressed view. CCR
retrieval makes this *recoverable* but not *prevented*. As of `main`-tip
capture the default exclude set covers `Read`, `Glob`, `Grep`, `Write`,
`Edit`, and `Bash`, which means the most common access patterns are already
protected today. The vulnerable surface is any future agent tool not in that
set, and any upstream reduction of the set.

**Failure mode 2: corrupted agent reasoning over `--format json` output.** The
`llm-wiki search --format json` path is consumed by skills via the agent's
`Bash` tool. With the proxy in the path and Bash output eligible for
compression (relevant only if a future upstream removes Bash from the default
exclude set), the agent's tool_result view of a curated, reranked search
result list can be re-sampled by SmartCrusher's statistical heuristic before
reaching the model on the next turn. The agent then writes a wiki page citing
items it never saw and ranks it does not have. This is an agent-reasoning
failure, not a Rust-binary JSON-contract failure: the `llm-wiki` process and
any direct downstream consumer of its JSON output (the Rust eval harness, a
shell script) never traverse Headroom. The risk lives entirely at the model's
view of the agent's tool_result.

**Failure mode 3: degraded recovery on CCR expiry mid-task.** Headroom's
proxy CCR cache defaults to 300 seconds
(`sources/12-headroom-source-compression-store.py:56`). Multi-source ingest
of a research bundle, full lint passes, or any long agent run can exceed that
window. When the model issues a retrieval against an expired entry, Headroom
returns an explicit string of the form
`"Entry expired (CCR TTL: N seconds; age: M seconds)"` or
`"Entry not found (CCR TTL: N seconds)"`
(`sources/12-headroom-source-compression-store.py:96-107`). This is a
recoverable error, not silent context loss: the agent receives an error and
can re-issue the original `Read`. The cost is wasted turns and degraded
ingest throughput, not lost data — but for long ingest runs that cost is
real, and the mitigation is one env var.

All three failure modes are described above in Claude Code's tool
vocabulary, where `Read`, `Glob`, `Grep`, and `Bash` are already in
`DEFAULT_EXCLUDE_TOOLS`. On the OpenAI Codex CLI the exposed surface is
larger, because Codex has **no native read tool** and routes file reads
and `llm-wiki search` alike through its shell-family tools. The OpenAI
function tool names emitted by Codex on `main`-tip as of 2026-06-20 are
`shell_command`, `exec_command`, `write_stdin` (all declared in
`raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs`
at upstream lines 210, 88, and 138), `apply_patch`
(`sources/02-codex-handlers-apply-patch-spec.rs:18`), and
`request_permissions` (`sources/01-codex-handlers-shell-spec.rs:242`,
bound to its handler at `sources/04-codex-handlers-request-permissions.rs:30`).
None of these is in `DEFAULT_EXCLUDE_TOOLS`. The framework ships its search
skill identically to both runtimes as
`llm-wiki search --mode auto --format json` (`assets/skills/wiki-query/SKILL.md`,
materialized into `.claude/skills/<skill>/` for Claude Code and
`.codex/<skill>/` for Codex), so on Claude Code the result list rides the
excluded `Bash` tool while on Codex the same call rides the unexcluded
`shell_command` / `exec_command` tool. On an unconfigured proxy, Failure
modes 1 and 2 are therefore reachable on Codex through the ordinary read
and search paths, not only through hypothetical future non-default tools.
The vetted profile (Part 2) closes this by naming Codex's shell and patch
tools explicitly.

Headroom does not expose a path-based exclusion mechanism in
`chopratejas/headroom` at the `main`-tip reviewed on 2026-06-15. The router
decides whether to compress a message based on the **tool name** that produced
it (read from the Anthropic or OpenAI tool_use envelope), plus content-type
heuristics and length thresholds. It cannot see filesystem paths inside tool
outputs. The practical implication is that "skip wiki/ and raw/" must be
expressed as "skip the tools that read wiki/ and raw/."

## Proposal

Make Headroom an opt-in, never-required companion. The framework does not ship
Headroom, does not depend on it at build time, and does not assume it is
running. When projects opt in, the framework provides a vetted configuration
and an agent-facing contract that guarantees wiki and raw content bypass the
compression pipeline.

The proposal has four parts.

### Part 1: Adopt the Read-only access contract for wiki and raw

Update `AGENTS.MD` and `templates/base/project_guidelines.md` to state, as a
hard convention, that agents reading any path under `wiki/` or `raw/` must use
the host's native `Read` tool (or the equivalent `read_file` MCP) and must not
use Bash shellouts (`cat`, `head`, `tail`, `sed`, `awk`, `less`) to access
those paths.

This is already the de facto convention for Claude Code; promoting it to a
framework-level invariant is what lets the Headroom carve-out hold even if
upstream removes `Bash` from `DEFAULT_EXCLUDE_TOOLS`.

The contract is asymmetric across harnesses, and that asymmetry is
load-bearing rather than cosmetic. Claude Code exposes a native `Read` tool
distinct from `Bash`, so "read with `Read`, never `cat`" is both expressible
and the thing that keeps wiki and raw reads on an excluded tool name. The
OpenAI Codex CLI has **no** dedicated file-read tool: it reads files by
running `cat`, `sed`, or `grep` through its shell-family tools
(`shell_command`, `exec_command`, `write_stdin`) and it edits via
`apply_patch`. None of those names is `Read`, `Bash`, or anything in
`DEFAULT_EXCLUDE_TOOLS`. Part 1's contract for Codex therefore cannot be
"use the native Read tool" — that tool does not exist on Codex. The Codex
contract is instead twofold: (a) the vetted profile must add Codex's shell
and patch tool names to the exclude set (Part 2), so that wiki reads, raw
reads, and search calls riding the shell tool are carved out; and (b)
because excluding the shell tools suppresses essentially all of Headroom's
compression on Codex — Codex funnels nearly all I/O through them — the
recommended Codex posture is Mode D or Mode B, not proxy Mode C (Part 3).
The durable fix that lets Codex benefit from proxy compression while
keeping search, wiki, and raw safe is the harness-independent MCP search
migration (Out Of Scope here; reserved names in Part 2): an MCP tool name
is identical regardless of which harness issues the call, so a single
exclude entry protects both. The Codex tool names cited above are all
declared at
`raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs`
(`shell_command`, `exec_command`, `write_stdin`) and
`sources/02-codex-handlers-apply-patch-spec.rs` (`apply_patch`).

A separate normative rule, equally load-bearing in Mode B below: agents must
not enable or call Headroom's `headroom_read` MCP tool against `wiki/` or
`raw/`. That tool ships with the description "Use this INSTEAD of the
built-in Read tool for significant token savings"
(`sources/13-headroom-source-mcp-server.py:557`) and is gated behind
`HEADROOM_MCP_READ=on` (`:76-78`), off by default. If enabled, it bypasses
the host's `Read` and routes file content through CCR with retrieval markers
— defeating Part 1 for any wiki or raw payload. The vetted profile must keep
`HEADROOM_MCP_READ` unset (relying on the off default) and the reference page
must explicitly forbid enabling it for projects that use this companion.

Lint can later check the Read-vs-Bash invariant by scanning recent agent
transcripts (out of scope for this proposal; see
`wiki/proposals/cli-observability.proposal.md` and related observability work
for the transcript surface that would make a check feasible).

### Part 2: Ship a vetted Headroom config for opt-in users

Add a new managed config file under `assets/headroom/llm-wiki.profile.env`
embedded into the binary, materialized on demand by
`llm-wiki install --with-headroom` (new flag) into the managed runtime home at
`~/.llm_wiki/headroom/llm-wiki.profile.env`. The contents are environment
variable defaults that downstream users source before starting
`headroom proxy`:

```sh
# llm-wiki vetted Headroom profile
# Source before: . ~/.llm_wiki/headroom/llm-wiki.profile.env && headroom proxy

# Defensive re-add of the entire DEFAULT_EXCLUDE_TOOLS set as captured on
# 2026-06-15 (sources/17-headroom-config-excerpt.py), so the framework's
# guarantees do not depend on upstream defaults remaining the same. Merged
# additively with DEFAULT_EXCLUDE_TOOLS (sources/14:373-376), so re-adding
# entries is a no-op when upstream agrees and a real protection when it
# drifts. Two additional non-default tools that surface bulky reference
# content (WebFetch, WebSearch) are added permanently.
#
# Codex tool names are added because the OpenAI Codex CLI uses none of Claude
# Code's tool names: it has no native read tool and reads files and runs
# `llm-wiki search` through its shell-family tools (`shell_command`,
# `exec_command`, `write_stdin`), edits via `apply_patch`, and requests
# extra sandbox capability through `request_permissions`. All five names
# verified on openai/codex main-tip 2026-06-20 — see
# raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs
# (shell_command at upstream line 210, exec_command at 88, write_stdin at
# 138, request_permissions at 242) and sources/02-codex-handlers-apply-patch-spec.rs
# (apply_patch at upstream line 18). Without these names every wiki read,
# raw read, and search call on Codex is eligible for compression. NOTE:
# excluding the shell-family tools suppresses essentially all compression on
# Codex since Codex routes nearly all I/O through them -- proxy Mode C is
# therefore low-value on Codex and Mode D or Mode B is recommended there
# (see Part 3). The names are kept in the profile so a user who runs Mode C
# on Codex anyway still gets the wiki/raw/search carve-out. Tool-name
# matching is case-insensitive (sources/14:3236-3241).
#
# Names previously considered and dropped: `shell` and `local_shell`. Neither
# is a function tool name on openai/codex main-tip 2026-06-20 (see
# raw/research/2026-06-20-codex-tool-surface/manifest.md "What Was Not Found"
# and research-summary.md). `local_shell` looks like an OpenAI Responses-API
# built-in tool *type*, not a function *name*; the captured Headroom router
# only extracts the routing key from tool_call.function.name and
# block.name (sources/10-headroom-source-content-router.py:1828, :1838),
# so a `local_shell` entry would be dead under the current router anyway.
# If a future Codex release or a future Headroom router change makes either
# name load-bearing, the install-time release pin (Acceptance Criterion 6
# for Headroom, the equivalent in Acceptance Criterion 10 for Codex) will
# catch the change and the profile can be amended.
#
# Two future custom MCP tool names are reserved (the framework owns these
# names; see Out Of Scope). Being MCP names they are harness-independent, and
# are the durable fix that lets Codex run proxy compression while keeping
# search/wiki/raw safe.
export HEADROOM_EXCLUDE_TOOLS="Read,Glob,Grep,Write,Edit,Bash,WebFetch,WebSearch,shell_command,exec_command,write_stdin,apply_patch,request_permissions,llm_wiki_search,llm_wiki_search_all"

# Per-tool compression bias for tools that are NOT excluded but still warrant
# a conservative posture. Bash and Grep are listed in the exclude set above,
# so their bias entry is dead unless that exclusion is later relaxed; we
# leave them in as forward compatibility so a relaxation of HEADROOM_EXCLUDE
# does not immediately drop them to default moderate. The WebFetch entry is
# a deliberate downgrade from the upstream default of
# WebFetch:aggressive (sources/17 DEFAULT_TOOL_PROFILES); ingest-heavy
# workflows that read whole upstream docs or release notes benefit from a
# conservative bias here.
export HEADROOM_TOOL_PROFILES="WebFetch:conservative,Bash:conservative,Grep:conservative"

# User-message compression is off by default in the proxy
# (sources/14:3491). We pin to a falsy literal so any future upstream flip
# of the code default to True is overridden at runtime. Pinning is best-
# effort only: it does not protect against upstream code that ignores this
# env var entirely. A change of that kind would have to be caught by the
# fixture test in Acceptance Criteria.
export HEADROOM_COMPRESS_USER_MESSAGES=0

# CCR retention long enough for multi-source ingest and full lint passes.
# Default is 300 seconds (sources/12:56). At 7200 seconds, a single ingest
# session can run for two hours before any retrieval misses.
export HEADROOM_CCR_TTL_SECONDS=7200

# Minimum payload size before SmartCrusher fires. The proxy CLI default is
# 500 tokens (sources/14:3360), which the proxy passes down to the
# SmartCrusher config (sources/14:3468). This floor is a proxy-layer
# property, not an SDK-wide invariant: a direct SDK caller sees the
# underlying SmartCrusher default of 200 tokens
# (sources/11-headroom-source-smart-crusher.py:121). We pin the proxy
# default explicitly.
export HEADROOM_MIN_TOKENS=500
```

The exclude list deliberately names two future custom MCP tool names,
`llm_wiki_search` and `llm_wiki_search_all`. Those names are not yet wired but
adopting them now keeps the profile stable through any future migration of
search from Bash-invoked CLI to a first-class MCP tool. The framework does not
have to ship those MCP tools to make the profile correct; they are reserved
(see Out Of Scope).

### Part 3: Document modes of operation

Add a new reference page,
`wiki/references/headroom-context-compression.reference.md`, captured in a
companion lint-friendly ingest from the same `raw/` bundle that this proposal
cites. The reference describes four modes:

1. **Mode A - Off (default).** Headroom is not running. No configuration
   required. Framework correctness is independent of Headroom.

2. **Mode B - MCP only, with `headroom_read` forbidden.** Users add
   `headroom mcp serve` to their Claude Code or Codex MCP configuration. The
   agent calls `headroom_compress` and `headroom_retrieve` explicitly.
   Compression is opt-in per tool call. This mode is safe for wiki and raw
   reads provided the operator does not set `HEADROOM_MCP_READ=on` and the
   agent does not call `headroom_read` against those paths. The reference
   page must state both rules normatively. Mode B is harness-neutral:
   compression happens only when the agent explicitly calls
   `headroom_compress`, so Codex's lack of a native `Read` tool and its
   shell-routed reads do not create an implicit-compression surface here.
   This makes Mode B the safest opt-in for Codex users who still want
   on-demand compression of bulky incidental output.

3. **Mode C - Proxy with carve-out.** Users start `headroom proxy` after
   sourcing the vetted profile from Part 2. With this profile sourced, the
   tool-name exclude set covers all standard wiki and raw access paths plus
   the reserved framework MCP names. Wiki and raw content bypass the proxy
   *only if* Part 1's Read-only access contract is followed AND the user is
   running a pinned Headroom release whose `DEFAULT_EXCLUDE_TOOLS` matches
   `sources/17-headroom-config-excerpt.py` (or at least its
   `{"Read","Glob","Grep","Write","Edit","Bash"}` superset, which the
   profile re-adds defensively). The pinned release is a hard prerequisite
   for Mode C, not an open item — see Acceptance Criteria 6. On the OpenAI
   Codex CLI, Mode C additionally depends on the profile's Codex tool
   names (`shell_command`, `exec_command`, `write_stdin`, `apply_patch`,
   `request_permissions`) to carve out wiki, raw, and search, because Codex
   has no native `Read` tool and routes those reads through its shell-family
   tools. But since that exclusion covers nearly all Codex I/O, Mode C
   yields little compression on Codex; Mode D or Mode B is recommended
   there instead.

4. **Mode D - Proxy with full optimization disabled.** Users run with
   `HEADROOM_OPTIMIZE=false` (or `--no-optimize`). The proxy still runs and
   continues to expose `/v1/retrieve/stats`, `/debug/warmup`, and other
   observability endpoints, but compression is documented to short-circuit.
   This behavior is documented in Headroom's README and is consistent with
   the `config.optimize` flag's wiring observed in the bundle
   (`sources/14-headroom-source-proxy-server.py:904, :954, :3113, :3442`); it
   is not fully verified by the captured source, because the per-request
   handler that invokes the pipeline is not in the bundle. The reference
   page must mark this caveat. Mode D is the recommended mode for
   `wiki-ingest` sessions, unconditionally and without dependency on any
   future lint capability, because it is the only mode that is robust to
   future framework-added tools that are not yet on the exclude list. It is
   also the recommended default mode for Codex sessions generally (not only
   ingest), since Codex's shell-routed reads and search make proxy Mode C
   both low-value and naming-fragile on that harness.

The reference page does not recommend Mode C for ingest sessions even after
Part 1 is enforced by lint. Mode D is the recommended default for ingest.

### Part 4: Refuse the SDK route explicitly

The TypeScript and Python SDKs are not adopted by this proposal. The framework
remains a single static Rust binary distributed by `cargo-dist`. Wiring a
Node or Python sidecar to call the SDK would:

- Break the single-binary distribution invariant from
  `wiki/decisions/llm-wiki-binary-distribution.decision.md` and
  `wiki/decisions/binary-path-bootstrap.decision.md`.
- Duplicate functionality the proxy and MCP server already provide
  out-of-process.
- Require the framework to track Headroom SDK API churn (v0.x).

If a future need genuinely requires in-process compression (none is known
today), the right path is to revisit this decision with a measured benchmark,
not to add the SDK speculatively.

## Configuration Reference

This is a normative summary of the Headroom configuration surface this
proposal relies on. All settings are CLI flags on `headroom proxy` with
matching `HEADROOM_*` environment variables. Source:
`sources/14-headroom-source-proxy-server.py`.

| Knob | CLI | Env var | Default | Notes |
| --- | --- | --- | --- | --- |
| Hard bypass (compress nothing) | `--no-optimize` | `HEADROOM_OPTIMIZE=false` | optimize on | Used in Mode D |
| Add tool names to permanent exclude set | `--exclude-tools NAME1,NAME2` | `HEADROOM_EXCLUDE_TOOLS` | merged additively with DEFAULT_EXCLUDE_TOOLS (sources/14:373-376; sources/17 for the captured default contents) | Case-insensitive (sources/14:3236-3241) |
| Per-tool compression bias | `--tool-profile NAME:level` (repeatable) | `HEADROOM_TOOL_PROFILES` | DEFAULT_TOOL_PROFILES from sources/17 | Levels: conservative, moderate, aggressive (sources/14:3276); only consulted for non-excluded tools |
| Compress user-role messages | `--compress-user-messages` | `HEADROOM_COMPRESS_USER_MESSAGES=1` | off (sources/14:3491) | Pin to 0 in profile |
| Minimum tokens before SmartCrusher fires | `--min-tokens N` | `HEADROOM_MIN_TOKENS` | 500 (proxy CLI, sources/14:3360) vs 200 (SmartCrusher SDK default, sources/11:121) | Proxy-layer floor only |
| Maximum items kept after crush | `--max-items N` | `HEADROOM_MAX_ITEMS` | 50 (sources/14:3361) | Not changed by this proposal |
| CCR retention | n/a (env-only) | `HEADROOM_CCR_TTL_SECONDS` | 300 (sources/12:56) | Raise to 7200 in profile |
| Disable Kompress ML compression only | `--disable-kompress` | `HEADROOM_DISABLE_KOMPRESS=1` | off (sources/14:3032) | Optional defense in depth |
| Disable cache | `--no-cache` | `HEADROOM_CACHE_ENABLED=false` | cache on | Not used by this proposal |
| Enable `headroom_read` MCP tool | n/a | `HEADROOM_MCP_READ=on` | off (sources/13:78) | Must remain off in Mode B |

What Headroom does **not** provide, in `chopratejas/headroom` at the
`main`-tip reviewed on 2026-06-15:

- Path-based include or exclude. The router does not see filesystem paths
  inside tool outputs. Wiki and raw safety is achieved entirely through the
  tool-name exclude list plus the Read-only access contract plus the
  `headroom_read` ban.
- Per-content-block opt-out from inside a tool result. A tool whose name is
  not excluded has its entire output evaluated by the router; there is no
  in-band "do not compress this part" marker.
- A built-in way to forbid compression of LLM provider system prompts other
  than the existing protection that system prompts are never compressed
  (`sources/07-headroom-docs-limitations.mdx:41`;
  `sources/03-headroom-docs-architecture.mdx:128`). The neighbouring
  `sources/03-headroom-docs-architecture.mdx:127` ("User messages: Never
  compressed") is the independent protection that backs the
  `HEADROOM_COMPRESS_USER_MESSAGES=0` pin in Part 2.

- Harness-neutral tool naming. `DEFAULT_EXCLUDE_TOOLS` is expressed entirely
  in Claude Code's tool vocabulary (`Read`, `Glob`, `Grep`, `Write`, `Edit`,
  `Bash`). The OpenAI Codex CLI emits a different set of function tool names
  on `main`-tip 2026-06-20 (`shell_command`, `exec_command`, `write_stdin`,
  `apply_patch`, `request_permissions`; see
  `raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs`
  and `sources/02-codex-handlers-apply-patch-spec.rs`) and has no native
  read tool, so the default carve-out does not transfer to Codex without
  the additional names added in Part 2. Name matching is case-insensitive
  (sources/14:3236-3241), which covers `Bash` vs `bash` but does nothing for
  genuinely different names like `Bash` vs `shell_command`.

These are upstream properties as captured; a future Headroom release could
introduce path-based filtering, in which case this proposal should be
revisited.

## What This Does Not Change

- The wiki framework's contract, schema, and document classes are unchanged.
- `wiki/`, `raw/`, and `AGENTS.MD` ownership is unchanged.
- The `llm-wiki` binary does not gain a runtime dependency on Headroom or on
  Node or Python.
- The default project bootstrap does not start Headroom.
- The current RTK section in `AGENTS.MD` is unaffected; it concerns a
  different token-optimization layer.

## Out Of Scope

- Implementing the optional `--with-headroom` install flag. This proposal
  authorizes it but a separate plan owns the implementation work.
- Migrating `llm-wiki search` from Bash invocation to a first-class MCP tool.
  The reserved exclude entries (`llm_wiki_search`, `llm_wiki_search_all`) are
  forward compatibility, not a commitment.
- Persistent memory features from Headroom's Python SDK (`with_memory`,
  `HierarchicalMemory`). The wiki itself is the project memory; using
  Headroom's would create a second source of truth.
- `headroom learn` mining of prior agent sessions to mutate `AGENTS.MD`
  directly. Any framework adoption of session-learned guidance would require a
  human-reviewed proposal under `wiki/proposals/`.
- The names `llm_wiki_search` and `llm_wiki_search_all` are reserved as
  framework-owned MCP tool names; third-party MCP servers in
  `llm-wiki`-managed projects must not surface tools under those names, so
  the vetted exclude list cannot be bypassed by name collision.
- Capturing `headroom_stats` (or any other Headroom telemetry) into
  `wiki/log.md` or any other framework-owned wiki file. Measurement of
  Headroom's real wins on this framework's workflows is deferred to a
  dogfood research bundle under `raw/research/`; only if those measurements
  justify durable documentation does the work promote to a
  `wiki/references/headroom-dogfood-evidence.reference.md` page. The reason
  this is explicitly out of scope is to keep the dependency direction
  one-way: wiki content does not assume Headroom is installed or running.

## Acceptance Criteria

A future plan promoting this proposal is complete when:

1. `wiki/references/headroom-context-compression.reference.md` exists, is
   linked from `wiki/index.md`, and describes Modes A through D with
   citations into `raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/`.
2. `AGENTS.MD` and `templates/base/project_guidelines.md` carry the Read-only
   access contract for `wiki/` and `raw/`, and the `headroom_read` ban, as
   normative rules — including the Codex-specific formulation, which states
   that Codex has no native `Read` tool, that its shell and patch tool names
   are carved out via the profile rather than by a Read-only convention, and
   that Mode D or Mode B is the recommended Codex posture.
3. An embedded asset `assets/headroom/llm-wiki.profile.env` exists with the
   exact contents documented in Part 2.
4. A fixture test pins the profile's env-var names and CLI flag equivalents
   against `sources/14-headroom-source-proxy-server.py` (catches CLI/env
   renames in the proxy layer).
5. A second fixture test pins the literal `DEFAULT_EXCLUDE_TOOLS` set and the
   `DEFAULT_TOOL_PROFILES` keys against `sources/17-headroom-config-excerpt.py`
   (catches regressions where upstream removes Read, Glob, Grep, Write,
   Edit, or Bash from defaults). Only the set membership and the dict keys
   are pinned, not the `DEFAULT_TOOL_PROFILES` *values*: in the captured
   excerpt those values are placeholder strings rather than verbatim
   `PROFILE_PRESETS` references, because the capture method (WebFetch
   summarization) could not return the preset objects verbatim. Pinning
   values would assert against a paraphrase. When this fixture fails, the
   bundled profile is the source of truth and the captured excerpt must be
   refreshed and the proposal revisited.
6. The `--with-headroom` install path verifies a pinned Headroom release
   version rather than letting users pull `chopratejas/headroom` at
   `main`-tip. The pinned version's `headroom/config.py` must agree with
   `sources/17-headroom-config-excerpt.py` at install time, or the install
   refuses with a clear diagnostic. This was previously an Open Question;
   it is promoted to a hard prerequisite for Mode C.
7. The reference page recommends Mode D as the default for `wiki-ingest`
   sessions, unconditionally.
8. No Headroom code is vendored, no Headroom SDK is added as a build or
   runtime dependency, and `llm-wiki` continues to install and run without
   Headroom present.
9. `llm-wiki doctor` emits a single advisory line when it detects
   `ANTHROPIC_BASE_URL` pointing at `http://127.0.0.1:8787` (Headroom's
   documented Mode C listen address; see Part 3 and Configuration
   Reference). The advisory text references
   `wiki/references/headroom-context-compression.reference.md` and says
   the user should ensure `~/.llm_wiki/headroom/llm-wiki.profile.env` is
   sourced before starting `headroom proxy`. The check does not read or
   validate the contents of any `HEADROOM_*` env var (a user may set a
   superset of the vetted profile and still be safe; validating would be
   brittle), does not fail `doctor`, and does not require Headroom to be
   installed. It is a pointer to the docs, not a guarantee. If
   `ANTHROPIC_BASE_URL` is unset or points anywhere other than that exact
   host/port, `doctor` emits nothing about Headroom.
10. The vetted profile's exclude set includes the Codex function tool names
   verified at `openai/codex` `main`-tip 2026-06-20 (`shell_command`,
   `exec_command`, `write_stdin`, `apply_patch`, `request_permissions`)
   alongside the Claude Code names, and the reference page documents that
   proxy Mode C is low-value on Codex (shell-tool exclusion suppresses most
   compression) with Mode D or Mode B recommended there. A fixture test
   pins these Codex names against the captured Codex sources at
   `raw/research/2026-06-20-codex-tool-surface/sources/01-codex-handlers-shell-spec.rs`
   and `sources/02-codex-handlers-apply-patch-spec.rs` so that an upstream
   Codex rename is caught by the fixture, not only by a local profile edit.
   The `--with-headroom` install path should additionally verify the
   installed Codex CLI's tool-name set against the same captures, mirroring
   the Headroom release pin in Acceptance Criterion 6; that verification is
   not required for first promotion of this proposal but should be a
   follow-up plan item.
