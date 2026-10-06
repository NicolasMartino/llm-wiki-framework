# MCP Field Test

- Document Class: Checklist
- Status: Active
- Date: 2026-06-29
- Updated: 2026-08-01
- Category: MCP surface, field test, release gate
- Scope: Paste-ready agent protocol for exercising the installed LLM Wiki MCP server end-to-end inside a target project, capturing raw output, and reporting pass/fail against the known correctness issues. This is the single field-test checklist and the correctness source of truth for the MCP surface.
- Sources: wiki/plans/mcp-first-agent-guidance.plan.md, src/search/metadata.rs, src/search/commands.rs, src/instance.rs
- Related: wiki/evals/mcp-first-host-parity.eval.md

## What This Checklist Is For

This is the **correctness gate** for the LLM Wiki MCP surface. It answers one question:
*do the `llm_wiki_*` tools behave correctly end-to-end inside a target project?*

It exercises setup (status / register / index), read fidelity, every search mode, the
metadata and rerank flags, and cross-project search — capturing raw output at each step
so a reader can re-derive every verdict without having seen the session.

## How To Use

Run this from inside the **target project** with the LLM Wiki MCP server configured for
the host. Registering the project wires the host MCP config automatically: `llm-wiki
register <path>` (or `llm-wiki init`) writes/merges the project-local `.mcp.json` and
ensures the global Codex config, so **no manual `.mcp.json` copy is needed** — just
launch Claude Code from the project root. (The staged template under
`~/.llm_wiki*/mcp/claude-project.mcp.json` is only a fallback for manual layouts; run
`llm-wiki doctor` to confirm a project is wired.)

It is written for an ordinary agent: paste the "Agent Prompt" section verbatim
into a fresh session and transcribe results into the "Report" template.

Two rules make the run trustworthy:

1. **Capture raw tool output verbatim.** Never paraphrase. `class: null`/`status: null`,
   readiness reasons, and `rerank_applied` flags must stay visible exactly as returned.
2. **Classify every result** as `OK`, `expected-in-a-fresh-project`, or `bug`. A fresh
   indexed project should use shipped semantic/hybrid defaults; recorded calibration is
   optional quality tuning, not a readiness gate.

### Instance-aware tool names

Use the names for the instance you installed. Production has no suffix; the test
instance adds `_test`.

| Operation | Production | Test instance |
| --- | --- | --- |
| status | `llm_wiki_status` | `llm_wiki_status_test` |
| register | `llm_wiki_register` | `llm_wiki_register_test` |
| index | `llm_wiki_index` | `llm_wiki_index_test` |
| read | `llm_wiki_read` | `llm_wiki_read_test` |
| search | `llm_wiki_search` | `llm_wiki_search_test` |
| search-all | `llm_wiki_search_all` | `llm_wiki_search_all_test` |

Below, the unsuffixed production names are used; substitute the `_test` names if you
installed the test instance.

## Agent Prompt

> You are field-testing the LLM Wiki MCP server in this project. Do not use shell,
> filesystem, or other tools for wiki/raw content — use the MCP tools. For every step:
> state the tool and arguments, paste the **raw** tool output verbatim, then write one
> line of verdict (`OK` / `expected-fresh` / `BUG: <detail>`). Work top to bottom and
> stop only if a step makes the next impossible (e.g. registration fails).
>
> As you go, consolidate everything into a single results file at the **root of your
> workspace**: `./mcp-field-test-results.md`. Write it in **extreme detail** — for every
> step record the exact tool name and full arguments, the complete raw tool output
> verbatim (never paraphrase or truncate; keep `class: null`/`status: null`, readiness
> reasons, and `rerank_applied` flags exactly as returned), your verdict, and the
> expected-vs-actual reasoning behind it. Also capture the environment up front (instance
> name, host, and the posture being tested). Finish by filling the Report template into
> the same file and listing every `BUG`/follow-up row as its own issue entry. The file
> must be complete enough that someone who never saw the session can reproduce the run and
> confirm each verdict from it alone.

### Phase A — Setup (run in order, before any search-mode testing)

