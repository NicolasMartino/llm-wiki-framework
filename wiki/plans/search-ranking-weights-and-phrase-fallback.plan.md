# Plan: Lexical Search Weights Titles And File Names, And Falls Back To Phrases

- Document Class: Plan
- Status: Draft
- Date: 2026-10-06
- Category: Search, tests
- Scope: Make a lexical search for a page's own title or file name put that
  page first, and make a query whose words no single page holds all of
  return the pages that hold its hyphenated names, each proven by a test on a
  frozen wiki.
- Sources:
  - Issue #25, "Search: Weight titles and file names, and fall back to
    phrases"
  - The investigation on issue #7, comment of 2026-10-06
    (https://github.com/NicolasMartino/llm-wiki-framework/issues/7#issuecomment-6020083709)
  - The code on `develop` at `9b64345`, read for this plan
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P16 (this plan), P9 (the
    investigation), P15 and P17 (compact search)
  - `wiki/plans/search-eval-test-frozen-wiki.plan.md` (the frozen wiki this
    plan's tests read)
  - `wiki/evals/natural-language-search.eval.md` (the lexical column this plan
    may move)
  - `src/search/qmd_rs.rs`, `src/search/sanitize.rs`,
    `tests/fixtures/search-eval/`

## What This Proves

Searching for a page by its own name finds it at the top, even when its
words are common across the wiki; and a query that names several pages at
once returns those pages instead of the index and the log.

## Where It Stands (2026-10-06)

Checked against the code at `9b64345`:

- The lexical index is one SQLite FTS5 table with three columns, `filepath`,
  `title` and `body`, tokenised with `porter unicode61`
  (`vendor/qmd-0.3.2/src/store.rs`, about line 224). The file name is
  indexed: `operation-manager.checklist.md` becomes the words
  `operation manager checklist md`.
- The query is cut into lowercase letters-and-digits words, repeats dropped,
  joined with spaces (`sanitize_fts_query`, `src/search/sanitize.rs`). FTS5
  reads a space as AND: every word must be in the page. A hyphen splits a
  word, so `headroom-wrap-command` becomes three separate words.
- The ranking is `bm25(documents_fts)` with no column weights
  (`immutable_search_fts`, `src/search/qmd_rs.rs`, the call at about line
  697). A word in the title or file name counts the same as any word in the
  body.
- `search_project` (same file, about line 99) asks the table for a window of
  rows larger than `limit` (four times it, at least 20), drops the rows the
  class and status filters reject, and keeps the first `limit`. Nothing
  re-runs the query when it returns too few.
- The investigation measured, on the live wiki at `ad18a00` (123 pages):
  - "operation manager": both words are in more than half the pages
    (62 and 78 of 123), so BM25's word weight is clamped to almost zero,
    every one of the 47 matching pages scores about 0.000, and the
    "Operation Manager" checklist came 6th. With weights 10, 10, 1
    (file path, title, body) it came 1st; for "operation manager checklist",
    2nd instead of 10th. Weights 20, 20, 1 gave the same ranks as 10.
  - The four plan names `headroom-mcp-merge-readiness-repair`,
    `macos-installed-binary-codesign-repair`, `headroom-passthrough-launcher`
    and `headroom-wrap-command` searched together become 12 words that must
    all be in one page: only the index, the log, the setup plan and the
    roadmap list all four, so no plan is returned. Each name searched alone
    finds its plan first or second. A plain OR of the 12 words brings back
    only two of the plans in the top 8, because common words (`repair`,
    `command`) flood the list. Each name kept as a phrase, the phrases joined
    by OR, with weights 10, 10, 1: three of the four plans in the top 8;
    the merge-readiness plan came below pages that mention it.
- The search quality test,
  `fixed_eval_queries_keep_expected_targets_in_top_two`
  (`src/search/qmd_rs.rs`, about line 1634), indexes the frozen wiki in
  `tests/fixtures/search-eval/wiki/` (the live wiki at `7940130`, 121 files)
  and checks that eight fixed queries each find an expected page in the top
  two. Its README says the copy is refreshed only together with the test's
  queries and targets.
- The frozen wiki holds the "Operation Manager" checklist and all four plans
  named above. A rough count for this plan (grep of word forms, not the
  index) puts "manager" and its stem in about 75 of 121 pages but
  "operation" in about 57, just under half; the index's own count decides.

## Target

- **Weights:** the lexical ranking weights the file path and title columns
  above the body; 10, 10, 1 is the measured starting point. The weights live
  in one place, named, with the reason in a comment. It is a query-side
  change: no reindex.
- **Fallback:** when the all-words query leaves fewer than `limit` results
  after the filters, search again with each hyphen-joined name of the raw
  query kept as one phrase (a lone word is a one-word phrase), the phrases
  joined by OR, and add the new pages after the all-words results, without
  repeating a page, up to `limit`. The all-words results keep their place:
  plain word-level OR ranks worse, so it never comes first. Filters apply to
  the added pages too. When the phrase query can only return pages the
  all-words query already found (a query of one name or one word), it is
  not run.
- **Tests on a frozen wiki:**
  - a page whose title's words are each in more than half the frozen wiki's
    pages, counted on its index, comes first when searched by its title;
  - the four plan names searched together return the plans (the bar is an
    open choice below);
  - each new test fails on the ranking as it is at `9b64345` (shown once, not
    committed), so it guards the fix rather than describing today.
- **The existing quality test** keeps its eight queries and targets and stays
  green. If the weights move a query out of the top two, the PR says which,
  by how much and why, and the owner decides before any target changes; no
  target is replaced to make it pass.
- **Unchanged:** the score scale and what the CLI prints (see "Open For The
  Owner"), semantic and hybrid ranking, and the index format.

## Phases

1. **Measure before changing.** Index the frozen wiki and record, in this
   plan: how many pages hold each word of the chosen title; the target's rank
   for its title with no weights; what the four plan names return; and where
   the eight existing queries' targets rank.
2. **Weights.** Add the column weights; rerun the measurements of phase 1 and
   record them.
3. **Fallback.** Keep hyphen-joined names as phrases for the fallback query
   only, and add the fallback after the all-words results as the Target says;
   rerun the measurements and record them.
4. **Tests.** Add the two regression tests on the frozen wiki and show each
   fails on the old ranking.
5. **Impacts.** A first list of what may move, to be rechecked by whoever
   does the work:
   - the snapshots and ordering assertions in `tests/search_commands.rs` and
     `tests/snapshots/`;
   - hybrid search's lexical branch, which calls the same `search_project`
     (seen in `src/eval.rs`); hybrid is out of scope, so a change in its
     results is recorded, not tuned;
   - the ignored `tests/natural_language_search_eval.rs`, which asserts
     hybrid and auto beat lexical: a better lexical column could make that
     assertion fail on its next manual run. Replay the eval page's lexical
     column (its query form, top 10) before and after on the same tree and
     record both counts here; the investigation saw 18 of 26 at `ad18a00`,
     with case C5 returning nothing.
6. **Prove it on the PR**: the fast check, and `just verify` locally.

## Done When

- A test on the frozen wiki puts a page first for its own title, where each
  word of that title is in more than half the frozen wiki's pages, and that
  test fails on the ranking at `9b64345`.
- A test on the frozen wiki searches the four plan names together and gets
  the plans, to the bar the owner sets below.
- `fixed_eval_queries_keep_expected_targets_in_top_two` passes with its
  queries and targets unchanged, or the PR names each change and its reason
  and the owner accepted it.
- This plan records the before and after measurements of phases 1 to 3 and
  the lexical eval replay.
- The fast check passes on the PR into `develop`, and `just verify` passes
  locally with nothing skipped.
- The plan is `Completed (develop)` and P16 is Completed, in the PR that does
  the work.

When done, the pages to update: this plan (what was found), P16's status,
and any wiki page that still says lexical search is all-words only or
unweighted, found by searching the wiki for it at that time.

## Open For The Owner

1. **The zero word-weight clamp.** BM25 gives a word in more than half the
   pages a weight of almost zero, so such a search prints scores of 0.000
   even when the order is right. The weights fix the order, not the scores.
   Making scores meaningful (normalising them, or a small added boost for a
   title or file-name match) changes what every lexical score means to its
   readers, hybrid fusion included. Recommendation: leave it out of this
   plan, whose promise is the order, and open its own roadmap entry only if
   a reader is found misled by the 0.000 scores.
2. **The bar for the four plan names.** The investigation measured three of
   the four plans in the top 8 on the live wiki, with the fallback and
   weights; the fourth came behind pages that mention it. Recommendation:
   the test asserts at least three of the four in the top 10 on the frozen
   wiki, and this plan records where the fourth lands; asking for all four
   would mean tuning beyond what was measured.
3. **Which title the first test uses.** The rough count above suggests
   "operation" is in just under half the frozen wiki's pages.
   Recommendation: use "operation manager" if the index's count puts both
   words over half; otherwise another page of the frozen wiki whose title
   words both are, chosen in phase 1 and named here. Only if no such page
   exists, add a small separate fixture built for this test, leaving the
   existing frozen wiki unrefreshed, as its README asks.
4. **Saying when the fallback ran.** Pages added by the fallback matched
   only some of the query, and their scores come from a different query, so
   the score column may rise after them. Recommendation: one line in the
   search result's existing warnings when the fallback added pages, so a
   reader knows the later pages are partial matches; no new field.

## Out Of Scope

- Semantic and hybrid search, beyond recording a change their lexical branch
  sees.
- Compact search paging (P17, issue #29) and the stale-index warning
  (issue #36): their own plans.
- The score scale and its printing, unless the owner chooses otherwise above.
- Refreshing `tests/fixtures/search-eval/wiki/`.
