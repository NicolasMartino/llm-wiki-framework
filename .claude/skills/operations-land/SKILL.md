---
name: operations-land
description: Land an llm-wiki-framework worker's PR as the operation manager - make sure its blind review ran, check the gates and CI, add its bookkeeping commit (log entry and backlog), mark it ready, check the owner's verdict, merge, move the board, and clean up. Use for "/operations-land <PR>", when a worker reports done, or when asked to mark ready, merge or clean up after a PR.
---

# Operations: land a PR

Steps: blind review → gate check → bookkeeping commit → ready → verdict →
check it yourself → merge → board → clean-up. The rules and their reasons are in
`wiki/checklists/operation-manager.checklist.md`, "Landing A PR". Load
`/operations` first if this session has not. An issue answered by a comment
(an investigation, a design with the owner) lands by the checklist's "Landing
A Comment" instead.

A step that fails stops the walk here: the PR stays as it is, and the owner
hears why. Workers are released when their report is read, not here
(checklist, "While Workers Run").

1. **The blind review.** It normally ran at the worker's review ask: a
   `## Feedback review — <commit>` comment is on the PR, and the fix round is
   pushed. If it did not run (a PR from outside a worker, say), start it now
   with the command in the checklist, "Landing A PR", then give its P1s and P2s
   to one fix round (checklist, "Starting A Worker", "A fix round"). The PR
   from `develop` into master gets no blind review.
2. **The gate check** (kinds with the full gates; a wiki PR skips the local
   part). Read the `just verify` log raw: every step ran and passed,
   nothing skipped. The `## Full gate run — <sha>` comment names the PR's
   current head (`gh pr view <n> --json headRefOid`), and CI passed on that head
   (`gh pr checks <n>`). Anything else: the PR stays draft, and a fix round
   fixes it; a skip is not a pass. The PR from `develop` into master has only
   the CI part: every job of the full CI passed on its head, with no
   `## Full gate run` comment expected (checklist, "Develop And Master").
3. **The bookkeeping commit** (moment (a), the usual one; checklist, "Landing
   A PR", has the commands). On the PR's branch, after merging `develop` into
   it: the PR's own entry at the top of `wiki/log.md` in this wiki's format,
   written as if merged and citing the PR, and any backlog roadmap entries
   waiting then, among them the entry of a PR from `develop` into master
   merged since the last one. No plan status: that came with the work. Then
   check CI passes on the new head; the `## Full gate run` comment keeps
   naming the head before it. The PR from `develop` into master gets none of
   its own: its entry goes in the next PR's.
4. **Ready.** If the PR is draft (`gh pr view <n> --json isDraft`),
   `gh pr ready <n>`, then tell the owner it waits for them, by name.
5. **The verdict.** The owner brings it: a PR comment whose heading contains
   PASS or FAIL; CHANGES REQUESTED counts as FAIL. Find the newest verdict and
   check it names the PR's current head. A PASS the owner gave in
   conversation: record it first, as a comment headed `## PASS — <full sha>`
   saying "the owner's verdict on this head, given in conversation". No
   verdict, or a PASS on an older commit: the PR stays open, and the owner
   hears which. A heading with both PASS and FAIL is no pass: ask the owner. A
   failed verdict: `gh pr ready <n> --undo`, then a fix round; when it is
   pushed, come back to step 2.

   Moment (b), just after the PASS, only when the bookkeeping commit was not
   added at step 3, or a backlog entry arrived since: add it the same way,
   then check `git diff --name-only <passed sha>..<new head>` names nothing
   but `wiki/log.md` and roadmap pages (when `develop` moved since the PASS,
   check the bookkeeping commit alone and a merge of `develop` with no
   conflict fixed by hand outside the log). Record it on the PR as a comment
   headed `## Bookkeeping after the verdict — <new head>`, naming the passed
   SHA, the new head and those files; merge pinned to the new head. Anything
   else in the diff: the PR waits for the owner's verdict on the new head.
6. **Check it yourself.** Read the diff against the issue, the plan and its
   Done when; a review can be wrong or incomplete. A PR with a plan marks it
   `Completed (develop)` with its Branch line removed, and updates its roadmap
   entry once the entry's work is all done (checklist, "The Board"); one that
   does not goes back to its worker, or to a fix round, before merging. That
   holds for a plan that was Active on `develop` before 2026-10-06 too: its
   spec names the exception, and its own PR completes it. Check the
   bookkeeping commit's log entry against the PR and its roadmap entries
   against their issues. Raise anything you find with the owner before
   merging.
7. **Merge.**

   ```bash
   gh pr merge <n> --squash --match-head-commit <full sha> --delete-branch \
     --subject "<the PR title> (#<n>)"
   ```

   The full SHA, not a short one: the head the PASS names, or the
   bookkeeping head recorded at moment (b). `--subject` because a one-commit PR would
   otherwise squash under its commit's title, not the PR's. One at a time; before the next, wait for its
   `mergeable` to leave UNKNOWN. Then check the issue closed
   (`gh issue view <issue> --json state`; GitHub closes it on a merge into
   `develop`, its default branch), and close it with a pointer to the PR if it
   did not. The PR from `develop` into master merges with a merge commit
   instead (checklist, "Develop And Master").
8. **The board.** The issue to Completed, and any plan it blocked: its card
   back to Active, or Draft if no worker had started it, with the title note
   off; the worker that picks it up sets the plan Active in its own PR. Not
   for a piece merged into an integration branch. The PR from `develop` into
   master changes no status (checklist, "Develop And Master").
9. **Clean up.** The PR's worker and its reviewer were released when their
   reports were read; check
   `orca orchestration worker-list --run <R> --terminal-state reclaimable --json`
   shows neither. Then remove the worktrees once nothing needs them:
   `orca worktree rm --worktree path:<worktree> --run-hooks`, for the worker's
   and the reviewer's. Never with uncommitted or unpushed work, never
   `--force` without the owner's yes.
10. **Tell the owner** in a line or two: what merged, by name, and what it
   unblocks.
