# Plan: The Workspace And The Strict Gates

- Document Class: Plan
- Status: Active
- Branch: `NicolasMartino/pm1-build-26`
- Date: 2026-10-06
- Category: poman development
- Scope: Carry out PM1 of the poman roadmap: the shared library crate and the
  `poman` crate in this workspace, both held to the strictest gates from their
  first commit, with one gate script running every gate; `llm-wiki install`
  and `uninstall` handling poman; and one release build producing both
  binaries.
- Sources:
  - `wiki/roadmaps/poman.roadmap.md`, PM1, and issue #20
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`, "The strictest
    gates": the gate list this plan carries out
  - The owner's decision of 2026-10-06 on install channels: "The release
    archive ships both binaries, and llm-wiki install copies poman from beside
    itself; people installing with cargo install also run cargo install poman."
    Amended the same day, once cargo-dist 0.28 proved to build one archive per
    package: the release ships two archives per target, `llm-wiki-rs-<triple>`
    and `poman-<triple>` (the owner's decision of 2026-10-06, "Two archives")
  - `Cargo.toml`, `justfile`, `src/install.rs`, `src/uninstall.rs`,
    `src/manifest/schema.rs`, `src/paths.rs` and `src/instance.rs` at
    `dba2537`, the state before the work, which "Where It Stands" now summarises
  - crates.io, queried 2026-10-06: which crate names are free
- Related:
  - `wiki/decisions/llm-wiki-binary-distribution.decision.md`: the install and
    release poman joins
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`: the plan
    comes before the code
  - `wiki/plans/develop-and-master-ci.plan.md`: the fast check and the full CI
    the new gates sit beside

## What This Proves

poman and the shared crate start strict and can be checked strict with one
command: every gate of the decision runs and passes on them, a deliberate slip
fails the gate it breaks, and nothing is skipped. And poman reaches people the
way llm-wiki does: one release build ships both binaries, and `llm-wiki
install` puts poman beside the managed `llm-wiki` and records it.

## Where It Stands (2026-10-06, PR #28)

What PM1 built, on branch `NicolasMartino/pm1-build-26` from `85c64d0`. The
plan as written before the work found the state of `dba2537`: one package,
`members = []`, no pinned toolchain or lint table, a manifest of schema 2
with one binary, and `uninstall` keeping the binary unless
`--include-binary`.

- **The workspace:** `crates/llm-wiki-core` (a `types` module splitting a wiki
  filename) and `crates/poman` (`--version`, `--help`, an unknown command
  refused) are members; `default-members = [".", "crates/poman"]`.
  `rust-toolchain.toml` pins 1.99.0 for the whole repository, and both crates
  take `rust-version = "1.99.0"` and the workspace's lint table
  (`[workspace.lints]`), which llm-wiki-rs does not take yet (PM8).
