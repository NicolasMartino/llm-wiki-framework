# Headroom And LLM Wiki Comparison Research Summary

- Research Date: 2026-06-11
- Status: Ready for ingest
- Scope: Compare Headroom's runtime context-compression architecture with this project's durable LLM Wiki framework.

## Executive Finding

Headroom should not replace the LLM Wiki framework. It operates at a different layer.

The LLM Wiki framework is a durable project-knowledge system: human-curated `raw/`, agent-owned typed `wiki/`, and `AGENTS.md` as the operating schema. Its core operations are ingest, query, and lint. Headroom is a runtime context optimization layer: it compresses tool outputs, logs, files, RAG chunks, and conversation history before they reach the model, while keeping originals retrievable through CCR.

Best decision: treat Headroom as an optional agent-runtime accelerator and source of design ideas, not as the project knowledge substrate.

## How Headroom Works

Headroom exposes several entry points over the same compression pipeline:

- Python and TypeScript library calls such as `compress(messages)`.
- Local proxy mode through `headroom proxy`.
- Agent wrappers such as `headroom wrap claude` and `headroom wrap codex`.
- MCP tools: `headroom_compress`, `headroom_retrieve`, and `headroom_stats`.

The core pipeline is:

1. CacheAligner detects volatile prompt material so stable prefixes can benefit from provider cache behavior.
2. ContentRouter detects content type and dispatches to specialized compressors.
3. Compressors handle JSON arrays, logs, search results, diffs, HTML/text, and optionally code.
4. CCR stores original content locally and emits retrieval markers.
5. The model or proxy can retrieve the original by hash, optionally filtered by query.

The important implementation detail is reversibility. Headroom can present a compressed view to the model while retaining access to the original payload. This is a good fit for large tool outputs and logs, where most content is redundant but occasional details matter.

## What Headroom Is Good At

Headroom is strongest for:

- Large JSON arrays from APIs, database rows, search tools, or structured logs.
- Build/test logs and repetitive incident/debug output.
- Long agent sessions where tool output accumulates.
- Cross-agent or multi-agent contexts where compressed handoff is useful.
- Runtime token and cost telemetry.

Headroom also has `headroom learn`, which mines prior agent sessions for concrete failure patterns and writes project-level corrections to files such as `CLAUDE.md` or `AGENTS.md`. That overlaps philosophically with LLM Wiki's "durable learning" goal, but Headroom's mechanism is operational and session-derived, not typed project documentation.

## What Headroom Does Not Replace

Headroom does not provide:

- Typed document classes such as specs, decisions, proposals, plans, references, evals, and checklists.
- Provenance-first compilation from immutable raw sources into durable markdown.
- The ingest/query/lint lifecycle.
- A canonical project index like `wiki/index.md`.
- Promotion flow from proposal to roadmap to plan to evidence to spec/decision.
- Human-readable project truth that can be reviewed in Git.

Its own limitations also matter: source code often passes through unchanged, short content passes through, compact grep/search results may not compress, and aggressive code compression is deliberately gated because active code is usually exactly what the user needs to inspect.

## Recommended Integration Shape

Use Headroom in three narrow ways:

1. Optional runtime wrapper for agents working inside LLM Wiki projects.
   - Document `headroom wrap codex` / MCP usage as an optional operating mode.
   - Do not require Headroom for framework correctness.

2. Optional compression for very large command/search outputs.
   - Agents can call Headroom MCP tools to compress noisy outputs and retrieve originals by hash when needed.
   - This complements `rtk`, which rewrites shell output, and `llm-wiki search`, which retrieves project knowledge.

3. Future design inspiration for `llm-wiki` output contracts.
   - A future `llm-wiki search-all` or diagnostics command could return concise result summaries plus stable retrieval handles.
   - This should be implemented in the Rust binary only if it solves a real workflow problem and preserves parseable JSON contracts.

## What Not To Do

Do not:

- Replace `wiki/` with Headroom memory.
- Let `headroom learn --apply` write directly into framework-owned `AGENTS.md` sections without review.
- Make Headroom a required dependency of `llm-wiki init`, `query`, or `ingest`.
- Store durable project truth only in Headroom's CCR or memory stores.
- Treat token compression as equivalent to knowledge synthesis.

## Proposed Framework Decision

If promoted into `wiki/`, the decision should be:

"Headroom is an optional runtime context-compression companion for LLM Wiki projects. The framework remains a durable markdown knowledge system. Headroom may be documented as an optional agent runtime and may inspire future reversible-output features, but it is not the canonical knowledge layer and is not a required dependency."

## Gaps

- No live Headroom proxy or MCP install was tested in this research pass.
- No benchmark was run against this repository's actual `llm-wiki search`, build, or test outputs.
- The downloaded source archive reflects GitHub `main` at review time, not a pinned release tarball.
- A pilot should verify real savings on this project before adding user-facing guidance.

## Suggested Follow-Up

Ingest this bundle as:

- `wiki/references/headroom-context-compression.reference.md`
- optionally `wiki/proposals/headroom-runtime-companion.proposal.md`

Only promote to a decision after a small dogfood run measures whether Headroom helps with this framework's actual agent workflows.
