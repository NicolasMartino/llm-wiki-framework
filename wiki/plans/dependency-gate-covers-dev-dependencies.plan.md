# Plan: The Dependency Gate Covers Dev-Dependencies

- Document Class: Plan
- Status: Active
- Branch: `NicolasMartino/deny-37`
- Date: 2026-10-07
- Category: Tooling, strict gates
- Scope: Carry out P22 of the framework roadmap: show that the strict gates'
  dependency check already covers the strict crates' dev-dependencies, the
  gap #37 described being in `cargo deny list` only, and say so where the
  gate is described, with the run that proves it recorded here.
- Sources:
  - Issue #37, "Tooling: Make the dependency gate cover dev-dependencies"
  - The blind review of PR #28, finding 5 (2026-10-06): with cargo-deny
    0.20.2, `cargo deny --manifest-path crates/llm-wiki-core/Cargo.toml list`
    shows the crate alone, with no proptest
  - The blind review of this plan's PR (#59), finding 1: a ban on proptest
    fails `cargo deny check bans` for both strict crates
  - `tools/strict-gates.sh`, `deny.toml`, `justfile` and the strict crates'
    `Cargo.toml` at `cad8988`, read for this plan; the `list` command and the
    review's probe rerun on this plan's worktree
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
in only by tests fails it like any other; and the gate's description says so.

## Where It Stands (2026-10-07, at `cad8988`)

- `just strict` runs `tools/strict-gates.sh`; its `deny` gate runs `cargo deny
  check advisories licenses bans sources` once per strict crate, with
  `--manifest-path` on `crates/llm-wiki-core` and `crates/poman` in turn.
- `deny.toml` at the root bans multiple versions and wildcards, allows MIT,
  Apache-2.0 and Unicode-3.0, denies yanked crates and unknown registries and
  git sources, and limits the graph to four targets. It sets nothing about
  dev-dependencies.
- Both strict crates have one dev-dependency, `proptest`, from the workspace.
- **`list` leaves them out**: with cargo-deny 0.20.2, `cargo deny
  --manifest-path crates/llm-wiki-core/Cargo.toml list` prints one crate,
  `llm-wiki-core` itself, with no proptest. This is what PR #28's review saw,
  and what #37 was filed on.
- **`check` covers them**: with a copy of `deny.toml` outside the repository
  holding `deny = [{ crate = "proptest" }]` under `[bans]`, `cargo deny
  --manifest-path crates/<crate>/Cargo.toml --config <copy> check bans`
  exits 2 with `error[banned]: crate 'proptest = 1.11.0' is explicitly
  banned`, its path `(dev) llm-wiki-core v0.2.15`, and the same on
  `crates/poman` with `(dev) poman v0.2.15`. Found by this PR's blind review
  and rerun for this plan with cargo-deny 0.20.2.
- So the gate the strict gates run already walks the dev-dependencies; the gap
  is smaller than #37 thought. What remains: the gate's comment and the
  strict-gates decision do not say that dev-dependencies are covered, and
  the proof is recorded nowhere but here.

## Target

- **The proof, recorded**: the run above, a deliberate ban on a dev-dependency
  rejected by `check` for each strict crate, kept in this plan with the
  command and its output. Not committed: the ban lives in a copy of the
  configuration outside the repository.
- **Said plainly**: the deny gate's comment in `tools/strict-gates.sh` says it
  covers dev-dependencies, and that `cargo deny list` does not show them.
  The strict-gates decision's wording is checked, and changed only if it
  says otherwise.

## The Proof (2026-10-07, PR for #37)

Rerun with cargo-deny 0.20.2 on this plan's PR branch, from `c6e4995`, where
`deny.toml` and the strict crates' manifests are as at `cad8988`. The copy of
`deny.toml`, kept outside the repository, adds one line under `[bans]`:

```toml
deny = [{ crate = "proptest" }]
```

For each strict crate, `cargo deny --manifest-path crates/<crate>/Cargo.toml
--config <copy> check bans` exits 2:

```text
error[banned]: crate 'proptest = 1.11.0' is explicitly banned
   ├ proptest v1.11.0
     └── (dev) llm-wiki-core v0.2.15
bans FAILED
```

and on `crates/poman` the same, with `(dev) poman v0.2.15`. With the
repository's own `deny.toml`, the deny gate of `just strict` passes for both.
In the same tree, `cargo deny --manifest-path crates/llm-wiki-core/Cargo.toml
list` still prints only `MIT (1): llm-wiki-core@0.2.15`: `list` is what hides
the dev-dependencies, not `check`.

## Done When

- The deliberate ban above is rejected by `cargo deny check bans` for both
  strict crates, recorded here (done for this plan, at `cad8988`; rerun on
  the PR's head).
- The deny gate's comment says what it covers.
- `just strict` reports every gate run and passed, nothing skipped, and the
  fast check passes on the PR into `develop`.

## Open For The Owner

Taken by the coordinator on 2026-10-07 while the owner was away: choice 1, as
recommended, to be confirmed by the owner's verdict on the PR for #37.

1. **Close P22 with a one-line comment in the gate script**, the proof being
   the run recorded here, rerun on that PR's head. Not chosen: closing P22 on
   this plan alone, with no change, which leaves the next reader of `list`
   to file #37 again; or a test that bans a crate in a scratch configuration
   on every gate run, which adds a slow check for a behaviour of cargo-deny,
   not of this repository.

## Out Of Scope

- llm-wiki's own crate under the strict gates (PM8).
- The other gates of the strict-gates script.
- Changing what `deny.toml` bans or allows.
