# Headroom: MCP-First Posture + One Bounded Launch Convenience

- Document Class: Decision
- Status: Accepted
- Date: 2026-06-30 (amended 2026-07-07; amended 2026-07-18; amended 2026-07-21)
- Category: Agent runtime, context compression, provenance safety
- Scope: The single framework posture for context compression. llm-wiki owns its
  own MCP tool outputs (lean, in Rust) and the agent routes wiki/raw reads and
  search through those tools. Headroom is an unmanaged, optional external tool the
  framework neither ships nor configures — with exactly one narrow, temporary
  exception (amended 2026-07-07, corrected 2026-07-18): a `llm-wiki headroom`
  passthrough convenience that injects `HEADROOM_MCP_READ=off` plus a
  best-effort exclude list, then execs the real `headroom` binary with forwarded
  Headroom args. This decision retired the earlier four-mode Headroom companion
  design (vetted
  profile, launcher, exclude-list asset, version pin, `install --with-headroom`,
  doctor probes, proxy e2e), which is removed from the repo.
- Sources:
  - Five-run 2026-06-29 field test: Codex proxy carve-out impossible, Anthropic
    Mode C verified-but-fragile (conclusion folded in below; the synthesized
    eval was removed in the 2026-07-07 no-legacy reduction, but the primary
    record survives in `raw/`: `raw/field-test/ketogenic_project_test.md` and
    the proxy captures under
    `raw/research/2026-06-21-headroom-dogfood/captures/` —
    `proxy.stdout`, `headroom_stats.json`, `health.json`, `retrieve_stats.json`,
    `profile.sha256`).
  - 2026-07-06 Codex proxy retry: an `llm_wiki_read_test` returned uncompressed,
    credited to `HEADROOM_MCP_READ=off` + MCP routing, not the exclude list
    (Candidate evidence; the synthesized eval was removed in the reduction — its
    primary record is the session transcript plus the same dogfood captures
    above).
  - wiki/evals/codex-mcp-field-test-pass.eval.md, wiki/evals/claude-mcp-field-test-pass.eval.md
    (2026-07-07 proxy-off passes: the MCP surface works with no Headroom present).
  - src/instance.rs (authoritative MCP tool-name derivation).
- Related:
  - wiki/plans/headroom-wrap-command.plan.md (implements the original
    convenience + the no-legacy reduction; process model superseded by the
    passthrough launcher).
  - wiki/plans/headroom-passthrough-launcher.plan.md (corrects the convenience
    to exec the real Headroom binary).
  - wiki/references/headroom-context-compression.reference.md.

## Choice

There is **one** framework posture for context compression:

> **Own our outputs; route reads/search through our MCP tools.** llm-wiki
> right-sizes its own MCP tool outputs in Rust so there is little bulk to
> compress, and the agent uses `llm_wiki_read` / `llm_wiki_search` /
> `llm_wiki_search_all` as the wiki/raw surface. This posture is identical
> whether or not Headroom is running, and framework correctness does not depend
> on Headroom behaving any particular way.

Headroom is **not an llm-wiki integration**. It is an optional external tool a
user may run, exactly like any other MCP server. The framework ships **one** and
only one Headroom-aware surface — the launch convenience below — and otherwise
owns zero Headroom configuration. It does **not** ship a vetted profile, a
`run-proxy.sh` launcher, a materialized `HEADROOM_EXCLUDE_TOOLS` asset, a version
pin, `install --with-headroom`, a doctor Mode-C advisory, a `--headroom-active-probe`,
a proxy e2e harness, or any blessing of the transparent HTTP proxy posture.

The one surviving safety rule, kept in generated guidance, is harness-neutral:
*if you run Headroom, use the llm-wiki MCP tools as the wiki/raw read path, and
never enable `HEADROOM_MCP_READ` or point `headroom_read` at `wiki/` or `raw/`.*

## The One Exception: `llm-wiki headroom` (temporary)

The `llm-wiki` binary ships a single, stateless launch convenience that removes
the retype-the-env footgun for a user who chooses to run Headroom anyway:

```bash
llm-wiki headroom -- wrap codex
```

It resolves the real `headroom` binary (or uses `--headroom-bin <PATH>`), sets
`HEADROOM_MCP_READ=off` and a `HEADROOM_EXCLUDE_TOOLS` list containing
`*llm_wiki*` plus entries derived from `instance.rs` (covering both the
production and test MCP namespaces), then
`exec`s `headroom` with forwarded args. `llm-wiki` itself starts, supervises,
and health-checks no proxy; Headroom owns whatever `wrap` or `proxy` does after
the exec.

- **Guaranteed:** `HEADROOM_MCP_READ=off` is set on the Headroom process (unless
  an explicit `--unsafe-mcp-read` override, which *removes* the variable so a
  parent `=on` cannot leak through).
- **Not guaranteed:** that `HEADROOM_EXCLUDE_TOOLS` prevents compression on any
  host. In installed `headroom-ai 0.24.0`, the Codex/OpenAI-Responses
  `function_call_output` path does not consult `exclude_tools` for normal
  tool-output compression. The command is never described as "wiki-safe" and is
  not a provenance boundary; wiki/raw provenance comes from routing through the
  MCP tools and rejecting/detecting compressed payloads when a host transforms
  tool output.