1. **status** — call `llm_wiki_status`. Record install state, whether this project is
   registered, index freshness, and search readiness. Expected on a fresh project: not
   registered and/or no index.
2. **register** — call `llm_wiki_register` for this project's root. Re-run
   `llm_wiki_status`; confirm it now reports registered.
3. **index** — call `llm_wiki_index`. Confirm it reports the number of indexed files.
   Then call `llm_wiki_index` again with `force: true` and confirm a full rebuild.
4. **calibrate thresholds (optional tuning)** — semantic and hybrid should work after
   indexing through shipped default thresholds. Calibration is now an optional quality
   override, not a readiness gate. If you want to test recorded thresholds, run the CLI
   step the readiness/tuning message names: `llm-wiki eval calibrate --record` (use
   `llm-wiki-test` for the test instance), then repeat the semantic/hybrid searches and
   confirm `thresholds_source: recorded`.

### Phase B — Read (`llm_wiki_read`)

5. **wiki read** — read `wiki/index.md`. Confirm the output is **real Markdown**. Record
   the `sha256` and `byte_len`.
6. **raw read** — read a file under `raw/` (any source/manifest). Confirm real content.
   Record its `sha256` and `byte_len`.
7. **absolute vs relative path** — read the same wiki page once with a project-relative
   path and once with an absolute path. Both should succeed and match.
8. **traversal rejection** — attempt to read a path that escapes the project (e.g.
   `../../etc/hosts` *and* an absolute path outside the project). Expected: a clear
   rejection in both cases, not file contents. A relative escape that returns a generic
   `not found` instead of an outside-tree rejection is `BUG` (Issue 8).
9. **missing-file error** — read a non-existent wiki page. Expected: a clear not-found
   error, not an empty success.

### Phase C — Search (`llm_wiki_search`)

For each search call, paste the full JSON and inspect: `selected_mode`,
`mode_selection_reason`, `fallback_reason`, `readiness_reason`, `rerank_applied` /
`rerank_reason`, the presence of a `results` array, and each hit's `class` / `status`.

10. **lexical** — `mode: lexical` with a keyword that exists in the wiki. Confirm a
    populated `results` array, and that hits carry non-null `class`/`status` where the
    pages have those headers. A missing/empty `results` array on a ready index with a
    matching term is `BUG` (Issue 10). Null `class`/`status` on a page that has
    `Document Class:`/`Status:` lines is `BUG` (Issue 0.2).
11. **auto** — `mode: auto` with a natural-language query. After indexing, this should
    select hybrid/semantic-capable search with `thresholds_source: default` unless
    recorded calibration overrides it, and return a populated `results` array. A hard
    `thresholds_unconfigured` error is `BUG` (Issue 0.3); ready metadata with no
    `results` array is `BUG` (Issue 11).
12. **semantic** — `mode: semantic`. After indexing, expect semantic hits and
    `thresholds_source: default` unless you recorded calibration in step 4. A
    `thresholds_unconfigured` hard failure after indexing is `BUG`; ready metadata with
    no hits is `BUG` (Issue 12).
13. **hybrid** — `mode: hybrid`. Same expectation as semantic: a populated `results`
    array with `thresholds_source: default` unless recorded calibration overrides it.
    Ready metadata with no hits is `BUG` (Issue 13).
14. **class + status filters** — repeat a query that has matching pages with a class
    filter and a status filter that match real pages in this wiki (inspect the wiki tree
    first; do not assume `Decision`/`Accepted` exists). Confirm the filter selects
    matching pages rather than excluding everything. Empty results when the wiki has
    matching pages is `BUG` (Issue 0.2).
15. **rerank true** — repeat with `rerank: true`. Inspect `rerank_applied` and
    `rerank_reason`. If no reranker model is configured, expect `rerank_applied: false`
    with a reason (`reranker_model_unconfigured` or `rerank_requires_hybrid_mode`), never
    a silent no-op. A null `reranker_model` with no flag is `BUG` (Issue 0.4).
16. **rerank false** — repeat with `rerank: false`; confirm `rerank_applied: false` with
    no reason.
