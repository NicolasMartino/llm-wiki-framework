# poman Reads Only Its Own File Types

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: Which files poman reads and checks, how its file types are defined
  and written, where their fields sit, and how reference fields work.
- Sources:
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`,
    section 2 ("File types and the deadline type"): the owner's answers VII,
    XI to XV in riseon's coordinating session, 2026-10-06
  - riseon, `wiki/decisions/riseon-reads-only-its-own-file-types.decision.md`
    and `wiki/decisions/deadline-files-hold-the-companys-deadlines.decision.md`,
    at `5f30a39`; they say "riseon" where they now mean poman
- Related:
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`: the types are
    defined once, in the shared crate
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`: the
    first type
  - `wiki/decisions/typed-documents.decision.md`: each document type has its
    own status vocabulary, poman's types included
  - `wiki/roadmaps/poman.roadmap.md`, PM3

## Decision

**poman's file types are defined in code, poman writes them, and poman checks
only files of those types.**

1. **A type is a filename suffix the code knows**, such as `.deadline.md`,
   with its folder, fields and statuses. Types are defined in code, not in a
   configuration file.
2. **poman writes its files**: `poman new <type> "<title>" …` writes the page
   from flags and asks on a terminal for any mandatory field left out (the
   owner: "have the cli tool of this project help with filing the different
   file types to avoid drift").
3. **`poman check` reads only files whose suffix is a known type** and ignores
   every other Markdown file (XI, XLII).
4. **A near miss gets a warning, not a failure** (XV): a suffix or a field name
   one or two letters off a known one (`.dealine.md`) is reported, so a
   mistyped file is not silently skipped.
5. **Every message names the file and the line.**
6. **Fields sit in the page's bullet metadata block**, the form the wiki uses;
   no YAML front matter (XII). The content below the block is free and not
   checked (XIII: "the content does not have to be so policed").
7. **A reference field holds comma-separated paths from the repository root,
   or `none`** (the owner: "any references should be a comma separated
   relative path list from repo root"). `poman check` makes sure each path
   exists, is a deadline file (the only type so far), and is not the page
   itself, and that no loop forms, so the graph can always be rebuilt from the
   files alone.
8. **A type's statuses are its own vocabulary**, separate from the wiki's
   document statuses and from the board's columns.
9. **The wiki index stays llm-wiki's job**: poman does not generate or check
   `wiki/index.md`.
10. **Each new type is added in code, with its own decision.** The first is
    the deadline type.

## Why

- **Checking every Markdown file would police pages poman has no rules for.**
  Reading by suffix keeps poman's rules on poman's files.
- **A typo should not hide a file**: the near-miss warning catches it without
  failing on files that are meant to be ignored.
- **Types in code** are tested like the rest of poman, and defined once for
  both binaries.
- **Writing the file from the command** keeps people from drifting from the
  type by hand.
- **The fields are the part that must be right; the prose is not.**
- **Paths, not tracker numbers**: a path lives in the repository, the only
  source of truth; an issue number exists only on a tracker.
- **`none` written out** tells a forgotten field from a deliberate empty one.

## Alternatives Considered

- **Checking every Markdown file.** Rejected by the owner.
- **YAML front matter.** Not chosen: the bullet block is the wiki's form.
- **Types in a configuration file.** Not chosen: one more thing to validate,
  and two binaries would read it.
- **poman generating the wiki index.** Rejected by the owner: that is
  llm-wiki's job.

## Consequences

- Adding a type is a code change, reviewed like any other.
- How close counts as a near miss is a rule for PM3's code.
- Whether a poman file also carries the wiki's metadata fields (Document
  Class, Category, Scope, Sources), and whether it is listed in
  `wiki/index.md` or exempt from the wiki's orphan check, is not decided
  (riseon, `wiki/proposals/0001-what-riseon-still-has-to-design.proposal.md`,
  "A Deadline File's Wiki Metadata", at `5f30a39`); PM3 settles it.

## What Would Revisit This

- Types the owner wants to add without a code change.
- Near-miss warnings that fire so often on legitimate files that they are
  ignored.
