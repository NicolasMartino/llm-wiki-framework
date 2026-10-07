# Plan: The Dependency Gate Covers Dev-Dependencies

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-10-07
- Category: Tooling, strict gates
- Scope: Carry out P22 of the framework roadmap: make the strict gates'
  dependency check cover the strict crates' dev-dependencies in all four of
  its checks, two of which (duplicates and licences) left them out by default,
  prove each check on a dev-dependency, and say so where the gate is
  described, with the runs that prove it recorded here.
- Sources:
  - Issue #37, "Tooling: Make the dependency gate cover dev-dependencies"
  - The blind review of PR #28, finding 5 (2026-10-06): with cargo-deny
    0.20.2, `cargo deny --manifest-path crates/llm-wiki-core/Cargo.toml list`
    shows the crate alone, with no proptest
  - The blind review of PR #59, which wrote this plan, finding 1: a ban on
    proptest fails `cargo deny check bans` for both strict crates
  - The blind review of PR #64, which carries it out (2026-10-07), findings
    1 and 2: the duplicate check leaves dev-dependencies out unless `[bans]`
    sets `multiple-versions-include-dev`, and the licence check unless
    `[licenses]` sets `include-dev`
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
- **Bans, advisories and sources cover them**: with a copy of `deny.toml`
  outside the repository holding `deny = [{ crate = "proptest" }]` under
  `[bans]`, `cargo deny --manifest-path crates/<crate>/Cargo.toml --config
  <copy> check bans` exits 2 with `error[banned]: crate 'proptest = 1.11.0'
  is explicitly banned`, its path `(dev) llm-wiki-core v0.2.15`, and the same
  on `crates/poman` with `(dev) poman v0.2.15`. Found by PR #59's blind
  review. The advisory and source checks walk the same graph (see "The
  Proof").
- **Duplicates and licences do not**: cargo-deny's duplicate check leaves
  dev-dependencies out unless `[bans]` sets `multiple-versions-include-dev =
  true`, and its licence check unless `[licenses]` sets `include-dev = true`;
  `deny.toml` sets neither. The tree already holds a duplicate reached only
  through proptest, `getrandom` 0.3.4 (through rand 0.9) and 0.4.2 (through
  tempfile), which the gate let through. Found by PR #64's blind review, after
  this plan had concluded the gate needed no change.

## Target

- **`deny.toml` covers dev-dependencies**: `multiple-versions-include-dev =
  true` under `[bans]` and `include-dev = true` under `[licenses]`. The
  `getrandom` duplicate is let through by one `skip` entry pinned to 0.3.4,
  with its reason: both versions come from inside proptest, so no change to
  this repository's dependencies removes it.
- **The proof, recorded**: for each of the four checks, a deliberate slip on
  the dev-dependency, made in a copy of the configuration outside the
  repository, rejected for each strict crate; kept in this plan with the
  commands and their output.
- **Said plainly**: the deny gate's comment in `tools/strict-gates.sh` says it
  covers dev-dependencies, through the two keys in `deny.toml`, and that
  `cargo deny list` does not show them. The strict-gates decision says
  "dependencies checked", which stays true, so it is unchanged.

## The Proof (2026-10-07, PR #64)

Run with cargo-deny 0.20.2 on PR #64's branch, with its `deny.toml`. Each copy
of `deny.toml` is kept outside the repository and changes one thing; the
command is `cargo deny --manifest-path crates/<crate>/Cargo.toml --config
<copy> check <check>` (with `--offline` for advisories and sources).

| Copy changes | Check | llm-wiki-core | poman |
|---|---|---|---|
| `deny = [{ crate = "proptest" }]` under `[bans]` | bans | exit 2, `error[banned]` proptest, `(dev) llm-wiki-core` | exit 2, the same, `(dev) poman` |
| the `getrandom` skip removed | bans | exit 2, `error[duplicate]` getrandom 0.3.4 and 0.4.2, both through `(dev) llm-wiki-core` | exit 2, the same, `(dev) poman` |
| the skip and `multiple-versions-include-dev` removed (the gate before this PR) | bans | exit 0, `bans ok` | exit 0, `bans ok` |
| `allow = ["Unicode-3.0"]` | licenses | exit 4, 29 crates rejected, proptest's 28 among them | exit 4, 46 rejected |
| the same, and `include-dev` removed (the gate before this PR) | licenses | exit 4, 1 rejected (the crate itself) | exit 4, 18 rejected |
| `db-path` to a copy of the advisory database holding a made-up advisory against every version of proptest | advisories | exit 1, `RUSTSEC-2099-0001`, proptest through `(dev) llm-wiki-core` | exit 1, the same, `(dev) poman` |
| `allow-registry` naming only another registry | sources | exit 8, 28 rejected (all proptest's) | exit 8, 45 rejected |

With the repository's own `deny.toml`, the deny gate of `just strict` passes
for both crates: `advisories ok, bans ok, licenses ok, sources ok`. In the same
tree, `cargo deny --manifest-path crates/llm-wiki-core/Cargo.toml list` still
prints only `MIT (1): llm-wiki-core@0.2.15`: `list` hides the
dev-dependencies whatever the configuration says.

## Done When

- Each slip in "The Proof" is rejected for both strict crates on the PR's
  head, and the two rows marked "the gate before this PR" show what the keys
  added.
- The deny gate's comment says what it covers.
- `just strict` reports every gate run and passed, nothing skipped, and the
  fast check passes on the PR into `develop`.

## Open For The Owner

Taken by the coordinator on 2026-10-07 while the owner was away: choice 1, as
recommended, to be confirmed by the owner's verdict on PR #64. PR #64's review
then found the duplicate and licence checks did leave dev-dependencies out, so
the change grew from the comment alone to the two `deny.toml` keys and the
`getrandom` skip above, as the coordinator directed in the fix round.

1. **Close P22 with a one-line comment in the gate script**, the proof being
   the run recorded here, rerun on that PR's head. Not chosen: closing P22 on
   this plan alone, with no change, which leaves the next reader of `list`
   to file #37 again; or a test that bans a crate in a scratch configuration
   on every gate run, which adds a slow check for a behaviour of cargo-deny,
   not of this repository.

## Out Of Scope

- llm-wiki's own crate under the strict gates (PM8).
- The other gates of the strict-gates script.
- Changing what `deny.toml` bans or allows, beyond the `getrandom` skip that
  covering dev-dependencies needs.