- **The gates:** `tools/strict-gates.sh` (`just strict`) runs every gate of the
  decision over the two crates, in the fast check and the full CI; each
  deliberate slip of phase 2 failed its gate (PR #28). The dated nightly is
  named in `tools/udeps-nightly` (`nightly-2026-10-01`). cargo-deny 0.20.2
  leaves dev-dependencies out of the graph it checks, so the deny gate covers
  the crates' normal and build dependencies.
- **Install:** `llm-wiki install` copies the poman of its own version from
  beside its binary into the managed bin folder and records it in a schema 3
  manifest; with none usable it keeps a recorded poman, else refuses. The
  tests run `llm-wiki` from a folder of its own with a stand-in poman beside
  it. `llm-wiki uninstall` removes both binaries; `--include-binary` is gone.
- **The release:** unchanged dist configuration; one release makes
  `llm-wiki-rs-<triple>.tar.xz` and `poman-<triple>.tar.xz`, and the release
  E2E unpacks both into one folder.
- **crates.io:** `llm-wiki-rs`, `llm-wiki-core` and `poman` are not published;
  publishing is the owner's.

## Target

### The crates

- **Two new members of the root workspace**, each in its own folder under
  `crates/`: the shared library crate and the `poman` crate. The root package
  `llm-wiki-rs` stays where it is; `tools/release-e2e` stays outside.
- **The shared crate is `llm-wiki-core`**, with a `types` module inside it
  ("Open For The Owner", choice 1).
- **A bare cargo command builds both binaries:** the workspace's
  `default-members` names the root package and poman, so `cargo build` and
  `cargo test` at the root leave `target/debug/poman` beside
  `target/debug/llm-wiki`, and every justfile recipe or script that means one
  binary names it (`--bin llm-wiki`). Whoever does the work rechecks each
  cargo command in the justfile, the scripts under `tools/` and the
  workflows.
- **`poman`** is a library target holding all its code and a binary whose
  `main` only calls it, so the coverage and mutation gates see everything but
  the entry point. The binary runs: `poman --version` and `poman --help`
  answer, and an unknown command is refused with an error and a non-zero exit.
- **A first piece of real behaviour in each crate**, enough that every kind of
  test has something to test: for the shared crate, splitting a wiki filename
  (`[slug].type.md` or `[index]-[slug].type.md`) into its parts ("Open For
  The Owner", choice 5). Moving the parser stays PM2's.
- **Publishable:** both crates carry the metadata crates.io asks for (the
  clippy `cargo` group checks it). Publishing them is a release act, the
  owner's.

### The gates

Every gate of `poman-lives-in-this-workspace.decision.md`, "The strictest
gates", is one check in the gate script, run over the two new crates. How each
is run is a first sketch; whoever does the work rechecks each tool's flags.

- **Toolchain:**
  - an exact toolchain pinned in `rust-toolchain.toml` (the current stable
    when the work starts, with rustfmt, clippy and `llvm-tools-preview`: rustup
    treats an exact pin as a toolchain of its own, so components installed for
    `stable` do not serve it), edition 2024, and each new crate's
    `rust-version` equal to the pin;
  - a dated nightly for the unused-dependencies check, no older than the
    stable pin: cargo refuses to build a crate whose `rust-version` is above
    the active rustc, so today's `nightly-2026-05-01` (rustc 1.97) would stop
    `cargo udeps` on the new crates. The nightly is named in one place, which
    the gate script and CI's unused-dependencies job both read, and that job
    moves to it;
  - checked by comparing `rustc --version` and each crate's `rust-version` and
    edition (from `cargo metadata`) to the pin, and the nightly's rustc
    version to the pin (not older).
  - The pin applies to the whole repository, llm-wiki included: CI's
    toolchain steps must agree with it, and `just verify` must still pass on
    it.
- **Formatting:** `cargo fmt --check` on each crate.
- **Clippy:**
  - `cargo clippy` on each crate, all targets and features, warnings as
    errors, with each crate's lint table setting the `pedantic`, `nursery` and
    `cargo` groups;
  - `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
    `unreachable`, `indexing_slicing`, `string_slice`, `print_stdout`,
    `print_stderr`, `dbg_macro` and `exit` set to `forbid`;
  - checked twice: clippy passes, and the script reads each crate's lint table
    and fails if any of these lints is missing or below `forbid`, so loosening
    the table is caught even when the code still passes;
  - test code is held to the same `forbid` lints ("Open For The Owner",
    choice 6): tests return `Result` and use `?`. clippy.toml's
    `allow-*-in-tests` keys lift those lints in tests while the lint table
    still says `forbid`, so the script also reads clippy.toml and fails on any
    such key.
- **Unsafe code, docs and rustdoc:**
  - `unsafe_code` and `missing_docs` forbidden, and the rustdoc lints
    forbidden, in each crate's lint table (checked as above);
  - the docs built with `cargo doc --no-deps` on each crate with
    `RUSTDOCFLAGS="-D warnings"`.
- **Tests:**
  - unit, integration, doc and property tests (`proptest`, already a
    workspace dependency), each kind run as its own check with `cargo test`:
    `--lib` for unit tests, `--doc`, the property tests in one named file per
    crate (`tests/properties.rs`, as llm-wiki has) run with `--test
    properties`, and every other file under the crate's `tests/` run by name
    as the integration tests (`--tests` alone would also run the unit tests,
    and an empty kind would never show);
  - a kind that runs no test in a crate counts as skipped, never passed;
  - unit tests sit in their own file, never inline in the module they test:
    the script fails on a source file that holds its tests inline.
- **Coverage:** `cargo llvm-cov` over the two crates, at 100 % of library
  lines (`--fail-under-lines 100`), with the test files left out of the count.
- **Mutants:** `cargo mutants` over the two crates' library code, with no
  mutant missed; a mutant that times out counts as a failure.
- **Dependencies:**
  - `cargo deny check` (advisories, licences, bans with one version of each
    crate, sources) over the two new crates' dependency graph only: llm-wiki's
    own graph comes in with the ratchet (PM8);
  - unused dependencies with `cargo udeps` on the dated nightly above, over
    the two crates.
- **The gate script:**
  - a script under `tools/`, run by one justfile recipe (recommended: `just
    strict`);
  - runs every gate even after one fails, then prints how many ran, passed,
    were skipped and failed, and exits non-zero on any failure or skip;
  - a missing tool (cargo-llvm-cov, cargo-mutants, cargo-deny, cargo-udeps,
    the pinned nightly) fails loudly, naming the tool and how to install it.
    A skip is never a pass.
- **In CI:** the gate script runs in both the fast check on PRs into
  `develop` and the full CI ("Open For The Owner", choice 4).

### Install and uninstall

Per the owner's decision of 2026-10-06 on install channels:

- **The release** ships `poman` in an archive of its own beside `llm-wiki`'s
  ("The release", below).
- **`llm-wiki install` copies poman from beside itself:** from the folder of
  the running `llm-wiki`, into the managed bin folder (`~/.llm_wiki/bin/poman`;
  the test instance's own managed home for `llm-wiki-test`), and records it in
  the manifest. This covers the three channels the decision named:
  - the release's two archives unpacked into one folder, or dist's shell
    installers, which put both binaries in Cargo's bin folder;
  - `cargo install llm-wiki-rs` followed by `cargo install poman`, both in
    `~/.cargo/bin`;
  - a binary run from anywhere, such as `target/debug/llm-wiki` after a
    workspace build, which leaves `poman` beside it (see "A bare cargo
    command builds both binaries" above).
- **poman is handled like the managed `llm-wiki`:**
  - its manifest entry keeps its path, its version, the installed copy's hash
    and the source's hash, as the `llm-wiki` entry does, so a second install
    copies nothing on macOS too, where signing changes the copy;
  - copied again only when the source's hash changed, signed on macOS, and an
    unmanaged `poman` already in the bin folder refused without `--force`;
  - a manifest written before PM1 is still read.
- **The same version as llm-wiki:** poman takes the workspace's version, and
  install takes a poman beside itself only when `poman --version` names
  llm-wiki's own version. A poman of another version (an older `cargo install
  poman` left in `~/.cargo/bin`) is treated as no poman, naming both versions.
- **No recorded poman is ever orphaned:**
  - an install that finds no usable poman beside itself keeps the poman entry
    the manifest already records, and its file, as they are, and says poman
    was not updated, the way install already carries the recorded assets and
    backups forward; so `status`, `uninstall` and the next install still know
    it. When none is recorded, install refuses and says how to get poman
    ("Open For The Owner", choice 2);
  - an llm-wiki from before PM1 must not rewrite a manifest that records
    poman, because it would drop the entry it does not know: the manifest's
    `schema_version` goes from 2 to 3, which today's llm-wiki already refuses
    with "unsupported manifest schema_version", and the new llm-wiki reads 1,
    2 and 3.
- **Uninstall removes everything it installed**, both binaries included, and
  the `--include-binary` flag goes away ("Open For The Owner", choice 3).
- **The install tests do not depend on a build's leftovers:** a test that
  needs poman beside `llm-wiki` copies `llm-wiki` into a temporary folder and
  puts its own `poman` beside it, so `cargo test --test post_install` alone
  (the post-install workflow) and a full workspace build give the same result.

### The release

- **One release build produces both binaries:** a local `dist build
  --artifacts=local` for `x86_64-unknown-linux-gnu`, as the justfile's
  `release-e2e-native-linux-dist-build` recipe runs it, produces
  `llm-wiki-rs-x86_64-unknown-linux-gnu.tar.xz` with `llm-wiki` inside and
  `poman-x86_64-unknown-linux-gnu.tar.xz` with `poman` inside. No tag, no
  published release, no CI release run.
- **Two archives, the owner's decision of 2026-10-06**, replacing "the release
  archive ships both binaries": cargo-dist 0.28 makes one release per package
  and fills it only from that package's own binaries, with no setting that
  puts another package's binary in it. So each target gets
  `llm-wiki-rs-<triple>.tar.xz` and `poman-<triple>.tar.xz` from the one
  release; dist's shell installers put both binaries in Cargo's bin folder,
  side by side, a person unpacking by hand unpacks both into one folder, and
  the release E2E unpacks the poman archive beside `llm-wiki`. The
  alternatives, a second `[[bin]]` in llm-wiki-rs (two packages writing
  `target/debug/poman`) and repacking after dist (dist's checksums and
  installers no longer matching), were not chosen.
- **`release.yml` stays generated:** if the dist config changes, it is
  regenerated by dist, never edited by hand.

## Phases

1. **The crates and the pin:** the toolchain pin, the two crates as workspace
   members with their lint tables, poman's binary running, and the first piece
   of behaviour in each; `just verify` still passes for llm-wiki on the pin.
2. **The gate script:** every gate above as one check; then, for each gate, a
   deliberate slip (an inline test module, a `forbid` lowered to `deny`, an
   `allow-*-in-tests` key the owner did not allow, a kind of test with no
   test, an uncovered line, a surviving mutant, a duplicate dependency
   version, a missing tool, a nightly older than the pin) shown to fail that
   gate and reverted.
3. **Install and uninstall:** `default-members` and the one-binary commands;
   then poman copied, recorded, refused, signed, kept and removed as above,
   and the manifest's schema version raised, proved under a temporary HOME by
   the install tests.
4. **The release build:** the local dist build, and both archives' contents
   listed.
5. **CI:** the gate script in the fast check and the full CI (choice 4), with
   the tools it needs installed there, and CI's unused-dependencies job on the
   dated nightly.

## Done When

- **The gates:** `just strict` (or the name chosen) reports every gate run and
  passed for the two crates, with 0 skipped and 0 failed, locally and in both
  the fast check and the full CI (choice 4).
- **The slips:** each deliberate slip of phase 2 made its gate fail, with the
  failing output recorded in the PR.
- **Install:** an install into a temporary HOME puts `poman` in the managed bin
  folder and the manifest; a second install copies nothing; `uninstall`
  removes both binaries and everything else it installed (choice 3); an
  install with no poman beside `llm-wiki`, or one of another version, keeps a
  recorded poman and refuses when none is recorded (choice 2); a manifest that
  records poman is refused by an llm-wiki from before PM1.
- **Release:** the local dist build produces both archives,
  `llm-wiki-rs-<triple>.tar.xz` listing `llm-wiki` and `poman-<triple>.tar.xz`
  listing `poman` (the owner's decision of 2026-10-06, "Two archives").
- **llm-wiki unchanged:** `just verify` passes with nothing skipped, on the
  pinned toolchain.

### Evidence Recorded

In the PR: the gate script's summary line and output, each slip's failing
gate, the names of the install tests that prove poman's install and removal,
the archives' listings, and the pinned toolchain version. In this plan, once it
lands: "Where It Stands" brought up to date.

### Wiki Pages To Update When Done

Whoever does the work rechecks this list:

- this plan, "Where It Stands";
- `wiki/decisions/poman-lives-in-this-workspace.decision.md`, "Consequences",
  which says the `members` list is empty and that the crate's name is not
  decided;
- `wiki/decisions/llm-wiki-binary-distribution.decision.md`, which issue #21
  amends with the install channels: PM1 checks that it matches what was built;
- `README.md`, its install section, for poman.

### What May Be Touched

A first list, to be rechecked by whoever does the work: the root `Cargo.toml`
(members, the dist config), `Cargo.lock`, a new `rust-toolchain.toml` and
`deny.toml`, a `clippy.toml` (setting no `allow-*-in-tests` key, choice 6),
the two new crates,
the justfile, a gate script under `tools/`,
`tools/test-instance-live-session-proof.sh`,
`src/install.rs`, `src/uninstall.rs`, `src/manifest/`, `src/paths.rs`,
`src/status.rs` and `src/doctor.rs` (both read the manifest), `tests/install.rs`,
`tests/post_install.rs`, `tests/status_doctor.rs`, `tools/release-e2e` (its
archive layout check), the CI workflows, and `release.yml` if dist regenerates
it.

### What Closes This Plan

The owner's PASS on the PR that meets "Done When", merged into `develop`.

## Open For The Owner

Asked on 2026-10-06 and answered by the owner the same day; each answer is
decided, and the text that pointed here follows it.

1. **The shared crate's name: `llm-wiki-core`, with a `types` module inside
   it** (the owner, 2026-10-06: "core then in core a types module").
   `llm-wiki-pages` and `llm-wiki-types` were the alternatives.
2. **Install with no usable poman beside llm-wiki, and none recorded:
   install refuses** (the owner, 2026-10-06: "there should always be a poman
   besides an llm wiki"). It says how to get one: the release ships both,
   and people who ran `cargo install llm-wiki-rs` also run `cargo
   install poman`. A poman already recorded is never orphaned (Target, "No
   recorded poman is ever orphaned"). Going on without poman was the
   alternative.
3. **Uninstall removes everything it installed, both binaries included**
   (the owner, 2026-10-06: "uninstall uninstalls all"). The
   `--include-binary` flag goes away, which changes today's behaviour: its
   help, the README, the specs and the tests that describe it are updated
   with it. Removing poman only with `--include-binary` was the alternative.
4. **The gate script runs in both the fast check on PRs into `develop` and
   the full CI** (the owner, 2026-10-06). A local gate only until PM8 was
   the alternative.
5. **The shared crate's first piece of behaviour: splitting a wiki filename**
   (`[slug].type.md` or `[index]-[slug].type.md`, AGENTS.MD, "Conventions")
   into its parts, which PM2's type definitions build on (the owner,
   2026-10-06).
6. **No lint exception for test code** (the owner, 2026-10-06: "lint all"):
   tests return `Result` and use `?`, clippy.toml sets none of the
   `allow-unwrap-in-tests`, `allow-expect-in-tests`,
   `allow-indexing-slicing-in-tests`, `allow-panic-in-tests` or
   `allow-print-in-tests` keys, and the gate script checks that. Naming the
   keys allowed was the alternative.

## Out Of Scope

- Moving the parser and defining the document types (PM2).
- llm-wiki's own modules under the strict gates, and the gate script's check
  that a switched module does not slip back (PM8).
- Cutting a release, tagging, and publishing either crate on crates.io: the
  owner's. The retired runner in the release workflow is issue #16, on hold.
- Editing the decision pages for the owner's answers of 2026-10-06: issue #21.
