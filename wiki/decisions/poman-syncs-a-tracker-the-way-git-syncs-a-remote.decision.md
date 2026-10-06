# poman Syncs A Tracker The Way Git Syncs A Remote

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: How poman shows a repository's work on a tracker (fetch, diff, push
  and the local mirror), what it manages there, which boards it draws, and
  what is GitHub's part (the first adapter) versus poman's own.
- Sources:
  - The owner's decisions, 2026-10-06: the repository as the only source of
    truth, fetch, diff and push with a local mirror, the scope of what poman
    manages, and poman drawing this repository's own board after fetch, diff
    and push
  - The owner, 2026-10-06: "gh is just a ui for poman so we should think this
    so as to be able to adapt it to other UIs if needed"
- Related:
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`: every
    board is a view of the repository (not restated here)
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`: the
    files poman shows first
  - `wiki/roadmaps/poman.roadmap.md`, PM6

## Decision

**poman treats a tracker the way Git treats a remote: it fetches the tracker's
state into a local mirror, computes what must change, and pushes exactly that.
poman's data and its fetch, diff and push say nothing about GitHub; GitHub is
the first adapter.**

### A tracker is a view, and only a view

- **The repository is the only source of truth; a tracker shows it** (for
  boards, `work-is-recorded-in-the-repository.decision.md`). An edit made
  on the tracker to something poman manages does not count: the next push
  overwrites it. Fetch reports it as drift, so it is seen before it is lost.
- **A tracked item is a summary of its file that links to it**; the detail is
  read in the file.
- **No daily snapshot file**: any day's forecast can be recomputed from the
  files' Git history.

### The tracker is an adapter (the owner: "gh is just a ui for poman")

- **poman's own model knows no tracker.** Items (one per file poman manages),
  the links between them ("Blocked by" and any other reference field a type
  defines), the board and its columns, the mirror and the changeset are
  defined in poman's terms.
- **An adapter maps that model onto one tracker** and does the reading and
  writing. Fetch, diff and push call the adapter; nothing else in poman talks
  to a tracker.
- **GitHub is the first adapter** (GitHub issues and a GitHub project). Another
  tracker or board is added as another adapter, without changing the model or
  the three commands.

### Fetch, diff, push

1. **`poman fetch`** reads the tracked items poman manages into a JSON mirror
   in a dot folder, and reports drift: any change made on the tracker since
   poman last wrote it.
2. **`poman diff`** compares the files with the mirror and writes the
   changeset to a file anyone can read: items to create, update, move or
   close, and links to add or remove.
3. **`poman push`** applies exactly that changeset, refuses while `poman check` fails
   (`deadline-files-hold-one-deadline-each.decision.md`, "How the files
   land"), and refuses if the tracker
   changed since the fetch.

- **The mirror is a local cache, never committed**, like `.git`; fetch can
  always rebuild it.
- **Each item poman manages names its file**, so fetch can find its items
  again with no other record.
- **Scope: only the board that shows the repository poman runs in**, its
  items and their links. Pull requests and comments stay outside it.

### Which boards poman draws

- **A repository's own work**: poman draws the board of the repository it runs
  in. Its first files are deadline files.
- **This repository's own board too** (the owner, 2026-10-06): poman draws it from this repository's roadmaps and plans, as a
  later deliverable that comes after fetch, diff and push. Until then the
  coordinator mirrors the plans onto the board by hand, per
  `work-is-recorded-in-the-repository.decision.md`, rule 5.

### The GitHub adapter

Settled:

- each GitHub issue poman manages carries a hidden marker naming its file;
- a "Blocked by" path becomes GitHub's own "Blocked by" link.

Proposed, not decided:

- GitHub's REST API through octocrab, and the board's GraphQL API through
  graphql_client, typed against GitHub's schema;
- the token from `GITHUB_TOKEN`, else the one `gh` holds;
- tests never touch the real GitHub: a fake behind a trait, and a mock server
  for HTTP.

## Why

- **The owner asked for it to work "a little like git"**: fetch, diff and push
  are steps a person already knows, and each can be run and read alone.
- **A changeset someone can read before it runs** makes a push predictable:
  nothing reaches the tracker that the diff did not show.
- **Refusing a push over a stale fetch** keeps a push from overwriting a change
  nobody has seen. Since tracker edits do not count, the point is to see them,
  not to keep them.
- **Keeping the tracker behind an adapter** follows the owner's principle:
  GitHub is one interface among others, so another one can be added later
  without reworking poman's data or its commands.
- **A narrow scope keeps the mirror small and the rules clear.**

## Alternatives Considered

- **Calling the tracker directly on every run, with no mirror.** Not chosen:
  there is no changeset to read first and no way to see drift.
- **Committing the mirror.** Not chosen: it is the tracker's state, not the
  repository's record, and it can be rebuilt.
- **A GitHub-shaped core** (poman's model written in GitHub's objects).
  Not chosen by the owner: "gh is just a ui for poman".

## Consequences

- poman's dot folder is ignored by Git; the change that adds the mirror adds
  the ignore line. Its name is PM6's.
- Open, for PM6:
  - an item whose marker is edited away, or whose file is renamed: the next
    diff would create a second item and leave the first behind;
  - a "Blocked by" link made by hand on the tracker between an item poman
    manages and one it does not: poman owns its items' links, so the next
    push would remove it.
- The mirror's and the changeset's formats, how the adapter logs in, and
  which board fields and views poman draws are PM6's to settle.

## What Would Revisit This

- Tracker content that poman must read back as truth (a field another person
  edits on the board).
- A need to manage something outside the board that shows the repository.
- A second adapter that the model cannot express without a change.
