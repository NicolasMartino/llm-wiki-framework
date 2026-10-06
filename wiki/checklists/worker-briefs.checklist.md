# Worker Briefs

- Document Class: Checklist
- Status: Active
- Date: 2026-10-06
- Category: Development process
- Scope: The briefs a worker is started with, one per kind of task: the
  names, each kind's model, effort and base text, and the reason for every rule
  a worker gets.
- Sources:
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`
    (sections 5 and 6)
  - The worker briefs runbooks of RepForge and riseon (2026-10-01 to
    2026-10-06), cut down to the kinds this repository starts with
- Related:
  - `wiki/checklists/operation-manager.checklist.md`
  - `.claude/skills/operations-start/`

## What This Is For

A worker given a spec written from scratch repeats old mistakes. So the text a
worker receives is fixed, per kind, in `.claude/skills/operations-start/`:

- `spec-template.md`: the short spec the coordinator writes per task;
- `base/kinds/<kind>.txt`: one base text per kind, pasted under the spec
  unchanged, never edited for one task; its last line names the shared parts
  that follow it;
- `base/shared/`: the shared rules and the add-ons;
- `base/blind-review.txt`: the whole spec of a PR's blind review.

This page keeps the reasons. A brief says what the worker must not get wrong,
never how to write the code. A list of files or impacts in a spec is a first
guess, and the spec says so.

## Names

- **An issue in a track:** `<Track> · <Area>: <what it does>`.
- **A track issue:** `<Track> track: <what it delivers>`.
- **Any other issue:** `<Area>: <what it does>`.
- **The PR, and so its squash commit:** the issue's pattern, imperative.
- **The Orca worktree:** `<Track> · #<issue> <kind>`, or `#<issue> <kind>`
  outside a track, linked to its issue with `orca worktree set --issue`.

## Model And Effort

- An investigation that ends in a report, or mechanical work: Sonnet, medium.
- Writing or coding: Opus, medium.
- Design decisions, a hard bug an investigation could not pin down: Opus, high.
- A blind review of a PR: Opus, xhigh.

A worker that fails or escalates because the task was harder than it looked is
retried one tier up. Check `launch.effective` in the start receipt.

## The Shared Rules, And Why

Every base text is followed by `base/shared/rules.txt`:

- **Read AGENTS.MD first, and check the start commit**: a new worktree can
  start from a stale master, and the worker is the last one who can see it.
- **Never edit `wiki/log.md`, AGENTS.MD, or a plan's Status and Branch lines**:
  the log is the rule most often broken when several workers run, a worker must
  not rewrite the instructions it is reviewed against, and the plans' statuses
  are the coordinator's record (`work-is-recorded-in-the-repository`).
- **Never change `templates/` unless the spec names it**: it is the framework's
  output, written into every project that runs `llm-wiki init`.
- **Stay in the spec's files; ask before any other**: only the coordinator sees
  every worker's files.
- **Start no workers, subagents or background agents**: fan-out is the
  coordinator's decision, and the machine has run out of memory before.
- **Titles follow the issue's pattern; push; never force-push; never merge,
  tag or release; no AI, model, agent or tool mentions in commits or on
  GitHub**: a merge needs the owner's PASS on the current head, releases are
  the owner's, and the record stays free of tool credits.
- **No heading containing PASS, FAIL or CHANGES REQUESTED**, and private files
  in the scratchpad: a verdict is recognised by those words, and a file left
  in the worktree is committed by mistake or blocks its removal.

## The Add-Ons, And Why

- **The full gates** (`base/shared/full-gates.txt`), for every kind that
  changes code or tooling:
  - `just verify` with zero skips, and CI passing on the PR's head, with the PR
    draft until the coordinator has read both: a green exit status once hid
    skipped gates;
  - the review ask before the gate run: the blind review's fix lands before
    the one gate run;
  - the gate comment's heading is `## Full gate run — <sha>`, so it cannot read
    as a verdict.
- **A wiki-only PR** (`base/shared/wiki-only.txt`): no local gates; it opens as
  draft like every PR, and asks for its review before reporting.
- **A fix round** skips the review ask: its spec says the review already ran.
- **No file changes** (`base/shared/no-file-changes.txt`): for a worker whose
  output is a comment.
- **Talking with the owner** (`base/shared/talking-with-the-owner.txt`): for a
  worker the owner talks to directly in its terminal.

## The Kinds

Add a kind when a task fits none: write its spec from the template and the
shared rules, and once it is done add a base text, a line in
`/operations-start` and a section here.

### A Code Change

A change to the crates and their tests.

- **Base text:** `base/kinds/code.txt`, then the shared rules and the full
  gates.
- **Model:** Opus, medium. Opus, high to design a new boundary.
- **Why its lines:** the spec or decision that describes the behaviour is
  updated in the same PR, so the wiki never describes code that does not
  exist; a change to `templates/` changes what every project gets, so its
  snapshot changes are listed and justified in the PR; a pain point met with
  `llm-wiki` is reported, since this repository is the product's own proof.

### Tooling

A change to `tools/`, `.github/`, the justfile, or how the framework is built
and checked. Not releases.

- **Base text:** `base/kinds/tooling.txt`, then the shared rules and the full
  gates.
- **Model:** Opus, medium.
- **Why its lines:** the page that tells people to run the command is updated
  in the same PR; a gate that skips is a setup to fix, not a caveat.

### Settling A Design With The Owner (No Code)

A worker the owner talks to directly, to settle a design before any code.

- **Base text:** `base/kinds/design-with-owner.txt`, then the owner add-on, the
  shared rules and the no-file-changes add-on.
- **Model:** Opus, high. No PR.
- **Why:** in RepForge a design went through five rounds of code before the
  owner said of the last one "I don't understand why we need a number here";
  the PR closed unmerged. A design the owner has not approved gets no coding
  worker.

### A Wiki PR

A change to wiki pages only, plans included.

- **Base text:** `base/kinds/wiki.txt`, then the shared rules and the wiki-only
  add-on.
- **Model:** Opus, medium.
- **Why its lines:** every claim a PR moves or keeps is rechecked against the
  code today, since a stale claim carried over once outlived its own review;
  the worker re-indexes its worktree after moving pages, because the worktree's
  word-match index is built when the worktree is made.

A **log PR** is the coordinator's own, written at merge.

### An Investigation

A question to answer, not a change to make. The output is one comment on the
issue.

- **Base text:** `base/kinds/investigation.txt`, then the shared rules and the
  no-file-changes add-on.
- **Model:** Sonnet, medium; a follow-up that could not pin the cause goes to
  Opus, high.
- **Why its lines:** numbers are measured, never estimated, and the result is
  posted whatever it is, with what was ruled out.

## A Blind Review

Every PR's one review before the owner's
(`wiki/decisions/the-pull-request-is-the-review-surface.decision.md`).

- **Base text:** `base/blind-review.txt` is the whole spec, `<PR>` replaced; no
  shared part follows it.
- **Model:** Opus, xhigh, on a new-child worktree from master; the start
  command is in the operation manager checklist, "Landing A PR".
- **Why no spec and no worker report:** it judges the change cold, as the owner
  does, so it does not inherit the spec's blind spots.
- **Why it never writes PASS or FAIL, and says first that it is feedback:** a
  heading with either word is a verdict, the verdict is the owner's, and every
  comment is posted under the owner's login.
- **Why it checks out detached:** the PR's branch is checked out in its
  worker's worktree, and git refuses a second checkout of it.
