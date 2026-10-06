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
- Related:
  - `wiki/decisions/poman-reads-only-its-own-file-types.decision.md`: what the
    shared file types are for
  - `wiki/decisions/llm-wiki-binary-distribution.decision.md`: the install
    and release poman joins
  - `wiki/roadmaps/poman.roadmap.md`: PM1 (the workspace and the gates), PM2
    (the shared reader and types), PM8 (the ratchet)

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
  - the release archive ships both binaries side by side;
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
  managed binary, so nothing new is asked of the person installing.
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

- The workspace's `members` list is empty today; PM1 adds the two crates.
- llm-wiki has no `[lints]` table and no pinned toolchain file today, so the
  ratchet starts from nothing switched to strict.
- The parser moves crate, so llm-wiki's search reads pages through the shared
  crate (today `src/search/qmd_rs.rs` and `src/search/semantic.rs` import
  it), and the parser is rewritten to the strictest gates as it moves. The parser also accepts
  `---` front matter and bold `**Key:**` lines today, while poman's own types
  use the bullet block only; how the shared reader serves both is PM2's to
  settle. Whoever does the work rechecks every caller.
- The shared crate's name is not decided; PM1 picks it.
- `llm-wiki-binary-distribution.decision.md` describes one binary; this page
  amends it with a second, shipped and installed as "Where `llm-wiki install`
  gets poman" sets out.
- An `llm-wiki` binary run with no poman beside it has nothing to copy; what
  `llm-wiki install` says then is PM1's to settle.
- `templates/base/project_guidelines.md` keeps describing the wiki's types for
  people and agents; once the types are also defined in code, the two must say
  the same, which PM2 makes sure of.

## What Would Revisit This

- poman needing a release cadence of its own, apart from llm-wiki's.
- A module of llm-wiki that cannot reach the strictest gates without a rewrite
  the owner does not want.
