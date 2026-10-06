# Plan: The Shared Page Reader And File Types

- Document Class: Plan
- Status: Draft
- Date: 2026-10-06
- Category: poman development
- Scope: Carry out PM2 of the poman roadmap: the page reader moved from
  llm-wiki's search into `llm-wiki-core` and rewritten to the strictest gates,
  both search callers reading through it, llm-wiki's document types defined
  there with their suffix, folder, fields and statuses, `llm-wiki init`
  scaffolding from those definitions, and proof that search, init and the
  guidelines see no change.
- Sources:
  - `wiki/roadmaps/poman.roadmap.md`, PM2, and issue #44
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`: the shared
    crate, the strictest gates, and "Consequences" on the parser's forms and
    on the guidelines agreeing with the code
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: poman's
    fields in the bullet block, no front matter, every message naming the
    file and the line
  - `wiki/decisions/typed-documents.decision.md`: each type has its own status
    vocabulary
  - `src/search/metadata.rs`, `src/search/qmd_rs.rs`, `src/search/semantic.rs`,
    `src/init/compose.rs`, `src/init/packs.rs`, `crates/llm-wiki-core/`,
    `tools/strict-gates.sh` and `templates/base/project_guidelines.md` at
    `92187d3`, the state "Where It Stands" summarises
  - riseon issue #11: the founding tasks to be written as deadline files once
    PM3 lands
- Related:
  - `wiki/plans/poman-workspace-and-strict-gates.plan.md`: PM1, which made the
    crate and the gates this plan builds on
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`: the
    first poman type, PM3's, which this plan makes room for
  - `wiki/plans/operations-setup-in-llm-wiki.plan.md` (PR #41, P19 of
    `wiki/roadmaps/framework-v1.roadmap.md`): it also changes what init
    writes; "Where This Meets The Operations Setup" below

## What This Proves

llm-wiki and poman can read pages one way. The reader that llm-wiki's search
uses today moves into the shared crate, held to the strictest gates, and
search reads every page exactly as before: same title, same fields, same
results. llm-wiki's document types are defined once, in code, and `llm-wiki
init` writes the same project from them as it does today. A test fails the
day the definitions and `templates/base/project_guidelines.md` stop saying
the same thing. And the reader already gives poman what PM3 needs: its fields
from the bullet block, each with its line.

## Where It Stands (2026-10-06, `92187d3`)

Whoever does the work rechecks each point against the commit they start from.

- **The reader is llm-wiki's own:** `parse_wiki_metadata` in
  `src/search/metadata.rs` returns a title and a map of fields sorted by key
  (`WikiMetadata`), with `document_class()`, `status()` and `field()`. It
  reads, in this order, each later block overriding a key an earlier one
  set:
  - a `---` block at the very top, when a second `---` closes it;
  - a block of `Key: Value` lines just before the first `# ` title, blank
    lines between allowed;
  - a block just after the title, or from the top when there is no title.

  A line is a field when it is not indented and holds a colon, with or
  without a leading `- ` and with `**` around the key stripped; a line
  indented by two spaces or a tab continues the field above it, joined with
  one space. Seven unit tests sit inline in the module.
- **One quirk:** any unindented line with a colon right under the title is a
  field, so a prose line such as "a ratio of 4:1" becomes a field named after
  the text before the colon. The unit test for prose only checks that
  `Document Class` and `Status` stay empty.
- **Two callers:**
  - `src/search/qmd_rs.rs` reads the title when it indexes a page, and the
    document class and status when it filters lexical results;
  - `src/search/semantic.rs` reads the title, document class, status,
    `Category`, `Scope` and `Sources` into each chunk.
  Both pass the page after `mask_search_ignored_spans`. No other code calls
  the parser.
- **The shared crate:** `crates/llm-wiki-core` holds one `types` module that
  splits a filename into index, slug and type (`WikiFilename`), with its unit
  tests in `src/types/tests.rs`, integration tests in `tests/filenames.rs`
  and property tests in `tests/properties.rs`. Its only dependency is
  proptest, for its tests, and `llm-wiki-rs` does not depend on it yet. The splitter refuses a type with a
  hyphen or a second dot.
- **The gates:** `tools/strict-gates.sh` (`just strict`) runs every gate over
  `llm-wiki-core` and `poman` only. Coverage and mutants run each crate's own
  tests, so llm-wiki's search tests count for neither.
