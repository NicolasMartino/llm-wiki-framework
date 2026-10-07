# Plan: Compact Search Pages Follow The Limit

- Document Class: Plan
- Status: Active
- Branch: `NicolasMartino/compact-29`
- Date: 2026-10-07
- Category: Search, MCP
- Scope: Carry out P17 of the framework roadmap: a compact search returns as
  many results as its `limit` asks for unless `page_size` says otherwise, the
  reply says plainly when more results exist, and the MCP tools' schemas, a
  spec and a test say so.
- Sources:
  - Issue #29, "Search: Make compact search pages follow the limit"
  - The investigation on issue #24, comment of 2026-10-06
    (https://github.com/NicolasMartino/llm-wiki-framework/issues/24#issuecomment-6020487843)
  - `src/search/commands.rs`, `src/mcp/mod.rs` and `wiki/specs/` at
    `cad8988`, read for this plan
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P17 (this plan) and P15 (the
    investigation)
  - `wiki/plans/search-ranking-weights-and-phrase-fallback.plan.md` (P16,
    ranking)
  - `wiki/plans/search-stale-index-warning-and-rebuild.plan.md` (P21, the
    reply's warnings)
  - `wiki/specs/wiki-query-skill.spec.md`

## What This Proves

An agent that asks `llm_wiki_search` for 12 compact results gets 12 when 12
match, and when there are more than it got, the reply tells it so in a field
it cannot misread, not only through a `next_offset` nobody documented.

## Where It Stands (2026-10-07, at `cad8988`)

- `limit` (default 10) caps how many hits the search keeps and sets
  `result_count`: the backend is asked for `limit` hits (`limit: args.limit`
  in the search command), so nothing in the reply knows whether more pages
  matched; `page_size` cuts how many of those the compact reply
  carries; `offset` is where the page starts.
- Unset, `page_size` is the constant `DEFAULT_COMPACT_SEARCH_PAGE_SIZE`, 3
  (`src/search/commands.rs`, used by `effective_page_size`). It never comes
  from `limit`.
- `print_compact_search_json` slices the page and sets `next_offset` only when
  the page ends before `result_count`; the compact envelope
  (`CompactSearchEnvelopeJson`) has no other sign of more results.
- `search` and `search-all` build their options the same way
  (`SearchJsonOptions::from_search_args` and `from_search_all_args`), so both
  default to 3.
- The MCP tools `llm_wiki_search` and `llm_wiki_search_all` pass `limit`,
  `page_size` and `offset` straight to the CLI (`search_cli_args`,
  `src/mcp/mod.rs`); their schemas give `page_size` no default and no
  description, and neither tool's description mentions paging.
- No spec under `wiki/specs/` mentions `page_size` or `next_offset`.
- Measured on #24: `--compact --limit 12` gave `result_count` 12, a page of 3
  and `next_offset` 3; `--page-size 12` gave all 12 and no `next_offset`.

## Target

- **The page size follows the limit**: with `page_size` unset, a compact page
  carries up to `limit` results; `page_size` stays the way to ask for smaller
  pages, and `offset` works as today. Search and search-all alike.
- **The reply says when more exist**: one plain field that is true when more
  pages match than this reply carries, past `limit` included, and false
  otherwise, beside `next_offset`, which stays. The search asks the backend
  for `limit + 1` hits and drops the extra one, so a default call (`page_size`
  unset, the page reaching `limit`) still says whether more matched.
- **Written down**: the MCP schemas of both search tools describe `limit`,
  `page_size` (its default), `offset`, `next_offset` and the new field; a spec
  states the compact paging contract (where: the owner's choice 1).
- **Tested**, for search and search-all:
  - `limit` 12 on a query more than 12 pages match: 12 results, the field
    true;
  - `limit` 12 on a query exactly 12 pages match: 12 results, the field
    false;
  - `limit` 12 and `page_size` 3: 3 results, the field true, `next_offset` 3.
  Each test fails on the code at `cad8988` (shown once, not committed).

## Done When

- The tests of the Target pass, and each failed on `cad8988`.
- The MCP `tools/list` reply shows the described paging fields for both
  search tools, checked by a test.
- The spec chosen in choice 1 states the contract.
- The fast check passes on the PR into `develop`.
- Pages that still say the compact page is 3, found by searching the wiki at
  that time, say what is true.

## Open For The Owner

1. **Where the contract is written: a section in
   `wiki/specs/wiki-query-skill.spec.md`**, whose "Proven By" already relies
   on what the search tools return. Not chosen: a new spec for the MCP search
   tools, cleaner but one more page for a few lines; or
   `wiki/specs/documentation-model.spec.md`, which is about the framework as a
   whole.
2. **The field's name: `has_more`.** It is the name the investigation used and
   the one most callers look for. Not chosen: `truncated`, which reads as if
   results were lost.

## Out Of Scope

- Ranking (P16, issue #25).
- The stale-index warning (P21, issue #36).
- Non-compact replies, which carry every hit up to `limit` already.
