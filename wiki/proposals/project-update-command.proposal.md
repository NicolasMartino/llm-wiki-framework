# Project Update Command

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-15
- Category: Tooling, project update, migration UX
- Scope: Add `llm-wiki update` as the project-scoped maintenance command.
  Update operates on the current or selected project only: it can refresh
  generated framework artifacts, adjust project layout, configure project-local
  search, update the same-root registry entry, and rebuild target-project
  search artifacts in host-local cache. Machine-wide defaults belong to
  `llm-wiki defaults`; global runtime assets and downloads belong to
  `llm-wiki install`.
- Sources: conversational input 2026-05-15, wiki/decisions/composable-project-init.decision.md, wiki/specs/wiki-init-skill.spec.md, wiki/plans/init-rerun-pack-drift.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/proposals/search-model-selection.proposal.md
- Related: wiki/specs/documentation-model.spec.md,
  wiki/specs/wiki-init-skill.spec.md,
  wiki/decisions/composable-project-init.decision.md,
  wiki/checklists/observability-contract.checklist.md

## Source Capture

This proposal currently cites conversational input because the direction was
introduced and reviewed in chat. Before promotion to a decision or
implementation plan, capture the conversation/review basis as a raw source or
replace it with another durable source path in the metadata.

## Question

Should project migration be a narrow `llm-wiki upgrade` command, or should the
framework expose a broader, scope-clear `llm-wiki update` command for existing
project maintenance while keeping global defaults and global runtime assets on
separate command surfaces?

## Proposal

Add `llm-wiki update [path]` as the explicit project-scoped maintenance
command. The command affects only the target project and host-local state keyed
to that project, such as its registry entry or search cache. It must not modify
machine-wide defaults under `~/.llm_wiki/`, global skill installs, managed
binaries, or model artifacts except through an explicit handoff to an
install-owned flow.

The top-level command model becomes:

- `llm-wiki init` creates a project.
- `llm-wiki update` changes an existing project and project-scoped host-local
  state.
- `llm-wiki defaults` changes machine-wide defaults.
- `llm-wiki install` changes installed runtime/assets.
- `llm-wiki search` queries one project.
- `llm-wiki search-all` queries registered projects.
- `wiki-lint` reasons about compiled wiki content, stale references, orphans,
  and optional archival.

`update` replaces the proposed standalone `upgrade` command. Framework-artifact
upgrade remains a supported task, but it is one project update task rather than
the whole command identity.

## Why

The binary manages three scopes of state:

- project-root state under a project root, such as `AGENTS.md`,
  `project_guidelines.md`, `.llm_wiki/init.toml`, project search config,
  and project folders
- target-project-scoped host-local state, such as the same-root registry entry
  and that project's rebuildable search cache/index
- machine/global state, such as the managed binary, installed skills, global
  defaults, model artifacts, accepted licenses, and cross-project search
  defaults

Mixing those scopes inside one command creates surprising behavior. A user who
runs `llm-wiki update` in a project should know it is changing that project and
its project-scoped host-local records, not global defaults. A user who runs
`llm-wiki defaults` should know existing projects are not being rewritten. A
user who runs `llm-wiki install` should know it is changing machine runtime
state.

This split also lets `init` become simpler over time. Rerun init currently has
migration-adjacent responsibilities: manifest prefill, pack changes, folder
creation, root schema refresh, schema-drift logging, index drift sections, and
registry update behavior. Those belong more naturally under project update.

## Command Surface

Interactive project-scoped update:

```text
llm-wiki update [path]
```

The interactive flow presents a checklist of project-scoped tasks:

```text
Project-scoped update tasks:
[x] Framework artifacts - refresh generated framework files for this binary version
[ ] Layout - change blueprint, packs, and pack-driven folders
[ ] Search - configure this project's search profile
[ ] Reindex - rebuild this project's search artifacts after changes
```

Scripted selection uses flags so related project-scoped tasks can run in one
transaction:

```text
llm-wiki update --framework --yes
llm-wiki update --layout
llm-wiki update --search
llm-wiki update --reindex
llm-wiki update --framework --layout --search --reindex --yes
```

Flags are preferred over subcommands for the first implementation because the
tasks are order-sensitive and often combined. One update run can create one
archive bundle, one transaction marker, one `wiki/log.md` entry, and one final
summary.

## Scope-First Help Contract

Every command that can read or write state must describe its scope in `--help`.
The help text should make the target explicit before listing options.

Required help sections:

```text
Scope:
  Project-scoped. Modifies project files and target-project host-local state
  only.
```

```text
Scope:
  Machine-wide defaults. Modifies ~/.llm_wiki defaults only.
```

```text
Scope:
  Global runtime. Modifies installed skills, managed binary, or model artifacts.
```

Specific command guidance:

