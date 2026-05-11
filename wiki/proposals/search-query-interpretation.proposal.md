# Semantic and Hybrid Search for Natural-Language Queries

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-11
- Category: Search UX, semantic retrieval, qmd-rs adapter
- Scope: Move `llm-wiki search` beyond the current FTS-only path by adding semantic search, hybrid fusion, query expansion, and reranking for natural-language project questions. The immediate trigger is that `llm-wiki search 'what are the most cutting edge battery technologies'` returned zero results while a manually optimized keyword query worked.
- Sources: wiki/decisions/search-backend-selection.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/evals/search-backend-selection.eval.md, assets/skills/wiki-query/SKILL.md (lines 32-34), src/search/qmd_rs.rs (current `store.search_fts` path), dogfooding session 2026-05-11 on `/Users/nicolasmartino/Documents/car/electric`
- Related: wiki/specs/wiki-query-skill.spec.md, wiki/decisions/search-backend-selection.decision.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/evals/search-backend-selection.eval.md, wiki/proposals/cli-observability.proposal.md

## Question

Should the framework fix natural-language search failures by teaching the
agent/skill to handcraft better FTS keywords, or by implementing the
LLM-enhanced search path qmd-rs was selected to enable: semantic retrieval,
hybrid fusion, query expansion, and reranking?

## Proposal

Implement the qmd-rs semantic/hybrid search path as the real fix for
natural-language project search.

The current `llm-wiki search` path is lexical FTS. It sanitizes the user's
string, sends the result to qmd-rs `search_fts`, and inherits FTS semantics.
That is fast and useful for exact technical queries, but it is not the product
vision captured in the search backend decision. qmd-rs was selected because it
keeps FTS, vector search, hybrid fusion, query expansion, and reranking behind
one Rust adapter boundary.

The new search contract should have three explicit retrieval modes:

1. `fts` - current deterministic lexical search, kept for exact terms, scripts,
   offline use, and fallback.
2. `semantic` - vector retrieval over wiki chunks/pages using a configured local
   embedding model.
3. `hybrid` - the intended natural-language path: expand the query, run lexical
   and semantic retrieval, fuse candidates, and rerank the top set.

`hybrid` should become the recommended mode for `wiki-query` and direct human
natural-language use once model readiness and eval gates pass. `fts` remains a
stable escape hatch and fallback. The binary must not silently download or use
large models; model resolution, cache paths, and missing-model guidance remain
explicit and visible through `doctor` and verbose diagnostics.

## Retrieval Pipeline

The hybrid path should be staged so each layer is observable and testable:

1. **Normalize the raw query.** Keep existing FTS sanitization for the lexical
   branch, but do not force natural-language input into an AND-only FTS query as
   the only retrieval path.
2. **Expand the query.** Use qmd-rs query expansion when configured to produce
   a small set of alternate phrases, synonyms, and domain-specific terms. Record
   expansions in verbose output and JSON metadata.
3. **Run lexical retrieval.** Search the original and expanded lexical queries
   through FTS/BM25 for exact terminology, filenames, command names, document
   classes, and project-specific identifiers.
4. **Run semantic retrieval.** Embed the natural-language query and retrieve
   semantically similar chunks/pages from the wiki index.
5. **Fuse candidates.** Combine FTS and semantic candidates with reciprocal rank
   fusion or qmd-rs hybrid helpers, preserving canonical wiki paths.
6. **Rerank top candidates.** Use a local reranker model when configured to
   reorder the fused top-K candidates against the original user query.
7. **Return retrieval results, not answers.** `llm-wiki search` and
   `search-all` still return pages/snippets/metadata. The host agent reads the
   pages and synthesizes cited answers.

## Command Contract

Add an explicit mode surface rather than overloading current FTS behavior:

```text
llm-wiki search --mode fts "qmd-rs search-all"
llm-wiki search --mode semantic "what are the most cutting edge battery technologies"
llm-wiki search --mode hybrid "what are the most cutting edge battery technologies"
```

Open command-surface details belong in the implementation plan, but the product
contract should be:

- `fts` works without model files and preserves current deterministic behavior.
- `semantic` and `hybrid` require model readiness and a compatible semantic
  index; if unavailable, the command should fail with actionable guidance or
  fall back only when the user/skill explicitly allows fallback.
- `search-all` supports the same modes but reports per-project readiness,
  skipped projects, and mixed-mode fallbacks clearly.
- text output stays concise; JSON output includes retrieval mode, model IDs,
  expansion terms, fusion/rerank metadata when available, and freshness.
- verbose diagnostics show raw query, FTS query, expansion terms, semantic model
  status, embedding index status, fusion/rerank stages, candidate counts, and
  final result count.

## Skill Contract

`wiki-query` should stop treating `llm-wiki search "<question>"` as a plain FTS
primitive when the index is large or the question is complex.

Once hybrid search exists, the skill should:

