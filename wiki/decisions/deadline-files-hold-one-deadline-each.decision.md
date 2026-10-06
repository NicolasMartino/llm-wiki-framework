# Deadline Files Hold One Deadline Each

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: poman's first file type, the deadline: where the files live, their
  fields and value formats, how the start date is found, and how the files
  land.
- Sources:
  - The owner's decisions, 2026-10-06: one file per deadline, its fields, the
    computed start date, and deadline files landing straight on master
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
  - `wiki/roadmaps/poman.roadmap.md`: PM3 (the type), PM4 (where the check
    runs), PM7 (the forecast)

## Decision

**Each deadline is one file, `wiki/deadlines/<slug>.deadline.md`, in the
repository it describes. poman checks its fields and computes its start
date.**

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

### The start date

- **poman computes it; nobody writes it**: the deadline, minus the
  duration, minus the time its blockers need, minus a buffer set by
  importance.
- **Durations count working days, Monday to Friday.** Public holidays
  come later, with absences (PM7).
- **The buffer's exact rule is not decided** (PM7).
- **Open: the rule as written takes the blockers' time off the wrong item.**
  Subtracting "the time its blockers need" moves the blocked item's start
  earlier, yet the blockers are the ones that must start earlier, and the rule
  gives a blocker no start date of its own unless it has a deadline. For
  example, X is due 2026-12-04 and takes 2 days, blocked by A (3 days, no
  deadline) and B (3 days, no deadline): the rule starts X six or three days
  early, depending on whether the blockers' times add, and says nothing about
  when A and B start. Recommended answer, for the owner in PM7: give each
  blocker a latest finish date, the earliest start date of the items it
  blocks, or its own deadline if that is earlier, so a blocker's time moves
  its own start date and not the blocked item's.

### How the files land

- **Deadline files land straight on master**: no pull request, no blind review,
  no log entry; Git history is their log. Every other change goes
  through a pull request.
- **Nothing runs `poman check` before such a file lands yet.** Where it runs
  (a pre-push hook, `poman push` refusing while the check fails) is open, and
  is PM4's.

## Why

- **A start date nobody computes by hand cannot be computed wrong by hand**:
  people write the inputs (deadline, duration, blockers, importance).
- **Whole days and three levels** are the simplest formats that let the start
  date be computed and checked.
- **Working days** match when the work can be done; holidays wait for
  absences, which change the same count.
- **No loop, no dangling path**: otherwise the start-date walk has no answer.
- **A deadline file is a record, not a change to the system**: a pull request,
  a review and a log entry for each would slow down keeping the record
  current.

## Alternatives Considered

- **All obligations in one YAML file** (the tool's original design). Replaced
  by one file per deadline.
- **A start date written by hand.** Not chosen by the owner.
- **"Blocked by" as tracker issue numbers.** Replaced by paths.
- **A track as a parent issue or a file of its own.** Not chosen: Track is a
  plain field.
- **A "Part of" field.** Not now, by the owner's answer.

## Consequences

- Until PM4 settles where the check runs, nothing stops a malformed deadline
  file from reaching master.
- **Open: landing straight on master conflicts with three standing rules**, in
  any repository that holds deadline files under `wiki/` (this one holds none
  until poman is used here):
  - `AGENTS.MD`, "How Work Runs": every change goes through a PR, with two
    named exceptions, neither of them deadline files;
  - `agent-owns-wiki.decision.md`: people do not edit `wiki/` directly, while
    a deadline file is written by a person through `poman new`;
  - `the-pull-request-is-the-review-surface.decision.md`: all work ends in a
    pull request.

  PM3 settles it with the owner, before any deadline file lands: those three
  pages name deadline files as an exception, or deadline files live outside
  `wiki/`.
- `Status` here is the deadline's own vocabulary, separate from the wiki's
  document statuses and from the board's columns
  (`typed-documents.decision.md`).
- Until PM7 settles the buffer, the start date cannot be computed in full.

## What Would Revisit This

- A deadline file pushed straight to master that breaks the board.
- Fields the owner finds missing once real deadlines are written.
- A need for half days, or a fourth importance level.
