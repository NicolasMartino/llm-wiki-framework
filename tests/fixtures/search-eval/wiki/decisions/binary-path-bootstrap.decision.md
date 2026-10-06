# Managed Binary Runtime Install

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-06
- Category: Distribution tooling, install UX
- Scope: Amend `llm-wiki install` so installed skills invoke a managed binary by absolute path and `PATH` is only terminal convenience.
- Sources: wiki/proposals/binary-path-bootstrap.proposal.md, proposal review 2026-05-06, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/roadmaps/framework-v1.roadmap.md (D8.1, D11), wiki/plans/binary-path-bootstrap.plan.md, wiki/specs/documentation-model.spec.md, wiki/specs/wiki-init-skill.spec.md
- Amends: wiki/decisions/llm-wiki-binary-distribution.decision.md

## Choice

Accept the managed binary install direction.

`llm-wiki install` should make installed skills independent of shell `PATH`.
The installer owns a managed runtime home:

```text
~/.llm_wiki/
~/.llm_wiki/bin/llm-wiki
~/.llm_wiki/manifest.json
```

Future Windows support uses the equivalent `%LOCALAPPDATA%\llm_wiki\` root and
`llm-wiki.exe`.

Installed skills must invoke the managed binary absolute path, not a bare
`llm-wiki` command. PATH guidance remains useful terminal convenience, but it is
not a correctness precondition.

Because the binary has not shipped broadly yet, D8.1 does not need public
migration semantics. The managed runtime home and manifest path replace the
pre-release D8 manifest location before release. Local dogfood or
development state may be overwritten through the normal `install --force`
discipline.

All binary and file hashes in managed install state use `sha256`.
Manifest v2 stores `hash_algorithm: "sha256"` beside each binary, skill, and
backup hash so records remain self-describing.

Backup snapshots cover both framework skill targets and a displaced managed
binary when `install --force` replaces an unmanaged file at the managed binary
path.

## Failure Semantics

The installer must resolve the running executable with `std::env::current_exe()`
and abort before modifying managed state if the path cannot be resolved or read.
It must not guess from `argv[0]` or `PATH`.

If the running executable already resolves to the managed binary path, install
skips copy-over-self and verifies the binary in place.

Install records an `~/.llm_wiki/install.partial.json` transaction marker before
copying the managed binary. Interrupted installs are recovered or rejected using
the marker and the same manifest-owned or `--force` collision rules as normal
installs.

The final manifest write happens before partial-marker deletion. If a crash
leaves both a valid final manifest and a partial marker behind, `doctor` treats
the marker as leaked cleanup state when the manifest and managed binary hashes
match.

## Command Shape

`llm-wiki install` remains the primary install command.

Accepted command additions:

```text
llm-wiki install --force
llm-wiki install --skip-path-guidance
llm-wiki path
```

`llm-wiki path` prints managed-bin PATH guidance without reinstalling skills.
This avoids overloading `install` with a non-installing mode.

## Acquisition Path Convergence

Every binary acquisition path should converge on the same install behavior.

`cargo install llm-wiki-rs` installs the executable onto the user's
Cargo bin path, but Cargo does not provide a reliable package-defined
post-install hook for mutating user home directories. The documented Cargo path
is therefore:

```bash
cargo install llm-wiki-rs
llm-wiki install
```

The release installer may invoke or offer to invoke `llm-wiki install`, but it
must not implement separate skill-copy, manifest, backup, or managed-binary
logic. One code path owns global runtime installation.

When `which llm-wiki` resolves to a different binary than the managed runtime
copy, `doctor` should warn about version drift. After `cargo install
llm-wiki-rs --force` or a release-installer upgrade, drift is the default
state until the user reruns `llm-wiki install`.

## Skill Rename

The agent-facing initialization skill was renamed from `init-project` to
`knowledge-init` in D8.1 for namespace consistency with the rest of the
framework skills. D11 then renamed the entire skill family from `knowledge*`
to `wiki-*` (and the Codex dispatcher from `knowledge` to `wiki`); the active
init-skill spec is `wiki/specs/wiki-init-skill.spec.md`.

The D8.1 rename was implemented as a separable phase. There is no public
`init-project` or `knowledge-init` compatibility surface to preserve — both
predate public release.

Historical proposal and log text may still mention `init-project` or
`knowledge-init` when describing pre-release state.

## Why

The D8 binary distribution decision made the binary the authority for global
skill installation, scaffolding, and projection, but the installed skills still
assumed `llm-wiki` was discoverable on `PATH`. That fails when a user downloads
a release binary and runs it directly from an ad hoc location such as
`Downloads`.

A managed binary path fixes the runtime target rather than trying to mutate
user shell profiles. It also gives `doctor`, `uninstall`, and future rollback
behavior a concrete state root to inspect.

## Consequences

- D8.1 is added as a follow-up deliverable because D8 is already completed.
- `wiki/plans/binary-path-bootstrap.plan.md` executes this decision.
- Specs are not updated yet; specs represent validated behavior, and this
  behavior remains pending until implementation and tests land.
- `wiki/decisions/llm-wiki-binary-distribution.decision.md` remains the base
  binary-distribution decision, amended by this decision for install runtime
  paths, manifest location, PATH guidance, and the D8.1 `knowledge-init`
  rename. D11 later renamed the full skill family from `knowledge*` to
  `wiki-*`.

## Revisit When

- Windows moves from compatibility design to released artifact support.
- Runtime skill systems add a package-relative way to invoke bundled tools.
- The framework adds self-update or automated rollback.
