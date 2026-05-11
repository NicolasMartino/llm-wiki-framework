# Plan: Semantic and Hybrid Search

- Document Class: Plan
- Status: Active
- Date: 2026-05-11
- Category: Search UX, semantic retrieval, qmd-rs adapter
- Scope: Implement natural-language `llm-wiki search` through explicit lexical, semantic, hybrid, and auto modes while preserving current lexical behavior and keeping model/index state inspectable under `~/.llm_wiki`.
- Sources: wiki/proposals/search-query-interpretation.proposal.md, wiki/proposals/cli-observability.proposal.md, wiki/decisions/search-backend-selection.decision.md, wiki/evals/search-backend-selection.eval.md, wiki/references/qmd-rs-search-crate.reference.md, assets/skills/wiki-query/SKILL.md, user instruction 2026-05-11 to treat CLI observability as finished in a separate worktree
- Related: wiki/proposals/search-query-interpretation.proposal.md, wiki/proposals/cli-observability.proposal.md, wiki/decisions/search-backend-selection.decision.md, wiki/evals/search-backend-selection.eval.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/specs/wiki-query-skill.spec.md, wiki/specs/documentation-model.spec.md

## Deliverable

`llm-wiki search` and `llm-wiki search-all` support a real semantic/hybrid
retrieval path for natural-language questions:

```text
llm-wiki search --mode auto "question"
llm-wiki search --mode lexical "exact identifier"
llm-wiki search --mode semantic "question"
llm-wiki search --mode hybrid "question"
llm-wiki search --mode hybrid --allow-lexical-fallback "question"
llm-wiki search --mode hybrid --rerank "question"
```

The default no-flag mode is `auto`. `auto` selects `hybrid` only when the saved
interactive install profile enables LLM search and the current project has a
fresh compatible semantic index. A binary invoked before the base interactive
install has completed fails with actionable `llm-wiki install` guidance. After
base install, `auto` selects lexical when LLM search was declined or when an
interrupted LLM-search setup left no completed search profile. Missing/stale
semantic state under an enabled profile remains a readiness error.

This plan treats CLI verbose diagnostics as an available substrate because that
work is being completed in a separate worktree. This branch does not mark the
observability proposal or plan complete; semantic/hybrid implementation must
still verify that per-stage diagnostics exist before the hybrid path ships.

## In Scope

- Accept `auto`, `lexical`, `semantic`, and `hybrid` search modes.
- Preserve current lexical/qmd-rs FTS behavior and existing filters.
- Add explicit `--allow-lexical-fallback` control for callers that can accept
  lexical degradation.
- Configure LLM search through the interactive install surface.
- Materialize selected models during install after user consent.
- Record search profile, model IDs, artifact hashes, accepted-license records,
  diagnostics, semantic indexes, and external dependency inventory under
  `~/.llm_wiki` when practical.
- Exclude large rebuildable model/index directories from macOS Time Machine
  where supported while keeping control-plane files backup-friendly.
- Add semantic index metadata and freshness checks keyed by source content,
  model artifact hash, embedding dimensions, chunking strategy, qmd-rs version,
  and adapter schema version.
- Implement chunk-level semantic retrieval with document rollup to canonical
  wiki page paths.
- Implement baseline hybrid retrieval: query expansion, lexical retrieval,
  semantic retrieval, relevance floors, rank fusion, and optional reranking.
- Make `search-all` run retrieval per project and merge by rank rather than
  comparing raw semantic scores across projects.
- Update `wiki-query` so it can use `auto` search for large or unclear queries
  and interpret retrieval metadata before reading pages and citing answers.

## Out Of Scope

- Hosted LLM APIs for query rewriting, embedding, or reranking.
- Answer synthesis inside the binary.
- Silent model downloads.
- Mandatory reranking for baseline hybrid.
- Replacing lexical search.
- Automatic fallback from explicit semantic/hybrid modes unless the caller uses
  `--allow-lexical-fallback`.
- Windows release artifacts beyond preserving a clean future path.
- Runtime support beyond the current binary command surface.

## Accepted Contracts

1. V1 acquisition is interactive `llm-wiki install`. Non-interactive public
   acquisition guidance is superseded for hybrid-capable installs.
