# Search Model Selection: Per-Project and Cross-Project Profiles

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-11
- Category: Search UX, install profile, runtime state
- Scope: Let users pick the model/profile bundle used by `llm-wiki search` and the cross-project profile used by `llm-wiki search-all` without rerunning the whole install flow manually. Project choices live in project-local `.llm_wiki/search.toml`; cross-project and new-project defaults live in `~/.llm_wiki/search.toml`. Interactive `--choose-model` may record a pending choice and hand off to the install flow, but only install-owned code may accept licenses, download artifacts, verify hashes, and promote the choice to active.
- Sources: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/references/llm-search-model-licensing.reference.md, review conversation 2026-05-11 establishing pending project choices for uninstalled models
- Related: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/decisions/search-backend-selection.decision.md, wiki/references/llm-search-model-licensing.reference.md, wiki/specs/wiki-query-skill.spec.md

## Question

The accepted semantic/hybrid search proposal and active plan currently tie LLM
search configuration to a single install profile administered by `install` and
`install --configure-search`. That keeps model downloads explicit, but it makes
per-project model experimentation awkward and leaves `search-all` without a
nearby surface for changing the cross-project search profile.

How should `llm-wiki search` and `llm-wiki search-all` let users change model
selection at the relevant scope while preserving explicit install consent,
hash verification, stale-index behavior, and parseable search output?

## Proposal

Adopt two active search scopes plus pending selections:

1. **New-project default** lives in `~/.llm_wiki/search.toml` under
   `[project_default]`. `llm-wiki init` copies this into new project-local
   `[project]` state. It is administered by `install` and
   `install --configure-search`.
2. **Cross-project search profile** lives in `~/.llm_wiki/search.toml` under
   `[global_search]`. `llm-wiki search-all` reads this profile directly and can
   administer it with `search-all --choose-model`.
3. **Project search profile** lives in `<project>/.llm_wiki/search.toml` under
   `[project]`. `llm-wiki search` reads this profile and can administer it with
   `search --choose-model`.
4. **Pending choices** are written only when the user interactively chooses an
   uninstalled model/profile bundle. Pending state is not active search state.
   It becomes active only after the install flow completes license acceptance,
   artifact download, hash verification, and managed-state recording.

The command chooses the scope. Running `search --choose-model` in a project
updates that project. Running `search-all --choose-model` updates the
cross-project profile. The proposal does not add a `--global` flag to `search`.

## Contract Delta From The Active Plan

This proposal intentionally extends two parts of the active semantic/hybrid
plan if accepted:

1. The model materialization rule remains install-owned, but interactive
   `search --choose-model` and `search-all --choose-model` may launch the
   install flow after writing pending state. `search` and `search-all` do not
   download model bytes themselves, do not accept licenses themselves, and do not
   promote unverified artifacts to active state.
2. `search-all` uses one cross-project semantic/hybrid profile from
   `[global_search]`, rather than each project's `[project]` override. This is a
   product simplification for predictable readiness and debuggability. Rank
   fusion still merges per-project result lists by rank and never merges raw
   semantic similarity scores.

The current reference says search commands must not become implicit download
paths. This proposal keeps that rule by making the handoff explicit and
interactive: only a user-selected `--choose-model` path can start install, and
the active profile remains unchanged when install is cancelled or fails.

## Profile Bundle Shape

Model selection is a profile bundle, not a single embedding-model string.
Baseline `hybrid` needs at least an embedding model and a query-expansion model;
`semantic` needs the embedding model; `--rerank` additionally needs the reranker.

The active profile records model IDs and profile intent. Artifact hashes, local
paths, immutable repository revisions, accepted-license records, and download
provenance remain in the managed install records required by the active plan and
licensing reference.

## File Layout

**Global**: `~/.llm_wiki/search.toml`

```toml
[project_default]
# Copied into new projects by `llm-wiki init`.
llm_search_enabled = true
profile = "balanced"
embedding_model = "<embedding-model-id>"
query_expansion_model = "<query-expansion-model-id>"
reranker_model = "<optional-reranker-model-id>"

[global_search]
# Used by `llm-wiki search-all` directly.
llm_search_enabled = true
profile = "balanced"
embedding_model = "<embedding-model-id>"
query_expansion_model = "<query-expansion-model-id>"
reranker_model = "<optional-reranker-model-id>"

[pending_global_search]
# Present only while `search-all --choose-model` has selected an uninstalled
# bundle and the install handoff has not completed successfully.
profile = "<pending-profile>"
embedding_model = "<pending-embedding-model-id>"
query_expansion_model = "<pending-query-expansion-model-id>"
reranker_model = "<pending-optional-reranker-model-id>"
status = "install_required"
requested_by = "llm-wiki search-all --choose-model"
```

