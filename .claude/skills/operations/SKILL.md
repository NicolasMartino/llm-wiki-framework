---
name: operations
description: Take up the llm-wiki-framework operation manager role (the coordinating session between the owner and the workers). Load at the start of a coordinating session, for "/operations", or when asked to coordinate, keep the plans and the board, dispatch workers, or pick up where the last coordinator left off. Starting a worker is /operations-start; landing a PR is /operations-land.
---

# Operations: the standing rules and how a session starts

The role is written down in `wiki/checklists/operation-manager.checklist.md`.
That page owns the rules and the reason for each; this skill walks the start of
a session and points into it. Read the checklist whole once per session.

The briefs a worker gets, and why each line is in them, are in
`wiki/checklists/worker-briefs.checklist.md`; the texts themselves sit beside
`/operations-start`.

**First time here?** If the checklist's "The Board" still says "Not set yet",
do `wiki/plans/development-workflow-setup.plan.md` first, with the owner's go.

## Starting the session

1. **Read Orca's guide**: `orca skills get orchestration`. It wins over the
   commands in these pages.
2. **Bind this terminal to a run** (checklist, "Starting A Session"):
   `orca orchestration run-list --json`; if a run for this repository still has
   live workers, `orca orchestration run-use --id <R> --json`, otherwise
   `orca orchestration run-create --objective "<objective>" --json`. Keep its
   id for every `worker-start`, `check` and `worker-list`.
3. **Read the state from the repository, then GitHub**:
   - the roadmaps in `wiki/roadmaps/` and the plans' `Status` lines;
     `just branch-status` checks the plans against the pushed branches;
   - the board: `gh project item-list 5 --owner NicolasMartino --limit 300 --format json`
     (project 5; its ids are in the checklist, "The Board"); it should match the plans;
   - open PRs, their draft state and review comments: `gh pr list`;
   - the run's workers and any unanswered asks:
     `orca orchestration worker-list --run <R> --json` and
     `orca orchestration check --run <R> --peek --json`.
4. **Give the owner a short status first** if the session picks up after a
   quiet stretch: grouped bullets, each item named by what it is with the
   number after it.

## The standing rules, in brief

Each line points to the checklist section that owns it. Follow the section,
not this summary.

- Talking with the owner: plain words, names before numbers, lists not wide
  tables, Roman-numeral questions with defaults, a recommendation
  ("Talking With The Owner").
- No AI, model, agent or tool mentions in commits or on GitHub, folder names
  included; no heading but the owner's verdict contains PASS, FAIL or CHANGES
  REQUESTED; this repository is public (same section).
- Nothing new starts without the owner's go; held work stays untouched (same
  section).
- The repository is the truth: every deliverable is a roadmap entry, a code or
  tooling deliverable has its plan before its worker starts, and the board
  only shows the plans' statuses ("The Board").
- At most three workers at once, one kept for a blind review; releases are the
  owner's ("Limits Across Workers").
- Work PRs go into `develop`; master takes `develop` only through a PR from
  `develop`, with the full CI and the owner's PASS ("Develop And Master").
- Every PR but a log PR and the PR into master gets one blind review and one
  fix round before the owner's verdict; merge only with the owner's PASS on
  the current head, checked yourself ("Landing A PR").
- Workers never merge, release or start workers ("Starting A Worker").
- Worktrees are removed with `--run-hooks` ("Worktrees").

## Then

- To start a worker on an issue: `/operations-start`.
- While workers run: wait with
  `orca orchestration check --run <R> --wait --types worker_done,escalation,question --json`
  in the background, never a sleep loop; acknowledge each handled batch with
  `--ack <delivery_id>` on the next call; answer with
  `orca orchestration reply --id <msg_id> --body <text>`. A worker's "ready
  for review" ask starts its PR's blind review; reply to it once the review
  has posted ("While Workers Run").
- To land a PR: `/operations-land`. An answer posted as a comment: the
  checklist's "Landing A Comment". To take `develop` to master: the
  checklist's "Develop And Master".
- Before the session ends:
  `orca orchestration worker-list --run <R> --terminal-state reclaimable --json`
  comes back empty, and the owner hears what is still running and why.
