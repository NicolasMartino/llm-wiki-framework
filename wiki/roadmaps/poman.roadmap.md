# poman Roadmap

- Document Class: Roadmap
- Status: Draft
- Date: 2026-10-06
- Category: poman development
- Scope: The order in which poman, the project-management binary moved here
  from riseon, is built: the workspace and the gates, the shared types, the
  deadline type, seeing the deadlines, the tracker sync, the forecast and what
  follows it, and the ratchet that brings llm-wiki's own code to the same
  gates.
- Sources:
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`,
    section 8 ("A Suggested First Roadmap For poman Here")
  - The owner's answers of 2026-10-06, as recorded in the decision pages
    below; XXVIII (`poman tree` and `poman list` as their own deliverable,
    after the deadline commands) is recorded here, in PM5
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P5 (the handover's ingest, which
    wrote this roadmap)
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`
  - `wiki/decisions/poman-tree-and-list-show-the-files.decision.md`
  - `wiki/decisions/poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`

## Objective

Build poman so that a repository's deadlines are files that poman writes,
checks, forecasts and shows on a tracker, with GitHub as the first tracker, and
so that every crate of this project meets the strictest gates.

## Sequencing Principles

1. The gates come first: poman and the shared crate are strictest from their
   first commit.
2. The types come before the commands that read them.
3. Each code deliverable gets its plan, approved by the owner, before its
   worker starts (`work-is-recorded-in-the-repository.decision.md`).
4. Open points are settled inside the entry that needs them, with the owner,
   before that entry's plan is written.

The handover notes what waits outside this repository: a repository's founding
tasks as deadline files wait on PM3, its board on PM6, its recurring
obligations on PM7.

---

### PM1 - The Workspace And The Strict Gates

Status: Draft
Promise: The shared crate and the poman crate exist in this workspace, both
held to the strictest gates from their first commit; `llm-wiki install`
installs poman; one release ships both binaries.
Depends On: None
Execution Plan: Not created yet

Included:
- the shared library crate and the `poman` crate, with a binary that runs
- the gates of `poman-lives-in-this-workspace.decision.md`, "The strictest
  gates", for the two new crates, and one gate script running them
- `llm-wiki install` putting poman in `~/.llm_wiki/bin/` and its manifest;
  `uninstall` removing it
- one release shipping both binaries
- the shared crate's name

Excluded:
- moving the parser (PM2)
- llm-wiki's own modules under the strict gates (PM8)

Proof:
- the gate script reports every gate run and passed for the two new crates,
  nothing skipped
- an install into a redirected home puts poman in the managed bin folder and
  the manifest, and uninstall removes it
- a release build produces both binaries

---

### PM2 - The Shared Page Reader And File Types

Status: Draft
Promise: The field-block parser lives in the shared crate, and llm-wiki's
document types are defined there with their fields and statuses, so llm-wiki
and poman read pages one way.
Depends On: PM1
Execution Plan: Not created yet

Included:
- `parse_wiki_metadata` (today `src/search/metadata.rs`) moved into the shared
  crate, and llm-wiki's search reading through it
- llm-wiki's document types defined there: suffix, folder, fields, statuses
- `llm-wiki init` scaffolding from those definitions
- how the shared reader serves the wiki's forms (front matter, bold keys) and
  poman's bullet-only block

Excluded:
- poman checking the wiki's types (a later choice, XLII)

Proof:
- llm-wiki's search and init tests pass unchanged through the shared crate
- the type definitions agree with `templates/base/project_guidelines.md`

---

### PM3 - The Deadline Type

Status: Draft
Promise: `poman new deadline` writes a deadline file and `poman check` holds
every deadline file to its type, with each message naming the file and line.
Depends On: PM2
Execution Plan: Not created yet

Included:
- the deadline type of `deadline-files-hold-one-deadline-each.decision.md`,
  its value formats included
- `poman new` and `poman check` as
  `poman-reads-only-its-own-file-types.decision.md` sets them, near-miss
  warnings included
- the reference checks: each path exists, is a deadline file, is not the page
  itself, and no loop forms
- whether a deadline file also carries the wiki's metadata fields, and whether
  it is listed in `wiki/index.md` or exempt from the orphan check (open)
- riseon's issue for the deadline commands, which moves here