- **The types are written down three times, none from the others:**
  - `templates/base/project_guidelines.md`, for people and agents: the seven
    core types (spec, decision, proposal, roadmap, plan, checklist, reference)
    and, with the ML pack, experiment and eval; their filename patterns, which
    of them take an index; the folder tree; the six metadata fields and four
    optional ones; and the "Status Vocabulary" table, which lists all nine;
  - `src/init/compose.rs`: the core folders init creates and the sections of
    the `wiki/index.md` it writes, as literal lists;
  - `src/init/packs.rs`: each pack's types (name, suffix, folder) and some
    status lists, rendered into the guidelines as "Pack Document Types" and
    "Pack Status Vocabulary". The ML pack defines experiment and eval again,
    with the eval statuses again. Some pack suffixes have a hyphen or a
    second dot (`model-card.md`, `transform.spec.md`), and one pack status
    list has no type of its own (`Incidents`).
- **The proof that init is unchanged exists already:** `tests/init.rs` with
  nine snapshots (`tests/snapshots/init__*.snap`), checked by `just
  snapshots`.

## Target

### The reader

- **One reader in `llm-wiki-core`**, in a module of its own beside `types`.
  It reads a page's text, never a file or a repository, so it serves this
  repository, any project llm-wiki inits, and riseon alike.
- **One pass, everything it found:** the title with its line, and each field
  with its key, value, line, and the form it came in (front matter, before
  the title, after the title; bullet, bare or bold), in page order.
- **Two views of that one pass:**
  - **the wiki's view**, which gives search exactly what
    `parse_wiki_metadata` gives today: the same title and the same fields,
    with the same precedence and the same quirk ("Open For The Owner",
    choice 3);
  - **the bullet-block view**, poman's: only the `- Key: Value` block right
    after the title, continuation lines included, each field with its line.
    Fields found in any other form are reported with their line, not
    dropped, so PM3 can refuse front matter by name
    (`poman-reads-only-its-own-file-types.decision.md`, rule 6). What poman
    says about them is PM3's.
- **Strictest from its first line:** no indexing or slicing, every public
  item documented, doc tests among its tests, unit tests in their own file, integration and property tests, 100 % of lines covered and
  no surviving mutant, all from `llm-wiki-core`'s own tests. Today's seven
  unit tests move with it and keep their inputs and expectations.
- **No new dependency for the shared crate**, unless the work finds one
  necessary; then the deny and unused-dependency gates cover it.

### Both callers read through it

- `llm-wiki-rs` depends on `llm-wiki-core` by path, with its version, as
  crates.io asks of a crate that may be published.
- `src/search/qmd_rs.rs` and `src/search/semantic.rs` call the wiki's view,
  and `src/search/metadata.rs` goes away. Whoever does the work rechecks for
  any other caller on the commit they start from.
- llm-wiki's own code stays outside the strict gates (PM8): only the shared
  crate's code is held to them.

### The type definitions

- **llm-wiki's document types are defined once, in `llm-wiki-core`'s `types`
  module**, as data in code (`poman-reads-only-its-own-file-types.decision.md`,
  rule 1): for each type, its name (`Plan`), its plural label (`Plans`), its
  suffix, its folder, whether its filenames take an index, its fields and its
  statuses.
- **Which types: the nine the guidelines name** ("Open For The Owner",
  choice 1): spec, decision, proposal, roadmap, plan, checklist, reference,
  and experiment and eval, marked as the ML pack's.
- **Fields:** the six every page carries (Document Class, Status, Date,
  Category, Scope, Sources) and the four optional ones the guidelines name
  (Owner, Supersedes or Superseded By, Related, Promotion Target).
- **Statuses:** each type's list from the guidelines' "Status Vocabulary",
  in its order. This repository also writes `Completed (develop)` and a
  `Branch` line on plans (`work-in-flight-is-a-pushed-branch.decision.md`);
  those are this repository's convention, not the guidelines', and nothing in
  PM2 reads them. PM9 settles them when poman reads this repository's plans.
