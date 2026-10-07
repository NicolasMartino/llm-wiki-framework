# Plan: The Deadline Type

- Document Class: Plan
- Status: Draft
- Date: 2026-10-07
- Category: poman development
- Scope: Carry out PM3 of the poman roadmap: the deadline type defined in
  `llm-wiki-core` with its value formats, `poman new deadline` writing a
  deadline file, `poman check` holding every deadline file to the type with
  each message naming the file and the line, where a repository sets its
  landing branch, the answers to PM3's open points, and the proof on fixtures
  and on the riseon repository's founding tasks.
- Sources:
  - `wiki/roadmaps/poman.roadmap.md`, PM3, and issue #54
  - `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`: the
    type, its fields and value formats, `should-start`, how the files land
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: `poman
    new`, `poman check`, near-miss warnings, reference fields, and the open
    point on the wiki's metadata fields and the index
  - `wiki/plans/poman-shared-page-reader-and-types.plan.md`, "What PM3 needs
    from PM2"
  - `crates/poman/src/lib.rs`, `crates/poman/Cargo.toml`,
    `crates/llm-wiki-core/src/page.rs`, `crates/llm-wiki-core/src/types.rs`,
    `crates/llm-wiki-core/src/types/llm_wiki.rs`,
    `crates/llm-wiki-core/tests/filenames.rs`, `src/search/qmd_rs.rs`,
    `src/mcp/mod.rs` and the root `Cargo.toml` at `301cb0c`, the state this
    plan was written from, kept in git history
  - The owner, 2026-10-07, on the poman track (#19): PM3 is proved on the
    riseon repository as well as on fixtures
  - riseon issue #11, "Company: Write the founding tasks as deadline files"
- Related:
  - `wiki/plans/poman-shared-page-reader-and-types.plan.md`: PM2, the reader
    and the type shape this plan builds on
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`: the shared
    crate and the strictest gates
  - `wiki/decisions/poman-tree-and-list-show-the-files.decision.md`: PM5,
    which shows the files this plan makes
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`

## What This Proves

A person, or an agent, in any repository can write a deadline with `poman new
deadline` and get a file that `poman check` accepts. `poman check` fails on
each broken field and reference and on each field written in a form poman does
not read, warns on near misses, ignores every other Markdown file, and names
the file and the line in every message. The type is defined once, in the
shared crate, so the code that writes a file and the code that checks it read
the same definition. And it works on a real repository: riseon's founding
tasks are written with `poman new deadline`, `poman check` accepts them, and
they land on riseon's master, which closes riseon issue #11.

## Where It Stands (2026-10-07, at `301cb0c`)

- **poman does nothing yet:** `crates/poman/src/lib.rs` builds a clap command
  with no subcommand; `run` prints the help or the version. The crate depends
  on clap only, not yet on `llm-wiki-core`.
- **The reader is ready** (PM2): `Page::bullet_block` gives the `- Key:
  Value` fields just after the title, each with its line; `elsewhere()` gives
  every field found in another block (front matter, before the title, a page
  with no title) or form (bare, bold, `* `), with its line;
  `byte_order_mark()` says whether the page starts with one.
- **The type shape has no value formats:** `DocumentType` holds a name,
  plural, suffix, folder, whether it is indexed, its fields and its statuses;
  `FieldDefinition` holds a key and whether it is required. llm-wiki's nine
  types are the `types::llm_wiki` set; poman has no type yet.
- **The filename splitter accepts `deadline`** (`tests/filenames.rs` in the
  shared crate). It reads leading digits and a hyphen as an index for every
  type, so `2026-taxes.deadline.md` splits as index `2026` and slug `taxes`.
- **llm-wiki's search reads every Markdown file under `wiki/`**
  (`collect_markdown` in `src/search/qmd_rs.rs`), so a deadline page is
  searchable today, read by the wiki view as a page with a title and a
  `Status` and no `Document Class`.
- **The orphan check is the agent's lint, not code**: the guidelines'
  "Lint", item 3, and the lint prompt in `src/mcp/mod.rs`. No llm-wiki
  command fails on a page missing from `wiki/index.md` or missing its
  metadata fields.
- **`toml` and `chrono` are already workspace dependencies**, which
  llm-wiki uses; the strict crates take neither.
- **No repository keeps deadline files yet.** riseon's issue #11 waits on
  PM3; riseon's main branch is master.

## Target

### The deadline type, defined once

- **In `llm-wiki-core`'s `types` module, as a set of poman's own** beside
  llm-wiki's (`poman-reads-only-its-own-file-types.decision.md`, rules 1 and
  10): name `Deadline`, suffix `deadline.md`, folder `wiki/deadlines`, not
  indexed, statuses `Todo`, `Doing`, `Waiting`, `Done`.
