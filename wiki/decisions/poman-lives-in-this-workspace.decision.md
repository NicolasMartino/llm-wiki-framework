# poman Lives In This Workspace

- Document Class: Decision
- Status: Accepted
- Date: 2026-10-06
- Category: poman design
- Scope: Where poman, the project-management binary, lives; the library crate
  it shares with llm-wiki; how it is installed and released; the gates every
  crate of this project is held to, and how llm-wiki's existing code reaches
  them.
- Sources:
  - The owner's decisions, 2026-10-06: poman's place in this workspace, the
    shared crate, the install, the strictest gates for every crate, and the
    ratchet for llm-wiki's existing code
  - The owner's decisions, 2026-10-06, answering this page's open point: the
    release archive ships both binaries, `llm-wiki install` copies poman from
    beside itself, and people installing with `cargo install` also run
    `cargo install poman`
  - `Cargo.toml` and `justfile` at `41ae502`: the workspace and today's gates
  - The owner's answers of 2026-10-06 to the open choices of
    `wiki/plans/poman-workspace-and-strict-gates.plan.md`
  - PR #64 (2026-10-07): the dependency check covers dev-dependencies, with
    the `getrandom` exception, taken by the coordinator for the owner's
    verdict on that PR to confirm
- Related:
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: what the
    shared file types are for
  - `wiki/decisions/llm-wiki-binary-distribution.decision.md`: the install
    and release poman joins
  - `wiki/roadmaps/poman.roadmap.md`: PM1 (the workspace and the gates), PM2
    (the shared reader and types), PM8 (the ratchet)
  - `wiki/plans/poman-shared-page-reader-and-types.plan.md`: PM2, which
    settled how the reader serves both forms

## Decision

**poman is a second crate and binary in this repository's Cargo workspace. A
shared library crate holds the page reader and the file types. Every crate of
this project is held to the strictest gates.**

### Where poman lives

- **A crate and a binary named `poman`** (project manager), beside
  `llm-wiki` in this workspace. The name was free on crates.io on
  2026-10-06.
- **A shared library crate holds the page reader and the file types** (the
  owner: "they can share types for files"):
  - the field-block parser, today llm-wiki's `parse_wiki_metadata`
    (`src/search/metadata.rs`), moves there;
  - each file type (its suffix, folder, fields and statuses) is defined once,
    there: llm-wiki's document types and poman's own;
  - `llm-wiki init` scaffolds from those definitions, and poman checks
    against them.
- **poman checks only its own types**, deadline first, even though the
  definitions are shared; checking the wiki's types (decisions, plans) is a
  later choice. The rules are in
  `poman-reads-only-its-own-file-types.decision.md`.
