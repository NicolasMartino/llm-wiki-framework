# Semantic and Hybrid Search for Natural-Language Queries

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-11
- Category: Search UX, semantic retrieval, qmd-rs adapter
- Scope: Move `llm-wiki search` beyond the current FTS-only path by adding semantic search, hybrid fusion, query expansion, and optional reranking for natural-language project questions. The immediate trigger is that `llm-wiki search 'what are the most cutting edge battery technologies'` returned zero results while a manually optimized keyword query worked.
- Sources: wiki/decisions/search-backend-selection.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/evals/search-backend-selection.eval.md, assets/skills/wiki-query/SKILL.md (lines 32-34), src/search/qmd_rs.rs (current `store.search_fts` path), dogfooding session 2026-05-11 on `/Users/nicolasmartino/Documents/car/electric`
- Related: wiki/plans/semantic-hybrid-search.plan.md, wiki/specs/wiki-query-skill.spec.md, wiki/decisions/search-backend-selection.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/evals/search-backend-selection.eval.md, wiki/proposals/cli-observability.proposal.md

## Question

How should `llm-wiki search` make natural-language questions work without
requiring humans or host agents to manually rewrite them into FTS keyword
queries?

## Proposal

Implement qmd-rs semantic/hybrid retrieval as the real fix for natural-language
project search, and make the default search mode `auto`.

Promotion note: accepted on 2026-05-11 for implementation planning in
`wiki/plans/semantic-hybrid-search.plan.md`. CLI observability is treated as a
completed prerequisite in a separate worktree for planning purposes; this branch
does not change the status of the observability proposal.

Implementation outcome: completed on 2026-05-12 through
`wiki/plans/semantic-hybrid-search.plan.md` and promoted into
`wiki/decisions/semantic-hybrid-search-mode.decision.md`. The validated v1
baseline is calibrated hybrid/auto search with scoped thresholds and
human-accepted labels; reranking remains opt-in and requires a separate
calibrated profile before it can become part of the default path.

The current `llm-wiki search` path is lexical FTS. It sanitizes the user's
string, sends the result to qmd-rs `search_fts`, and inherits FTS semantics.
That is fast and useful for exact technical queries, but it is not the product
vision captured in the search backend decision. qmd-rs was selected because it
keeps FTS, vector search, hybrid fusion, query expansion, and reranking behind
one Rust adapter boundary.

The no-flag command must get better once semantic readiness exists. Therefore
the user-facing mode surface should be:

1. `auto` - default. Uses the saved install profile: `hybrid` when LLM search
   is enabled, `lexical` when LLM search was declined or is not configured.
2. `lexical` - deterministic FTS/BM25 search, kept for exact terms, scripts,
   offline use, and fallback. Internally this maps to the current FTS adapter
   path.
3. `semantic` - vector retrieval over wiki chunks using a configured local
   embedding model.
4. `hybrid` - natural-language retrieval: query expansion, lexical retrieval,
   semantic retrieval, and rank fusion. Reranking is not required for baseline
   hybrid.

`hybrid` should become the recommended path for `wiki-query` and direct human
natural-language use once model readiness and eval gates pass. `lexical`
remains the stable escape hatch and fallback. The binary must not silently
download or use large models; model resolution, managed state paths, and
missing-model guidance remain explicit and visible through `doctor`, search
result metadata, and verbose diagnostics.

Hybrid search availability is configured during install. Interactive
`llm-wiki install` should ask whether to enable LLM search features, explain
the disk/RAM/latency/model-download implications, run a local machine
diagnostic when the user opts in, and present ordered model/profile choices by
how likely they are to work well on that machine. The user's choice is saved
under the managed runtime home so future searches can select the best available
mode without the user or agent re-deciding per query.
Profile recommendations should be ordered by practical fit first and quality
second: fits in RAM, has enough disk headroom, can use available acceleration
when present, avoids expected thermal/latency cliffs, then prefers the highest
quality tier that still fits those constraints.

