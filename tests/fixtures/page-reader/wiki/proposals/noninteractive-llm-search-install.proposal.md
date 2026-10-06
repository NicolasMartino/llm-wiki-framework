# Non-Interactive LLM Search Install

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-23
- Category: Install UX, search model materialization, automation
- Scope: Add an explicit non-interactive `llm-wiki install` path that enables
  the balanced LLM search profile, accepts the selected profile's model
  licenses/terms by named flag, and confirms model downloads without requiring
  a terminal prompt. Keep model materialization install-owned and treat a
  package version bump as one-off verification evidence, not as a recurring
  mechanism.
- Sources: conversational input 2026-05-23, wiki/plans/semantic-hybrid-search.plan.md, wiki/plans/idempotent-search-model-install.plan.md, wiki/references/llm-search-model-licensing.reference.md, wiki/proposals/search-model-selection.proposal.md
- Related: wiki/plans/noninteractive-llm-search-install.plan.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/references/llm-search-model-licensing.reference.md, wiki/plans/idempotent-search-model-install.plan.md, wiki/proposals/search-model-selection.proposal.md, wiki/checklists/observability-contract.checklist.md

## Source Capture

This proposal cites conversational input because the direction was introduced
in chat on 2026-05-23 while testing whether the installed `llm-wiki` binary was
actually updated. Before promotion to a decision or implementation plan,
capture the chat basis as raw source material or replace it with another
durable source path.

## Question

Should `llm-wiki install` support an explicit scripted path for enabling LLM
search, including profile selection, model download confirmation, and
license/terms acknowledgement, so automated installs do not have to choose
lexical-only search merely because they lack an interactive terminal?

## Proposal

Yes. Add a non-interactive enablement path:

```text
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

`--non-interactive` declares that the command must not prompt. `--enable-llm-search`
selects the semantic/hybrid LLM search install path without showing the
interactive posture picker. `--profile balanced` selects the existing balanced
profile explicitly. `--confirm-model-downloads` confirms search model artifact
downloads or replacement downloads. `--accept-profile-licenses` acknowledges
the licenses and terms for every model required by the selected profile.

The flags are intentionally explicit. Plain non-interactive `llm-wiki install`
should still fail closed because it cannot ask the user whether to enable LLM
search. The recommended state-independent automation command passes both
consent flags:

```text
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

That full command is safe to use even when artifacts and license records are
already current. The behavior matrix below defines the minimum required flags
for each observed machine state; portable scripts should prefer the full
command unless they intentionally want a narrower fail-closed preflight.

Scripts that do not want LLM search should choose:

```text
llm-wiki install --non-interactive --disable-llm-search
```

Interactive `llm-wiki install` keeps the current prompt-first behavior. The
existing `llm-wiki install --disable-llm-search` form remains valid for
backward-compatible automation, but documentation should prefer the explicit
`--non-interactive --disable-llm-search` spelling.

## Flag Semantics

`--non-interactive`:

- declares that the install flow may not ask terminal prompts
- requires an explicit search posture for new scripted installs:
  `--enable-llm-search` or `--disable-llm-search`
- does not approve model downloads, license acceptance, or `--force` behavior
- makes missing state-dependent confirmations fail as errors instead of
  falling back to prompts

`--enable-llm-search`:

- conflicts with `--disable-llm-search`
- requires `--non-interactive` in the first implementation
- bypasses the search posture prompt and chooses semantic/hybrid LLM search
- does not by itself accept licenses or approve downloads
- uses the same install-owned artifact planning and materialization path as
  the interactive enabled profile

`--profile balanced`:

- is required with `--enable-llm-search` for the first implementation
- accepts only the existing `balanced` profile until more profile bundles are
  accepted
- is deliberately required even though `balanced` is the only profile today, so
  scripted installs name the intended profile before future profiles exist
- should later share validation with the model/profile catalog used by search
  model selection work

`--confirm-model-downloads`:

- confirms search model artifact downloads and replacement downloads
- does not accept licenses or terms
- does not auto-answer unrelated install prompts
- does not override hash-mismatch safety; `--force` remains required for
  replacing a mismatched managed artifact

`--accept-profile-licenses`:

- applies only to the selected install profile, not the whole model catalog
- records accepted-license entries for the models required by that profile
- is required when any required model license/terms record is missing or stale
- is allowed when all licenses are already current, but then has no additional
  effect beyond making scripted intent clear

## Behavior Matrix

These are the minimum required flags for each runtime state. Passing
`--confirm-model-downloads` and `--accept-profile-licenses` in states that do
not need them is a no-op expression of scripted intent.

| State | Minimum required flags | Behavior |
| --- | --- | --- |
| Fully verified artifacts and current licenses | `--non-interactive --enable-llm-search --profile balanced` | Reuse artifacts and repair/write control-plane records without prompt; `--confirm-model-downloads` and `--accept-profile-licenses` may be accepted as harmless explicit intent |
| Missing artifacts, licenses current | `--non-interactive --enable-llm-search --profile balanced --confirm-model-downloads` | Download only missing artifacts, verify hashes, and reuse existing artifacts |
| Missing artifacts, licenses missing or stale | `--non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses` | Record license acceptance, download only missing artifacts, verify hashes, and enable the profile |
| Verified artifacts, licenses missing or stale | `--non-interactive --enable-llm-search --profile balanced --accept-profile-licenses` | Record license acceptance and enable the profile without download wording |
| Hash mismatch | `--non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses --force` | Replace only mismatched artifacts after existing force checks |

If a required flag is missing, the command must fail before downloading model
bytes or mutating managed install/search state. In particular, a partially
confirmed run such as `--accept-profile-licenses` without a required
`--confirm-model-downloads` must not write `accepted-licenses.toml`,
`models/artifacts.toml`, `search.toml`, `external-dependencies.toml`, model
bytes, indexes, the managed binary, installed skills, or the install manifest.
The error should name the missing flag and keep the current fail-closed install
posture.

## Validation Model

Validation has two layers:

1. Static CLI validation catches shape errors before install planning:
   `--enable-llm-search` conflicts with `--disable-llm-search`; `--enable-llm-search`
   requires `--non-interactive`; `--enable-llm-search` requires
   `--profile balanced`; `--profile` requires `--enable-llm-search`;
   `--confirm-model-downloads` and `--accept-profile-licenses` require
   `--enable-llm-search`.
2. Runtime validation runs after accepted licenses and model artifact
   classifications are loaded, but before any managed-state mutation. It
   decides whether `--confirm-model-downloads`, `--accept-profile-licenses`,
   and `--force` are actually required for the current machine state. Missing
   runtime confirmations fail before model bytes, license records, artifact
   records, search config, external dependency records, installed skills,
   managed binary bytes, or manifest state are written.

This split is necessary because install already plans from the actual local
state: verified artifacts, missing artifacts, hash mismatches, current
accepted-license records, and stale accepted-license records.

The implementation plan should therefore add an early non-interactive
enabled-search preflight before the existing install mutation phase. It may
reuse the same planning function later during configuration, but missing
runtime confirmations must be detected before the command starts writing
install-owned state.

## Interactive Consent

The current interactive enabled-search prompt combines license acknowledgement
and download consent when both are needed. The implementation plan should split
those into two logical gates internally:

1. license/terms acknowledgement for required profile models
2. model artifact download or replacement confirmation

The interactive UI may still present one combined confirmation when both gates
are required, but the state machine must track them separately so the
non-interactive flags map cleanly to the same decisions.

## Existing Configure Flag

`--configure-search` remains a legacy explicit-reconfiguration marker. It does
not make install non-interactive by itself.

