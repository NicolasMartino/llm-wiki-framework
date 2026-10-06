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
  - `raw/handover/2026-10-06-riseon-handover-poman-and-coordination.md`,
    sections 1, 2 ("Where poman lives"), 3 (XLV) and 4: the owner's answers
    XL, XLI and XLII in riseon's coordinating session, 2026-10-06
  - The owner, 2026-10-06, confirming the handover's XLV: llm-wiki's existing
    code reaches the strictest gates by a ratchet
  - riseon, `wiki/decisions/riseon-is-written-in-rust.decision.md` and
    `wiki/runbooks/rust-gates.runbook.md`, at `5f30a39`: the gates as riseon
    built them
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
  `llm-wiki` in this workspace (XLI). The name was free on crates.io on
  2026-10-06.
- **A shared library crate holds the page reader and the file types** (XLI;
  the owner: "they can share types for files"):
  - the field-block parser, today llm-wiki's `parse_wiki_metadata`
    (`src/search/metadata.rs`), moves there;
  - each file type (its suffix, folder, fields and statuses) is defined once,
    there: llm-wiki's document types and poman's own;
  - `llm-wiki init` scaffolds from those definitions, and poman checks
    against them.
- **poman checks only its own types**, deadline first, even though the
  definitions are shared; checking the wiki's types (decisions, plans) is a
  later choice (XLII). The rules are in
  `poman-reads-only-its-own-file-types.decision.md`.
- **`llm-wiki install` installs poman** into `~/.llm_wiki/bin/` and records it
  in its manifest, and one release ships both binaries (the owner: "then we
  could have llm wiki install poman").

### The strictest gates

- **Every crate of this project gets the strictest gates, llm-wiki's existing
  code included** (XL: "no all project crates should be strictest").
- **The gates are the ones riseon built and proved** (handover, section 4;
  riseon, `wiki/runbooks/rust-gates.runbook.md`, at `5f30a39`):
  - an exact pinned toolchain, edition 2024, and `rust-version` equal to it;
  - formatting checked; clippy with warnings as errors, the `pedantic`,
    `nursery` and `cargo` groups, and every strict lint (`unwrap_used`,
    `expect_used`, `panic`, `indexing_slicing`, `print_stdout` and the rest
    the handover lists) set to `forbid`, not `deny`, so a local `#[expect]`
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
- **llm-wiki's existing code gets there by a ratchet** (the owner, 2026-10-06,
  the handover's XLV):
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
- **A tool that rewrites boards has to be trusted** (riseon's reason for the
  strictest level), and the owner extended that to every crate here.
- **`forbid` instead of `deny`**: in riseon's blind review, one
  `#[expect(lint, reason)]` line lifted a `deny` rule past every gate; with
  `forbid`, that line stops the build.
- **The ratchet does not hold poman back.** Bringing all of llm-wiki up first
  would take weeks (about 450 KB of Rust, 80 % coverage today, no `forbid`
  lints and no mutation testing, per the handover), while poman can start
  strict and stay strict.

## Alternatives Considered

- **poman in its own repository** (riseon, where it started). Not chosen by
  the owner: the tool moved here so it can share the file types (XXXVIII,
  XLI).
- **All of llm-wiki brought to the strictest gates before poman starts.** Not
  chosen, for the reason above.

## Consequences

- The workspace's `members` list is empty today; PM1 adds the two crates.
- llm-wiki has no `[lints]` table and no pinned toolchain file today, so the
  ratchet starts from nothing switched to strict.
- The parser moves crate, so llm-wiki's search reads pages through the shared
  crate (today `src/search/qmd_rs.rs` imports it). The parser also accepts
  `---` front matter and bold `**Key:**` lines today, while poman's own types
  use the bullet block only; how the shared reader serves both is PM2's to
  settle. Whoever does the work rechecks every caller.
- The shared crate's name is not decided; PM1 picks it.
- `templates/base/project_guidelines.md` keeps describing the wiki's types for
  people and agents; once the types are also defined in code, the two must say
  the same, which PM2 makes sure of.

## What Would Revisit This

- poman needing a release cadence of its own, apart from llm-wiki's.
- A module of llm-wiki that cannot reach the strictest gates without a rewrite
  the owner does not want.
