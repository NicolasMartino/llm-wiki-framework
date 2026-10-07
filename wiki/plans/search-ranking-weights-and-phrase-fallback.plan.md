# Plan: Lexical Search Weights Titles And File Names, And Falls Back To Phrases

- Document Class: Plan
- Status: Completed (develop)
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
  class and status filters reject, and keeps the first `limit`. When a class
  or status filter is active and too few rows survive it, it re-runs the same
  query with the window doubled, up to 100,000 rows, until it has `limit`
  results or the table runs out (about lines 121 to 188; its comment says a
  fixed window silently drops filtered matches ranked below it). Without a
  filter it never re-runs, and no query other than the all-words one is ever
  tried.
- Every search reaches `search_project` through `perform_project_search` and
  `search_attempt` (`src/search/commands.rs`, about lines 3615 and 3677).
  The default mode is `auto` (`src/cli.rs`, about line 287), which runs
  hybrid when its models are ready, so most real searches take
  `perform_hybrid_project_search` (same file, about line 2849). Its lexical
  branch asks for `max(limit, 20)` rows per expanded query. `search_project`
  returns only its results (`src/search/adapter.rs`, about line 174); the
  reply's warnings are built in `src/search/commands.rs`.
- The eval's lexical column calls `search_project` directly
  (`src/eval.rs`, about line 955), not through the search command.
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
  named above. Measured by the blind review of this plan on an FTS5 table
  built from the frozen wiki with the index's columns and tokenizer (porter
  stems operation, operations, operate, operator and operating to one term):
  - "operation" is in 62 of 121 pages and "manager" in 78, both over half;
  - with no weights the checklist ranks 6th of 47 for "operation manager"
    and 11th of 25 for "operation manager checklist"; with weights 10, 10, 1
    it ranks 1st and 2nd;
  - with the phrase-OR fallback after the four all-words pages and weights
    10, 10, 1, the four plans come 5th, 6th, 7th and 10th;
  - with the weights, scores over 15 queries rose by at most 18%, and none
    crossed hybrid's strong-lexical floor of 10.0.
  Phase 1 rechecks these on llm-wiki's own index.

## Target

- **Weights:** the lexical ranking weights the file path and title columns
  above the body; 10, 10, 1 is the measured starting point. The weights live
  in one place, named, with the reason in a comment. It is a query-side
  change: no reindex.