Excluded:
- the start date's importance buffer (PM7)
- showing the files (PM5)

Proof:
- `poman new` writes a file that `poman check` accepts
- `poman check` fails on each broken field and reference, warns on near
  misses, and ignores every other Markdown file

---

### PM4 - Where poman check Runs

Status: Draft
Promise: A deadline file cannot reach master without `poman check` having
passed on it, although such files land with no pull request.
Depends On: PM3
Execution Plan: Not created yet

Included:
- the choice, open today: a pre-push hook, `poman push` refusing while the
  check fails, or both (the handover, section 2)
- the change that puts it in place

Excluded:
- checks on any change that goes through a pull request, which CI already
  runs

Proof:
- a malformed deadline file is refused before it lands, by the chosen means

---

### PM5 - Seeing The Deadlines

Status: Draft
Promise: `poman tree` and `poman list` show the files as
`poman-tree-and-list-show-the-files.decision.md` sets them.
Depends On: PM3 (the owner, 2026-10-06, the handover's XXVIII: their own
deliverable, after the deadline commands)
Execution Plan: Not created yet

Included:
- `poman tree` with `--invert` and the `(*)` repeat mark
- `poman list`
- the shared filters, `--type` with its near-miss error and hidden-link stubs,
  and `--json`
- the sort `poman list` uses until the forecast exists, and what `--ready`
  selects

Excluded:
- the sort by start date in full, which needs the buffer (PM7)

Proof:
- both commands on a fixture of deadline files with shared blockers, a loop
  refused by `check`, and each filter

---

### PM6 - The Tracker Sync, GitHub First

Status: Draft
Promise: `poman fetch`, `poman diff` and `poman push` keep a tracker's board
a view of the repository's files, through an adapter, GitHub's first, as
`poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md` sets them.
Depends On: PM3
Execution Plan: Not created yet

Included:
- poman's tracker-free model: items, links, the board, the mirror and the
  changeset, and their formats
- the adapter boundary, and the GitHub adapter behind it
- the GitHub adapter's API choice, open today: the handover proposes octocrab
  for the REST API and graphql_client for the board's GraphQL API, a token
  from `GITHUB_TOKEN` else `gh`'s, and tests that never touch the real GitHub
- the open cases: a marker edited away or a file renamed; a "Blocked by" link
  made by hand
- which board fields and views poman draws, and the dot folder's name

Excluded:
- drawing this repository's own board from its roadmaps and plans: a later
  deliverable after this one (the owner, 2026-10-06, the handover's XXXIII,
  recorded in the sync decision); its roadmap entry comes when this one lands
- pull requests and comments

Proof:
- fetch, diff and push against a fake tracker: drift reported, a changeset
  written and applied exactly, a push refused after a change on the tracker
- the mirror rebuilt from the tracker alone
- one run against a real GitHub project made for the test

---

### PM7 - The Forecast, Recurring Deadlines, Absences, The Morning Run

Status: Draft
Promise: poman tells each morning what must start, from the files alone, and
keeps recurring obligations and absences in the repository.
Depends On: PM3; PM6 for showing the forecast on the tracker
Execution Plan: Not created yet (one plan per part, in the order below)

Included:
- the forecast: the 15-day window, the importance buffer's exact rule (open
  today), and the "still not done" nudge for important items with no deadline
- recurring deadlines: a later type whose occurrences are deadline files named
  by period, one file per occurrence; conditions on a recurrence (only above
  a threshold, only if some event happens)
- absences, declared in the repository, and public holidays with them
  (`deadline-files-hold-one-deadline-each.decision.md`, "The start date")
- the morning run: a local scheduled job

Excluded:
- a daily snapshot file: any day's forecast is recomputed from Git history

Proof:
- a past day's forecast recomputed from the files at that day's commit
- one file per occurrence, with no duplicate after two runs

---

### PM8 - The Ratchet

Status: Draft
Promise: llm-wiki's existing modules reach the strictest gates one at a time,
and none slips back once switched.
Depends On: PM1
Execution Plan: Not created yet

Included:
- the ratchet as `poman-lives-in-this-workspace.decision.md`, "The strictest
  gates", sets it, as a track of its own

Excluded:
- holding any of PM2 to PM7 until the ratchet ends

Proof:
- each module switched records its gate run; the gate script fails on a
  deliberate slip in a strict module