If the user declines LLM search during install, the relevant active section
still exists with `llm_search_enabled = false` and no required model bundle.
Then `auto` resolves to lexical, matching the active plan's declined-search
contract.

**Project-local**: `<project>/.llm_wiki/search.toml`

```toml
[project]
llm_search_enabled = true
profile = "balanced"
embedding_model = "<embedding-model-id>"
query_expansion_model = "<query-expansion-model-id>"
reranker_model = "<optional-reranker-model-id>"
source = "project_default"
source_install_id = "<install-id-or-hash>"

[pending_project]
# Present only while `search --choose-model` has selected an uninstalled bundle
# and the install handoff has not completed successfully.
profile = "<pending-profile>"
embedding_model = "<pending-embedding-model-id>"
query_expansion_model = "<pending-query-expansion-model-id>"
reranker_model = "<pending-optional-reranker-model-id>"
status = "install_required"
requested_by = "llm-wiki search --choose-model"
```

`llm-wiki init` creates the project-local file by copying
`[project_default]` into `[project]`. Existing projects are not rewritten when
`[project_default]` changes later.

The picker uses a versioned runtime model catalog embedded in the binary and
derived from the licensing reference. Runtime commands must not parse wiki
reference pages to discover model candidates.

## Command Surface

```text
llm-wiki search --choose-model "question"
llm-wiki search --model <id-or-profile> "question"
llm-wiki search-all --choose-model "question"
llm-wiki search-all --model <id-or-profile> "question"
```

**`search --choose-model`** (interactive, project scope):

1. Requires an initialized project. It reads project-local `[project]` when
   present, or uses the legacy-project behavior defined below when the file is
   absent.
2. Lists installed profile bundles and available-but-uninstalled bundles from
   the embedded model catalog plus managed records under `~/.llm_wiki/models/`.
3. If the chosen bundle is already installed and verified, writes it directly to
   project-local `[project]`, clears any matching `[pending_project]`, marks the
   old project semantic index stale, and continues to readiness checks.
4. If the chosen bundle is not installed, writes `[pending_project]`, then
   launches the install handoff for that pending bundle. The user-facing effect
   should be equivalent to running an install-owned command such as
   `llm-wiki install --configure-search --activate-pending project`; the
   implementation may call the same handler internally rather than spawning a
   second process.
5. The install handoff owns license/terms acknowledgement, download, immutable
   revision pinning, SHA-256 verification, accepted-license records, and
   managed artifact records.
6. If install succeeds, promote `[pending_project]` into `[project]`, clear the
   pending section, mark the old project semantic index stale, and continue to
   readiness checks.
7. If install is cancelled or fails, keep the previous `[project]` active. If no
   previous active LLM profile existed, `auto` remains lexical. Report the
   pending choice as inactive and do not run semantic/hybrid search with it.
8. After a successful active-profile change, run the query only when a
   compatible index and configured thresholds already exist. Otherwise stop with
   `semantic_index_missing`, `semantic_index_stale`, or
   `thresholds_unconfigured` guidance.

**`search-all --choose-model`** (interactive, cross-project scope):

1. Uses the same picker and pending-install handoff.
2. Writes installed, verified bundles to global `[global_search]`.
3. Writes uninstalled choices to `[pending_global_search]`, launches the install
   handoff, and promotes to `[global_search]` only after install succeeds.
4. Marks cross-project semantic readiness stale for any registered project that
   lacks an index keyed to the new `[global_search]` profile.
5. Runs the cross-project query only for projects with compatible global-profile
   indexes, or reports skipped projects and `llm-wiki index-all` guidance.

**Scripted sibling `--model <id-or-profile>`** (non-interactive, both commands):

1. Selects an already installed, verified model/profile bundle for this
   invocation only.
2. Fails fast with `model_not_installed` if any required artifact for the
   requested mode is missing from managed install records.
3. Does not trigger downloads, does not prompt for license acceptance, and does
   not write to either `search.toml`.
4. Still performs normal readiness checks: compatible semantic index, chunking
   strategy, adapter schema, qmd-rs version, model artifact hashes, embedding
   dimensions, and configured thresholds.