For v1, install is intentionally interactive-only. Do not support Cargo, Homebrew,
scripted, or non-interactive install paths as user-facing acquisition flows. A
binary invoked before the interactive install has completed should fail with an
actionable message telling the user to run `llm-wiki install`, rather than
limping forward with partial state. This supersedes earlier install guidance
that treated Cargo/release installers as acceptable acquisition paths.

## Product Contract

The command contract should be explicit:

```text
llm-wiki search "what are the most cutting edge battery technologies"
llm-wiki search --mode auto "what are the most cutting edge battery technologies"
llm-wiki search --mode lexical "qmd-rs search-all"
llm-wiki search --mode semantic "what are the most cutting edge battery technologies"
llm-wiki search --mode hybrid "what are the most cutting edge battery technologies"
llm-wiki search --mode hybrid --rerank "what are the most cutting edge battery technologies"
```

Default `auto` selection:

1. If the saved install profile declined LLM search or no completed interactive
   install profile exists, use `lexical` and report `selected_mode=lexical` with
   reason `llm_search_disabled` or `install_profile_missing`.
2. If the saved install profile enables LLM search, use `hybrid`.
3. If `auto` selects `hybrid` but the current project has no compatible semantic
   index, fail with actionable guidance to run `llm-wiki index`; do not silently
   downgrade to lexical.
4. If `auto` selects `hybrid` but the current project semantic index is stale
   because the wiki, model profile, model artifact, chunking strategy, schema,
   or qmd-rs version changed, fail with `semantic_index_stale` guidance to run
   `llm-wiki index`; do not silently downgrade to lexical.
5. Callers who want deterministic lexical retrieval on a hybrid-enabled install
   use `--mode lexical` explicitly.
6. Explicit `--mode` always wins. If `--mode semantic` or `--mode hybrid` is
   requested and readiness is missing, fail with actionable guidance unless the
   caller explicitly allows lexical fallback.

Fallback should be an explicit contract, not hidden magic. The first plan should
choose the final flag shape, but the proposal's intent is:

```text
llm-wiki search --mode hybrid --allow-lexical-fallback "question"
```

For no-flag `auto`, the saved install profile determines lexical versus hybrid.
For explicit `semantic` or `hybrid`, fallback is opt-in so callers do not
mistake lexical results for semantic/hybrid results.
`--allow-lexical-fallback` is mainly for scripts and `wiki-query`-style
automation that prefer hybrid results but can accept lexical degradation rather
than failing the whole workflow.

Text output stays concise. JSON output must include at least:

- requested mode
- selected mode
- mode selection reason
- fallback reason when fallback occurred
- model IDs and model artifact hashes when semantic/hybrid stages ran
- expansion terms when query expansion ran
- fusion metadata and rerank metadata when available
- zero-result reason when no results are returned
- readiness/error reason when search cannot run in the selected mode

## Install And Runtime State

LLM search should use the managed runtime home as its state root:

```text
~/.llm_wiki/
  install-manifest.toml
  search.toml
  external-dependencies.toml
  diagnostics/
  models/
  indexes/
```

All new files created for LLM search should live under `~/.llm_wiki` when
practical: model files, model manifests, accepted-license records, machine
diagnostics, search profile selection, semantic indexes, hybrid index metadata,
and project index state. This keeps removal, backup, and support simple: users
can inspect one directory, and uninstall/cleanup commands can clearly explain
what they will remove or preserve.

The macOS Time Machine concern is real, but it should be handled by backup
policy rather than by scattering state. On macOS, the installer should create
large rebuildable subdirectories such as `~/.llm_wiki/models/` and
`~/.llm_wiki/indexes/`, mark them as excluded from Time Machine where the
platform supports that, and record the exclusion decision in
`install-manifest.toml` before the first model download or index build. Small
control-plane files such as manifests, diagnostics summaries, profile choices,
and accepted-license records should remain easy to back up. Users should be
able to opt into backing up model/index bytes if they prefer.