17. **allow_lexical_fallback** — call `mode: semantic` with `allow_lexical_fallback: true`
    only in a deliberately not-ready posture (for example before indexing or with search
    artifacts unavailable). Confirm it returns lexical results with the readiness reason
    recorded. After a normal index, semantic should use `thresholds_source: default`
    instead of falling back for lack of calibration.

### Phase D — Cross-project (`llm_wiki_search_all`)

18. **search-all** — run a query across all registered projects. Confirm
    readiness/backend state is reported per project in `projects[]`. Top-level
    `results[]` are a globally ranked flat list, not grouped rows; each hit must
    carry `project_id` / `project_name` so the project attribution is explicit.
    A flat `results[]` array is `OK` when per-project readiness is intact.
19. **include / exclude** — repeat with an include filter and an exclude filter on
    project ids; use the exact keys `include` and `exclude`. Confirm the set of
    searched projects changes accordingly. If a misspelled or obsolete key (for
    example `include_projects` / `exclude_projects`) is accepted but ignored,
    record that as a follow-up because it can mask caller typos; the correctness
    gate still depends on the documented keys working.

## Consolidated Results File

Write all of the above into one file at the workspace root:
`./mcp-field-test-results.md`. This is the single deliverable of the run; the session
transcript is not. Required structure, in order:

1. **Run header** — date, instance (production / `_test`), host + version, and the
   posture from the Posture checklist below.
2. **Per-step log** — one section per checklist step (1–19), each containing: tool name,
   full arguments, the **complete raw output verbatim**, and the verdict with a sentence
   of expected-vs-actual reasoning.
3. **Report table** — the template below, one row per step.
4. **Issues** — every `BUG`/follow-up row written out as a standalone issue (symptom,
   exact reproducing step, raw signal) ready to file.

Keep raw output uncut: do not summarize, pretty-trim, or drop fields. The file must let a
reader who never saw the session reproduce the run and re-derive every verdict from it
alone.

## Report

Fill this in from the raw transcripts. Keep one row per step.

| Step | Tool | Verdict | Notes (raw signal) |
| --- | --- | --- | --- |
| 1 status | | | |
| 2 register | | | |
| 3 index / --force | | | |
| 4 calibrate | | | |
| 5 wiki read | | | |
| 6 raw read | | | |
| 7 abs/rel path | | | |
| 8 traversal | | | |
| 9 missing file | | | |
| 10 lexical | | | |
| 11 auto | | | |
| 12 semantic | | | |
| 13 hybrid | | | |
| 14 class/status filters | | | |
| 15 rerank true | | | |
| 16 rerank false | | | |
| 17 allow_lexical_fallback | | | |
| 18 search-all | | | |
| 19 include/exclude | | | |

### Pass Criteria

- **Issue 0.1 (read fidelity):** steps 5–6 return real Markdown/source content with a
  matching `sha256`.
- **Issue 0.2 (class/status):** steps 10 and 14 show non-null `class`/`status` for pages
  that carry those headers, and filters select them.
- **Issue 0.3 (auto/thresholds):** step 11 returns a populated `results` array instead of
  erroring, and steps 12–13 work after indexing with `thresholds_source: default` unless
  recorded calibration overrides it.
- **Issue 0.4 (rerank):** step 15 reports `rerank_applied` plus a model name or a reason —
  never a silent no-op.
- **Cross-project shape:** step 18 reports per-project readiness in `projects[]`;
  top-level `results[]` may be a flat globally ranked list as long as each hit
  carries project attribution. Grouped hit arrays are not required unless a
  future schema intentionally adds them.
- **Cross-project filters:** step 19 uses the documented `include` / `exclude`
  keys and shows the searched project set changes. Unknown/misspelled filter
  keys should be rejected or reported; silent ignore is a follow-up, not a pass
  signal for filtering.

### Posture

Record which posture this run covered:

- [ ] New project (fresh `init`)
- [ ] Existing project not yet migrated (register + index an existing wiki)
- [ ] Migration candidate (an external project moving onto the MCP)

File any `BUG` rows as new issues before relying on this surface.
