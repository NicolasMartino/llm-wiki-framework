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
   issue are enough.
3. **The board shows the plans.** Each deliverable has one issue on the
   project's board, whose body points to its roadmap entry and plan. The
   board's Status column has the plan statuses as its options, Draft, Active,
   Blocked and Completed, and shows the plan's. When the board and the
   repository disagree, the repository wins, and the board is corrected.
4. **Review states stay on the pull request**, not on the board: a draft PR is
   being worked on or fixed, a ready PR waits for the owner, and the feedback
   review and the owner's verdict are comments on it
   (`the-pull-request-is-the-review-surface.decision.md`).
5. **Until a tool draws the board** (poman's `push`), the
   coordinator mirrors each plan's Status onto its issue by hand, in the same
   step as the plan changes, and a blocked issue carries the plan's blocker as
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
- **This framework already works this way**: its deliverables live in
  `wiki/roadmaps/` and its plans carry a Status. The board adds a view for the
  owner, not a second record.
- **The review states belong to the attempt**, and the PR is scoped to one
  attempt; a column for them duplicated what the PR already shows.

## Consequences

- Moving a card is never the act that changes a status: the plan changes, and
  the card follows.
- A plan's status change made on a worker's branch reaches master only when
  that branch merges; how status-only edits land before that is settled in
  `wiki/checklists/operation-manager.checklist.md`, "The Board".
- RepForge's "the GitHub project is where work is tracked" does not apply here.

## What Would Revisit This

- poman drawing the board: rule 5's hand-mirroring then goes, and this page
  names the command instead.
- More than one person tracking work, where the board's permissions and views
  become the question.