2. `llm-wiki install --configure-search` is the profile refresh surface. It
   reruns diagnostics, lets the user change LLM search settings, materializes
   required models, updates `~/.llm_wiki/search.toml`, and marks incompatible
   semantic indexes stale.
3. `auto` is driven by the saved install profile, not query-shape heuristics.
4. Explicit `semantic` and `hybrid` modes fail on missing readiness unless the
   caller opts into lexical fallback with `--allow-lexical-fallback`.
5. New LLM search state belongs under `~/.llm_wiki` unless a dependency cannot
   reasonably live there; external state is recorded in
   `~/.llm_wiki/external-dependencies.toml`.
6. Search stores migrate into the managed runtime home. The target project
   index root is `~/.llm_wiki/indexes/<project-key>/`, containing the lexical
   qmd-rs FTS store, semantic index metadata, and semantic artifacts. Existing
   `~/.cache/llm-wiki/indexes/<project>/qmd-rs.sqlite` stores are legacy
   rebuildable caches: `search` may read them only until the next
   `llm-wiki index`, `index` writes the managed root, and `doctor` reports cleanup or
   rebuild guidance. Do not dual-write indefinitely.
7. Baseline `hybrid` requires the embedding model and query-expansion model from
   the selected profile. `semantic` requires only the embedding model. Reranking
   remains opt-in and requires a reranker only when `--rerank` is requested.
8. Text output stays concise. JSON output includes selected mode, fallback
   reason, model/profile metadata, threshold/fusion/rerank metadata when
   applicable, and zero-result or readiness reasons.

## Stage 0 - Preconditions And Eval Shape

Close the non-code preconditions before semantic implementation begins.

1. Verify model licensing, default download URLs, caching behavior, accepted
   terms, artifact hashes, redistribution implications, and qmd-rs resolver
   behavior for the embedding, query-expansion, and reranker candidates.
2. Record the accepted model candidates and license findings in a reference or
   eval page before wiring downloads into install.
3. Finalize the first natural-language eval suite and expected target pages.
   The agent may draft targets from the wiki, but a human must approve the
   expected targets before they can tune thresholds or gate implementation.
4. Define threshold methodology:
   - expand the seed set to 30 queries before calibration
   - use 10 labeled calibration queries and 20 labeled hold-out queries
   - keep no-expected-match queries corpus-sensitive and revalidate that they
     still have no expected target at each eval run
   - pin model artifact hashes, embedding dimensions, chunking strategy, qmd-rs
     version, adapter schema version, and threshold values
   - fail closed until numeric threshold values are recorded; semantic/hybrid
     modes must return `thresholds_unconfigured` rather than using arbitrary
     default floors
   - define a regression as any expected-match query losing its expected target
     from top-K, any exact identifier query losing lexical precision, or any
     no-expected-match query returning candidates above the relevance floor
5. Confirm the separate observability work exposes the baseline `CliContext` /
   verbose diagnostics surface; semantic/hybrid stages may extend it but should
   not invent a second diagnostics system.

Stage 0 outputs and gate:

1. `wiki/references/llm-search-model-licensing.reference.md` records model
   candidates, licenses, terms, download sources, artifact hashes, qmd-rs
   resolver behavior, and redistribution implications.
2. `wiki/evals/natural-language-search.eval.md` records the 30-query eval set,
   expected targets, calibration/hold-out split, labeling owner, initial numeric
   thresholds, threshold rationale, and no-expected-match maintenance notes.
3. The observability merge evidence is recorded in this plan or the eval before
   Stage 1 starts. It must name the landed `CliContext` path, confirm a global
   `verbose` field is passed from CLI parsing into command handlers, and confirm
   `search` / `search-all` already emit selected-project, store-path,
   index-state, raw-query, normalized-query, filter, result-count, and no-hit
   diagnostics. If the separate worktree does not land this substrate in time,
   semantic/hybrid work stops at Stage 0 unless this plan is updated with a
   narrower compatibility shim.
4. Stage 1 cannot start until all Stage 0 artifacts exist and are linked from
   `wiki/index.md`.

Seed eval shape:

| ID | Query | Purpose |
| --- | --- | --- |
| NL1 | `what are the most cutting edge battery technologies` | Original dogfood failure shape; conceptual query with weak lexical overlap |
| NL2 | `how does this framework make an agent productive in a fresh project` | Conceptual cross-page framework query |
| NL3 | `why did we choose qmd-rs instead of sqlite` | Decision rationale |
| NL4 | `where should model files and semantic indexes live` | Managed runtime state |
| NL5 | `how should a project answer questions once the index is too large` | Search scale guidance |
| NL6 | `what happens when a semantic index is stale` | Readiness behavior |
| NL7 | `how does search-all combine results from different projects` | Cross-project rank merge |
| NL8 | `what does interactive-only install mean for releases` | Install contract |
| NL9 | `which files prove the wiki init skill works` | Skill/spec retrieval |
| NL10 | `what is the agent allowed to edit` | Agent ownership rule |
| NL11 | `qmd-rs search-all` | Exact identifier preservation |
| NL12 | `--allow-lexical-fallback` | Exact fallback flag retrieval; this plan freezes the flag name |
| NL13 | `D10 composable init packs` | Mixed identifier plus concept |
| NL14 | `wiki-query search metadata` | Skill integration |
| NL15 | `install.partial.json` | Exact file/state retrieval |
| NL16 | `Time Machine model indexes` | Backup policy |
| NL17 | `why is reranking optional in hybrid search` | Reranking contract and rejected mandatory-rerank alternative |
| NL18 | `semantic index chunk ordinal source span` | Index metadata retrieval |
| NL19 | `GPU shader compiler roadmap` | No expected match |
| NL20 | `PostgreSQL connection pooling` | No expected match |
| NL21 | `browser automation plugin release checklist` | No expected match or low-confidence match |
| NL22 | `how do accepted proposals become plans` | Documentation model query |
| NL23 | `what command changes the search profile later` | New plan contract |
| NL24 | `why should hybrid not silently downgrade to lexical` | Fallback contract |

Before code implementation, convert this seed set into an eval page with 30
queries, expected targets, pass/fail rules for lexical, semantic, hybrid, and
auto where each mode applies, and maintenance notes for no-expected-match
queries that may become valid as the wiki grows.

## Stage 1 - Install Profile And Runtime State

1. Extend interactive `llm-wiki install` with an LLM search choice:
   - decline and save `llm_search_enabled = false`
   - enable, run diagnostics, select model/profile, accept license terms,
     download models, verify hashes, and then write a completed profile
2. Add `llm-wiki install --configure-search` for later profile changes.
3. Add `~/.llm_wiki/search.toml` and
   `~/.llm_wiki/external-dependencies.toml`.
4. Record accepted-license data and model artifact hashes.
5. Make interrupted profile setup recoverable: partial downloads can be reused
   only when hashes match, and incomplete profiles must not make `auto` choose
   hybrid. Explicit `--mode hybrid` in this state hard-fails with readiness
   guidance unless the caller passes `--allow-lexical-fallback`.
6. Record managed binary path, managed runtime home, framework version, and
   install ID/hash in generated projects' `.llm_wiki/` metadata.

## Stage 2 - Semantic Indexing

1. Define the semantic chunk schema and adapter schema version.
2. Chunk wiki pages with inherited metadata:
   - canonical wiki path
   - chunk ordinal and source span
   - title
   - Document Class, Status, Category, Scope, and Sources
   - source page content hash
   - model/profile metadata
3. Store lexical and semantic index state under `~/.llm_wiki/indexes/` keyed by
   project ID or canonical root plus model/profile metadata. Migrate away from
   the legacy `~/.cache/llm-wiki/indexes/` qmd-rs store on the next index
   rebuild, and report any remaining legacy cache as rebuildable cleanup state
   in `doctor`.
4. Re-embed changed pages by content hash; rebuild only when model, chunking,
   schema, qmd-rs version, or source snapshot requires it.
5. Extend `llm-wiki index` and `index-all` so they can build/update semantic
   indexes when LLM search is enabled.
6. Extend `doctor` and verbose diagnostics with semantic index readiness,
   freshness, model state, and rebuild guidance.

## Stage 3 - Mode Contract And Readiness