- **`llm-wiki install` installs poman** into `~/.llm_wiki/bin/` and records it
  in its manifest, and one release ships both binaries (the owner: "then we
  could have llm wiki install poman").
- **Where `llm-wiki install` gets poman** (the owner's decisions,
  2026-10-06):
  - the release ships both binaries, each in an archive of its own per
    target (`llm-wiki-rs-<triple>`, `poman-<triple>`), because cargo-dist
    builds one archive per package (the owner, 2026-10-06, amending "the
    release archive ships both binaries side by side"); dist's shell
    installers put both in Cargo's bin folder, and a person unpacking by
    hand unpacks both into one folder;
  - `llm-wiki install` copies poman from beside itself, the folder its own
    binary runs from;
  - people installing with `cargo install llm-wiki-rs` also run
    `cargo install poman`, which puts poman beside llm-wiki in Cargo's bin
    folder.

### The strictest gates

- **Every crate of this project gets the strictest gates, llm-wiki's existing
  code included** (the owner: "no all project crates should be strictest").
- **The gates:**
  - an exact pinned toolchain, edition 2024, and `rust-version` equal to it;
  - formatting checked; clippy with warnings as errors, the `pedantic`,
    `nursery` and `cargo` groups, and every strict lint (`unwrap_used`,
    `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`,
    `indexing_slicing`, `string_slice`, `print_stdout`, `print_stderr`,
    `dbg_macro`, `exit`) set to `forbid`, not `deny`, so a local `#[expect]`
    cannot lift one;
  - `unsafe_code`, `missing_docs` and the rustdoc lints forbidden, and the
    docs built with warnings as errors;
  - unit, integration, doc and property tests, with unit tests in their own
    file, left out of coverage;
  - 100 % of library lines covered, and zero surviving mutants in the
    library;
  - dependencies checked (advisories, licences, bans with one version of each
    crate, sources, unused dependencies);
    - one dated exception (2026-10-07, PR #64): `getrandom` 0.3.4 is let
      through beside 0.4.2, because both come from inside proptest, a
      dev-dependency of both strict crates, so no change to this
      repository's dependencies removes the pair
      (`wiki/plans/dependency-gate-covers-dev-dependencies.plan.md`);
  - one gate script that runs every gate even after one fails, prints how many
    ran, passed, were skipped and failed, and fails loudly on a missing tool.
    A skip is never a pass.
- **llm-wiki's existing code gets there by a ratchet** (the owner,
  2026-10-06):
  - poman and the shared crate are strictest from their first commit;
  - llm-wiki's modules are brought up one at a time, as a track of their own,
    and each is switched to strict once it passes;
  - the gate script fails if a module already switched to strict slips back.

## Why

- **One workspace lets the two binaries share one definition of each file
  type.** A type defined twice drifts; defined once, `llm-wiki init` writes
  what poman checks.
- **One install and one release** keep poman where llm-wiki already puts its
  managed binary, so the person installing from the release is asked
  nothing new; one installing with `cargo install` runs one more command.
- **A tool that rewrites boards has to be trusted**, and the owner extended that to every crate here.
- **`forbid` instead of `deny`**: one `#[expect(lint, reason)]` line lifts a
  `deny` rule past every gate; with `forbid`, that line stops the build.
- **The ratchet does not hold poman back.** Bringing all of llm-wiki up first
  would take weeks (over 900 KB of Rust under `src/` at `41ae502`, a coverage
  gate of 80 % of lines, no `forbid` lints and no mutation testing), while poman can start
  strict and stay strict.

## Alternatives Considered

- **poman in a repository of its own**, as where it started. Not chosen by
  the owner: the tool moved here so it can share the file types.
- **All of llm-wiki brought to the strictest gates before poman starts.** Not
  chosen, for the reason above.

## Consequences

- PM1 added the two crates to the workspace, `crates/llm-wiki-core` and
  `crates/poman`, and `default-members` builds poman with llm-wiki. The
  strictest lint table is the workspace's (`[workspace.lints]`), which only the
  strict crates take; `tools/strict-gates.sh` (`just strict`) runs every gate
  over them, in the fast check and the full CI.
- `rust-toolchain.toml` pins the toolchain for the whole repository, but
  llm-wiki-rs does not take the lint table, so the ratchet starts from nothing
  of llm-wiki switched to strict.
- The parser moved crate in PM2, rewritten to the strictest gates:
  llm-wiki's search reads pages through `llm-wiki-core`'s `page` module
  (`src/search/qmd_rs.rs` and `src/search/semantic.rs` call it), and
  `src/search/metadata.rs` is gone. One pass finds the title and every field
  with its line, block and form. Its wiki view gives search what the old
  parser gave, `---` front matter and bold `**Key:**` lines included; its
  bullet-block view gives poman the `- Key: Value` block under the title and,
  apart, every field found in another block or form with its line, so poman
  can refuse those by name.
- The shared crate is `llm-wiki-core`, with a `types` module inside it (the
  owner, 2026-10-06); its first piece is splitting a wiki filename into its
  parts.
- `llm-wiki-binary-distribution.decision.md` describes one binary; this page
  amends it with a second, shipped and installed as "Where `llm-wiki install`
  gets poman" sets out.
- An `llm-wiki` binary run with no poman of its own version beside it has
  nothing to copy: `llm-wiki install` keeps a poman it recorded before, and
  with none recorded it refuses and says how to get one (the owner,
  2026-10-06: "there should always be a poman besides an llm wiki").
  `llm-wiki uninstall` removes both binaries ("uninstall uninstalls all").
- The release tool configured today (cargo-dist, `[workspace.metadata.dist]`
  in `Cargo.toml`) builds one archive per package and cannot put poman in
  llm-wiki's, as PM1 found; the owner chose two archives from the one release
  (2026-10-06), and `llm-wiki install` still finds poman beside itself once
  both are unpacked into one folder or installed by dist's shell installers.
- `templates/base/project_guidelines.md` keeps describing the wiki's types for
  people and agents. Since PM2 the nine are also defined in `llm-wiki-core`'s
  `types` module, `llm-wiki init` takes its core folders, index sections and
  the ML pack's experiment and eval rows from them, and a test renders the
  guidelines with and without the ML pack and fails when the two stop saying
  the same thing (`tests/guidelines_types.rs`). The packs' other types stay
  in `src/init/packs.rs` for now.

## What Would Revisit This

- poman needing a release cadence of its own, apart from llm-wiki's.
- A module of llm-wiki that cannot reach the strictest gates without a rewrite
  the owner does not want.
