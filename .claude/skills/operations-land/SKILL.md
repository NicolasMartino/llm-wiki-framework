---
name: operations-land
description: Land an llm-wiki-framework worker's PR as the operation manager - make sure its blind review ran, check the gates and CI, mark it ready, check the owner's verdict, merge, write the log entry and the plan's status, and clean up. Use for "/operations-land <PR>", when a worker reports done, or when asked to mark ready, merge or clean up after a PR.
---

# Operations: land a PR

Steps: blind review → gate check → ready → verdict → check it yourself → merge
→ log and status → clean-up. The rules and their reasons are in
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
   to one fix round (checklist, "Starting A Worker", "A fix round"). A log PR
   gets no blind review, nor does the PR from `develop` into master.
2. **The gate check** (kinds with the full gates; a wiki or log PR skips the
   local part). Read the `just verify` log raw: every step ran and passed,
   nothing skipped. The `## Full gate run — <sha>` comment names the PR's
   current head (`gh pr view <n> --json headRefOid`), and CI passed on that head
   (`gh pr checks <n>`). Anything else: the PR stays draft, and a fix round
   fixes it; a skip is not a pass.
3. **Ready.** If the PR is draft (`gh pr view <n> --json isDraft`),
   `gh pr ready <n>`, then tell the owner it waits for them, by name.
4. **The verdict.** The owner brings it: a PR comment whose heading contains
   PASS or FAIL; CHANGES REQUESTED counts as FAIL. Find the newest verdict and
   check it names the PR's current head. A PASS the owner gave in
   conversation: record it first, as a comment headed `## PASS — <full sha>`
   saying "the owner's verdict on this head, given in conversation". No
   verdict, or a PASS on an older commit: the PR stays open, and the owner
   hears which. A heading with both PASS and FAIL is no pass: ask the owner. A
   failed verdict: `gh pr ready <n> --undo`, then a fix round; when it is
   pushed, come back to step 2.
5. **Check it yourself.** Read the diff against the issue, the plan and its
   Done when; a review can be wrong or incomplete. For a log PR, check every
   entry against the merges it records, in GitHub and in Git. Raise anything
   you find with the owner before merging.
6. **Merge.**

   ```bash
   gh pr merge <n> --squash --match-head-commit <full sha> --delete-branch
   ```

   The full SHA, not a short one. One at a time; before the next, wait for its
   `mergeable` to leave UNKNOWN. Then check the issue closed
   (`gh issue view <issue> --json state`; GitHub closes it on a merge into
   `develop`, its default branch), and close it with a pointer to the PR if it
   did not. The PR from `develop` into master merges with a merge commit
   instead (checklist, "Develop And Master").
7. **The log and the status.** Add the merge's entry at the top of
   `wiki/log.md` in this wiki's format, mark its plan Completed (the word after
   it is in the checklist, "The Board") with the Branch line removed, and update its roadmap entry, all in a log PR
   (checklist, "Landing A PR", has the commands); land that PR by these same
   steps when it is ready. Then the issue to Completed on the board, and any
   plan it blocked: unblock it in the repository first, then on the board. Not
   for a log PR itself, nor for a piece merged into an integration branch. The
   PR from `develop` into master gets one entry and changes no status
   (checklist, "Develop And Master").
8. **Clean up.** The PR's worker and its reviewer were released when their
   reports were read; check
   `orca orchestration worker-list --run <R> --terminal-state reclaimable --json`
   shows neither. Then remove the worktrees once nothing needs them:
   `orca worktree rm --worktree path:<worktree> --run-hooks`, for the worker's
   and the reviewer's. Never with uncommitted or unpushed work, never
   `--force` without the owner's yes.
9. **Tell the owner** in a line or two: what merged, by name, and what it
   unblocks.
