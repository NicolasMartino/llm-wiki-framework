# poman tree And poman list Show The Files

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: How poman shows its files on a terminal: `poman tree`, `poman list`
  and the filters they share.
- Sources:
  - The owner's decisions, 2026-10-06: `poman tree` and `poman list` as
    proposed to the owner, with the owner's addition of `--type`
- Related:
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: the
    reference fields the tree draws
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`
  - `wiki/roadmaps/poman.roadmap.md`, PM5: when they are built

## Decision

**`poman tree` draws the reference graph; `poman list` gives a flat list. Both
take the same filters.**

- **`poman tree`** draws the graph from the final deadlines down to what they
  wait on.
  - A file under two parents is marked `(*)` after its first appearance, as
    `cargo tree` does.
  - `--invert` turns the tree upside down.
  - It draws every reference field a type defines, not only "Blocked by".
- **`poman list`** gives a flat list, sorted by start date once the forecast
  exists.
- **Shared filters:**
  - `--type`, one type or several comma-separated (the owner's addition); an
    unknown type is an error with a near-miss suggestion; a link to a hidden
    file shows as a stub, such as `… 1 hidden`;
  - status (`Done` hidden by default), importance, track, who, deadline
    before, and `--ready`;
  - `--json` on both.
- **`poman check` gets no type filter**: it always checks every known type.

## Why

- **The reference fields are a graph, and a tree is how people read one**;
  marking repeats like `cargo tree` keeps a shared blocker from being drawn
  twice in full.
- **A flat list sorted by start date** answers "what do I start next".
- **One set of filters** for both commands keeps them easy to learn.
- **`check` checks everything**, so a filter cannot hide a broken file.

## Consequences

- The sort by start date waits on the forecast (PM7); until then `poman list`
  sorts by another key, which PM5 picks.
- What `--ready` selects exactly is not written down yet; PM5 writes it
  down.

## What Would Revisit This

- A second type with reference fields whose graph does not read well as a
  tree.
