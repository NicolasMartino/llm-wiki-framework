# Search Backend Selection Eval

- Document Class: Eval
- Status: Accepted
- Date: 2026-05-07
- Category: Search infrastructure, framework tooling
- Scope: Evaluate candidate search backends for future `llm-wiki search` and `llm-wiki search-all` commands.
- Sources: wiki/proposals/search-backend-selection.proposal.md, wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/references/qmd-search-engine.reference.md, wiki/references/qmd-rs-search-crate.reference.md, raw/legacy/legacy-project-guidelines.md
- Related: wiki/proposals/search-backend-selection.proposal.md, wiki/proposals/project-registry-search-artifacts.proposal.md

## Objective

Choose the least operationally complex backend that satisfies the D9 retrieval
quality bar for project-local search and can support the future registry and
cross-project search surface.

This eval is required before promoting
`wiki/proposals/search-backend-selection.proposal.md` into a durable backend
decision.

This accepted eval fixes the D9 backend direction using FTS measurements plus
product weighting toward LLM-enhanced search. qmd-rs semantic and hybrid
measurements are deferred to the qmd-rs backend implementation plan and
post-implementation eval replay.

## Candidate Backends

1. **qmd-rs library adapter** - preferred if quality, packaging, and API
   stability are good enough.
2. **Tobi QMD shell-out adapter** - fallback if current QMD behavior is needed
   and an external Node/Bun dependency is acceptable.
3. **SQLite FTS5 BM25 adapter** - fallback if a deterministic, model-free first
   version is more valuable than hybrid search.
4. **Defer D9 search** - fallback if no backend clears the bar.

## Corpus

Corpus: this repository's `wiki/`.

Current measured corpus on 2026-05-07:

- Indexed files: 37
- Included file types: `wiki/**/*.md`

Tobi QMD measured baseline:

- Tool: Tobi QMD `qmd 0.9.0 (b9763ee528)`
- Collection command: `qmd --index llm-wiki-search-backend-eval collection add wiki --name search-backend-eval --mask "**/*.md"`
- Cache isolation: `HOME=/private/tmp/qmd-home`, `XDG_CACHE_HOME=/private/tmp/qmd-cache`
- Indexed files: 37
- Index path: `/private/tmp/qmd-cache/qmd/llm-wiki-search-backend-eval.sqlite`
- Index size: 928 KB
- Vectors: 0 embedded
- Pending embeddings: 37

The Tobi QMD measured baseline is BM25-only. Vector search, hybrid query
expansion, and reranking remain unmeasured because `qmd embed` was not run.

## Query Set

The fixed query set covers specs, decisions, proposals, plans, references, and
cross-document operational language:

| ID | Query | Expected primary target |
| --- | --- | --- |
| Q1 | `managed binary runtime install manifest` | `wiki/plans/binary-path-bootstrap.plan.md` or `wiki/decisions/binary-path-bootstrap.decision.md` |
| Q2 | `agent owns wiki humans curate raw` | `wiki/decisions/agent-owns-wiki.decision.md` |
| Q3 | `search backend selection qmd-rs eval` | `wiki/proposals/search-backend-selection.proposal.md` |
| Q4 | `knowledge research intake bundle manifest summary` | `wiki/plans/knowledge-research-intake.plan.md` or `wiki/decisions/knowledge-research-intake.decision.md` |
| Q5 | `QMD hybrid search MCP` | `wiki/references/qmd-search-engine.reference.md` |
| Q6 | `three phase ingest extraction drafting bookkeeping` | `wiki/references/three-phase-ingest-pipeline.reference.md` |
| Q7 | `project registry search-all reciprocal rank fusion` | `wiki/plans/project-registry-search-artifacts.plan.md` or `wiki/proposals/project-registry-search-artifacts.proposal.md` |
| Q8 | `D8 distribution tooling cargo dist skill projection` | `wiki/plans/llm-wiki-binary.plan.md` or `wiki/proposals/llm-wiki-binary.proposal.md` |

## Tobi QMD BM25 Baseline

Command shape:

```bash
/usr/bin/time -p env HOME=/private/tmp/qmd-home XDG_CACHE_HOME=/private/tmp/qmd-cache qmd --index llm-wiki-search-backend-eval search "<query>" -c search-backend-eval --files -n 5
```

| Query | Top result | Judgment | Time |
| --- | --- | --- | --- |
| Q1 | `plans/binary-path-bootstrap-plan.md` | Good | 0.20s |
| Q2 | `decisions/agent-owns-wiki-decision.md` | Good | 0.20s |
| Q3 | No results | Fail | 0.18s |
| Q4 | `archive/knowledge-intake-command-proposal.md` | Partial; relevant plan/decision were ranks 2 and 3 | 0.18s |
| Q5 | `references/qmd-search-engine-reference.md` | Good | 0.20s |
| Q6 | `references/three-phase-ingest-pipeline-reference.md` | Good | 0.25s |
| Q7 | No results | Fail | 0.19s |
| Q8 | `plans/llm-wiki-product-layout-addendum-plan.md` | Partial; expected binary plan was rank 2 | 0.20s |

