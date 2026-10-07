# poman Roadmap

- Document Class: Roadmap
- Status: Active
- Date: 2026-10-06
- Category: poman development
- Scope: The order in which poman, the project-management binary, is built: the workspace and the gates, the shared types, the
  deadline type, seeing the deadlines, the tracker sync, the forecast and what
  follows it, and the ratchet that brings llm-wiki's own code to the same
  gates.
- Sources:
  - The owner's decisions, 2026-10-06: the suggested order of poman's first
    deliverables, and the answers recorded in the decision pages below
  - The owner's answer, 2026-10-06, recorded here in PM5: `poman tree` and
    `poman list` are their own deliverable, after the deadline commands
  - The owner's decision, 2026-10-07, on the poman track (#19), recorded
    here in PM3's Proof: PM3 is proved on the riseon repository as well as
    on fixtures
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P5, which wrote this roadmap
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

Repositories that use poman wait on it too: their deadline files on PM3, their
boards on PM6, their recurring deadlines on PM7.

---

### PM1 - The Workspace And The Strict Gates

Status: Completed (develop)
Promise: The shared crate and the poman crate exist in this workspace, both
held to the strictest gates from their first commit; `llm-wiki install`
installs poman; one release ships both binaries.
Depends On: None
Execution Plan: `wiki/plans/poman-workspace-and-strict-gates.plan.md`

Included:
- the shared library crate and the `poman` crate, with a binary that runs
- the gates of `poman-lives-in-this-workspace.decision.md`, "The strictest
  gates", for the two new crates, and one gate script running them
- `llm-wiki install` putting poman in `~/.llm_wiki/bin/` and its manifest;
  `uninstall` removing it
- one release shipping both binaries
- the shared crate's name
- `llm-wiki install` getting poman as
  `poman-lives-in-this-workspace.decision.md`, "Where `llm-wiki install` gets
  poman", sets it (the owner's decisions, 2026-10-06), and what it says when
  no poman sits beside it

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

Status: Completed (develop)
Promise: The field-block parser lives in the shared crate, and llm-wiki's
document types are defined there with their fields and statuses, so llm-wiki
and poman read pages one way.
Depends On: PM1
Execution Plan: `wiki/plans/poman-shared-page-reader-and-types.plan.md`

Included:
- `parse_wiki_metadata` (`src/search/metadata.rs` until PM2) moved into the shared
  crate and rewritten to its strictest gates (no indexing or slicing, doc
  comments, unit tests in their own file, full coverage, no surviving
  mutants), ahead of PM8's ratchet for the rest of llm-wiki
- both callers reading through it: `src/search/qmd_rs.rs` and
  `src/search/semantic.rs`
- llm-wiki's document types defined there: suffix, folder, fields, statuses
- `llm-wiki init` scaffolding from those definitions
- how the shared reader serves the wiki's forms (front matter, bold keys) and
  poman's bullet-only block

Excluded:
- poman checking the wiki's types (a later choice)

Proof:
- llm-wiki's search and init tests pass unchanged through the shared crate
- the gate script reports every gate run and passed for the parser in the
  shared crate
- the type definitions agree with `templates/base/project_guidelines.md`

---

### PM3 - The Deadline Type

