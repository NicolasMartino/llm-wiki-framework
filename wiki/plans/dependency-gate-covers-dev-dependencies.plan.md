# Plan: The Dependency Gate Covers Dev-Dependencies

- Document Class: Plan
- Status: Draft
- Date: 2026-10-07
- Category: Tooling, strict gates
- Scope: Carry out P22 of the framework roadmap: the strict gates' dependency
  check covers the strict crates' dev-dependencies as well as their normal
  ones, shown by a dev-dependency that breaks a ban failing the gate once.
- Sources:
  - Issue #37, "Tooling: Make the dependency gate cover dev-dependencies"
  - The blind review of PR #28, finding 5 (2026-10-06): with cargo-deny
    0.20.2, `cargo deny --manifest-path crates/llm-wiki-core/Cargo.toml list`
    shows the crate alone, with no proptest
  - `tools/strict-gates.sh`, `deny.toml`, `justfile` and the strict crates'
    `Cargo.toml` at `cad8988`, read for this plan; the `list` command above
    rerun on this plan's worktree
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P22 (this plan)
  - `wiki/decisions/poman-lives-in-this-workspace.decision.md`, "The
    strictest gates" (the gates this check belongs to)
  - `wiki/plans/poman-workspace-and-strict-gates.plan.md` (PM1, which built
    the gate)
  - `wiki/roadmaps/poman.roadmap.md`, PM8 (llm-wiki's own modules)

## What This Proves

The deny gate checks everything the strict crates build with, tests
included, so a banned, duplicated, unlicensed or advised-against crate pulled
in only by tests fails it like any other.

## Where It Stands (2026-10-07, at `cad8988`)

- `just strict` runs `tools/strict-gates.sh`; its `deny` gate runs `cargo deny
  check advisories licenses bans sources` once per strict crate, with
  `--manifest-path` on `crates/llm-wiki-core` and `crates/poman` in turn.
- `deny.toml` at the root bans multiple versions and wildcards, allows MIT,
  Apache-2.0 and Unicode-3.0, denies yanked crates and unknown registries and
  git sources, and limits the graph to four targets. It sets nothing about
  dev-dependencies.
- Both strict crates have one dev-dependency, `proptest`, from the workspace.
- With cargo-deny 0.20.2, the version on this machine, `cargo deny
  --manifest-path crates/llm-wiki-core/Cargo.toml list` prints one crate,
  `llm-wiki-core` itself: no proptest, and none of its own dependencies.
  Rechecked for this plan. The review also saw that running from the root
  with `--workspace --exclude llm-wiki-rs` lists 19 crates, still without
  proptest.
- Why cargo-deny leaves them out is not known yet; finding it is phase 1.

## Target

- **The gate covers dev-dependencies**: the deny gate's checks see each strict
  crate's dev-dependencies and theirs, through cargo-deny's own settings or
  the way it is called if that is enough, or else through a second check in
  the same gate (the owner's choice 1).
- **Shown once**: a dev-dependency that breaks a ban (a second version of a
  crate already in the graph, or a wildcard version) makes the deny gate
  fail, recorded in this plan with the command and its output, then removed;
  not committed.
- **Said plainly**: the gate script's comment and the strict-gates decision's
  wording say what the deny gate covers. If some part still cannot be covered,
  this plan and the decision say which, and what covers it instead.

## Phases

1. **Find why**: record which cargo-deny setting, flag or limit leaves the
   dev-dependencies out (`exclude-dev`, the targets list, the per-crate
   manifest path, or the version), with the command that shows it.
2. **Cover them** by the smallest change that works, per choice 1.
3. **Prove it**: the deliberate break above, then the full gate script and
   the fast check green.

## Done When

- `cargo deny ... list` over each strict crate, as the gate calls it, shows
  proptest and its dependencies, or the second check of choice 1 lists them.
- A dev-dependency that breaks a ban failed the deny gate once, recorded here.
- `just strict` reports every gate run and passed, nothing skipped, and the
  fast check passes on the PR into `develop`.

## Open For The Owner

1. **If cargo-deny's settings cannot reach the dev-dependencies: move to the
   newest cargo-deny that does**, pinned in the gate script's install line.
   Not chosen: a second check built from `cargo tree --edges dev` in the gate
   script, which repeats cargo-deny's rules in shell and drifts; or naming the
   gap in the decision and leaving it, which the issue rules out ("a gate
   that checks only part of what it claims is a setup to fix").

## Out Of Scope

- llm-wiki's own crate under the strict gates (PM8).
- The other gates of the strict-gates script.
- Changing what `deny.toml` bans or allows.