This proposal intentionally prefers `~/.llm_wiki` over scattered host cache
locations for LLM search artifacts. Earlier D9 FTS search work used rebuildable
cache paths; the hybrid implementation plan must either migrate search stores
into the managed runtime home or document a compatibility bridge. New semantic
model and index state should not introduce another storage root.

When an LLM search dependency cannot reasonably live under `~/.llm_wiki`, the
installer must record it in `~/.llm_wiki/external-dependencies.toml` for later
inspection, diagnosis, and removal guidance. Examples include OS GPU drivers,
system libraries, package-manager-owned runtimes, or a qmd-rs/model helper that
hardcodes an external cache path. Each entry should record the dependency name,
external path or resolver, why it is external, detected version, hash when
available, install source, owning tool, last-checked timestamp, whether
`llm-wiki` believes it may remove it, and user-facing cleanup guidance. This
keeps `~/.llm_wiki` as the source of truth even when all bytes cannot be stored
there.

The install flow should support:

1. **Decline LLM search.** Save `llm_search_enabled = false`; `auto` resolves to
   `lexical`.
2. **Enable LLM search.** Run local diagnostics, show ordered profiles/models,
   let the user select a profile and acknowledge license/terms, download the
   selected models during the same interactive install, verify them against
   expected artifact hashes, then record the selected profile, model IDs,
   artifact hashes, license/terms acknowledgement, and fallback policy in
   `~/.llm_wiki/search.toml`. Do not leave a "configured but not materialized"
   LLM-search state.
   If install is interrupted before all selected models are downloaded and
   verified, do not write the completed search profile. A later `auto` search
   should resolve to lexical with `install_profile_missing`; rerunning
   interactive `llm-wiki install` retries the setup and reuses any already
   downloaded files whose artifact hashes match.
3. **Change later.** Provide a command to rerun diagnostics and update the
   profile. Changing model/profile marks existing semantic indexes stale.
4. **Remove cleanly.** Provide a cleanup/uninstall surface that can remove
   model files and indexes from `~/.llm_wiki`, then report any external
   dependencies from `external-dependencies.toml` with explicit removal
   guidance instead of hunting through cache directories.

Install configures the global model/profile preference. `auto` reads that
profile only to choose lexical versus hybrid. If the profile selects hybrid,
the current project must have a compatible semantic index under the managed
runtime home, keyed by project ID/root plus model/profile metadata. A missing
or stale semantic index is a hard error with `llm-wiki index` guidance, not a
reason for `auto` to silently select lexical.

Each generated project should also record the managed install path somewhere in
its project-local `.llm_wiki/` directory, either in the existing `init.toml` or
in a dedicated runtime manifest such as `.llm_wiki/runtime.toml`. At minimum it
should include the managed binary path, managed runtime home, framework version,
and install ID/hash written by interactive install. That gives future agents and
upgrade tooling a project-local breadcrumb back to the correct installed binary
without relying on shell `PATH`.

## Result Contract

Hybrid search must be allowed to return zero results.

Vector retrieval always has a top-K, but a low-quality top-K is worse than a
clear miss. The adapter must apply a relevance floor before returning
semantic/hybrid results. A candidate can survive through lexical match strength,
semantic similarity above the configured floor, fusion rank, or an explicit
reranker score when reranking is requested. Candidates below the floor are
dropped.

If lexical returns zero hits and semantic/hybrid candidates all fall below the
floor, the command returns no results with an explicit reason, for example:

```text
No results. Hybrid candidates were below the relevance threshold.
```

JSON output and JSON-formatted errors must distinguish honest zero-result
outcomes from readiness failures:

- zero-result outcomes: `semantic_below_threshold`, `filters_excluded_all`, and
  `no_matching_candidates`