With `--non-interactive --enable-llm-search` or
`--non-interactive --disable-llm-search`, `--configure-search` is redundant and
has no extra effect. The explicit posture flag decides the search path. Without
an explicit non-interactive posture, `--configure-search` keeps the current
interactive prompt behavior.

## Search Config Equivalence

The non-interactive enabled path must write the same search configuration as
the interactive enabled path. In the current schema, that means
`SearchConfig::enabled` writes both `[project_default]` and `[global_search]`
from the selected profile. `--disable-llm-search` likewise keeps writing the
same disabled profile as today.

## Safety Contract

Model bytes remain install-owned. Search, search-all, index, index-all,
doctor, eval, and wiki-query must continue to inspect managed state and fail
with readiness guidance rather than downloading models.

The non-interactive path must print or otherwise expose the same profile,
model, license, terms, repository, file, and SHA-256 information that the
interactive path uses before materialization. In JSON-free human output, this
can remain normal stdout because `install` is not a machine-parsed query
surface.

Accepted-license records stay model-specific: model id, license, and terms URL
must match the current embedded catalog entry. A stale license record still
requires `--accept-profile-licenses`.

## Observability Contract

This proposal changes command arguments and fail-closed safety behavior, so it
must satisfy the active observability checklist.

Verbose diagnostics for the affected install paths should explain:

- non-interactive mode selection and the selected search posture
- selected profile id and whether it came from an explicit `--profile`
- model artifact classification counts: verified, missing, hash-mismatched,
  replacement-forced
- license classification counts: current, missing, stale
- whether interactive prompts were skipped because `--non-interactive` was
  supplied
- which state-dependent confirmation was missing when the command refuses
  before mutation
- whether `--configure-search` was accepted as redundant beside an explicit
  non-interactive posture

Diagnostics must use `CliContext` / tracing stderr, keep normal stdout stable,
and never contaminate JSON output from other commands. At least one nearby
verbose install test should assert the essential facts for the enabled
non-interactive path and one refusal path.

## Out Of Scope

- Per-project search model selection or pending profile promotion.
- New model/profile bundles beyond `balanced`.
- Runtime commands downloading model artifacts.
- Changing model hashes, repository revisions, or license metadata.
- Automatic version bumping or release automation.
- Adding a broad `--yes` / answer-all-prompts install flag.
- Changing the disabled-search automation path.
- Fixing any separate qmd-rs direct-run issue observed after install.

## Implementation Touchpoints

- `src/cli.rs`: add install flags and clap conflicts/requires relationships.
- `src/install.rs`: choose the enabled search posture from flags, validate
  non-interactive confirmations before materialization, keep interactive
  prompts unchanged, and improve the non-interactive plain-install guidance.
- `src/search_models.rs`: no catalog change is expected for the first
  implementation.
- `tests/install.rs`: add redirected-`HOME` tests for missing
  `--confirm-model-downloads`, missing `--accept-profile-licenses`, conflict
  with `--disable-llm-search`, missing explicit posture under
  `--non-interactive`, successful reuse when artifacts/licenses are already
  verified, verified artifacts with stale licenses, missing artifacts with
  current licenses, hash mismatch without `--force`, hash mismatch with
  `--force`, no-op acceptance of extra consent flags when everything is
  current, no managed-state writes after missing runtime confirmation, and
  verbose diagnostics for one success and one refusal path.
- `README.md`: document the scripted enabled-search install command.
- `wiki/plans/*`: create an implementation plan if this proposal is accepted.

## Verification Expectations

Focused tests:

- `cargo test --test install`
- `cargo test --bin llm-wiki install::tests`
- `cargo test --test status_doctor`
- `cargo test --test search_commands`
- `git diff --check`

Manual proof after implementation:

1. Bump the package version once as implementation evidence, not as a
   recurring update mechanism.
2. Run `llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses --skip-path-guidance`.
3. Confirm the managed binary reports the bumped version.
4. Confirm repeated install reuses verified artifacts and does not redownload
   unchanged model bytes.