This exception is why the amendment narrows the "zero framework-owned config"
rule: the framework may *compute and emit* this env on demand from a subcommand —
never as an installed asset, profile, or version-pinned file.

## Why

1. **The modes were the proxy.** Every mode beyond "off" existed to make a
   transparent, unsafe-by-default HTTP interceptor safe for wiki/raw: tool-name
   excludes, a version pin, a managed launcher, and a give-up "optimize-off"
   mode. MCP-only inverts the default — compression is opt-in per call, so it is
   structurally never in the read path — and needs none of that apparatus.

2. **The proxy was most broken where it mattered.** The 2026-06-29 five-run field
   test found the Codex carve-out impossible, and the 2026-07-18 source review of
   installed `headroom-ai 0.24.0` confirmed the mechanism: the OpenAI-Responses
   `function_call_output` handler ignores `HEADROOM_EXCLUDE_TOOLS`, while the
   Chat-Completions/Anthropic content-router path is where exclude handling
   lives. The Anthropic carve-out was verified but fragile (five attempts, a
   substitute oracle). This is exactly why the guarantee above rests on
   `HEADROOM_MCP_READ=off`, not the exclude list.

3. **It contradicted our first principles.** Single static Rust binary, no
   vendoring, no Python/Node runtime dependency, framework correctness
   independent of Headroom. The proxy apparatus pulled against all of that. The
   2026-07-07 proxy-off passes on both Codex and Claude make "correctness
   independent of Headroom" literally true: the MCP surface works with no proxy
   in the path.

4. **The real value is ours to capture.** The largest token sinks are wiki/raw
   reads and search/index payloads — which we own in Rust and right-size natively
   (suppressing `index` embedder stderr, shaping search payloads, and adding
   compression-aware pagination where useful), harness-neutral and
   single-binary-safe.

5. **The launch convenience is a small, honest bridge.** Its safety-critical part
   (`HEADROOM_MCP_READ=off`) is a one-liner most likely to be fumbled by hand, and
   the 2026-07-06 Codex retry credits the uncompressed read to it. A stateless
   subcommand couples nothing: no file, no version pin, correctness holds with
   Headroom absent.

## No-Legacy Reduction (2026-07-07)

This decision is a no-legacy cutover. The passthrough `headroom` command is the
*only* Headroom concept anywhere in the repo. Removed from code: the proxy apparatus, the
`--with-headroom` / `uninstall --headroom` flags, the profile asset, the launcher,
doctor probes, the proxy e2e harness, and the stale-manifest compat shim
(`ManagedAssetKind::Unknown` and its cleanup) — safe because `--with-headroom`
never shipped to production, so no manifest in the wild carries a Headroom asset.
Removed from the wiki: the exploratory proxy plans, the runtime-companion
proposal/decision, and the candidate/rejected proxy evals (their load-bearing
conclusions are folded into this decision). Kept: this decision, the trimmed
reference, the wrap plan, the safety rule, and the two 2026-07-07 proxy-off
passes as proof. `install` / `uninstall` / `doctor` build and run cleanly with
zero Headroom references.

## Proper Configuration Interface (Future — Exit Condition)

The `wrap` convenience is a bridge, not a destination. It exists only because
Headroom offers no first-class way to protect specific tools. It is re-evaluated
and simplified or removed when Headroom exposes either:

- **path-based read exclusion** (so wiki/raw is protected by path, not by
  enumerating tool names), or
- **a stable config surface** (a config file or flag naming protected tools) that
  the framework can point at instead of hand-assembling `HEADROOM_EXCLUDE_TOOLS`.

When such an interface lands, the preferred design is for llm-wiki to emit its own
tool names (or a path set) through a neutral introspection output and for the user
(or that interface) to consume it — keeping Headroom-specific wiring out of the
framework binary again.

### Option A Headroom Support Posture (2026-07-21)

After live Headroom 0.32.0 + Codex field tests with the managed `llm-wiki 0.2.13`
binary, the project chooses Option A: compact search is the supported
Headroom-safe discovery surface, and exact large `wiki/` / `raw/` reads are not
supported through Headroom. `llm_wiki_read` remains the exact-read contract for
pure MCP sessions; when a Headroom-launched session returns a CCR marker,
compression envelope, omitted field, or hash/length mismatch, that is a failed
delivery check. The framework does not add Headroom read pagination in the
current product posture; reopening that would require a separate decision.

## Consequences

- A single top-level `headroom` verb appears in `llm-wiki --help`. Accepted
  trade-off (operator decision, 2026-07-07): discoverability over surface
  minimalism.
- A real, tested apparatus was deleted (profile asset, launcher, exclude list,
  version pin, `--with-headroom`, doctor probes, proxy e2e, and their tests). This
  is a deliberate reversal of the earlier four-mode companion work.
- The MCP onboarding + guidance work is unaffected and complementary: this
  decision is *why* the MCP tools are the surface; those plans wire the server per
  project and route generated AGENTS.md to it.

## Non-Goals

- Not restoring any managed Headroom apparatus (profile, launcher, version pin,
  `install --with-headroom`, doctor probes, proxy e2e).
- Not claiming Headroom proxy mode is a framework-supported correctness path, or
  that `HEADROOM_EXCLUDE_TOOLS` is a provenance guarantee.
- Not making Headroom required for any llm-wiki operation.
- Not weakening the rule that wiki/raw reads go through the MCP tools.
- Not vendoring Headroom or taking any runtime dependency.
