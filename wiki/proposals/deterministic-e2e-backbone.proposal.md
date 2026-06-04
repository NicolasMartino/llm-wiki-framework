# Deterministic Workspace E2E Backbone

- Document Class: Proposal
- Status: Proposed
- Date: 2026-06-04
- Category: E2E, release engineering, verification
- Scope: Promote deterministic fixture-backed workspace scenarios as the
  backbone of cross-platform E2E testing for the `llm-wiki` binary, while
  keeping LLM/agent ingest characterization as a separate future lane.
- Sources: conversational request 2026-06-04;
  wiki/plans/cross-platform-release-e2e-harness.plan.md;
  wiki/proposals/full-windows-support.proposal.md;
  wiki/proposals/project-update-command.proposal.md;
  wiki/specs/documentation-model.spec.md;
  wiki/specs/wiki-ingest-skill.spec.md;
  wiki/checklists/v1-fixture-smoke.checklist.md;
  tools/release-e2e/src/main.rs; src/cli.rs; src/init/sources.rs
- Related: wiki/plans/cross-platform-release-e2e-harness.plan.md,
  wiki/proposals/full-windows-support.proposal.md,
  wiki/proposals/project-update-command.proposal.md,
  wiki/checklists/v1-fixture-smoke.checklist.md,
  wiki/specs/wiki-ingest-skill.spec.md

## Source Capture

This proposal cites conversational input from 2026-06-04 because the direction
was developed in chat while reviewing the existing release E2E harness. Before
promotion to an implementation plan or decision, capture the conversation as a
durable raw source or replace it with another project-owned source path.

## Question

Should cross-platform E2E remain a sequence of command smoke checks, or should
it use deterministic fixture-backed workspaces as the reusable scenario model
for install, init, update, indexing, search, cleanup, and future migration
commands?

## Proposal

Adopt deterministic fixture-backed workspace scenarios as the routine E2E
backbone for the `llm-wiki` binary.

Each deterministic scenario creates isolated machine state and an isolated
workspace, runs the release artifact through documented CLI commands, seeds the
workspace with committed raw fixtures, verifies command effects after each
step, writes a canonical tree manifest, compares that manifest against an
expected fixture snapshot, and cleans up.

The canonical base scenario is:

```text
create isolated home + workspace
run llm-wiki install
run llm-wiki init <workspace> --initial-sources fixtures/basic-raw
assert raw source copied
run register/index/search
assert search finds expected wiki pages
write canonical tree manifest
compare against fixture snapshot
cleanup/assert cleanup
```

This scenario should become the reusable backbone for current and future
deterministic lanes. The release E2E runner already owns artifact execution,
isolated homes, stdout/stderr capture, JSON/JUnit reports, checksum handling,
and command assertions. This proposal extends that harness from command-level
proof toward workspace-state proof.

## Deterministic Lane

The deterministic lane is suitable for normal CI and multi-platform release
proof because it does not require an LLM, network access, hidden model state, or
test-only runtime hooks.

It should cover:

1. release artifact acquisition or direct artifact execution
2. checksum verification before execution
3. isolated managed runtime install
4. generated skill files and managed-binary invocation
5. project init with committed `--initial-sources`
6. copied raw-source provenance under `raw/initial/`
7. project-local `.llm_wiki/` config
8. registry, index, search, and search-all behavior
9. semantic/hybrid fail-closed readiness when LLM search is disabled or missing
10. cleanup through `forget` and `uninstall`
11. future `update` or upgrade/migration behavior once the project update
    command is implemented

The lane should remain black-box: it runs the built or packaged `llm-wiki`
artifact through the target platform shell instead of calling internal Rust
APIs.

## Canonical Tree Manifest

Each scenario should write a canonical tree manifest for the workspace and
selected managed-runtime paths. The manifest should record stable facts rather
than raw filesystem noise.

For each included file, record:

1. repo- or scenario-relative path
2. file kind
3. normalized byte length
4. SHA-256 of normalized content
5. optional semantic tags such as `generated`, `raw-source`, `wiki-page`,
   `project-config`, or `runtime-skill`

Normalization should handle:

1. line endings
2. absolute temporary paths
3. platform path separators where the product contract is path-agnostic
4. known timestamp fields when the timestamp is not the behavior under test

The manifest should exclude or separately classify nondeterministic rebuildable
artifacts such as stdout/stderr logs, report timestamps, transient temp
directories, and qmd-rs SQLite stores. Search/index behavior should be asserted
through command JSON and expected result paths rather than byte-for-byte cache
snapshots.

## Fixture Matrix