- **A shape poman's types fit too:** a type is a name, suffix, folder,
  fields (mandatory or optional) and statuses of its own. llm-wiki's types
  and poman's are kept as separate sets, so poman checking only its own is
  the default (`poman-reads-only-its-own-file-types.decision.md`, rule 3).
  PM2 defines no poman type; the deadline type and its value formats are
  PM3's.
- **The filename splitter and the definitions agree:** a type's suffix is one
  the splitter accepts, which a test checks for every definition.

### init scaffolds from the definitions

- The core folders init creates and the sections of the `wiki/index.md` it
  writes come from the definitions instead of their literal lists, the
  folders that are not a type (`raw`, `wiki/archive`) staying init's own.
  init keeps the order it writes the index sections in today, which is not
  the guidelines' order.
- The packs' types stay in `src/init/packs.rs` (choice 1).
- **What init writes does not change, byte for byte:** the nine snapshots
  pass untouched.

### Proving nothing changed for search

- **Today's answers recorded before the move:** in the first phase, before
  any code moves, a test in `llm-wiki-rs` reads every Markdown page under
  `wiki/` and `tests/fixtures/`, plus the forms of today's unit tests, with
  `parse_wiki_metadata` and records the title and fields as a snapshot. The
  same test, reading through the shared crate's wiki view, must then match
  that snapshot untouched, and it stays as a regression test.
- **The two side by side while both exist:** a property test generates pages
  from the forms above (front matter, blocks before and after the title,
  bullets, bare and bold keys, continuations, prose with colons, no title)
  and checks the old reader and the new view agree; it is run and recorded
  in the PR, then removed with the old reader.
- **llm-wiki's tests pass unchanged:** every search, MCP and init test, and
  `just verify`, pass with no test edited, except `metadata.rs`'s seven unit
  tests, which move to the shared crate as they are.

### Proving the definitions match the guidelines

- **A test in `llm-wiki-rs`** renders `project_guidelines.md` through init,
  once without and once with the ML pack, reads from it the core and ML type
  tables, the filename patterns, the folder tree, the metadata fields and the
  "Status Vocabulary" table, and compares them with the definitions both
  ways: every type the guidelines name is defined, every definition is named,
  and suffix, folder, index, fields and statuses are the same.
- **A deliberate slip fails it:** a status changed in a definition, and a
  type left out, each shown failing the test and reverted, recorded in the
  PR.
- **The guidelines are read, not rewritten** ("Open For The Owner",
  choice 2): `templates/` stays as it is.
- The shared crate never reads `templates/`: it is published on its own and
  runs in other repositories.

### What PM3 needs from PM2