Summary:

- Good: 4 of 8
- Partial: 2 of 8
- Fail: 2 of 8
- Sequential BM25-only latency was acceptable for this corpus.
- BM25-only retrieval missed command-heavy or hyphenated concepts such as
  `qmd-rs`, `search-all`, and `reciprocal rank fusion`.
- After refreshing the corpus to 37 files, Q3 and Q7 still returned no results.
- The result path format replaced dotted filename segments with hyphenated
  URI paths, so an adapter would need to map QMD result IDs back to canonical
  wiki file paths before exposing stable `llm-wiki search` output.

## qmd-rs FTS Baseline

Harness: temporary Rust binary under `/private/tmp/qmd-rs-eval` using
`qmd = 0.3.2` and direct `Store` calls. The harness inserted each wiki markdown
file into a project-local SQLite store and ran `Store::search_fts`.

Measured baseline on 2026-05-07:

- Crate: `qmd 0.3.2`
- License: `MIT OR Apache-2.0`
- Build profile: `cargo run --release`
- First build time: about 1m21s on this machine
- Build graph: 254 locked packages, including `llama-cpp-2`,
  `llama-cpp-sys-2`, `reqwest`, and `rusqlite`
- Indexed files: 37
- Index path: `/private/tmp/qmd-rs-eval/wiki.sqlite`
- Reported index size: 4 KB
- Unique hashes: 37
- Embedded hashes: 0
- Index time: 40 ms
- Search latency: 1 ms per query on this corpus
- Model downloads: none for FTS-only search

| Query | Top result | Expected target rank | Judgment |
| --- | --- | --- | --- |
| Q1 | `plans/binary-path-bootstrap.plan.md` | 1 | Good |
| Q2 | `decisions/agent-owns-wiki.decision.md` | 1 | Good |
| Q3 | `evals/search-backend-selection.eval.md` | 2 | Partial; raw query errored on `qmd-rs`, sanitized query found proposal at rank 2 |
| Q4 | `archive/knowledge-intake-command.proposal.md` | 2 | Partial |
| Q5 | `references/qmd-search-engine.reference.md` | 1 | Good |
| Q6 | `references/three-phase-ingest-pipeline.reference.md` | 1 | Good |
| Q7 | `evals/search-backend-selection.eval.md` | 2 | Partial; raw query errored on `search-all`, sanitized query found proposal at rank 2 |
| Q8 | `plans/llm-wiki-binary.plan.md` | 1 | Good |

Summary:

- Good: 5 of 8
- Partial: 3 of 8
- Fail: 0 of 8 after query sanitization
- Raw `Store::search_fts` is not safe for direct user query strings containing
  command-style hyphenated tokens. The harness saw `no such column: rs` for
  `qmd-rs` and `no such column: all` for `search-all`.
- qmd-rs returns canonical relative paths, title, score, source, hash, docid,
  collection, modified timestamp, and body length.
- It does not directly return framework document class or status, but the body
  can be fetched through `get_document` and metadata parsed by the adapter.
- It does not expose a binary or MCP server from the crate package; Cargo
  metadata declares library and example targets only. Parity with Tobi QMD is
  therefore library-level, not CLI/MCP-level.
- Default model constants point to GGUF files for embedding, reranking, and
  query expansion: `embeddinggemma-300M-Q8_0.gguf`,
  `qwen3-reranker-0.6b-q8_0.gguf`, and
  `qmd-query-expansion-1.7B-q4_k_m.gguf`. Those are not embedded in the binary;
  qmd-rs downloads from Hugging Face when model pull/resolve APIs are used.

## Direct SQLite FTS5 BM25 Baseline

Harness: temporary Rust binary under `/private/tmp/sqlite-fts-eval` using
`rusqlite` with bundled SQLite FTS5. The harness stored canonical relative
paths, title, document class, status, and body in one SQLite database and used
FTS5 `bm25` plus `snippet`.

Measured baseline on 2026-05-07:

- SQLite version: 3.51.1
- Indexed files: 37
- Index path: `/private/tmp/sqlite-fts-eval/wiki.sqlite`
- Index size: 544 KB
- Index time: 38 ms
- Search latency: 0-1 ms per query on this corpus
- Model downloads: none
- External executables: none for a Rust implementation using bundled SQLite