Start with a small fixture matrix that exercises product behavior without
making the runner noisy:

| Fixture | Purpose |
| --- | --- |
| `basic-raw` | One or more raw notes copied through `--initial-sources`; proves provenance and init copying |
| `path-with-spaces` | Workspace and runtime path quoting across shell substrates |
| `rerun-init` | Re-running init preserves compiled wiki knowledge and records schema drift when relevant |
| `registered-search` | Register, index, lexical search, search-all, forget, and cache cleanup |
| `project-update` | Future versioned fixture matrix for `llm-wiki update` once the command exists |

Fixtures should be committed source material, not generated at random. Random
data has little value here because the goal is stable release proof and
regression diagnosis.

## Versioned Update E2E

Once `llm-wiki update` exists, deterministic E2E should use git-managed
framework versions to prove that local projects created by older tool versions
can be transformed to the current project shape.

The update lane should use release tags or compatibility tags as the source of
old project states. A scenario should create a temporary git worktree or
archive checkout for each supported previous version, create a simulated user
workspace from that version, then run the current release artifact's
`llm-wiki update` command against the simulated workspace.

Preferred black-box flow:

```text
for each supported previous version tag:
  create temporary git worktree at <tag>
  build or acquire that tag's llm-wiki artifact when available
  use the tagged artifact to init a simulated project workspace
  seed committed raw fixtures and any version-specific project state
  switch to the current release artifact under test
  run llm-wiki update --framework --layout --reindex <workspace>
  assert generated framework artifacts match the current version
  assert compiled wiki content and raw provenance are preserved
  assert an update archive and manifest exist
  assert wiki/log.md records the update
  assert registry identity is stable
  assert index/search still work
  write and compare the canonical tree manifest
```

If an old tag's binary cannot be built or acquired on the current platform, the
runner may fall back to a committed snapshot of the old workspace shape. That
fallback must be labeled in the report so it is not mistaken for full
black-box old-binary proof.

The version matrix should be explicit. At minimum it should cover:

1. the immediately previous released version
2. the oldest version still claimed as update-compatible
3. any version that changed generated framework artifacts, init manifests,
   pack defaults, schema-drift handling, or project-local search config

The update lane should not check out tags in the main working tree. It should
use temporary worktrees, archives, or copied fixture snapshots so the release
runner does not mutate the developer's current checkout.

## Relationship To Existing Release E2E

The active cross-platform release E2E harness remains the tactical execution
vehicle. This proposal narrows the desired routine product story: deterministic
workspace fixtures should become the shared scenario contract that runs on
macOS, Linux, and Windows lanes.

The current command-story search lane is still valuable. It should evolve into
or sit beside the deterministic workspace lane rather than be discarded.

## Future LLM Lane

Agent-driven ingest, query save-back, and lint behavior remain future
non-deterministic work.

`wiki-ingest` is a skill workflow, not a current `llm-wiki` binary command. It
requires judgment: reading raw sources, extracting facts, choosing document
types, detecting contradictions, updating existing pages, maintaining
`wiki/index.md`, appending `wiki/log.md`, and refreshing search when
registered. A small local model can make this testable, but it should not be a
routine deterministic release gate.

The future LLM lane should be separate, for example:

```text
release-e2e ingest-llm
```

That lane may use a small local model or managed GGUF runtime when available.
It should compare structural invariants before prose snapshots:

1. expected page exists
2. metadata block is valid
3. `Sources` references the raw fixture
4. `wiki/index.md` links the page
5. `wiki/log.md` records the ingest
6. search returns the created or updated page
7. no unresolved orphan or contradiction is left behind

Exact prose checks should be normalized and selective. They should not make the
release gate fail only because an acceptable LLM output changed wording.

## Out Of Scope

This proposal does not add a new `llm-wiki ingest` command by itself. It also
does not require normal CI to download model artifacts, run a local LLM, or
byte-compare generated prose from an agent workflow.

It does not replace platform-specific host proof. Windows still needs native
host or VM coverage for Known Folder paths, PowerShell invocation, `.exe`
runnability, path escaping, and cleanup.

## Acceptance Criteria

The proposal can be promoted to an implementation plan when:

1. the fixture root and manifest format are chosen
2. the deterministic scenario contract is mapped to runner commands
3. volatile paths and normalization rules are defined
4. at least one committed raw fixture has expected workspace assertions
5. the existing release E2E plan references this proposal or its promoted plan
6. versioned update E2E defines its tag/worktree matrix and fallback snapshot
   semantics before `llm-wiki update` becomes a release gate
7. future LLM/agent characterization remains explicitly separate from routine
   deterministic release proof