1. Add the CLI mode enum: `auto`, `lexical`, `semantic`, `hybrid`.
2. Keep no-flag search equivalent to `--mode auto`.
3. Implement selected-mode resolution:
   - missing base interactive install -> hard error with `llm-wiki install`
     guidance
   - `llm_search_enabled = false` -> lexical with `llm_search_disabled`
   - interrupted or absent LLM-search profile after base install -> lexical with
     `install_profile_missing`
   - enabled profile with fresh semantic index -> hybrid
   - enabled profile with missing/stale semantic index -> readiness error
4. Add `--allow-lexical-fallback` for scripts and `wiki-query` style
   automation. This is the final fallback flag name.
5. Return structured readiness and zero-result reasons in JSON output.
6. Keep verbose diagnostics on stderr and JSON stdout parseable.

## Stage 4 - Retrieval Pipeline

1. Normalize the raw query for the lexical branch using the existing sanitizing
   path.
2. In `hybrid`, run query expansion with the selected profile's expansion model
   before lexical retrieval. Missing expansion readiness is a hybrid readiness
   error unless `--allow-lexical-fallback` is present. `semantic` does not run
   expansion.
3. Run semantic retrieval against chunk embeddings.
4. Apply relevance floors before returning semantic/hybrid candidates.
5. Fuse lexical and semantic candidates by rank.
6. Optionally rerank with `--rerank` when the reranker is configured.
7. Roll chunk matches up to canonical wiki pages with best snippets.
8. Preserve class/status filters and result limits across all modes.

## Stage 5 - Cross-Project Search And Skills

1. Make `search-all` run per-project retrieval with each project's selected
   mode and readiness metadata.
2. Merge project result lists by reciprocal rank fusion or equivalent rank
   merge, not by raw semantic score.
3. Report skipped projects and lexical fallback per project.
4. Tag every `search-all` result with the project ID, requested mode, selected
   mode, and fallback/readiness reason so mixed-readiness result lists are
   obvious in text and JSON output.
5. Update `wiki-query` so it reads `wiki/index.md` first, uses `auto` search
   only when index navigation is insufficient, inspects retrieval metadata, then
   reads returned pages before answering.

## Stage 6 - Verification, Promotion, And Docs

1. Add unit tests for mode selection, readiness errors, fallback behavior,
   semantic freshness checks, threshold filtering, and rank fusion.
2. Add integration tests with redirected `HOME` for install profile state,
   indexing, search, search-all, doctor, and JSON output.
3. Add model-enabled tests behind explicit fixtures or environment gates; normal
   tests must not silently download models.
4. Run the natural-language eval and record results with model artifact hashes.
5. Keep existing lexical evals and command contracts green.
6. Update README, affected specs, `wiki-query` skill docs, index, and log.
7. Promote validated durable choices into a search-mode decision after evidence
   exists.

## Verification Gates

Required before implementation completion:

1. `cargo fmt`
2. `cargo test --workspace`
3. `just verify`
4. `git diff --check`
5. Natural-language eval page records lexical, semantic, hybrid, and auto
   results where applicable.
6. JSON outputs parse cleanly while verbose diagnostics stay on stderr.
7. No default command silently downloads models.
8. Explicit readiness failures and zero-result outcomes are distinguishable in
   text and JSON output.

## Pages To Update On Completion

- `wiki/proposals/search-query-interpretation.proposal.md` - keep accepted and
  record completion outcome.
- `wiki/decisions/search-backend-selection.decision.md` or a new search-mode
  decision - record durable mode and install/profile choices after validation.
- `wiki/specs/documentation-model.spec.md` - record validated search scale and
  semantic/hybrid behavior.
- `wiki/specs/wiki-query-skill.spec.md` - record how `wiki-query` consumes
  search metadata.
- `wiki/evals/` - record natural-language eval results.
- `wiki/index.md` - keep status and summaries current.
- `wiki/log.md` - append implementation and promotion entries.

## What Closes The Plan

The plan closes when all verification gates pass, `llm-wiki search` and
`search-all` implement the accepted mode contract, interactive install can
configure and materialize LLM search state, semantic indexes are fresh/stale
aware, hybrid retrieval improves the natural-language eval without regressing
exact lexical queries, diagnostics make readiness and zero-result cases
explainable, `wiki-query` consumes the improved retrieval surface, and
specs/decisions/evals record the validated outcome.