- **Its fields, in the order `poman new` writes them:** mandatory `Status`,
  `Deadline`, `Duration`, `Importance`, `Blocked by`; optional `Track`, `Who`
  (`deadline-files-hold-one-deadline-each.decision.md`, "Fields").
- **Each field's value format joins the shape**, so the format is defined
  once too. llm-wiki's fields take a free-text format and nothing about them
  changes. How the formats sit in `FieldDefinition` is the work's choice.
- **A deadline file takes no index:** the whole name before `.deadline.md`
  is its slug, so `2026-taxes.deadline.md` is the slug `2026-taxes`. Whether
  the splitter learns that a type is not indexed or poman joins the two
  parts back is the work's choice; a test proves that filename.

### The value formats

Every value is written exactly: no other case, nothing after it.

- **Status:** `Todo`, `Doing`, `Waiting` or `Done`.
- **Deadline:** a date written `YYYY-MM-DD` that exists on the calendar
  (`2026-02-30` fails), or `none`.
- **Duration:** a whole number of working days, at least one, written
  `10 days`; `1 day` or `1 days` is the owner's choice 5.
- **Importance:** `low`, `medium` or `high`.
- **Blocked by:** `none`, or paths from the repository root separated by
  commas, with or without a space after each comma. Each path is written as
  the file's path from the root, with no leading `./` or `/` and no `..`.
- **Track, Who:** free text on one line, not empty.

### poman check

- **Where it looks:** every file under the repository's `wiki/` folder. The
  repository root is the nearest folder upward holding `.git`, a folder or,
  in a worktree, a file. Files outside `wiki/` are not read: the type lives
  under it, and walking the whole tree would read build output. A repository
  with no `wiki/` has nothing to check, and `poman check` says so and passes.
  Outside a Git repository it refuses.
- **It reads only files whose suffix is a known type** and ignores every
  other file, the wiki's own pages included
  (`poman-reads-only-its-own-file-types.decision.md`, rule 3).