- `llm-wiki init --help`: creates a new project at the target path; does not
  change global defaults or installed runtime assets.
- `llm-wiki update --help`: modifies only the target project plus
  target-project-scoped registry/cache state; does not change `~/.llm_wiki/`
  defaults, install skills, or download models directly.
- `llm-wiki defaults --help`: modifies machine-wide defaults; does not rewrite
  existing projects.
- `llm-wiki install --help`: modifies global runtime assets; does not rewrite
  project files.
- `llm-wiki search --help`: reads one project and its project-local search
  configuration.
- `llm-wiki search-all --help`: reads registered projects and the
  cross-project search profile.
- `llm-wiki doctor --help`: inspects project and global state; any repair mode
  must state the exact scope it writes.

Top-level help should group commands by scope:

```text
Project commands:
  init       Create a new project
  update     Update the current or selected project
  search     Search one project

Registry commands:
  register   Add a project to the host-local registry
  forget     Remove a project from the host-local registry
  projects   List registered projects
  search-all Search registered projects

Machine commands:
  defaults   Configure machine-wide defaults for future projects and search-all
  install    Install or repair global runtime assets
  path       Print managed binary PATH guidance
  doctor     Inspect project and global state
```

Error and confirmation text should also name scope. Examples:

```text
This updates project files under /path/to/project and target-project cache or
registry records. It will not change ~/.llm_wiki defaults.
```

```text
This changes machine-wide defaults under ~/.llm_wiki/search.toml. Existing
projects are unchanged until you run llm-wiki update --search in that project.
```

## Project Update Tasks

### `--framework`

Refresh generated framework-owned project artifacts from the currently running
binary. This is the former "upgrade" behavior.

It archives previous framework-owned artifacts under:

```text
<project>/.llm_wiki/archive/artifacts/<from-version>/<timestamp>/
```

Then it overwrites generated framework-owned artifacts with the current
binary's rendered outputs while preserving compiled `wiki/` content outside
generated sections and preserving source references.

The archive manifest records:

- source project root
- from framework version and to framework version
- UTC timestamp
- command arguments
- selected update tasks
- init answers used for the new render
- files archived, including SHA-256 hashes
- files rewritten, including SHA-256 hashes before and after
- files skipped, with reasons
- conflicts or dirty generated artifacts, if any

The archive stores copies of prior artifacts, not moves. This keeps the project
usable if update fails after archive creation but before all writes complete.

### `--layout`

Replay the init answer flow for project layout: project name, description,
blueprint, and packs. Values are prefilled from `.llm_wiki/init.toml` when
available, with documented generated-file recovery for older projects.

Layout updates are additive:

- create folders newly claimed by the current resolved pack set
- never delete, archive, move, or prune folders that are no longer claimed
- leave orphaned folder content on disk
- record added and orphaned folders in `wiki/log.md`
- refresh the generated `## Schema Drift` section in `wiki/index.md`

Folder drift is computed from the previous manifest's `resolved_folders` and
the current composition output. For legacy manifests without `resolved_folders`,
update can detect pack-set drift and seed the new field, but same-pack
folder-composition drift is only reliable after a manifest with
`resolved_folders` exists.

### `--search`

Configure this project's search profile in `<project>/.llm_wiki/search.toml`.
This is project-local. It does not change `~/.llm_wiki/search.toml`, new-project
defaults, or the cross-project `search-all` profile.

If the selected model/profile bundle is not installed, update may write
project-local pending state and offer an explicit handoff to the install-owned
flow. `update --search` must not accept licenses, download model bytes, verify
model artifacts, or promote global managed artifacts itself.

Machine-wide search defaults belong to:

```text
llm-wiki defaults search
```

### `--reindex`

Rebuild this project's search artifacts after update tasks complete. Reindexing
is project-scoped host-local cache work: it may write outside the project root
under the framework cache directory, but only for the target project's search
artifacts. It does not change cross-project search artifacts unless the user
explicitly runs an existing cross-project command such as `llm-wiki index-all`.

If `--reindex` is not selected and update touches `wiki/log.md` or the
generated `## Schema Drift` section in `wiki/index.md`, the existing
file-snapshot freshness model marks those wiki files stale. Update should
report that state and give `llm-wiki index --force` guidance.

## Defaults Command

Add or reserve `llm-wiki defaults` for machine-wide defaults. At minimum, the
search defaults surface should be:

```text
llm-wiki defaults search
llm-wiki defaults show
llm-wiki defaults reset
```

`defaults search` administers `~/.llm_wiki/search.toml`, including new-project
defaults and the cross-project `search-all` profile. It does not rewrite
existing project-local `.llm_wiki/search.toml` files. Existing projects can opt
into changed defaults through `llm-wiki update --search`.

## Relationship To Search Model Selection

This proposal amends the open
`wiki/proposals/search-model-selection.proposal.md` command surface. That
proposal assigned new-project defaults to `install` and
`install --configure-search`. Under the scope-first command model:

- `llm-wiki defaults search` administers machine-wide search defaults in
  `~/.llm_wiki/search.toml`, including `[project_default]` and
  `[global_search]`.
- `llm-wiki update --search` administers one project's
  `<project>/.llm_wiki/search.toml`.
- `llm-wiki install` remains the owner for license acceptance, model downloads,
  hash verification, and managed artifact promotion.

`defaults search` and `update --search` may hand off to install-owned code when
the selected model/profile bundle is missing, but they must not materialize
models themselves.

## Framework-Owned Artifact Set

The first implementation should treat these as framework-owned generated
artifacts for `update --framework`:

- the block init owns in `AGENTS.md` (or `AGENTS.MD`), `CLAUDE.md` and
  `project_guidelines.md`, between `<!-- llm-wiki:managed:start -->` and
  `<!-- llm-wiki:managed:end -->`; the text outside it is the project's
  (`wiki/decisions/composable-project-init.decision.md`, 2026-10-07)
- `.llm_wiki/init.toml`
- the generated `## Schema Drift` section in `wiki/index.md`
- generated update state or manifests under `.llm_wiki/`, excluding
  `.llm_wiki/archive/**`

Potential legacy framework artifacts can be detected and archived when present,
but should not be deleted unless they are replaced by a current artifact in the
same operation. Examples include project-local skill mirrors from older
dogfooding states or pre-rename generated schema files.

The archive tree is never part of the update input artifact set. Update must
not recursively archive previous archives, previous archive manifests, or
artifact bundles under `.llm_wiki/archive/**`.

Legacy artifact detection is intentionally bounded. The implementation plan
derived from this proposal must enumerate the exact legacy paths it will detect
and archive. Until a legacy path is enumerated, update must leave it alone
rather than scanning broadly for framework-looking files.

## Overwrite Semantics

Overwrite mode means "overwrite framework-owned generated artifacts after
archiving their previous contents." It does not mean "overwrite all colliding
paths."

Recommended default:

- archive every previous framework-owned artifact before writing replacements
- proceed when the file is recognized as framework-owned, even if its current
  bytes differ from what the current binary would render
- record archived before/after hashes in the archive manifest and verbose
  diagnostics
- require `--force` only for ambiguous ownership or non-framework collisions

The first implementation should not add an artifact-hash table to
`.llm_wiki/init.toml`. The current manifest does not record per-artifact hashes,
so "dirty" cannot mean "changed since the last generated hash." In this
proposal, dirty/generated drift means a recognized framework-owned artifact
exists and is about to be replaced; the archive bundle records the concrete
before and after hashes for audit. A future manifest schema may add durable
artifact hashes if rollback or stricter dirty checks become important.

## Wiki Preservation

Existing compiled `wiki/` content remains project knowledge and stays intact
outside generated sections.

Update may append to `wiki/log.md` and may add or refresh an explicitly
generated section in `wiki/index.md`. The first implementation should reuse
the current `## Schema Drift` generated section rather than stacking a second
update-history section. That section may include the latest update/folder drift
summary and point at `wiki/log.md` for durable history. Update must preserve
all catalog entries outside generated sections. It must not decide that old
wiki pages, references, or folders are obsolete. That requires semantic
judgment and belongs to `wiki-lint` or a future archival workflow.

The log entry should be fixed-shape, for example:

```text
## [YYYY-MM-DD] update | framework artifacts | <from-version> -> <to-version>

Archive: .llm_wiki/archive/artifacts/<from-version>/<timestamp>/
Tasks: framework, layout
Answers: blueprint=<blueprint>; packs=<packs>
Rewritten artifacts: <comma-separated paths>
Skipped artifacts: <comma-separated paths or "none">
Added folders: <comma-separated paths or "none">
Orphaned folders: <comma-separated paths or "none">
Search index: stale; run llm-wiki index --force
Wiki content: preserved; Schema Drift section refreshed
```

The first implementation uses the existing `update` log operation because the
current framework log vocabulary is `ingest`, `update`, `lint`, `promote`,
`archive`, and `create`.

## Effect On Init

If accepted, future implementation should simplify init:

- Fresh init remains responsible for creating a new project.
- Existing-project rerun can be deprecated or reduced to a compatibility
  handoff that says "this is an initialized project; run `llm-wiki update`."
- Any remaining init rerun path should delegate to shared answer-loading and
  rendering code rather than owning migration logic itself.
- Schema-drift behavior can move to update, or init can keep only the narrow
  no-overwrite drift audit until update fully replaces rerun usage.

This proposal does not remove current rerun behavior immediately. It defines
the direction that should let a later plan collapse rerun responsibilities
safely.