- `auto` lexical selection reasons: `llm_search_disabled` and
  `install_profile_missing`
- hard readiness errors: `semantic_index_missing`, `semantic_index_stale`, and
  `model_missing`

`model_missing` should be rare after install-time materialization; it means a
previously verified model file was removed or corrupted after install, and
`doctor` should catch it.

Thresholds are mode/model contract, not incidental constants. Evals must pin
model artifact hashes and record the threshold used so regressions are
explainable.

## Indexing Contract

Semantic indexing should use chunk-level embeddings with document rollup.

Each embedded chunk must carry:

1. canonical wiki page path
2. chunk ordinal and source span when available
3. page title
4. inherited Document Class, Status, Category, Scope, and Sources metadata
5. source page content hash
6. chunking strategy version
7. embedding model ID, embedding model artifact hash, and embedding dimensions
8. qmd-rs version and adapter semantic-index schema version

Class/status filters apply to the inherited page metadata before final ranking.
Text output rolls chunk matches up to page-level results, showing the best
snippet per page. JSON output can include chunk metadata for debugging and
future UI work, but citations remain canonical wiki page paths.

Indexing must be incremental by page content hash. A wiki edit should re-embed
only changed pages/chunks, not cold-rebuild the whole semantic index unless the
model, chunking strategy, schema version, or qmd-rs version changes.

## Cross-Project Search

`search-all` must not merge raw semantic similarity scores across projects.

Projects may have different embedding models, model hashes, dimensions,
quantization variants, or semantic-index schema versions. Their scores are not
globally comparable. The framework should run retrieval per project, then merge
project result lists by rank using reciprocal rank fusion or an equivalent rank
merge. This matches the existing cross-project direction better than pretending
similarity scores are comparable across heterogeneous indexes.

`search-all --mode auto` should report per-project selected mode and readiness:

- hybrid ready and used
- lexical fallback used
- project skipped because semantic index metadata is missing or stale
- project skipped because explicit semantic/hybrid mode was requested but not
  ready
- project skipped because semantic index metadata is incompatible

## Reranking Contract

Baseline `hybrid` does not require a reranker.

Hybrid should mean query expansion, lexical retrieval, semantic retrieval, and
rank fusion. Reranking is opt-in with `--rerank` because the reranker is likely
to be the slowest stage and can require another large local model. The qmd-rs
eval already identified default model names for embedding, reranking, and query
expansion; together they imply gigabytes of disk and meaningful RAM pressure on
some machines. The default hybrid path should not require all of that before it
can fix the natural-language search failure.

If `--rerank` is requested and the reranker model is missing, the command should
fail with setup guidance unless the caller explicitly allows rerank fallback.
The result metadata must state whether reranking ran.

## Skill Contract

`wiki-query` should mostly treat search improvement as transparent retrieval
infrastructure, not as a new prompt-engineering responsibility.

Once `auto` mode and hybrid readiness exist, the skill should:

1. read `wiki/index.md` first, as today
2. read obvious relevant pages from the index, as today
3. use ordinary `llm-wiki search "<question>"` or explicit
   `llm-wiki search --mode auto "<question>"` when index navigation is
   insufficient
4. inspect result metadata to see whether `auto` selected hybrid or lexical
5. continue reading returned pages before answering and cite wiki paths

The skill should not need a heavyweight `doctor` call on every invocation.
Normal search output, especially JSON output, must expose the selected mode,
readiness/fallback state, and zero-result reason. `doctor` remains the full
diagnostic surface for setup and repair.

This keeps the agent responsible for answer synthesis and citation discipline,
while moving retrieval optimization into the search adapter where qmd-rs can use
semantic retrieval, expansion, fusion, and optional reranking.

## Decision Preconditions

Do not promote this proposal to a durable decision until these preconditions are
answered:

1. **Model licensing and download terms.** Verify the licenses, terms, default
   download URLs, caching behavior, and redistribution implications for the
   embedding, query-expansion, and reranker model candidates. This matters even
   if the binary does not embed the model files.
2. **Interactive-only install.** Accept that v1 has exactly one user-facing
   acquisition path: the custom interactive `llm-wiki install`. Remove or
   explicitly supersede Cargo, Homebrew, scripted, release-installer, and other
   non-interactive install guidance before promoting hybrid search.
3. **Default mode contract.** Accept `auto` as the no-flag default, driven only
   by the saved interactive install profile: hybrid when LLM search is enabled,
   lexical when it is declined or unconfigured.
4. **Threshold methodology.** Define how relevance floors are chosen per
   `(mode, model_artifact_hash)`, identify the hold-out set used for tuning,
   and specify what counts as a regression when a model artifact changes.
5. **Observability sequencing.** Land the CLI verbose diagnostics work before
   or alongside hybrid search. Hybrid must not ship without per-stage
   diagnostics for selected mode, readiness, query expansion, lexical
   candidates, semantic candidates, fusion, optional reranking, thresholds, and
   zero-result reasons.
6. **Natural-language eval shape.** Define the first NL eval set before
   implementation planning. It should include about 20-30 queries covering the
   dogfood failure shape, mixed lexical/semantic questions, exact identifiers
   that must stay precise, and no-expected-match queries.

## Implementation Notes

The implementation plan should stage the pipeline so each layer is observable
and independently testable:

1. normalize the raw query for the lexical branch
2. check semantic/hybrid model and index readiness
3. expand the query when expansion is configured
4. run lexical retrieval against original and expanded lexical queries
5. run semantic retrieval against chunk embeddings
6. filter and threshold candidates
7. fuse candidates by rank
8. optionally rerank when `--rerank` is requested
9. roll chunk results up to canonical wiki page paths
10. return retrieval results, not synthesized answers

`llm-wiki search` and `search-all` still return pages, snippets, scores, and
metadata. The host agent reads the pages and synthesizes cited answers.

## Why

1. **The observed bug is a symptom of FTS-only search.** The failed battery
   query was not proof that the skill needs a better stopword list; it showed
   that a natural-language question was being forced through the wrong
   retrieval mode.
2. **This matches the accepted backend decision.** qmd-rs was chosen over a
   simpler SQLite FTS adapter because it supports the later semantic/hybrid path
   behind one Rust adapter.
3. **Query optimization belongs in search.** The host LLM should not need to
   know whether a query needs expansion, semantic retrieval, rank fusion, or
   reranking. Those are retrieval-layer responsibilities.
4. **Hybrid preserves exactness and recall.** Lexical search catches exact
   project terms; semantic search catches conceptual matches; expansion and
   fusion improve top-K quality for realistic questions.
5. **It keeps `search` as retrieval, not answer generation.** The CLI should not
   become a hidden question-answering agent. It should return better candidate
   pages for the host agent to inspect.

## Alternatives Considered

1. **Rely on users or agents to rewrite queries.** Cheap, but it leaves direct
   CLI search weak and makes every runtime carry search optimization rules that
   belong in the backend.
2. **FTS stopword stripping and OR fallback.** A reasonable interim patch, but
   it addresses only one lexical failure mode and can reduce precision for
   exact queries unless carefully gated.
3. **Make lexical OR the default.** Rejected. It improves recall for
   question-shaped input but weakens precise technical search.
4. **Route `auto` by query shape.** Rejected. Hybrid already includes a lexical
   branch for exact identifiers, and query-shape heuristics create awkward
   collisions for mixed natural-language-plus-identifier queries. Users who need
   deterministic lexical retrieval use `--mode lexical`.
5. **Have the CLI call an external hosted LLM to rewrite queries.** Rejected for
   v1. It adds API-key, privacy, latency, cost, and availability concerns that
   conflict with the local binary direction.