PM3, the deadline type, is to be proved on the riseon repository, whose
founding tasks wait to be written as deadline files (riseon issue #11). PM2
makes ready:

- the bullet-block view, giving each field with its line, so `poman check`
  can name the file and the line;
- the fields found outside the bullet block, with their lines, so `poman
  check` can refuse front matter and bold keys by name;
- the type shape, in which PM3 adds the deadline type with its own statuses
  (`Todo`, `Doing`, `Waiting`, `Done`) and its value formats, and the
  filename splitter, which already accepts `deadline`;
- a reader that reads text only, so poman run in riseon reads riseon's pages
  as it would this repository's.

And it keeps llm-wiki indifferent to poman's files: a deadline page under
`wiki/deadlines/` is read by search's wiki view like any page without
`Document Class` today. Getting a poman binary into riseon, from a build or a
release, belongs to PM3's proof, not to PM2.

### Where This Meets The Operations Setup

P19 (`wiki/plans/operations-setup-in-llm-wiki.plan.md`, PR #41) also changes
what init writes: it puts everything init renders in `AGENTS.md`, `CLAUDE.md`
and `project_guidelines.md` inside a managed block, and adds a pack. Where the
two meet, for whoever lands second to recheck:

- **the nine init snapshots:** PM2 proves init unchanged against the
  snapshots on its own starting commit; if P19 lands first, those are P19's;
- **`src/init/compose.rs` and `src/init/packs.rs`:** both change them; a new
  pack's types go where the packs' types are then kept;
- **the guidelines check:** it reads the rendered `project_guidelines.md`,
  which P19 wraps in markers; the test reads inside them.

This plan does none of P19's work.

## Phases

1. **Today's answers recorded:** the snapshot test over every page, run with
   `parse_wiki_metadata`, committed before any code moves.
2. **The reader in the shared crate:** the one pass and its two views, with
   their unit, integration, doc and property tests; `just strict` passes for
   the shared crate.
3. **The callers moved:** `llm-wiki-rs` takes the shared crate; the property
   test comparing old and new run and recorded; both callers switched; the
   snapshot of phase 1 matches untouched; the old reader and the comparison
   test removed.
4. **The definitions:** the nine types in the `types` module, strictest like
   the rest; the guidelines check and its deliberate slips.
5. **init from the definitions:** the core folders and index sections taken
   from them; the nine snapshots pass untouched.

## Done When

- **The gates:** `just strict` reports every gate run and passed for both
  strict crates, the reader and the definitions included, 0 skipped and 0
  failed, locally and in the fast check.
- **Search unchanged:** the phase 1 snapshot matches with no change, the
  comparison property test passed while both readers existed, and every
  search and MCP test passes with no test edited.
- **init unchanged:** `tests/init.rs` and the nine snapshots pass with no
  snapshot changed.
- **The guidelines agree:** the guidelines check passes, and each deliberate
  slip made it fail.
- **llm-wiki unchanged:** `just verify` passes with nothing skipped.
- **One reader:** `src/search/metadata.rs` is gone and nothing else in
  llm-wiki parses a page's metadata block.

### Evidence Recorded

In the PR: `just strict`'s summary line and output; the phase 1 snapshot's
commit and the commit where it still matched; the comparison property test's
run; each slip's failing output; `just verify` and `just snapshots`. In this
plan, once it lands: "Where It Stands" brought up to date.

### Wiki Pages To Update When Done

Whoever does the work rechecks this list:

- this plan, "Where It Stands";
- `wiki/decisions/poman-lives-in-this-workspace.decision.md`,
  "Consequences", which says how the reader serves both forms is PM2's to
  settle and that the guidelines and the code must agree;
- any page that names `src/search/metadata.rs` as where pages are read
  today: on `92187d3` no spec does, and the completed plans that name it
  (`wiki/plans/qmd-rs-search-backend.plan.md` among them) are history and
  stay as they are.

### What May Be Touched

A first list, to be rechecked by whoever does the work: `crates/llm-wiki-core/`
(a new reader module, the `types` module, their tests, README), the root
`Cargo.toml` and `Cargo.lock` (the dependency), `src/search/` (the two callers,
the module list, `metadata.rs` removed), `src/init/compose.rs`, a new test file
under `tests/` with its snapshot, and `tools/strict-gates.sh` only if a gate
needs a new path left out of coverage.

### What Closes This Plan

The owner's PASS on the PR that meets "Done When", merged into `develop`.

## Open For The Owner

Each choice has a recommendation; the Target above follows it until the owner
answers.

1. **Which types PM2 defines: the nine the guidelines name (recommended).**
   The packs' types (API spec, design, runbook, threat model and the rest)
   stay in `src/init/packs.rs` for a later entry: some of their suffixes have
   a hyphen or a second dot, which PM1's filename splitter refuses, and their
   status lists do not match their types one to one. The alternative is all
   of them now, with the splitter widened and the pack lists made whole.
2. **How the definitions and the guidelines are kept the same: a test that
   compares them (recommended).** It is what the roadmap's proof asks, and
   leaves `templates/` as it is. The alternative is rendering the guidelines'
   type tables from the definitions, so they cannot differ, which changes
   `templates/base/project_guidelines.md` into a template fed by code.
3. **Today's quirk kept: a prose line with a colon right under the title is
   read as a field (recommended).** Search only reads six fields, so the
   quirk shows nowhere today, and keeping it lets "nothing changed" be
   proved exactly. Fixing it later is a change of its own with its own
   proof. The alternative is fixing it in PM2, and proving instead that only
   such pages' fields change.

## Out Of Scope

- poman checking the wiki's types (a later choice; PM9 reads this
  repository's plans).
- The deadline type, `poman new` and `poman check` (PM3), and every plan of
  PM3 and later.
- llm-wiki's other modules under the strict gates (PM8).
- The roadmap entries' own fields (Status, Promise, Depends On, Execution
  Plan): they are sections of a roadmap, not pages, and PM9 decides how poman
  reads them.
- P19's managed block and its pack.
- Changing `templates/` (choice 2).