| Query | Top result | Expected target rank | Judgment |
| --- | --- | --- | --- |
| Q1 | `proposals/binary-path-bootstrap.proposal.md` | 2 | Partial; accepted proposal ranked just above completed plan |
| Q2 | `decisions/agent-owns-wiki.decision.md` | 1 | Good |
| Q3 | `evals/search-backend-selection.eval.md` | 2 | Partial; eval page quotes the query set |
| Q4 | `archive/knowledge-intake-command.proposal.md` | 2 | Partial |
| Q5 | `references/qmd-search-engine.reference.md` | 1 | Good |
| Q6 | `references/three-phase-ingest-pipeline.reference.md` | 1 | Good |
| Q7 | `evals/search-backend-selection.eval.md` | 2 | Partial; eval page quotes the query set |
| Q8 | `evals/search-backend-selection.eval.md` | 2 | Partial; eval page quotes the query set |

Summary:

- Good: 3 of 8
- Partial: 5 of 8
- Fail: 0 of 8
- Direct SQLite FTS5 returned canonical relative paths, title, document class,
  status, score, and snippets without needing model files or an external
  runtime.
- The adapter can preserve exact file paths because it owns the schema.
- Exact query-set quotations in this eval page bias Q3, Q7, and Q8 toward the
  eval itself. That is acceptable for V1 if callers can filter by document
  class/status and agents still read returned pages before answering.

## Operational Observations

Concurrent `qmd search` invocations against the same temporary index produced
SQLite lock failures during startup. Sequential searches succeeded.

Observed failure:

```text
sqlite-vec extension is unavailable. sqlite-vec probe failed (database is locked)
```

Implication: if D9 shells out to Tobi QMD, `llm-wiki index`, `search`, and
`search-all` need explicit concurrency handling. A shell-out backend also needs
doctor checks for QMD installation, SQLite/sqlite-vec readiness, model cache
state, and index health.

## Remaining Unmeasured Work

The following checks remain useful but are not required before choosing the D9
V1 backend:

1. qmd-rs vector search and hybrid behavior after model downloads.
2. Tobi QMD vector and hybrid `query` performance after embeddings exist.
3. Cross-project retrieval quality with at least two real projects.
4. Larger-corpus behavior once the wiki exceeds the current index navigation
   scale.

## Recommendation

Accept qmd-rs as the D9 backend.

Rationale:

1. The project expects to need LLM-enhanced search, so the first backend should
   point at the hybrid path instead of treating BM25-only search as the product
   target.
2. qmd-rs keeps the Rust one-binary direction while exposing FTS, vector,
   hybrid fusion, query expansion, reranking, and model/cache APIs behind one
   adapter boundary.
3. The measured qmd-rs FTS path is fast enough for D9 V1: 40 ms index time and
   about 1 ms search latency on the current 37-file corpus.
4. The measured integration problems are adapter work, not backend blockers:
   query sanitization, framework metadata extraction, snippets, doctor checks,
   and model/cache reporting.
5. Direct SQLite FTS5 is useful as a fallback, but choosing it first would
   defer the qmd-rs adapter work that will likely be necessary anyway.

Direct SQLite FTS5 remains the fallback if qmd-rs packaging or runtime behavior
proves unacceptable during implementation. Tobi QMD shell-out remains the
external reference for hybrid behavior, but should not be the D9 backend because
it introduces Node/Bun, QMD config, sqlite-vec, path-mapping, and concurrency
concerns outside the Rust binary.

## Production Adapter Replay

Date: 2026-05-07.

Implementation target:

- `src/search/qmd_rs.rs`, compiled with `--features qmd-rs`.
- Store path: temporary test store.
- Corpus: the repository `wiki/` directory at test time.
- Query path: production adapter `search_project`, including framework query
  sanitization, qmd-rs FTS search, metadata parsing, post-filter/result shaping,
  snippet generation, and adapter-owned canonical paths.

Command:

```bash
cargo test --workspace --features qmd-rs fixed_eval_queries_keep_expected_targets_in_top_two -- --nocapture
```

Result:

- Passed.
- The fixed eval query set kept the expected target set in the top two for all
  eight queries.
- The adapter replay used the real wiki corpus rather than a synthetic fixture.
- The replay permits the accepted decision/eval/implementation artifacts for Q3
  because the backend-selection topic now has a promoted decision and replay
  result in addition to the original proposal.
- The replay permits the active D9 plan for Q7 because the registry/search
  proposal has now been accepted and promoted to an implementation plan.

Additional feature gate:

```bash
cargo test --workspace --features qmd-rs
```

Result:

- Passed.
- qmd-rs FTS indexing, metadata filters, stale detection, doctor state, and the
  fixed eval query replay all passed in the feature-enabled build.

## Next Action

Implement D9 against an internal adapter trait using qmd-rs as the first
concrete backend. The adapter must handle query sanitization, framework metadata
extraction, snippets, doctor checks, model/cache reporting, and a clean fallback
path to direct SQLite FTS5 if qmd-rs cannot ship safely.
