# The Pull Request Is The Review Surface

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: Development process
- Scope: Where a review lives, the blind feedback review every PR gets first,
  what counts as the owner's verdict, and what a merge needs.
- Sources:
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`,
    section 6 (the blind reviews of riseon's PRs; a PASS given in
    conversation)
  - RepForge's decision of the same name (2026-09-19, amended 2026-10-03 and
    2026-10-06), carried over through riseon
- Related:
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`
  - `wiki/checklists/operation-manager.checklist.md`, "Landing A PR"
  - `.claude/skills/operations-start/base/blind-review.txt`

## Decision

**Work finishes by opening a pull request, and every review lives on that pull
request as a comment.** Not in a terminal, not in a file in a worktree, not in
a worker's report.

1. **The branch is still the unit of work in flight**
   (`work-in-flight-is-a-pushed-branch.decision.md`). A PR wraps a pushed
   branch; `gh pr diff` gives one fixed answer to "what changed".
2. **One blind feedback review first.** Every PR gets one review before the
   owner's: a worker that reads only the PR, the issues it closes and the
   repository, with no spec and no account from the worker that wrote it. It
   posts one comment headed `## Feedback review — <commit>` whose first line
   says it is feedback, not the owner's verdict, never containing PASS or
   FAIL, and changes nothing. No other heading on GitHub carries those
   words either, so a verdict is never mistaken. Its P1s and P2s are fixed in one fix
   round; its P3s are fixed there or filed as issues. One review round and one
   fix round, no more.
3. **The verdict is the owner's.** A verdict is a PR comment whose heading
   contains PASS or FAIL, wherever the word sits; CHANGES REQUESTED counts as
   FAIL, and a heading with both is no pass. It names the commit it reviewed.
   A verdict the owner gives in conversation is recorded on the PR before the
   merge, as `## PASS — <full sha>` saying so.
4. **A merge needs the owner's PASS naming the PR's current head**, checked by
   the coordinator against the diff, and, for a PR whose kind has gates, the
   full gates passed on that head. It is a squash merge pinned to that head.
5. **A log PR gets no blind review.** It holds only the coordinator's record of
   merges already reviewed, which the coordinator checks line by line against
   GitHub and Git; the owner's PASS still decides it.

## Why

- **A review nobody else can reach has the same value as no review.** In
  RepForge a 33-file review lived only in one worktree's terminal, so nothing
  showed the branch had been reviewed at all.
- **The diff has to be fixed.** Two defensible diff commands once gave
  opposite answers on the same branch; a PR's diff has one.
- **The blind review catches what the author cannot see.** In RepForge's
  trial (2026-10-04 to 06) its first round found a P1 or P2 on every PR it
  read, among them an open redirect and a gate that could never pass, before
  the owner saw them. It gets no spec so it does not inherit the spec's blind
  spots. A second reviewer per PR found nothing the first had missed, so there
  is one.
- **Merges without a recorded PASS happened.** PRs merged after CHANGES
  REQUESTED and a fix with no PASS ever posted; pinning the merge to the
  reviewed head stops a later push slipping in.

## What Would Revisit This

- Reviews done in GitHub's own review UI, which would make the heading
  convention friction.
- The blind review routinely finding nothing, or the owner's reviews routinely
  finding what it should have.