6. **Replace lexical search entirely with semantic search.** Rejected. Exact
   command names, filenames, class/status filters, and project identifiers still
   need lexical search.
7. **Make reranking mandatory for hybrid.** Rejected for the baseline. It would
   raise model, disk, RAM, and latency requirements before proving that fusion
   alone is insufficient.

## Consequences and Tradeoffs

- Model management becomes part of the search product. `doctor` must report
  embedding, expansion, and reranker readiness, managed state paths, versions,
  artifact hashes, and rebuild guidance.
- Semantic indexes need schema metadata that includes model IDs, model artifact
  hashes, chunking strategy, embedding dimensions, qmd-rs version, adapter
  schema version, and source wiki freshness.
- Query results become less purely deterministic in `semantic` and `hybrid`
  modes. Evals must pin model artifact hashes, not just model names.
- Cold-start and first-index costs increase when model downloads or embedding
  generation are required. The binary must keep downloads explicit and avoid
  embedding large GGUF files in release artifacts.
- LLM search increases the importance of runtime-state ownership. New models,
  diagnostics, search profile config, and semantic indexes should live under
  `~/.llm_wiki` so users can remove or inspect framework-owned state from one
  place.
- `search-all` needs mixed-readiness handling because not every registered
  project will have semantic indexes or compatible model metadata.
- Verbose diagnostics are a prerequisite for supportability. Users need to know
  whether a zero result came from no lexical hits, missing semantic readiness,
  below-threshold semantic candidates, filters, stale indexes, or reranking.
- Lexical search remains the compatibility and offline path. Existing lexical
  evals and command contracts should keep passing.

## Promoted Implementation Plan

This proposal is promoted into
`wiki/plans/semantic-hybrid-search.plan.md`. The plan owns these work items:

1. Implement the adapter and CLI mode contract with `auto`, `lexical`,
   `semantic`, and `hybrid`, preserving current lexical behavior.
2. Implement the interactive-only install flow, including local machine
   diagnostics, ordered model/profile recommendations, license/terms
   acknowledgement, blocking model download, hash verification,
   `~/.llm_wiki/search.toml`, and partial-install recovery.
3. Move or bridge search artifact storage so new LLM search state lives under
   `~/.llm_wiki`: models, diagnostics, semantic indexes, profile manifests, and
   model/index metadata.
4. Implement macOS backup policy for large rebuildable `~/.llm_wiki`
   subdirectories, excluding model/index bytes from Time Machine where supported
   while keeping control-plane manifests easy to back up.
5. Implement `~/.llm_wiki/external-dependencies.toml` for every LLM search
   dependency that cannot live under the managed runtime home.
6. Record the managed install path in each generated project's `.llm_wiki/`
   metadata so future agents and upgrade tooling can find the installed binary
   without relying on shell `PATH`.
7. Add explicit readiness, model/cache diagnostics, selected-mode reporting,
   hard-error reasons, and zero-result reasons in `doctor`, verbose search
   output, and JSON search metadata.
8. Add semantic index metadata and freshness checks keyed by model artifact
   hash, chunking strategy, qmd-rs version, adapter schema version, and wiki
   source snapshot.
9. Implement chunk-level semantic retrieval with document rollup and canonical
   wiki path preservation.
10. Implement query expansion, lexical/semantic rank fusion, relevance floors,
   and optional `--rerank` with transparent result metadata.
11. Implement `search-all` as per-project retrieval plus rank merge rather than
   global similarity-score merge.
12. Update `wiki-query` so it relies on `auto` search for large/complex
   questions and treats selected mode, fallback, and zero-result reasons as
   retrieval metadata rather than instructions to handcraft query rewrites.
13. Verify text and JSON outputs, mixed-readiness behavior, incremental
   re-embedding, model reuse after later enablement, offline/missing-model
   failure messages, and explicit fallback flags.

## Remaining Questions

(none for planning)
