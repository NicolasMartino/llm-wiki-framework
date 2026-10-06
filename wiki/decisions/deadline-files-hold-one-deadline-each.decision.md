# Deadline Files Hold One Deadline Each

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: poman's first file type, the deadline: where the files live, their
  fields and value formats, how its `should-start` date is found, how the
  files land, and where `poman check` runs on them.
- Sources:
  - The owner's decisions, 2026-10-06: one file per deadline, its fields, the
    computed start date (since named `should-start`), and deadline files
    landing straight on master
  - The owner's decisions, 2026-10-06, answering this page's open points: the
    landing branch is the repository's main branch, master by default and
    settable, each repository being independent; the computed date is named
    `should-start` and works backwards through the blocked items; `poman
    check` runs in three places
  - The owner's answers, 2026-10-06: a duration is whole days, written
    `10 days`; importance is `low`, `medium` or `high`; durations count working
    days, Monday to Friday, with public holidays later, with absences; no
    "Part of" field for now
- Related:
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: the
    rules every poman type follows (fields in the bullet block, reference
    fields as paths, `poman new` and `poman check`)
  - `wiki/decisions/poman-syncs-a-tracker-the-way-git-syncs-a-remote.decision.md`:
    how deadline files become tracked items
  - `wiki/decisions/the-pull-request-is-the-review-surface.decision.md`: the
    review every other change gets
  - `wiki/roadmaps/poman.roadmap.md`: PM3 (the type), PM4 (putting the check
    in its three places), PM7 (the forecast)

## Decision

**Each deadline is one file, `wiki/deadlines/<slug>.deadline.md`, in the
repository it describes. poman checks its fields and computes its
`should-start` date.**

### Fields

- **Mandatory:**
  - `Status`: `Todo`, `Doing`, `Waiting` or `Done`;
  - `Deadline`: a date, or `none` written out for an item with no deadline;
  - `Duration`: a whole number of days, written `10 days`;
  - `Importance`: `low`, `medium` or `high`;
  - `Blocked by`: the deadline files this one waits on, or `none`.
- **Optional:**
  - `Track`: a plain field naming the stream of work; a track has no file of
    its own;
  - `Who`: who does it.
- **No "Part of" field for now**: a final deadline blocked by its steps
  already reads as a tree.

For example:

```markdown
# Renew The Domain

- Status: Todo
- Deadline: 2026-12-01
- Duration: 2 days
- Importance: medium
- Blocked by: wiki/deadlines/choose-the-registrar.deadline.md
```

The example shows poman's fields only; whether the wiki's own metadata fields
join them is open (`poman-reads-only-its-own-file-types.decision.md`,
"Consequences").

### should-start

- **poman computes it; nobody writes it.** `should-start` is the latest day
  a task can start and still let every task after it finish in time (the
  owner's decisions, 2026-10-06: the name, and the rule below).
- **The rule works backwards from the deadlines:**
  - a task's latest finish is its own deadline or the earliest `should-start`
    of the tasks it blocks, whichever comes first;
  - its `should-start` is that latest finish minus its duration and minus its
    buffer.

  So a blocker's time moves the blocker's own `should-start`, never the
  blocked task's, and a blocker with no deadline of its own still gets one.
  For example, X has a deadline and takes 2 days, and is blocked by A, which
  takes 3 days and has no deadline: X's `should-start` is 2 working days and
  X's buffer before its deadline; A's latest finish is X's `should-start`, and
  A's `should-start` is 3 working days and A's buffer before that.
- **Durations count working days, Monday to Friday.** Public holidays
  come later, with absences (PM7).
- **The buffer's exact rule is not decided**: it is set by importance, and
  PM7 settles it.

### How the files land

- **Deadline files land straight on the repository's main branch**: no pull
  request, no blind review, no log entry; Git history is their log (the
  owner's decisions, 2026-10-06).
  - The branch is master by default and can be set to another, such as
    `develop` in this repository. Each repository is independent and sets its
    own.
  - In a repository that keeps deadline files, that landing is a named
    exception to "every change goes through a pull request". The repository
    names it in its own rules before its first deadline file lands; here, that
    means `AGENTS.MD`, "How Work Runs", `agent-owns-wiki.decision.md` and
    `the-pull-request-is-the-review-surface.decision.md`. This repository
    keeps no deadline files yet.
  - Every other change goes through a pull request.
- **`poman check` runs in three places** (the owner's decisions,
  2026-10-06):
  - a pre-push hook, which each person turns on (opt-in);
  - `poman push`, which refuses to update the tracker while the check fails;
  - CI, on every push to the branch deadline files land on.

  PM4 puts the three in place.

## Why

- **A `should-start` nobody computes by hand cannot be computed wrong by
  hand**: people write the inputs (deadline, duration, blockers,
  importance).
- **Working backwards puts each task's time on that task**: a blocker starts
  earlier because of its own duration, and the task it blocks keeps its own.
- **Whole days and three levels** are the simplest formats that let
  `should-start` be computed and checked.
- **Working days** match when the work can be done; holidays wait for
  absences, which change the same count.
- **No loop, no dangling path**: otherwise the `should-start` walk has no answer.
- **A deadline file is a record, not a change to the system**: a pull request,
  a review and a log entry for each would slow down keeping the record
  current.
- **Each repository sets its landing branch** because their branches differ:
  most will never have a `develop` branch, while this one works on it.
- **Three places for the check** because none covers every push alone: the
  hook runs only where it is turned on and is skipped by
  `git push --no-verify`, `poman push` keeps a broken file off the tracker but
  not off the branch, and CI catches it on the branch, after it lands.

## Alternatives Considered

- **All obligations in one YAML file** (the tool's original design). Replaced
  by one file per deadline.
- **A `should-start` written by hand.** Not chosen by the owner.
- **Taking the blockers' time off the blocked task's start**, as this page
  first wrote it. Replaced by the backwards rule: it started the blocked task
  early and gave a blocker with no deadline no date.
- **Deadline files outside `wiki/`, or through a pull request**, to avoid an
  exception. Not chosen: the owner made the landing a named exception.
- **"Blocked by" as tracker issue numbers.** Replaced by paths.
- **A track as a parent issue or a file of its own.** Not chosen: Track is a
  plain field.
- **A "Part of" field.** Not now, by the owner's answer.

## Consequences

- Until PM4 puts the check in place, nothing stops a malformed deadline file
  from reaching the landing branch.
- **The landing exception must be named before a repository's first
  deadline file lands**, in each rule it departs from ("How the files land").
  Whoever adds the first deadline file to a repository rechecks which of its
  rules say every change goes through a pull request.
- `Status` here is the deadline's own vocabulary, separate from the wiki's
  document statuses and from the board's columns
  (`typed-documents.decision.md`).
- Until PM7 settles the buffer, `should-start` cannot be computed in full.

## What Would Revisit This

- A deadline file pushed straight to the landing branch that breaks the board.
- Fields the owner finds missing once real deadlines are written.
- A need for half days, or a fourth importance level.
