# Work In Flight Is A Pushed Branch

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: Development process
- Scope: What counts as work in flight, what a worktree is, and how a plan
  names its branch and where its proof holds.
- Sources:
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`,
    section 5 (plans before work; the `status` recipe)
  - RepForge's decision of the same name (2026-09-18), carried over through
    riseon
- Related:
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`
  - `justfile`, recipe `branch-status`

## Decision

**A branch on `origin` is work in flight. A worktree is not.**

1. **`origin` is the authority.** `git ls-remote --heads origin` answers what is
   live. `git worktree list` answers only what one machine has checked out.
2. **A worktree is a local convenience.** It carries no status, and its
   absence proves nothing about whether work exists.
3. **Unpushed is invisible.** A branch that never reached `origin` cannot be
   seen, reviewed, continued or rescued by anyone else. Workers push early.
4. **A plan whose work is under way names its branch**: `- Branch: \`<branch>\``
   under `- Status: Active` or `- Status: Blocked`. The coordinator writes it
   when the worker starts and removes it when the plan is Completed
   (`wiki/checklists/operation-manager.checklist.md`, "The Board").
   `just branch-status` fails on a plan naming a branch `origin` lacks
   (UNPUSHED) and on a plan status outside the vocabulary (STALE). A pushed
   branch no plan names is listed as "no plan", for information: wiki work,
   investigations and log PRs have none.
5. **A plan completed from now on says where its proof holds**:
   `Completed (master)` once merged, `Completed (local)` or
   `Completed (spike)` when the proof holds only there. Plans completed before
   2026-10-06 keep their bare `Completed`, which the check still accepts.

## Why

In RepForge a status page described four worktrees as in flight. They existed,
but on another machine; read from this one, the table looked like four lost
tracks, and a session following its instructions exactly reported them gone. A
rule whose truth depends on which machine reads it is not a rule. Pushing
answers the same question for every machine.

A bare `Completed` could not answer the question people actually ask, and a
plan proven only on a laptop had been read as shipped.

## What Would Revisit This

- More than one remote, or a fork-based flow.
- poman checking plans: `just branch-status` then gives way to it.