Status: Active
Promise: `poman new deadline` writes a deadline file and `poman check` holds
every deadline file to its type, with each message naming the file and line.
Depends On: PM2
Execution Plan: `wiki/plans/poman-deadline-type.plan.md`

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
- the landing branch, master by default and settable, as
  `deadline-files-hold-one-deadline-each.decision.md`, "How the files land",
  sets it (the owner's decisions, 2026-10-06); where and how a repository
  sets it is open, and PM3's plan decides it

Excluded:
- the importance buffer in `should-start` (PM7)
- showing the files (PM5)

Proof:
- `poman new` writes a file that `poman check` accepts
- `poman check` fails on each broken field and reference, warns on near
  misses, and ignores every other Markdown file
- on the riseon repository (the owner, 2026-10-07): its founding tasks are
  written with a locally built poman's `poman new deadline`, `poman check`
  accepts them, and they land on riseon's master, which closes riseon issue
  #11

---

### PM4 - Where poman check Runs

Status: Draft
Promise: A deadline file that `poman check` refuses is caught in the three
places the deadline decision, "How the files land", names (the owner's
decisions, 2026-10-06), although such files land with no pull request: a
pre-push hook stops it for those who turn it on, `poman push` keeps it off the
tracker, and CI flags it once it is on the landing branch.
Depends On: PM3; PM6 for the `poman push` refusal
Execution Plan: Not created yet

Included:
- the opt-in pre-push hook, which runs only where it is turned on
  (`core.hooksPath`) and is skipped by `git push --no-verify`
- `poman push` refusing while the check fails, which keeps the file off the
  tracker but not off the branch
- `poman check` in CI on every push to the landing branch, which flags the
  file after it lands

Excluded:
- checks on any change that goes through a pull request, which CI already
  runs

Proof:
- a malformed deadline file is caught in each place as it promises: refused
  by the hook, refused by `poman push`, and a failed CI run on the landing
  branch

---

### PM5 - Seeing The Deadlines

Status: Draft
Promise: `poman tree` and `poman list` show the files as
`poman-tree-and-list-show-the-files.decision.md` sets them.
Depends On: PM3 (the owner, 2026-10-06: their own deliverable, after the
deadline commands)
Execution Plan: Not created yet

Included:
- `poman tree` with `--invert` and the `(*)` repeat mark
- `poman list`
- the shared filters, `--type` with its near-miss error and hidden-link stubs,
  and `--json`
- the sort `poman list` uses until the forecast exists, and what `--ready`
  selects

Excluded:
- the sort by `should-start` in full, which needs the buffer (PM7)

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
- `poman push` refusing while `poman check` fails
  (`deadline-files-hold-one-deadline-each.decision.md`, "How the files land"),
  which PM4 then proves
- the GitHub adapter's API choice, open today: the proposal is octocrab
  for the REST API and graphql_client for the board's GraphQL API, a token
  from `GITHUB_TOKEN` else `gh`'s, and tests that never touch the real GitHub
- the open cases: a marker edited away or a file renamed; a "Blocked by" link
  made by hand
- which board fields and views poman draws, and the dot folder's name

Excluded:
- drawing this repository's own board from its roadmaps and plans (PM9)
- pull requests and comments

Proof:
- fetch, diff and push against a fake tracker: drift reported, a changeset
  written and applied exactly, a push refused after a change on the tracker
- the mirror rebuilt from the tracker alone
- one manual run against a real GitHub project made for it, recorded once,
  outside the test suite

---

### PM7 - The Forecast, Recurring Deadlines, Absences, The Morning Run

Status: Draft
Promise: poman tells each morning what must start, from the files alone, and
keeps recurring obligations and absences in the repository.
Depends On: PM3; PM6 for showing the forecast on the tracker
Execution Plan: Not created yet (one plan per part, in the order below)

Included:
- the forecast: the 15-day window, `should-start` computed backwards through
  the blockers as the deadline decision, "should-start", sets it (the owner's
  decisions, 2026-10-06), whether a latest finish means done before that day
  starts or by its end (open today), the importance buffer's exact rule (open
  today), and
  the "still not done" nudge for important items with no deadline
- recurring deadlines: a later type whose occurrences are deadline files named
  by period, one file per occurrence; conditions on a recurrence (only above
  a threshold, only if some event happens)
- absences, declared in the repository, and public holidays with them
  (`deadline-files-hold-one-deadline-each.decision.md`, "should-start")
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

---

### PM9 - poman Draws This Repository's Board

Status: Draft
Promise: The board of this repository is drawn by poman from its roadmaps and
plans, and the coordinator stops mirroring statuses by hand.
Depends On: PM6
Execution Plan: Not created yet

Included:
- poman reading this repository's roadmap entries and plans, and how: through
  the shared type definitions, which amends
  `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`
- drawing them onto the board through the tracker sync (the owner,
  2026-10-06, recorded in
  `wiki/decisions/poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`)
- `wiki/decisions/work-is-recorded-in-the-repository.decision.md`, rule 5,
  and `wiki/checklists/operation-manager.checklist.md`, "The Board", naming
  the command instead of the hand-mirroring

Excluded:
- review states, which stay on the pull request

Proof:
- a `poman diff` against this repository's board shows no change after a
  push, and a plan's status change shows up as one card move