First implementation contract: update lands additively. `llm-wiki init` rerun
continues to work unchanged in the release that introduces update. A follow-up
plan decides which rerun responsibilities move to update and when initialized
project `init` calls become a handoff.

## Registry And Search Behavior

When registration is enabled, update should update the existing same-root
registry entry and keep the project id stable. It must not create a duplicate
registry entry for the updated project.

Project update never changes the cross-project `search-all` profile. That
profile belongs to `llm-wiki defaults search`.

## Observability

`llm-wiki update -v` should explain:

- command scope: project-scoped
- resolved project root
- selected update tasks
- previous and current framework versions
- manifest answers loaded and any fallback values recovered from generated
  files
- selected blueprint and packs
- archive directory, when `--framework` is selected
- artifacts classified as framework-owned
- artifacts rewritten, skipped, or considered ambiguous
- generated wiki sections refreshed
- project search configuration changes
- registry update outcome
- search-index staleness and reindex guidance
- any explicit handoff to install-owned model materialization
- recovery guidance when update fails

JSON output, if added, must remain free of diagnostic text and include the same
core facts as structured fields. The command must follow
`wiki/checklists/observability-contract.checklist.md`.

## Safety And Failure Semantics

Update should be staged:

1. resolve target project and selected tasks
2. collect or recover answers
3. render new artifacts into memory or a temp directory
4. classify old artifacts
5. create the archive bundle when `--framework` is selected
6. write a project-local update transaction marker
7. write replacement artifacts through same-directory temp files and atomic
   renames
8. write the new manifest through a same-directory temp file and atomic rename
9. update `wiki/log.md` and the generated `## Schema Drift` section through the
   same atomic-write discipline
10. run project-local reindex if `--reindex` is selected
11. mark the archive manifest complete and remove the transaction marker

If the command fails before archive creation, no project files should change.
If it fails after archive creation, the archive remains as recovery evidence
and the error message points to it. If the command fails during replacement,
the transaction marker records the intended write set, the archive directory,
and which atomic writes completed before failure. The next update or doctor run
should report that incomplete transaction before attempting another project
maintenance run. Future implementation may add an explicit rollback command,
but rollback is not required for the first proposal.

## Out Of Scope

- Binary self-update or network download of a newer `llm-wiki` executable.
- Global defaults mutation from `llm-wiki update`.
- Global skill installation, model download, license acceptance, or managed
  artifact promotion from `llm-wiki update`.
- Automatic archival, deletion, or rewriting of compiled `wiki/` content
  outside generated sections. Generated sections explicitly owned by init or
  update are excluded from this restriction.
- Semantic migration of decisions, specs, references, or roadmaps.
- User-defined packs or project-local template overrides.
- Automatic rollback command.

## Open Questions

- Should `llm-wiki update --framework --layout` be the default selected
  checklist for interactive update, or should only `--framework` be preselected?
- Should dirty recognized generated files require `--force`, or is archive plus
  manifest evidence sufficient? This proposal recommends proceeding by default
  for recognized framework-owned artifacts.
- Should `llm-wiki init` immediately refuse initialized projects once update
  exists, or keep a compatibility rerun path for one release?
- Should a later update release trigger immediate reindex by default after
  touching `wiki/index.md` or `wiki/log.md`?

## Acceptance Criteria

- `llm-wiki update` help clearly states that it is project-scoped and may write
  target-project host-local registry/cache state.
- Top-level help groups project commands separately from machine/global
  commands.
- `llm-wiki update --framework` archives previous framework-owned artifacts
  under `.llm_wiki/archive/artifacts/<from-version>/<timestamp>/`.
- `llm-wiki update --layout` replays init answers from `.llm_wiki/init.toml`
  and can recover older projects where documented fallbacks are available.
- `llm-wiki update --framework` overwrites current framework-owned artifacts
  from the current binary.
- Compiled `wiki/` content outside generated sections and raw references are
  preserved.
- Newly claimed pack folders are created; orphaned folders remain on disk.
- `wiki/log.md` records a fixed-shape `update | framework artifacts` entry.
- `wiki/index.md` preserves existing catalog entries outside the generated
  `## Schema Drift` section.
- The existing same-root registry entry is updated without changing its project
  id or creating a duplicate.
- `llm-wiki update --search` writes only project-local search configuration and
  uses install-owned handoff for missing model materialization.
- `llm-wiki defaults search` is the machine-wide search-defaults surface and
  does not rewrite existing projects.
- Update reports search-index staleness unless `--reindex` is selected.
- Verbose diagnostics satisfy the observability checklist.
- Tests cover help scope text, fresh current projects, older manifests, legacy
  manifests without framework version, recognized generated artifacts with
  changed bytes, ambiguous collisions, interrupted archives, partial
  replacement failure, registry id stability, project-local search updates,
  defaults/search separation, search staleness reporting, and no compiled
  wiki-content mutation outside generated sections.