1. read `wiki/index.md` first, as today
2. read obvious relevant pages from the index, as today
3. use `llm-wiki search --mode hybrid "<question>"` when index navigation is
   insufficient and hybrid readiness is available
4. fall back to `--mode fts` with corpus-derived keyword phrases only when
   hybrid search is unavailable or explicitly not desired
5. continue reading returned pages before answering and cite wiki paths

This keeps the agent responsible for answer synthesis and citation discipline,
while moving retrieval optimization into the search adapter where qmd-rs can use
semantic retrieval, expansion, fusion, and reranking.

## Why

1. **The observed bug is a symptom of FTS-only search.** The failed battery
   query was not proof that the skill needs a better stopword list; it showed
   that a natural-language question was being forced through the wrong
   retrieval mode.
2. **This matches the accepted backend decision.** qmd-rs was chosen over a
   simpler SQLite FTS adapter because it supports the later semantic/hybrid path
   behind one Rust adapter.
3. **Keyword rewriting is an interim workaround.** Asking the host LLM to
   invent better FTS queries can help, but it duplicates work that the search
   system should own and can introduce terms that do not exist in the corpus.
4. **Hybrid preserves exactness and recall.** FTS catches exact project terms;
   semantic search catches conceptual matches; query expansion and reranking
   improve the top-K result quality for realistic questions.
5. **It keeps `search` as retrieval, not answer generation.** The CLI should not
   become a hidden question-answering agent. It should return better candidate
   pages for the host agent to inspect.

## Alternatives Considered

1. **Skill-only keyword extraction.** Cheap and useful as a fallback, but it
   leaves direct CLI search weak and makes every runtime skill carry search
   optimization rules that belong in the backend.
2. **FTS stopword stripping and OR fallback.** A reasonable interim patch, but
   it addresses only one lexical failure mode and can reduce precision for
   exact queries unless carefully gated.
3. **Make FTS OR the default.** Rejected. It improves recall for question-shaped
   input but weakens precise technical search.
4. **Have the CLI call an external hosted LLM to rewrite queries.** Rejected for
   v1. It adds API-key, privacy, latency, cost, and availability concerns that
   conflict with the local binary direction.
5. **Replace FTS entirely with semantic search.** Rejected. Exact command names,
   filenames, class/status filters, and project identifiers still need lexical
   search.

## Consequences and Tradeoffs

- Model management becomes part of the search product. `doctor` must report
  embedding, expansion, and reranker model readiness, cache paths, versions,
  and rebuild guidance.
- Semantic indexes need schema metadata that includes model IDs, chunking
  strategy, embedding dimensions, qmd-rs version, and source wiki freshness.
- Query results become less purely deterministic in `semantic` and `hybrid`
  modes. Evals must pin model versions and record mode metadata.
- Cold-start and first-index costs increase when model downloads or embedding
  generation are required. The binary must keep downloads explicit and avoid
  embedding large GGUF files in release artifacts.
- `search-all` needs mixed-readiness handling because not every registered
  project will have semantic indexes or compatible model metadata.
- Verbose diagnostics become more important. Users need to know whether a zero
  result came from no lexical hits, missing semantic readiness, expansion
  failure, stale indexes, filters, or reranking.
- FTS remains the compatibility and offline path. Existing lexical evals and
  command contracts should keep passing.

## What Would Close This Proposal

Acceptance, promotion to a decision and a dedicated implementation plan, then:

1. Confirm the qmd-rs APIs, model defaults, cache behavior, licensing, and
   release-footprint implications for embeddings, query expansion, hybrid
   fusion, and reranking.
2. Extend the adapter and CLI mode contract with `fts`, `semantic`, and
   `hybrid`, preserving current FTS behavior.
3. Add explicit model/cache diagnostics in `doctor` and verbose search output.
4. Add semantic index metadata and freshness checks keyed by model ID, chunking
   strategy, qmd-rs version, and wiki source snapshot.
5. Implement semantic retrieval over wiki chunks/pages with canonical wiki path
   preservation.
6. Implement query expansion, hybrid fusion, and optional reranking with
   transparent result metadata.
7. Update `wiki-query` so it uses hybrid search for large/complex questions
   when ready, and falls back to FTS keyword phrases only when hybrid is
   unavailable.
8. Add eval coverage for both exact lexical queries and natural-language
   questions, including the dogfooding failure shape where the answer exists in
   the wiki but the raw FTS query returns zero results.
9. Verify text and JSON outputs, `search-all` mixed-readiness behavior, and
   offline/missing-model failure messages.

## Remaining Questions

1. Should `hybrid` become the default for interactive `search` once ready, or
   should users and skills opt in with `--mode hybrid` until a larger eval proves
   quality and latency?
2. What command should explicitly prepare model/index readiness: `doctor`
   guidance only, `index --mode hybrid`, or a separate model/cache command?
3. Should reranking be required for `hybrid`, or optional when the reranker
   model is not installed?
4. What is the first accepted natural-language eval set, and what top-K quality
   threshold is enough to promote hybrid search as the recommended path?
