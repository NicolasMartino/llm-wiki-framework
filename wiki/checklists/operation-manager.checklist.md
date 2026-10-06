# Operation Manager

- Document Class: Checklist
- Status: Active
- Date: 2026-10-06
- Category: Development process
- Scope: What the coordinating session does between the owner and the
  workers: talking with the owner, the plans and the board, issues and names,
  starting a worker, fix rounds, the limits across workers, worktrees,
  integration branches, the blind review, landing a PR or a comment, the log
  entry, taking `develop` to master, and clean-up. Each rule with its reason.
- Sources:
  - The owner's decisions of 2026-10-06 on how this repository is worked on,
    and that work lands on `develop` ("all issues/PR should be pointing at
    develop now")
  - The operation manager runbooks of RepForge and riseon (2026-10-02 to
    2026-10-06), cut down for this repository; the reasons cite what happened
    there
- Related:
  - `wiki/plans/development-workflow-setup.plan.md` (done once, first)
  - `wiki/checklists/worker-briefs.checklist.md`
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`
  - `wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md`
  - `wiki/decisions/the-pull-request-is-the-review-surface.decision.md`
  - `wiki/plans/develop-and-master-ci.plan.md`

## What This Is For

The owner decides; workers do the work in their own worktrees; the operation
manager (the coordinator) is the session in between. It keeps the plans and the
board true, files and names issues, starts each worker with the right brief,
holds the limits no single worker can see, starts each PR's blind review,
checks the work before it lands, and cleans up after it. It writes no product
code itself.

Three repo commands walk the steps and point back here:

- `/operations` (`.claude/skills/operations/SKILL.md`): load at the start of a
  coordinating session.
- `/operations-start <issue>` (`.claude/skills/operations-start/`): issue →
  kind → plan → facts → spec → checklist → worker → status. The base texts sit
  beside it.
- `/operations-land <PR>` (`.claude/skills/operations-land/SKILL.md`): blind
  review → gate check → ready → verdict → check it yourself → merge → log and
  status → clean-up.

Workers run under Orca. The machine-wide rules (every agent is an Orca worker,
the worker lifecycle, the spec shape) are in the owner's global Claude
instructions; Orca's own guide, `orca skills get orchestration`, matches the
installed version and wins over any command here.

This repository is public. These pages may name tools (Orca, Claude, model
names), but never a path on the owner's machine. Commit messages and GitHub
text name no tool at all ("Talking With The Owner").

This repository is also the framework's own proof: a pain point that the
coordinator or a worker meets with `llm-wiki` is a bug in the product, so it
becomes a roadmap entry and an issue, not a workaround.

**If "The Board" below still says "Not set yet", do
`wiki/plans/development-workflow-setup.plan.md` before anything else.**

## Starting A Session

- **Read Orca's guide once per session**: `orca skills get orchestration`.
- **Bind this terminal to a run.** Workers belong to a run, and their messages
  go to that run only. First `orca orchestration run-list --json`: if a run for
  this repository still has live workers, take it over with
  `orca orchestration run-use --id <R> --json`; otherwise
  `orca orchestration run-create --objective "<objective>" --json`.
- **Read the state from the repository first**: the roadmaps and the plans'
  `Status` lines (`just branch-status` checks plans against the pushed
  branches), then the board, the open PRs (`gh pr list`), the run's workers
  (`orca orchestration worker-list --run <R> --json`) and their unanswered
  asks (`orca orchestration check --run <R> --peek --json`). Why: a session
  starts with none of the previous one's conversation, and the repository is
  the truth.
- **After a quiet stretch, give the owner a short status first.**

## Talking With The Owner

- **Plain, concrete words**, with a small example where it helps.
- **Name an issue or PR by what it is, with the number after it**: "the install
  progress bar (#12)", never a bare "#12, #13". Why: the owner follows several
  streams at once and cannot map numbers to work from memory.
- **Grouped bullet lists, never wide tables**; a table only for two or three
  short columns.
- **Questions are numbered in Roman numerals (I, II, …), each with a default.**
  The count runs on across sessions and across workers: keep the last number
  in the coordinator's memory, give a worker that talks to the owner the
  numeral to start from, and carry on after the last one it used. The owner
  answers some; restate each unanswered one with its default and ask for a
  one-word confirmation before acting on it. When the owner answers "you do
  your best", decide carefully and report what you decided and why.
- **A recommendation, not a survey of options.**
- **Nothing new starts without the owner's go**, and work the owner holds stays
  untouched until they lift the hold. The hold is written in its plan.
- **No mention of AI, models, agents or the tools that do the work in commit
  messages or on GitHub** (titles, bodies, comments, and folder names quoted in
  them): no footer, no trailer, no "Generated with", no Co-Authored-By, no
  local paths. Why: the owner wants the project record free of them; riseon
  first committed a folder named after a tool and it showed up in two issue
  bodies, whose edit history GitHub keeps. Backstops: `.githooks/commit-msg`
  strips the usual trailers, and `.claude/settings.json` turns Claude Code's
  attribution off in this repository.
- **No heading on GitHub contains PASS, FAIL or CHANGES REQUESTED except the
  owner's verdict**: a verdict is recognised by those words, in capitals, in a
  comment's heading. Gate results go under `## Full gate run — <sha>`.

## The Board

The rules are in
`wiki/decisions/work-is-recorded-in-the-repository.decision.md`: the plans hold
the status, and the board shows it.

- **A status changes in the plan first**, then on the board, in the same step:
  - Draft when the plan is written and its issue made;
  - Active when its worker starts, with `- Branch: \`<branch>\`` under the
    plan's Status line;
  - Blocked the moment it waits on something: the plan's body says what it
    waits on and what starts it; the issue gets the title note and, when the
    blocker is an issue, GitHub's "Blocked by" link;
  - Completed when its PR merges into `develop`: `Completed (develop)` (or
    `(local)`, `(spike)`), the Branch line removed, and its roadmap entry
    updated. Which word follows Completed is
    `work-in-flight-is-a-pushed-branch.decision.md`'s, rule 5: `(develop)`,
    decided by the owner on 2026-10-06.
- **Status-only edits go straight to `develop`.** A commit that changes nothing
  but plans' `Status` and `Branch` lines, and the matching roadmap entries'
  `Status:` lines, is the coordinator's, made in the main checkout on
  `develop` and pushed to `develop` with no PR: "Wiki: Mark <plan> Active".
  Everything else in a plan goes through a PR. Workers never edit a plan's
  Status or Branch lines. Why: a status written on a worker's branch reaches
  `develop` only at the merge, so `develop` (and `just branch-status`) would
  show the work as not started for its whole life.
- **Review states are on the PR, not the board**: draft while worked on or
  fixed, ready while it waits for the owner.
- **Check every move** with `gh project item-list` afterwards. Nothing moves an
  item by itself beyond what the board's workflows do.
- **How.** The ids, filled in by the setup plan:

  Project 5 of NicolasMartino
  (<https://github.com/users/NicolasMartino/projects/5>), id
  `PVT_kwHOAYhnl84Bl7R_`; Status field `PVTSSF_lAHOAYhnl84Bl7R_zhkmNdg` with
  options Draft `e7bf0810`, Active `f7d97c22`, Blocked `767d1b02`, Completed
  `4b72a5d5`. Set up 2026-10-06.

  ```bash
  # an issue's item id
  gh project item-list 5 --owner NicolasMartino --limit 300 --format json \
    -q '.items[] | select(.content.number==<n>) | .id'
  # an issue or PR not yet on the board: add it, which prints its item id
  gh project item-add 5 --owner NicolasMartino --url <issue url> --format json --jq .id
  # move it
  gh project item-edit --id <item> --project-id PVT_kwHOAYhnl84Bl7R_ \
    --field-id PVTSSF_lAHOAYhnl84Bl7R_zhkmNdg --single-select-option-id <option>
  # the title note, set or taken off
  gh issue edit <n> --title "<title> (blocked: <short name> #<blocker>)"
  # the native "Blocked by" link, when the blocker is an issue
  gh api -X POST repos/NicolasMartino/llm-wiki-framework/issues/<n>/dependencies/blocked_by \
    -F issue_id=$(gh api repos/NicolasMartino/llm-wiki-framework/issues/<blocker> --jq .id)
  ```

  If `gh project field-list 5 --owner NicolasMartino` shows other ids, trust
  it and fix this page.

## Issues And Names

- **One issue per deliverable**, its body pointing to the roadmap entry and,
  for code or tooling, the plan. An issue says what and why, never the code to
  change; a list of impacts says plainly that whoever does the work rechecks
  it. Its body: what, why, where (a first list), out of scope, done when.
- **Backlog means a roadmap entry**, and its issue on the board. A line in a
  plan or a chat alone gets lost.
- **Do not change an issue while its worker writes against it.** A change goes
  in a comment the worker is told about, or waits for the PR. Why: in riseon
  two issues were filed against a page while its worker was writing it, and
  the review found the mismatch as a P1.
- **Titles carry the track** when there is one: `<Track> · <Area>: <what>`; a
  track issue `<Track> track: <what>`; any other issue `<Area>: <what>`. A PR,
  and so its squash commit, takes the pattern of the issue it closes, with an
  imperative summary. A worker's Orca worktree is `<Track> · #<issue> <kind>`
  (`#<issue> <kind>` outside a track), linked to its issue.
- **A task that mixes kinds is split** into one issue per kind first.
- **How:**

  ```bash
  gh issue create --repo NicolasMartino/llm-wiki-framework --title "<title>" --body-file <file in your scratchpad>
  gh project item-add 5 --owner NicolasMartino --url <issue url>
  ```

## Starting A Worker

`/operations-start` walks the steps; the worker briefs checklist keeps each
kind's model, effort and reasons.

1. **The issue** is on the board, its deliverable is in a roadmap, and the
   owner gave the go.
2. **The kind** is the coordinator's pick, never a helper's.
3. **The plan**, for code or tooling: it exists before the worker starts, and
   says what the work proves and how. If it does not exist, the first worker
   is a wiki worker that writes it, and the owner approves it.
4. **The facts** come from a read-only in-session helper (the `investigator`
   subagent on Sonnet) with the fixed prompt beside `/operations-start`. **The
   coordinator checks each fact before it goes into the spec.**
5. **The spec** is short (Goal, Files, Out of scope, Context, Done when), with
   the commit the worktree starts from named in Context. The kind's base text
   is pasted under it **unchanged**, then the shared parts it names. Why: a
   rule changes in one place, and every worker of a kind gets the same rules.
6. **The checklist** in `/operations-start`, every start.
7. **The worker**, with the kind's model and effort. Check what Orca actually
   launched (`launch.effective` in the receipt).
8. **The status**: the plan to Active with its Branch line (a status-only
   commit to `develop`), then the issue on the board.

**Model and effort:** Opus, medium for writing and coding; Opus, high for
design or a hard bug; Sonnet, medium for an investigation that ends in a report
or for mechanical work; Opus, xhigh for a blind review, which wins over the
"high for reviewing" line of the owner's general tier table. A worker that
fails or escalates because the task was harder than it looked is retried one
tier up (`worker-start --retry-of`).

**Start each worker from a fresh base.** Workers start from `develop`, never
from master. `--base-branch develop` means the *local* `develop`, which lags
GitHub after a merge. Before every start, in the main checkout, on `develop`:
`git fetch origin && git merge --ff-only origin/develop`; then pass
`--base-branch develop` and check `git -C <worktree> log --oneline -1` against
the SHA in the spec.

**A fix round** (after the owner's FAIL, or when the PR's own worker is gone
before the blind review's findings): a worker of the same kind, Opus medium,
in the PR's existing worktree (`--worktree path:<worktree>`), or, if that is
gone, a new-child worktree from `origin/<PR branch>` after `git fetch origin`.
Its spec's Goal says "Fix round on PR #<n>: <the review's link>" and its
Context says "the review already ran: skip the review ask". One blind review
and one fix round per PR.

**What workers may not do**, which the shared rules tell them: open a PR into
master, merge, tag or release, start workers of their own, touch files outside the spec, edit
`wiki/log.md`, AGENTS.MD, a plan's Status or Branch lines, or the framework's
output templates unless the spec names them. They push and open PRs, and ask
the coordinator (`orchestration ask`) for anything else.

## Limits Across Workers

- **At most three workers at once, one of the three kept for a blind review**:
  at most two workers doing the work itself. When the review slot is full,
  queue the review rather than start a fourth worker. Check `free -g` before
  starting one. Why: on 2026-10-05 the 30 GB machine ran out of memory with
  ten workers and reviews running, and the kernel killed Orca's daemon and
  every worker with it; and a worker blocked on its review ask waits for a
  reviewer. After a crash: `journalctl -k | grep -i oom`, check each worktree
  for uncommitted work first, then retry the dead dispatches on the same
  worktree (`--worktree path:<p>`).
- **Releases are the owner's.** No worker tags, publishes, or runs
  `just release-*`; a version bump goes through a PR like any change.
- **Sequencing.** When one owner answer affects several queued workers, decide
  the order they start in and say it to the owner.

## Worktrees

- **Orca makes and removes them; `orca.yaml` runs a script at each end.** After
  a worktree is made, `tools/wiki-worktree.sh register` gives it its own wiki
  search: it registers the worktree with llm-wiki and builds a word-match
  index, in under a second, and forgets any registration of this repository
  whose folder is gone. Before removal, `forget` takes it away. It does nothing
  unless the main checkout is registered, and never fails.
  `setupAgentStartupPolicy: wait-for-setup` makes a worker wait for it. Why:
  without it `llm_wiki_search` fails in every worktree ("not registered"); a
  full meaning-based index per worktree would take minutes and more than 1 GB
  each.
- **Remove a worktree with `orca worktree rm --worktree path:<p> --run-hooks`.**
  Without `--run-hooks`, Orca skips the archive script; a worktree removed any
  other way is cleaned up at the next worktree's setup.
- **A worktree's search is its own branch's wiki, as it was made.** A worker
  that adds or moves pages re-indexes with `~/.llm_wiki/bin/llm-wiki index`
  in its worktree (the wiki kind says so).
- **Remove a worktree only once its work is merged, pushed, or discarded by the
  owner**; never with uncommitted or unpushed work, never `--force` without the
  owner's yes.

## Integration Branches

- **A feature built in several PRs, and of no use until all of them are in,
  gets one integration branch**, cut from `develop`, with one draft PR from it
  into `develop`. Each piece is its own issue, worked on a branch taken from
  it, with a draft PR into it, landed by the same steps as any PR. Why:
  `develop` stays usable, and can go to master at any time, while a feature is
  half built.
- **Start a piece** after `git fetch origin`, with
  `--base-branch origin/<integration branch>`, that head's SHA in the spec's
  Context, and the PR's base named in the Goal.
- **Keep it level with `develop`**: a tooling worker merges `develop` into it,
  never rebases, before each piece branches and before the integration PR's last
  gate run.
- **It merges once, whole**, with its own gate run and the owner's PASS. Its log
  entry names the pieces; a piece's merge gets none. Close a piece's issue by
  hand when it merges into the integration branch, since GitHub closes issues
  only on merges into its default branch, `develop`.

## While Workers Run

- **Wait on events, never a sleep loop**:
  `orca orchestration check --run <R> --wait --types worker_done,escalation,question --json`,
  run in the background. `--types` only sets what wakes the wait; the batch
  that comes back is always whole, heartbeats included. Once a batch is
  handled, acknowledge it on the next call (`check --ack <delivery_id> …`); an
  unacknowledged batch comes back again. A released worker's late duplicate
  report arrives with a lifecycle rejection: acknowledge it.
- **Answer questions** with `orca orchestration reply --id <msg_id> --body <text>`.
  A file outside the spec gets a yes or no; a design question goes to the
  owner.
- **The review ask.** A worker whose PR is open asks "ready for review: PR #<n>
  at <sha>" before its gate run (or, for a wiki PR, before it reports). Note
  the message id, acknowledge the batch, and start the blind review. Reply to
  that id once the review has posted, with its findings as the fix round.
- **Read each worker_done report**, then release the worker
  (`orca orchestration worker-release --dispatch <id>`). Retain a failed or
  stuck worker (`worker-retain`) until it has been looked at. Before a session
  ends, `worker-list --run <R> --terminal-state reclaimable --json` comes back
  empty.

## Landing A PR

`/operations-land` walks these steps.

- **One blind review first**, for every PR but a log PR and the PR from
  `develop` into master ("Develop And Master"). At the worker's review ask:

  ```bash
  sed "s/<PR>/<n>/g" .claude/skills/operations-start/base/blind-review.txt > <scratchpad>/review-<n>.txt
  orca orchestration worker-start --run <R> --spec "$(cat <scratchpad>/review-<n>.txt)" \
    --task-title "Blind review of PR #<n>" --agent claude --model opus --effort xhigh \
    --worktree new-child --base-branch develop --name review-<n> \
    --display-name "#<n> blind review" --json
  ```

  It gets no spec and no worker report. It posts one comment headed
  `## Feedback review — <commit>` whose first line says it is feedback, not
  the owner's verdict, never with PASS or FAIL in it, and changes nothing.
  Then reply to the PR worker's ask with the findings: P1s and P2s are fixed,
  P3s fixed there or filed as roadmap entries and issues. Release the reviewer
  and remove its worktree once its comment is up. Why: in RepForge's trial and
  on every riseon PR it found a real P1 or P2 before the owner saw it; doing it
  before the gates means the fix lands before the one gate run.
- **The gate check, before a PR leaves draft.** The worker's `just verify` log
  read raw (every step ran and passed, nothing skipped), the
  `## Full gate run — <sha>` comment names the current head, and CI passed on
  that head (`gh pr checks <n>`; which CI runs on a PR into `develop`, and
  which on the PR into master, is `wiki/plans/develop-and-master-ci.plan.md`'s).
  Why: a verify script once exited 0 with gates skipped, and the
  owner refused a PR whose gates had been skipped ("no excuses"). Wiki PRs run
  no local gates; CI still runs on them.
- **Ready**: `gh pr ready <n>`, then tell the owner it waits for them.
- **The verdict is the owner's.** A PR comment whose heading contains PASS or
  FAIL; CHANGES REQUESTED counts as FAIL; a heading with both is no pass. A
  PASS given in conversation is recorded on the PR before merging, as a
  comment headed `## PASS — <full sha>` saying "the owner's verdict on this
  head, given in conversation". Why: workers and the coordinator post under the
  owner's GitHub login, so the owner once took a blind review's comment for a
  verdict of their own. A failed verdict: `gh pr ready <n> --undo`, then a fix
  round.
- **Merge only with the owner's PASS on the PR's current head**, after reading
  the diff yourself against the issue and the plan:

  ```bash
  gh pr merge <n> --squash --match-head-commit <full sha> --delete-branch \
    --subject "<the PR title> (#<n>)"
  ```

  `--match-head-commit` needs the full SHA; `--subject`, because a one-commit
  PR would otherwise squash under its commit's title, not the PR's. One at a time; wait for the next
  PR's `mergeable` to leave UNKNOWN. Then check the issue closed: GitHub
  closes it on a merge into `develop`, its default branch. The PR from
  `develop` into master merges another way ("Develop And Master").
- **The log entry and the plan's status at merge.** Only the coordinator writes
  `wiki/log.md`: one entry per merge into `develop`, at the top of the log, in
  this wiki's format (`## [<date>] <operation> | <subject>`, a paragraph on
  what changed and why citing the PR, then `Pages affected: …`). The same log
  PR, into `develop`, marks the plan Completed ("The Board"), removes its
  Branch line, and updates its roadmap entry. A log PR may gather several
  merges; it gets no log entry of its own and no blind review, and the owner's
  PASS decides it. Then the issue to Completed on the board. The PR from
  `develop` into master gets its own entry ("Develop And Master"). Both as
  written here, decided by the owner on 2026-10-06.

  ```bash
  git fetch origin && git switch -c wiki-log-<n> origin/develop   # in the main checkout
  # add the entries at the top of wiki/log.md; mark the plans and roadmap entries
  git commit -am "Wiki: Log the merge of #<n>" && git push -u origin wiki-log-<n>
  gh pr create --base develop --title "Wiki: Log the merge of #<n>" --body "Logs #<n>." --draft
  git switch develop
  ```
- **Remove the worktree** of a merged PR ("Worktrees").

## Develop And Master

The rule is
`wiki/decisions/the-pull-request-is-the-review-surface.decision.md`, rule 6:
work PRs go into `develop`, and master takes `develop` only through a PR from
`develop`, with the full CI and the owner's PASS. Nothing else ever targets,
merges into or is pushed to master. What follows was decided by the owner on
2026-10-06.

- **When:** when the owner asks for it, before a release, or when a proof
  needs master. The coordinator opens the PR; no worker does.
- **How:**

  ```bash
  git fetch origin
  git push origin origin/develop:refs/heads/to-master-<date>   # develop at a fixed commit
  gh pr create --base master --head to-master-<date> --title "Develop: Bring master up to date" \
    --body "<the merges it carries, by name and number>"
  ```

  Why a branch at a fixed commit: `develop` keeps moving while the PR waits
  for the owner, and a moving head restarts the slow full CI and leaves the
  owner's verdict on an older commit.
- **Its check is the full CI**
  (`wiki/plans/develop-and-master-ci.plan.md`). It gets no blind review: each
  change it carries had one. It merges only once every job of the full CI
  passed on its head (`gh pr checks <n>`; there is no `## Full gate run`
  comment), and with the owner's PASS on that head, as for any PR ("Landing A
  PR").
- **The merge keeps `develop`'s history**: a merge commit, never a squash;
  `develop` is never deleted, only the fixed-commit branch:

  ```bash
  gh pr merge <n> --merge --match-head-commit <full sha> --delete-branch
  ```

  Why: a squash would put on master one commit `develop` lacks, so the next PR
  from `develop` would carry every earlier change again and conflict with it.
- **Its log entry** is written in the next log PR into `develop`, after the
  merge: one entry for the PR into master, naming the PRs it carried. Plans'
  statuses do not change ("The Board").

### Why These Steps

- **Where the log entry is written** (decided by the owner on 2026-10-06): one entry per merge into
  `develop`, in a log PR into `develop`, as "Landing A PR" says; and one entry
  for each PR from `develop` into master, in the next log PR into `develop`
  after it merges. Why: every change to the log goes through `develop` like
  any other, and master gets its log with the next PR from `develop`.
- **When `develop` goes to master, who opens the PR and how it merges**:
  the steps above, decided by the owner on 2026-10-06.
- **What follows Completed** is
  `work-in-flight-is-a-pushed-branch.decision.md`'s, "What Completed (master) Became".

## Landing A Comment

An investigation or a design settled with the owner ends in a comment on its
issue, not a PR. When its worker reports: read the comment, check its facts
against the links it cites, and tell the owner what it answers. Once the owner
agrees and every follow-up it names is a roadmap entry (and an issue), the
settled points go into the wiki through a wiki PR, the issue closes with a
pointer to the comment, and its worker and worktree are released and removed.

## What Would Change This

- poman drawing the board: "The Board" then names the command and drops the
  hand-mirroring.
- A rule changing: change it where it lives, once. A worker's rule lives in its
  base text beside `/operations-start`, with its reason in the worker briefs
  checklist; a coordinator's rule lives here; the board's in the
  work-is-recorded decision. The skills only point.