## Resolution Order

`search` (single project):

1. CLI `--model <id-or-profile>` as a one-shot override, installed and verified.
2. Project-local `[project]`.
3. Legacy-project lexical behavior when `.llm_wiki/init.toml` exists but
   `.llm_wiki/search.toml` does not.
4. Hard error with `llm-wiki init` guidance when the directory is not an
   initialized project.

`search-all`:

1. CLI `--model <id-or-profile>` as a one-shot cross-project override,
   installed and verified.
2. Global `[global_search]`.

`[project_default]` is not a search-time fallback. It exists only to seed new
projects at init time.

## Project Without Local Search State

Newly initialized projects should always have `.llm_wiki/search.toml`. The file
can represent either an enabled LLM profile or a declined/disabled profile.

For older initialized projects that have `.llm_wiki/init.toml` but no
`.llm_wiki/search.toml`, no-flag `search` remains usable:

1. `auto` selects lexical with reason `project_profile_missing`.
2. Explicit `--mode semantic` or `--mode hybrid` fails with guidance to run
   `llm-wiki search --choose-model` or the relevant repair/configure command.
3. `search --choose-model` may create the project-local file through the normal
   installed-or-pending flow.

For directories with no initialized project metadata, `search` hard-errors with
`llm-wiki init` guidance. It does not silently read `[project_default]` from the
global file.

## Search-All Indexing Contract

`search-all` semantic/hybrid retrieval uses the `[global_search]` profile or a
non-interactive `--model` override. A registered project's own `[project]`
override does not change the profile used by `search-all`.

Index stores are keyed by project plus model/profile metadata, so multiple
semantic stores may coexist for one project:

1. `llm-wiki index` builds or refreshes the current project's `[project]` index.
2. `llm-wiki index-all` builds or refreshes registered projects for
   `[global_search]` so they can participate in `search-all`.
3. If a project's `[project]` profile matches `[global_search]`, the same
   compatible semantic store can satisfy both commands.
4. If they differ, the project may have one store for project-local search and a
   second store for cross-project search.
5. Changing `[global_search]` invalidates only the search-all-compatible store
   set. It does not rewrite project-local `[project]` choices.

`search-all` still merges per-project result lists by reciprocal rank fusion or
an equivalent rank merge. It does not merge raw semantic similarity scores.

## Install Flow Integration

`install` and `install --configure-search` keep owning model materialization and
global search configuration. The global file now has two active sections:

- `[project_default]` - copied into new projects during init.
- `[global_search]` - used by `search-all` immediately.

On first LLM-search enablement, both sections default to the same profile
bundle. Later `install --configure-search` may let the user set them
independently.

When install is launched from a pending `--choose-model` selection, it receives
the target scope and pending bundle, materializes all required artifacts, then
promotes the pending section to active only after verification succeeds.
Interrupted installs leave pending state inactive; subsequent search commands
must not treat pending state as selected search state.

## JSON Output

`search` and `search-all` JSON output should include:

- `requested_mode`, `selected_mode`, fallback reason, readiness reason, and
  zero-result reason as already required by the accepted proposal.
- `profile`, `embedding_model_id`, `query_expansion_model_id` when used,
  `reranker_model_id` when used, and artifact hashes for every semantic,
  expansion, or rerank artifact that actually ran.
- `model_source`: one of `cli_override`, `project`, or `global_search`.
- `model_choice_state`: one of `active`, `pending_install`, or
  `inactive_pending_after_failed_install` when relevant.
- `model_choice_path`: the absolute path of the active file consulted, such as
  `/Users/x/proj/.llm_wiki/search.toml` or
  `/Users/x/.llm_wiki/search.toml`.

`project_default` should not appear as a search-time `model_source` because it
is an init template, not a runtime retrieval source.

## Skill Contract Impact

`wiki-query` operates inside a single project, so baseline behavior reads
project-local `[project]` through ordinary `llm-wiki search`. No skill-level
model selection is required for normal use.

`wiki-query` invocations that need deterministic behavior across machines can
pass `--model <id-or-profile>`. That path remains non-interactive and fails
fast when the requested bundle is not installed or the compatible index is not
ready. Skill automation should not use `--choose-model` because it may prompt.

## Why

1. **Per-project experimentation needs a low-cost surface.** Users should be
   able to try a domain-appropriate profile from the command they already use,
   without changing every project on the machine.