- **Fallback:** when the all-words query leaves fewer than `limit` results
  after the filters, search again with each name of the raw query kept as
  one phrase, the phrases joined by OR, and add the new pages after the
  all-words results, without repeating a page, up to `limit`.
  - **What a name is:** a name ends at whitespace and at a comma, semicolon,
    quote or backtick. Inside a name, hyphens, slashes and dots join its
    words into one phrase, in order. So `qmd-rs,search-all` gives the
    phrases "qmd rs" and "search all", and
    `wiki/plans/headroom-wrap-command.plan.md` gives one phrase that matches
    that page's file path, not a lone "wiki" that every page holds. A lone
    word is a one-word phrase; a query with no joined names makes the
    fallback a plain OR of its words.
  - **Order:** the all-words results keep their place; plain word-level OR
    ranks worse, so it never comes first.
  - **Filters:** the fallback runs after the all-words query's window loop,
    and its own query goes through the same window growth when a filter is
    active, so a filtered search is not cut short by a fixed window.
  - **When it does not run:** when the phrase query can only return pages
    the all-words query already found (a query of one name or one word).
  - **Where it runs:** in lexical search only, never in hybrid's lexical
    branch (the owner's choice, below).
- **Tests on a frozen wiki:**
  - a page whose title's words are each in more than half the frozen wiki's
    pages, counted on its index, comes first when searched by its title;
  - the four plan names searched together return at least three of the
    plans in the top 10, and with the plan filter (`--class plan`) all four
    in the top five, behind the setup plan that names them all (the owner's
    choice, below, as changed on 2026-10-07);
  - unit tests for how a query splits into names: `qmd-rs,search-all` gives
    two phrases, and `wiki/plans/headroom-wrap-command.plan.md` gives one;
  - each new test fails on the ranking as it is at `9b64345` (shown once, not
    committed), so it guards the fix rather than describing today.
- **The existing quality test** keeps its eight queries and targets and stays
  green. If the weights move a query out of the top two, the PR says which,
  by how much and why, and the owner decides before any target changes; no
  target is replaced to make it pass.
- **Unchanged:** the score scale and what the CLI prints (the owner's
  choice, below), semantic ranking, and the index format. The weights do reach
  hybrid's lexical scores, which `hybrid_candidate_survives_final_gate`
  (`src/search/commands.rs`, about line 3398) compares with a calibrated
  strong-lexical floor (10.0 by default); the review saw scores rise by at
  most 18% and none cross it, and phase 2 rechecks that.

## Phases

1. **Measure before changing.** Index the frozen wiki and record, in this
   plan: how many pages hold "operation" and "manager"; the checklist's rank
   for "operation manager" with no weights; what the four plan names return;
   and where the eight existing queries' targets rank.
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
   - hybrid search, through `perform_hybrid_project_search`, the path
     most real searches take: the weights reach its lexical branch in any
     case; the fallback does not (the owner chose "lexical only").
     Hybrid is out of scope, so a change in its results is recorded, not
     tuned;
   - the ignored `tests/natural_language_search_eval.rs`, which asserts
     hybrid ≥ 22 and auto ≥ 22 passes of 30, at most 8 misses each, and
     that hybrid and auto beat lexical: a better lexical column could make
     the last fail on its next manual run. Replay the eval page's lexical
     column before and after on the same tree, through the path a lexical
     search takes (`llm-wiki search --mode lexical --format json`, top 10,
     as the investigation did), and record both counts here. The eval's own
     lexical column calls `search_project` directly, so it sees the fallback
     only if the fallback lives there; the record says which was measured.
     The investigation saw 18 of 26 at `ad18a00`, with case C5 returning
     nothing.
6. **Prove it on the PR**: the fast check, and `just verify` locally.

## Done When

- A test on the frozen wiki puts the "Operation Manager" checklist first for
  "operation manager", both words being in more than half its pages, and
  that test fails on the ranking at `9b64345`.
- A test on the frozen wiki searches the four plan names together and gets
  at least three of the plans in the top 10, recording the fourth's rank; the
  same search with the plan filter gets all four plans in the top five,
  behind the setup plan that names them all (the owner, 2026-10-07).
- The two unit tests on how a query splits into names pass.
- The fallback runs in lexical search only: a test shows hybrid's lexical
  branch gets the all-words results alone, with no fallback pages.
- When the fallback added pages, the reply's warnings say so in one line.
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

## What Was Found (2026-10-07)

Measured on `develop` at `cad8988` with this plan's branch on top. "Before"
is the ranking as it was (no column weights, no fallback), rebuilt on the
branch by setting the weights to 1, 1, 1 and turning the fallback off; the
frozen wiki is the 121-page copy in `tests/fixtures/search-eval/wiki/`.

**Phase 1, before:**
- "operation" is in 62 of 121 pages, "manager" in 78.
- The "Operation Manager" checklist ranks 6th of 47 for "operation manager"
  (every score 0.000) and 11th of 25 for "operation manager checklist".
- The four plan names searched together return four pages, the index, the
  setup plan, the log and the roadmap, and no plan; with the plan filter,
  only the setup plan.
- Each of the eight fixed queries has a target in the top two.

**Phase 2, the weights 10, 10, 1** (the file path, title and body columns,
named constants in `src/search/qmd_rs.rs`, bound into `bm25()`; no reindex):
- the checklist ranks 1st for "operation manager" and 2nd for "operation
  manager checklist";
- the four plan names still return only the same four pages;
- each of the eight fixed queries keeps a target in the top two; the
  binary-path-bootstrap plan moves from 2nd to 1st for "managed binary
  runtime install manifest";
- hybrid's strong-lexical floor: over the eval page's 30 queries, the eight
  fixed queries and the two "operation manager" queries, on llm-wiki's own
  index (129 pages), the 384 all-words pages found both before and after
  rose by at most 9.0% (scores over 0.5), and none crossed 10.0.

**Phase 3, the fallback** (`fts_phrase_fallback_query` in
`src/search/sanitize.rs`; `search_project_with_phrase_fallback` in
`src/search/qmd_rs.rs`, called only by the lexical path in
`src/search/commands.rs`):
- the four plan names, unfiltered, top 10: the four all-words pages, then
  the wrap-command plan 5th, the codesign plan 6th, the passthrough plan 7th,
  and the merge-readiness plan 10th, below two pages that mention it;
- with the plan filter: the setup plan 1st, then the four plans 2nd to 5th.
  The setup plan names all four plans, so it holds all twelve words and is
  the one all-words result, and it holds every phrase too (it comes first in
  the phrase query alone, 11.3 against 10.5). "All four in the top four" can
  only hold by moving an all-words result below the fallback, which the
  Target rules out, so the test asserts all four in the top five, behind the
  setup plan; the owner confirmed this bar on 2026-10-07.
- The fallback's own window grows with a filter as the all-words query's
  does; the pages it adds keep the all-words results first and never repeat
  one.

**Phase 4, the tests**, each run once on the "before" ranking (not
committed): the title test, both plan-name tests, the order test in
`src/search/qmd_rs.rs` and the lexical-versus-hybrid test in
`src/search/commands.rs` all fail; the single-name test is a guard that
passes on both. The blind review's fix round added three more, each failing
on the code it fixes: the fallback's window growth under a filter (fails
with a fixed window), the fallback line in the JSON reply beside a stale
index's line, and search-all naming only the fallback pages it prints (both
fail on the reviewed head, which kept only the first warning in JSON and
copied each project's line before fusion and the limit).

