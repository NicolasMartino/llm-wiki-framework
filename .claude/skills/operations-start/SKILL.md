---
name: operations-start
description: Start an llm-wiki-framework worker on an issue as the operation manager - pick the kind, check the plan, gather and check the facts, write the spec, paste the kind's base text, run the checklist, start the Orca worker and move its issue to Active on the board. Use for "/operations-start <issue>", or when asked to start, dispatch or brief a worker on an issue.
---

# Operations: start a worker

Steps: issue → kind → plan → facts → spec → checklist → worker → board. The
rules and their reasons are in `wiki/checklists/operation-manager.checklist.md`,
"Starting A Worker" and "Limits Across Workers"; each kind's reasons are in
`wiki/checklists/worker-briefs.checklist.md`. Load `/operations` first if this
session has not.

## Beside this skill

- `spec-template.md`: the spec's shape, and what each kind's spec fills.
- `facts-helper-prompt.md`: the fixed prompt for the facts helper.
- `base/kinds/<kind>.txt`: one fixed base text per kind. Its last line names
  the shared parts that follow it, in order.
- `base/shared/`: the shared rules and the add-ons (the plan's status, full
  gates, wiki-only, no file changes, talking with the owner).
- `base/blind-review.txt`: the whole spec of a PR's one blind review, `<PR>`
  replaced by its number (checklist, "Landing A PR").

Never edit a base text or a shared part for one task. A rule changes there for
every task, with its reason in the worker briefs checklist.

## The kinds

- **A code change**: Opus, medium (Opus, high to design a new boundary);
  `base/kinds/code.txt`.
- **Tooling** (tools/, .github/, the justfile; never releases): Opus, medium;
  `base/kinds/tooling.txt`.
- **Settling a design with the owner (no code)**: Opus, high; no PR;
  `base/kinds/design-with-owner.txt`.
- **A wiki PR** (plans included): Opus, medium; no local gates;
  `base/kinds/wiki.txt`.
- **An investigation**: Sonnet, medium (a follow-up that could not pin the
  cause goes to Opus, high); `base/kinds/investigation.txt`.

**A fix round** (after the owner's FAIL, or a blind review whose PR worker is
gone) uses the kind of the PR's work, in the PR's worktree; the checklist,
"Starting A Worker", "A fix round", has the placement and the lines its spec
must carry, among them, after a failed verdict, `git pull --no-rebase` first,
since the branch carries the coordinator's bookkeeping commit.

A task that fits none: write the spec from the template and the shared rules,
and once it is done add a kind here, a base text, and its section in the
worker briefs checklist.

## Steps

1. **The issue.** `gh issue view <n> --comments`. Its deliverable is in a
   roadmap, it is on the board, the owner gave the go, and no hold covers it.
2. **The kind.** Pick it yourself from the list above; split a task that mixes
   kinds into one issue per kind first.
3. **The plan.** For code or tooling, its plan exists in `wiki/plans/` and is
   Draft or Blocked-and-now-free. `develop` shows a plan in flight as Draft,
   so also check no open PR or live worker already has it: the board card is
   not Active, and `gh pr list` shows no PR for its issue. No plan yet: the first worker is a wiki
   worker that writes it, and the owner approves it before the code worker.
4. **The facts.** Start the read-only `investigator` subagent (Sonnet) with
   `facts-helper-prompt.md`, `<n>` filled. Check each fact it returns yourself
   (open the link, run the command) before it goes into Context; drop what you
   cannot confirm.
5. **The spec.** Bring the main checkout's `develop` up to date first (on
   `develop`: `git fetch origin && git merge --ff-only origin/develop`). Write
   the spec from `spec-template.md` into a file in your scratchpad, Goal to
   Done when, with that `develop` SHA in Context. Then paste under it, unchanged, the kind's
   base text and the shared parts its last line names, in that order:

   ```bash
   b=.claude/skills/operations-start/base
   { cat <spec file>; for f in $b/kinds/<kind>.txt $b/shared/<part>.txt …; do echo; cat "$f"; done; } > <full spec file>
   ```

   (run from the repository root; both spec files in your scratchpad, never in
   the repository). Read the result through once.
6. **The checklist.** Every item, every start:
   - **Base branch:** `--base-branch develop` (or the integration branch the
     spec names), never master, on a new-child worktree, local `develop`
     fast-forwarded first, and that SHA in the spec's Context. The PR goes
     into `develop` (or that integration branch).
   - **The gates line:** a code or tooling kind has `full-gates.txt`; a wiki
     PR has `wiki-only.txt`; a design with the owner or an investigation has
     `no-file-changes.txt`. Each kind's last line names its parts.
   - **The plan's status:** a code, tooling or wiki kind has `plan-status.txt`
     after the shared rules, and the spec's Context names the plan and its
     roadmap entry (or says there is no plan); the worker sets the status in
     its PR (checklist, "The Board").
   - **No tool mentions:** the shared rules are pasted whole, and nothing in
     the spec asks for a footer, a trailer, tool wording or a local path in
     commits or on GitHub.
   - **Names:** the worktree `<Track> · #<issue> <kind>` (an issue in no track
     drops `<Track> · `); the PR and commit titles follow the issue's pattern.
   - **Model and effort:** the kind's tier from the list above.
   - **Limits:** at most two workers doing the work itself after this one
     starts, so a third slot stays free for a blind review; `free -g` checked;
     nothing on hold (checklist, "Limits Across Workers").
   - **The owner's questions:** a worker that talks to the owner gets, in its
     Context, the Roman numeral to start from.
7. **The worker.**

   ```bash
   orca orchestration worker-start --run <R> --spec "$(cat <full spec file>)" \
     --task-title "<title>" --agent claude --model <opus|sonnet> --effort <medium|high> \
     --worktree new-child --base-branch develop --name <slug> \
     --display-name "<Track> · #<issue> <kind>" --json
   ```

   Confirm `launch.effective` in the receipt shows the model and effort asked
   for. Then link the worktree:
   `orca worktree set --worktree path:<worktree> --issue <n> --json`, and check
   `git -C <worktree> log --oneline -1` shows the SHA in the spec.
8. **The board.** The issue to Active on the board, any "(blocked: …)" note
   off its title. The plan's own status changes in the worker's PR, not here
   (the `plan-status.txt` part; checklist, "The Board"); the coordinator
   commits no status. A fix round changes no status, and leaves a plan its PR
   already marked Completed as it is. A plan that was Active on `develop`
   before 2026-10-06: name it in the spec's Context as an exception, so its
   own PR completes it (checklist, "The Board"). Tell the owner in one
   line: the work by name, the kind, the tier.
