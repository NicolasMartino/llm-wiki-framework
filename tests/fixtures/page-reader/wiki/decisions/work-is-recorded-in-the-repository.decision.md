# Work Is Recorded In The Repository

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: Development process
- Scope: Where the state of a piece of work is kept, what the GitHub board is,
  and what its columns mean.
- Sources:
  - The owner's decisions of 2026-10-06 in riseon's coordinating session: the
    repository is the truth and every board is a view of it, the board's
    columns are the plan statuses, and "remember the repo is the truth"
  - The owner, 2026-10-06: "the status change should be part of the PR that
    contains the work that is suppose to achieve this status change, so no
    separate commit or PR"
- Related:
  - `wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md`
  - `wiki/decisions/the-pull-request-is-the-review-surface.decision.md`
  - `wiki/checklists/operation-manager.checklist.md`, "The Board"
  - `wiki/plans/development-workflow-setup.plan.md`

## Decision

**The repository is the only source of truth for work. Every GitHub board is a
view of it, nothing else.**

1. **Every deliverable is a roadmap entry.** The roadmaps in `wiki/roadmaps/`
   hold the order; nothing is worked on that no roadmap names. Backlog means a
   roadmap entry (or a proposal, for an idea not yet accepted), not only an
   issue.
2. **A code or tooling deliverable has its plan before its worker starts**, in
   `wiki/plans/<slug>.plan.md`. The plan's `Status` is where the work stands:
   Draft, Active, Blocked or Completed. A Blocked plan says in its body what it
   waits on and what starts it. Wiki-only work, an investigation, or a design
   settled with the owner needs no plan of its own; its roadmap entry and its
   issue are enough. **A status changes in the PR that does the work**: Active,
   with the branch, in its first push; Blocked, in the same PR, if its work
   stops to wait on something; Completed before it leaves draft, made true by
   the merge, with its roadmap entry once the entry's work is all done. No
   separate commit or PR changes a status. A PR that writes a plan leaves it
   Draft; a plan blocked before any worker starts stays Draft, its issue
   carrying the blocker; a settled investigation's or design's roadmap status
   goes in the wiki PR that records its outcome.
3. **The board shows the plans.** Each deliverable has one issue on the
   project's board, whose body points to its roadmap entry and plan. The
   board's Status column has the plan statuses as its options, Draft, Active,
   Blocked and Completed, and shows the plan's as the work's pushed branch has
   it: `develop` shows a plan in flight as Draft until its PR merges, so a
   card Active or Blocked is checked against `origin/<branch>`, not
   `develop`. When the board and the repository read that way disagree, the
   repository wins, and the board is corrected.
4. **Review states stay on the pull request**, not on the board: a draft PR is
   being worked on or fixed, a ready PR waits for the owner, and the feedback
   review and the owner's verdict are comments on it
   (`the-pull-request-is-the-review-surface.decision.md`).
5. **Until a tool draws the board** (poman's `push`), the
   coordinator moves each issue by hand: Active when its worker starts,
   Completed when its PR merges, and a blocked issue carries the plan's blocker as
   a short title note (` (blocked: <short name> #<n>)`) and GitHub's own
   "Blocked by" link when the blocker is an issue.
6. **A stage is a milestone** only once the owner lays out stages; until then
   the roadmaps' deliverables are the stages, and issues carry no milestone.

## Why

- **The owner wants the record in the repository, not in chat or issues**
  (riseon, 2026-10-06, finding no plans: "are you really making the effort of
  doing the llm wiki?", then "remember the repo is the truth").
- **A board that holds state drifts from the pages that describe the work.**
  In RepForge the board was the authority, and the wiki, the issues and the
  board had to be reconciled by hand; with one source, there is nothing to
  reconcile, only a view to redraw.
- **A status belongs with the work that makes it true.** A separate status
  commit is one more thing to forget, and a status on `develop` that no merged
  work backs is a claim, not a record.
- **This framework already works this way**: its deliverables live in
  `wiki/roadmaps/` and its plans carry a Status. The board adds a view for the
  owner, not a second record.
- **The review states belong to the attempt**, and the PR is scoped to one
  attempt; a column for them duplicated what the PR already shows.

## Consequences

- Moving a card is never the act that changes a status: the plan changes in
  its PR, and the card follows.
- `develop` shows a plan as Draft until its PR merges. What is in flight is
  the pushed branch, the plan as that branch has it, and the board
  (`wiki/checklists/operation-manager.checklist.md`, "The Board").
- RepForge's "the GitHub project is where work is tracked" does not apply here.

## What Would Revisit This

- poman drawing the board: rule 5's hand-mirroring then goes, and this page
  names the command instead.
- More than one person tracking work, where the board's permissions and views
  become the question.