2. **Pending state keeps active search state honest.** A selected but
   uninstalled model is not runnable. Writing it as active would create broken
   defaults after cancelled or failed installs.
3. **Install remains the materialization boundary.** Search commands can launch
   the install flow after explicit user selection, but they do not duplicate or
   bypass license, download, hash, or provenance handling.
4. **Cross-project search gets a clear profile.** A single `[global_search]`
   profile makes search-all readiness, index-all behavior, and user support
   easier to explain while still preserving rank-based fusion.
5. **Non-interactive callers remain deterministic.** `--model <id-or-profile>`
   never prompts, never downloads, and never changes saved defaults.

## Alternatives Considered

1. **Single global model, `install --configure-search` only.** Rejected because
   per-project model selection is a real need and forcing every change through a
   separate install command is too heavy for A/B testing.
2. **Write uninstalled selections directly to `[project]`.** Rejected because
   interrupted or failed installs would leave the project pointing at a model
   that cannot run. Pending state preserves the previous active profile.
3. **Let `search --choose-model` download directly.** Rejected. It would create
   a second materialization path and weaken the licensing/hash/provenance
   contract owned by install.
4. **`search --choose-model --global` for the global scope.** Rejected because
   it conflates project and cross-project administration in one command.
5. **Allow `search-all` to honor per-project semantic overrides.** Rejected for
   this proposal as a product contract, not because rank fusion requires shared
   vector spaces. The active plan's heterogeneous-project design remains
   technically valid, but this proposal prefers one explicit cross-project
   profile for supportability.
6. **Fall back from missing `[project]` to `[project_default]`.** Rejected.
   `[project_default]` is a template. Search-time fallback would obscure which
   profile produced results. Legacy initialized projects instead get explicit
   lexical behavior until repaired or configured.

## Consequences and Tradeoffs

- The global `search.toml` schema splits into `[project_default]`,
  `[global_search]`, and temporary `[pending_global_search]`.
- Project `search.toml` gains `[project]` and temporary `[pending_project]`.
- Profile records must name complete bundles, including embedding and
  query-expansion models for hybrid. Reranker remains optional and is required
  only when `--rerank` is requested.
- `init` gains a one-shot copy step from `[project_default]` to project
  `[project]`, including disabled-profile state when LLM search was declined.
- `search --choose-model` and `search-all --choose-model` gain an install
  handoff. This improves UX but means these flags are interactive
  administration surfaces, not pure retrieval flags.
- Changing a project profile invalidates that project's project-local semantic
  store. Changing `[global_search]` invalidates search-all-compatible semantic
  stores and requires `llm-wiki index-all` or equivalent guidance.
- JSON output grows model/profile provenance fields. Existing consumers that
  ignore unknown fields are unaffected.
- `wiki-query` behavior is unchanged for ordinary use and remains
  non-interactive unless a caller deliberately invokes `--choose-model`.

## Promotion Path

This proposal extends the accepted semantic/hybrid search proposal and its
active plan. On acceptance, fold the scope contract into:

1. Stage 1 for install profile, pending state, profile-bundle schema, and
   install handoff.
2. Stage 2 for multiple model-keyed semantic stores per project.
3. Stage 3 for resolution, readiness, JSON metadata, and legacy missing-profile
   behavior.
4. Stage 5 for the stricter `[global_search]` search-all profile and
   `index-all` rebuild behavior.

Do not promote to a durable decision until:

1. The semantic/hybrid plan ships at least the global-only search profile or is
   explicitly updated to include this proposal before implementation.
2. Behavioral evidence shows per-project overrides are useful enough to justify
   the schema split, for example dogfood sessions or evals where the best
   project-local profile differs from the cross-project profile.
3. The runtime model catalog exists as binary-owned data derived from the
   licensing reference.
4. A human-curated raw note or equivalent durable source captures the
   conversation requirement for pending local choices before this proposal is
   promoted to a decision.

## Remaining Questions

1. What exact install handoff flag should be exposed publicly, if any, versus an
   internal command-handler handoff?
2. Should `search-all --choose-model` warn before changing `[global_search]`,
   given that it can make many registered projects temporarily ineligible for
   semantic/hybrid `search-all`?
3. Should `llm-wiki doctor` show every registered project's `[project]` profile
   alongside `[global_search]` and report whether project-local and search-all
   semantic stores both exist?
4. Should `search --choose-model` leave failed pending state for inspection or
   clear it automatically after reporting the failed install?