- **It fails on**, each as `path:line: error: …`:
  - a deadline file outside `wiki/deadlines/`, or a name the splitter
    refuses;
  - a page with no title, a page starting with a byte-order mark (the
    owner's choice 4), a mandatory field missing, a field written twice, or
    a key in the bullet block that is neither the type's nor a near miss of
    one;
  - a field written anywhere but the bullet block: front matter, before the
    title, bare, bold, or a `* ` list item, named by its form. The message
    names the line, not only the key, since `* **Status:** Todo` reads the
    key `**Status` (PM2's note);
  - a value not in its format, naming the format expected;
  - a reference that is broken: a path that does not exist, is not a
    deadline file, is the page itself, or is listed twice;
  - a loop of `Blocked by` paths, reported once, naming each file of the loop
    and its `Blocked by` line.
- **It warns on near misses**, as `path:line: warning: …`, without failing:
  - a filename whose type is one or two letters off a known one, anywhere
    under `wiki/` (`x.dealine.md`, `x.deadlines.md`);
  - a Markdown file in `wiki/deadlines/` without the `.deadline.md` suffix,
    which would otherwise be skipped silently;
  - a key in the bullet block one or two letters off a field's key
    (`Dedline`, `Blocked By`). The field it was meant to be is then missing,
    which fails on its own.

  One or two letters means at most two single-letter insertions, deletions
  or changes, a change of case counting as one.
- **Every message names the file and the line**
  (`poman-reads-only-its-own-file-types.decision.md`, rule 5). A finding
  about the file as a whole, its name, folder or missing title, names line 1;
  a missing field names the title's line.
- **Output:** the findings sorted by path, then line, then one summary line:
  files checked, errors, warnings. It exits with success when there is no
  error, warnings or not, and with a failure code of its own otherwise, kept
  apart from the code for output it could not write.

### poman new deadline

- **`poman new deadline "<title>"` with a flag for each field** (`--status`,
  `--deadline`, `--duration`, `--importance`, `--blocked-by`, `--track`,
  `--who`) and `--slug`, writes `wiki/deadlines/<slug>.deadline.md` from the
  repository root: the title, a blank line, the bullet block in the type's
  order with the optional fields only when given, and nothing below it. The
  content below the block is the person's.
- **A mandatory field left out is asked for on a terminal**
  (`poman-reads-only-its-own-file-types.decision.md`, rule 2); for `Status`,
  pressing Enter takes `Todo`. Off a terminal it asks nothing: it refuses and
  names every flag missing.
- **Each value is checked as `poman check` checks it, before anything is
  written:** a bad answer is asked again with the reason; a bad flag is
  refused with the reason and no file written. Each `Blocked by` path must
  already exist and be a deadline file.
- **The slug comes from the title:** lowercase ASCII letters and digits
  kept, every other run of characters made one hyphen, trimmed at both ends.
  A title with letters outside ASCII is not guessed at: poman asks for the
  slug on a terminal and refuses without `--slug` off one.
- **It never overwrites a file** and creates `wiki/deadlines/` when absent.
  It commits and pushes nothing; its last line names the file written and the
  branch it lands on.
- **Another type name** is refused, listing the known types (one so far),
  with a near miss suggested.

### Where a repository sets its landing branch

The owner's choice 3. Recommended: a `poman.toml` file at the repository
root, committed, with one key, `landing-branch = "develop"`; with no file, or
no key, the branch is master
(`deadline-files-hold-one-deadline-each.decision.md`, "How the files land").

- **Read by `poman new` now**, for its last line, and by PM4's hook and CI
  later, which run on pushes to that branch.
- **Checked by `poman check`:** a key poman does not know, or a value that is
  not a branch name, fails with its line.
- **This repository adds no `poman.toml` in PM3**: it keeps no deadline
  files. Whoever adds its first one sets `develop` there.

### PM3's open points, answered

- **A deadline file carries poman's fields only**, not the wiki's metadata
  block (the owner's choice 1). Both blocks would need a `Status`, with two
  meanings on one key, and `Document Class`, `Category`, `Scope`, `Sources`
  and `Date` say nothing about a task that Git history and the title do not.
- **Deadline files are not listed one by one in `wiki/index.md`, and the
  orphan check passes over `wiki/deadlines/`** (the owner's choice 2). The
  index gets one line pointing to the folder. A line per file could not stay
  true: the files land with no pull request, while the index changes through
  one.
- **Each repository names both exemptions in its own rules**, beside the
  landing exception, before its first deadline file lands, as the deadline
  decision's "Consequences" already asks for the landing. PM3 changes neither
  `templates/` nor what `llm-wiki init` writes.

### What does not change

- llm-wiki's code: search reads deadline pages as it does today, and init
  creates no `wiki/deadlines/`.
- `templates/`, and the nine init snapshots.

### Proving it

- **On fixtures:** a valid tree of deadline files sharing blockers, with
  every optional field, a `poman.toml`, wiki pages and other Markdown that
  must be ignored; and one broken file per failure and per warning above,
  each with the exact message expected (file, line, text). `poman check`
  passes the valid tree with no finding and gives each broken file its
  message.
- **On this repository's wiki:** `poman check` run here finds no deadline
  file and gives no warning on its pages, so the near-miss rule does not fire
  on legitimate files.
- **`poman new`:** a file written from flags matches its expected text byte
  for byte and `poman check` accepts it; answers given through the prompt
  give the same file; off a terminal a missing flag is refused; an existing
  file is never overwritten. `run` cannot open a terminal in a test, so
  input and whether it is a terminal reach it as arguments; how is the
  work's choice.
- **Property tests:** any deadline `poman new` writes from valid values is
  accepted; any one field made invalid fails, naming that field's line.
- **The gates:** `just strict` runs every gate over both strict crates, the
  new type and both commands included.

### The riseon proof

The owner, 2026-10-07: PM3 is proved on riseon as well as on fixtures. riseon
is private and this repository public, so nothing here names a riseon task,
file or date.

1. **When:** once PM3's PR has had its blind review and fix round (the
   owner's choice 6), so the owner sees the proof before passing it.
2. **How poman gets there:** built locally from PM3's PR head in this
   repository (`cargo build --release -p poman`); no release is made.
3. **riseon names the exceptions first:** through riseon's own pull request,
   its rules name the landing exception and this plan's two exemptions
   before its first deadline file lands.
4. **The tasks:** a worker in riseon writes the founding tasks with `poman
   new deadline`, one file per task, blockers by path.
5. **The check:** `poman check` in riseon reports no error, and any warning
   is explained.
6. **The landing:** the files are committed and pushed straight to riseon's
   master, with no `poman.toml`, which proves the default branch; this closes
   riseon issue #11.

Recorded in PM3's PR: the poman commit used, how many files, `poman check`'s
summary line, the riseon commit on master, and that #11 closed. A gap found
in riseon (a value the tasks need, a message that misled) comes back to
PM3's PR as a fix, or becomes a roadmap entry if the owner says so.

## Phases

1. **The type:** the deadline type and the value formats in `llm-wiki-core`,
   with their tests; `just strict` passes.
2. **`poman check`:** the walk, the findings and their messages, the exit
   codes, the fixtures; run on this repository's wiki.
3. **`poman new deadline` and the landing branch:** the command, its prompt
   and refusals, `poman.toml`; the property tests.
4. **The pages and the review:** the pages under "Wiki Pages To Update When
   Done", the PR's evidence, the blind review and its fix round.
5. **The riseon proof**, recorded in the PR.

## Done When

- **The gates:** `just strict` reports every gate run and passed for both
  strict crates, 0 skipped and 0 failed, locally and in the fast check.
- **`poman check`:** the valid fixture passes with no finding; each broken
  fixture gives its exact message and the exit code expected; this
  repository's wiki gives no finding.
- **`poman new deadline`:** its files match their expected text and pass
  `poman check`; each refusal is shown.
- **llm-wiki unchanged:** `just verify` passes with no test or snapshot
  changed.
- **riseon:** the founding tasks are on riseon's master, `poman check`
  accepts them, and riseon issue #11 is closed.

### Evidence Recorded

In the PR: `just strict`'s summary line and output; `poman check`'s output on
the fixtures and on this repository; `poman new`'s files and refusals; `just
verify`; the riseon proof as "The riseon proof" lists it. In this plan, once
it lands: "Where It Stands" brought up to date.

### Wiki Pages To Update When Done

Whoever does the work rechecks this list:

- this plan, "Where It Stands";
- `wiki/decisions/deadline-files-hold-one-deadline-each.decision.md`: the
  example's note that the wiki's metadata fields are open, and where a
  repository sets its landing branch;
- `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`,
  "Consequences": the metadata fields, the index, and how close counts as a
  near miss;
- `wiki/roadmaps/poman.roadmap.md`, PM3: the two points marked open.

### What The Work Touches

The two strict crates, their tests and the lock file; llm-wiki's own code,
`templates/` and init do not change. Whoever does the work rechecks this.

### What Closes This Plan

The owner's PASS on the PR that meets "Done When", the riseon proof included,
merged into `develop`.

## Open For The Owner

Each with the plan's recommendation first; the Target above follows it.

1. **The wiki's metadata fields on a deadline file: none.** poman's fields
   only, as the deadline decision's example shows. Not chosen: the six
   fields as well, which puts two `Status` lines with two meanings on one
   page and four fields to fill on every task.
2. **The index: one line for the folder, and the orphan check passes over
   it.** Not chosen: a line per file, which would have to land with no pull
   request too, or go stale.
3. **The landing branch: a committed `poman.toml` at the root.** Every clone,
   hook and CI run reads the same value, and `toml` is already in the
   workspace. Not chosen: Git's own config (`git config poman.landingBranch`),
   which each clone and CI would have to set; the remote's default branch,
   which need not be the landing branch and is not always known to a clone.
4. **A byte-order mark: refused.** `poman new` never writes one, and
   llm-wiki's search would not read the title of a page that has one. Not
   chosen: stripping it before reading, which leaves the page broken for
   search.
5. **A duration of one day: `1 day`.** Every other number takes `days`, and
   `1 days` fails with the form expected. Not chosen: `days` always, which
   reads wrong; both, which gives two ways to write one value.
6. **When the riseon proof runs: after the blind review's fix round, before
   the owner's PASS.** The owner sees it before passing; a format change the
   owner then asks for means rewriting riseon's files with `poman new` or by
   hand and rechecking them. Not chosen: after the merge, which leaves PM3
   merged with its proof outstanding.

## Out Of Scope

- `should-start`, the importance buffer and the forecast (PM7).
- Showing the files: `poman tree`, `poman list` (PM5).
- The pre-push hook, CI on the landing branch (PM4), and `poman push`
  refusing (PM6, proved by PM4).
- poman checking the wiki's own types (PM9).
- A release of poman: the riseon proof uses a local build.
- Any file in the riseon repository now: this plan says how its proof runs.
- `templates/` and what `llm-wiki init` writes.
- Deadline files in this repository.
