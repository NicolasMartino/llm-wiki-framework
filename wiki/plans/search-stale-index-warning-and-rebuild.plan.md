# Plan: Search Says When Its Index Is Stale, And Rebuilds A Small One

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-10-07
- Category: Search, MCP
- Scope: Carry out P21 of the framework roadmap: a stale index is reported
  where an agent reads it, in the CLI and in the JSON and MCP replies'
  warnings list, with the exact command that rebuilds it; and a word-match
  index rebuilds itself before answering when its lock is free and its cache
  writable, falling back to the warning otherwise.
- Sources:
  - Issue #36, "Search: Say when the index is stale, and rebuild a small one"
  - The investigation on issue #34, comment of 2026-10-06
    (https://github.com/NicolasMartino/llm-wiki-framework/issues/34#issuecomment-6021433019)
  - `src/search/commands.rs`, `src/mcp/mod.rs` and `tools/wiki-worktree.sh`
    at `cad8988`, read for this plan; one stale search run on this plan's
    worktree with llm-wiki 0.2.15
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P21 (this plan) and P20 (the
    investigation)
  - `wiki/plans/compact-search-pages-follow-the-limit.plan.md` (P17, the
    compact reply)
  - `wiki/decisions/search-backend-selection.decision.md` (read-only,
    immutable store reads)

## What This Proves

A worker that edits pages and then searches either gets answers from the
pages as they are, or reads, in the place it already reads, that the index is
stale and the one command that fixes it. A held lock or a read-only cache
never turns that search into a failure.

## Where It Stands (2026-10-07, at `cad8988`)

- Search computes the wiki's snapshot (content hash and mtime per page) on
  every run and compares it with the one stored beside the index; any
  difference, a bare `touch` included, is `BackendState::Stale` (#34).
- A stale index is searched anyway: the readiness checks in
  `src/search/commands.rs` accept `BackendState::Ready | BackendState::Stale`.
- `stale_warning` already names the command:
  ``search index stale for project <id>; run `llm-wiki index --project <id>` ``.
- Where it shows, for `search`: one `Warning:` line in the CLI's text; in
  JSON, the top-level `warning` string. The `warnings` list stays empty (the
  search command passes an empty list to the JSON printer), measured on
  this plan's worktree after a `touch` of one page (`--format json`, lexical:
  `"warning"` set, `"warnings": []`). The compact reply drops an empty list
  and carries `warning` alone. The MCP search tools always ask the CLI for
  JSON (`search_cli_args`, `src/mcp/mod.rs`), so they show the same.
- `search-all` already collects each project's stale warning into its
  `warnings` list, and reaches each project through the same lexical path
  as `search`.
- `stale_warning` writes `llm-wiki` into the command whatever binary runs it;
  the test build is `llm-wiki-test` (`src/instance.rs`), and other places name
  the running binary with `instance::binary_stem()` (`src/registry/mod.rs`,
  `src/mcp_wiring.rs`).
- Index builds take a lock with `try_lock` on `qmd-rs.lock` in the project's
  index folder and never wait; a second concurrent `llm-wiki index` fails with
  "lock acquisition failed because the operation would block". With the
  index folder read-only, search still answers (stale, with the warning) and
  `llm-wiki index` fails with "Permission denied" (#34).
- A worktree's word-match index (124 pages) rebuilt in about 0.65 s, full
  each time, and takes 4.9 MB (#34). `tools/wiki-worktree.sh` writes each
  worktree a `.llm_wiki/search.toml` with `llm_search_enabled = false`,
  registers it and builds its index once; its comment still says about
  20 MB.

## Target

- **The warning where it is read**: when the index is stale, the JSON reply's
  `warnings` list carries it, in the full and the compact reply, beside the
  `warning` string, which stays for current readers. The text names the
  project's exact `<binary> index --project <id>` command, `<binary>` being the
  running binary (`llm-wiki`, or `llm-wiki-test` for the test build), and says
  it takes about a second for a word-match index. The MCP reply shows the same,
  since it is the CLI's JSON.
- **A small index rebuilds itself**: when the index is stale, the project's
  search is word-match only, the lock is free and the cache writable, search
  rebuilds the index first, then answers from it, fresh, with no stale
  warning. `search` only: `search-all` keeps warning and never rebuilds,
  since one call from one worktree would otherwise rewrite every other
  worktree's index (about 0.65 s each, in other workers' caches).
- **Fallback, never failure**: when the lock is held or the cache is not
  writable, search answers from the stale index as today, with the warning
  above and the reason in a few words ("another index build is running",
  "the search cache is read-only").
- **Opt-out**: a project can turn the rebuild off with
  `rebuild_stale_index = false` in the `[project]` table of its
  `.llm_wiki/search.toml`, beside `llm_search_enabled`; unset means on
  (the owner's choice 1).
- **The setup script's comment** says about 5 MB.

## Done When

- A test edits a page of a word-match project and searches: the reply is
  fresh and finds the new text.
- A test holds the index lock, edits a page and searches: the reply is
  stale, its `warnings` list names the exact command and the reason, and the
  exit code is success. A second test does the same with a read-only cache.
- A test shows a project with LLM search on is never rebuilt by search, only
  warned, and one shows `search-all` over a stale word-match project warns
  and does not rebuild it.
- A test reads the MCP search reply on a stale index and finds the warning in
  its `warnings` list.
- Each of these fails on `cad8988` (shown once, not committed).
- The fast check passes on the PR into `develop`.

## Open For The Owner

The coordinator took both recommendations below on 2026-10-07, while the
owner was away; the owner's verdict on the PR confirms or changes them.

1. **The opt-out: a setting in the project's `.llm_wiki/search.toml`**, beside
   `llm_search_enabled`, on by default for word-match projects. Not chosen: an
   environment variable, which a host or worker cannot see in the project's own
   record; no opt-out, which leaves search writing in a place someone may want
   read-only.
2. **Many workers editing at once.** Each stale search would race for the
   lock; a loser falls back to the warning, so nothing fails, but some
   answers come from the old index. Recommended: accept it, and add one test
   that starts three searches together on a stale index and checks every one
   answers. Not chosen: waiting for the lock, which would add a whole
   rebuild's time, or more, to a search that can answer at once with the
   warning.

## Out Of Scope

- Rebuilding the main checkout's meaning-based index, which takes minutes.
- Ranking (P16, issue #25) and compact paging (P17, issue #29).
- Making the index build incremental.
