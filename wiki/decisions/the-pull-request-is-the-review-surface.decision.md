# The Pull Request Is The Review Surface

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: Development process
- Scope: Where a review lives, the blind feedback review every PR gets first,
  what counts as the owner's verdict, what a merge needs, and which branch a
  PR goes into.
- Sources:
  - The owner's decisions of 2026-10-06, and what riseon's blind reviews of
    that day found; a PASS given in conversation
  - RepForge's decision of the same name (2026-09-19, amended 2026-10-03 and
    2026-10-06), carried over through riseon
  - The owner's decisions of 2026-10-06 on `develop`: "They take forever" (of
    the full CI matrix); "I prefer the develop
    branch with the merge the pr to master triggering the longer CI"; "a merge
    into develop PR only triggers fast CI"; "all issues/PR should be pointing
    at develop now"
- Related:
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`
  - `wiki/checklists/operation-manager.checklist.md`, "Landing A PR"
  - `.claude/skills/operations-start/base/blind-review.txt`
  - `wiki/plans/develop-and-master-ci.plan.md` (which CI runs on which PR)

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
   full gates passed on that head. Every PR into `develop` or into an
   integration branch is a squash merge pinned to that head.
5. **A log PR gets no blind review.** It holds only the coordinator's record of
   merges already reviewed, which the coordinator checks line by line against
   GitHub and Git; the owner's PASS still decides it.
6. **Work PRs go into `develop`; master takes `develop` only through a PR.**
   `develop` is GitHub's default branch, and every work PR, log PR and
   integration branch targets it. Master changes only by a PR from `develop`,
   which runs the full CI and needs the owner's PASS on its head like any
   other; it merges only once every job of the full CI passed on that head
   (`gh pr checks <n>`). Recommended, until the owner settles it: its head is
   a branch cut from `develop` at a fixed commit, it gets no blind review,
   since every change it carries had its own, and it merges with a merge
   commit, not a squash, pinned to its head, with `develop` never deleted
   (open question (c), "Open For The Owner").

## Open For The Owner

- **(c) How `develop` reaches master.** Recommended: the coordinator opens the
  PR into master when the owner asks for it, from a branch cut from `develop`
  at a fixed commit; it gets no blind review, and it merges with a merge
  commit, never deleting `develop`. Why a fixed commit: `develop` keeps
  moving while the PR waits, and a head that moves restarts the slow full CI
  and leaves the owner's verdict on an older commit. Why a merge commit: a
  squash would put on master one commit `develop` does not have, so the next
  PR from `develop` would carry every earlier change again and conflict with
  it; a merge commit keeps master's history a superset of `develop`'s. The steps are in
  `wiki/checklists/operation-manager.checklist.md`, "Develop And Master".

## Why

- **Work lands on `develop` so master only takes what passed the full CI**:
  the full matrix takes too long for every work PR ("They take forever", the
  owner, 2026-10-06), so work PRs get a fast check and the PR into master the
  whole CI (`wiki/plans/develop-and-master-ci.plan.md`).
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