**Phase 5, the impacts:**
- `tests/snapshots/`: no change. `tests/search_commands.rs`: no existing
  case changed; the two CLI cases above were added. The single-project JSON
  reply's `warnings` list now holds every warning (`warning` still holds the
  first), so the stale-index line and the fallback line both reach the MCP
  tool. In search-all, the line counts only the fallback pages among the
  printed results, per project, and does not call them "the last", since
  fusion places them among other projects' results.
- Hybrid: its lexical branch gets the weights and never the fallback
  (`hybrid_lexical_branch_search`, tested); results not tuned.
- The eval page's lexical column, replayed through
  `llm-wiki search --mode lexical --format json`, top 10, on llm-wiki's own
  index at this branch: **18 of 26 before, 22 of 26 after** (4 not
  applicable). C3, C5, H15 and H19 now pass (C5 returned nothing before);
  none was lost; C7, C9, H16 and H18 still miss. The eval's own lexical
  column (`src/eval.rs`) calls `search_project` directly, so it sees the
  weights but not the fallback. The ignored
  `tests/natural_language_search_eval.rs` asserts that hybrid and auto beat
  lexical; with lexical at 22, its next manual run fails unless hybrid and
  auto pass at least 23. Hybrid and auto could not be measured here: LLM
  search is disabled on the machine that did the work, and enabling it
  would change a shared setup. The assertion is unchanged; whether it
  should change is the owner's call, with the eval recipe (#49).

## The Owner's Choices

All four decided by the owner on 2026-10-06, each as recommended.

1. **Where the fallback runs.** Hybrid's lexical branch asks for
   `max(limit, 20)` rows, so with the default limit of 10, a fallback inside
   `search_project` would run whenever the all-words query finds fewer than
   20 pages: six of the eight fixed queries on the frozen wiki. For "three
   phase ingest extraction drafting bookkeeping" (5 all-words pages, no
   hyphens) it becomes a plain word OR matching 94 of 121 pages, and 15
   partial matches would enter hybrid's fusion with lexical ranks they never
   had. Issue #25 leaves hybrid out of scope.
   - Lexical only: the fallback runs for a lexical search (chosen, or
     selected by auto), never in hybrid's lexical branch.
   - Everywhere: it also runs in hybrid's lexical branch, and the Done When
     adds the hybrid before-and-after above.
   Decided by the owner on 2026-10-06: lexical only. It keeps the default search as it is
   beyond the weights, keeps the change inside the issue, and the place that
   runs the fallback can then also say so in the reply (choice 4).
2. **The zero word-weight clamp.** BM25 gives a word in more than half the
   pages a weight of almost zero, so such a search prints scores of 0.000
   even when the order is right. The weights fix the order, not the scores.
   Making scores meaningful (normalising them, or a small added boost for a
   title or file-name match) changes what every lexical score means to its
   readers, hybrid fusion and its strong-lexical floor included.
   Decided by the owner on 2026-10-06: leave it out of this plan, whose promise is the order,
   and open its own roadmap entry only if a reader is found misled by the
   0.000 scores.
3. **The bar for the four plan names.** The investigation measured three of
   the four plans in the top 8 on the live wiki; the review measured all
   four in the top 10 on the frozen wiki, the fourth at 10th, right at the
   cutoff. Decided by the owner on 2026-10-06: the test asserts at least three of the four in
   the top 10, and this plan records where the fourth lands; asking for all
   four would pass today only at the edge. The owner also asked whether a
   filter on the file type does it: it does, and search has it already
   (`--class plan`, or `class` in the MCP tool). So a second check runs the
   same four names with the plan filter on and asserts all four plans in the
   top four; the unfiltered bar stays, because it is what an agent runs when
   it does not know which kind of page it wants. Changed by the owner on
   2026-10-07: all four plans in the top five with the plan filter, behind
   the setup plan, which names all four, holds all twelve words and every
   phrase, and so rightly comes first ("What Was Found", phase 3).
4. **Saying when the fallback ran.** Pages added by the fallback matched
   only some of the query, and their scores come from a different query, so
   the score column may rise after them. The reply's warnings are built in
   `src/search/commands.rs`, but `search_project` returns only its results,
   so the warning needs a signal from wherever the fallback runs: either the
   fallback is run by the search command's lexical path, which then knows
   itself, or the backend tells its caller, by a marker on each added result
   or a changed return type (four callers outside tests).
   Decided by the owner on 2026-10-06: one line in the reply's existing warnings when the
   fallback added pages, with the signal following choice 1: with "lexical
   only", from the lexical path that runs the fallback; the PR names which
   way it took.

## Out Of Scope

- Semantic and hybrid search, beyond recording a change their lexical branch
  sees.
- Compact search paging (P17, issue #29) and the stale-index warning
  (issue #36): their own plans.
- The score scale and its printing (the owner's choice, above).
- Refreshing `tests/fixtures/search-eval/wiki/`.
