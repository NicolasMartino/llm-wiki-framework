# D10 Composable Project Init — Pre-merge Review

Branch: `d10-composable-project-init` vs `master`
Scope reviewed: 74 files / +4292 / −653. Focus on Rust source under `src/init/`,
build glue, base + pack templates, and integration tests. Wiki narrative files
are skimmed for consistency only — they don’t affect behavior.

Review criteria, per the user’s ask: **code quality, safety, legibility over
cost**. Findings are graded:

- **Blocker** — fix before merge.
- **Should-fix** — fix before merge if cheap; otherwise file a follow-up.
- **Nit** — style/clarity, optional.

There are no Blockers. The branch is mergeable. Several Should-fix items below
are worth landing as a small follow-up commit.

---

## Summary

The branch replaces the old `--type` / `--scale` enum with a `--blueprint` +
repeatable `--pack` model. It introduces an Askama template engine for both the
base `AGENTS.md` / `project_guidelines.md` and per-pack fragments, a manifest
file (`.llm_wiki/init.toml`), a framework-collision preflight, and an
`--initial-sources` copier.

Strengths:

- Pure data layer (`compose.rs`) cleanly separates planning from filesystem
  side-effects (`scaffold.rs`).
- Enum-based `Blueprint` and `Pack` with exhaustive matches, round-trip
  `FromStr`/`Display` tests, and a sanity test that all default packs are in
  the catalog.
- Snapshot tests cover seven blueprint/pack matrices end-to-end via the
  installed binary, plus collision and retired-flag rejection.
- `Option<Vec<Pack>>` correctly distinguishes "use blueprint defaults" (`None`)
  from "explicitly empty" (`Some(vec![])`), and there is a dedicated test for
  that semantic (`compose.rs` line 338).
- Build script validates every skill markdown and reads every template `.md` at
  compile time — invalid Askama syntax fails `cargo build`, not runtime.

Themes worth addressing:

1. Whitespace artifacts in rendered templates from non-trimming Askama tags.
2. A few API shapes that overstate uncertainty (`Result<Option<String>>` that
   never returns `None`).
3. A partial-failure path around `--initial-sources` that strands a half-built
   project and blocks re-run recovery.
4. Some duplication that could be folded together.

---

## File-by-file

### `Cargo.toml`

- Fine. New dep is `askama = "0.16"` in workspace deps and `[dependencies]`.
- Pinned to compatible major versions; no `*` or `git` deps.
- `tempfile` remains a runtime dep (not just dev) — already used by
  `src/install.rs` and `src/manifest/schema.rs` for atomic writes via
  `NamedTempFile::new_in`. Not introduced by this branch; mention only so a
  reviewer doesn’t flag it as new. **No action.**

### `build.rs`

- Validates skill markdown via `llm_wiki_schema::parse` and walks `templates/`
  to read every `.md` (forces a panic if a template file is unreadable). Good
  belt-and-suspenders.
- **Should-fix (Nit):** `validate_templates` only emits
  `cargo:rerun-if-changed` for individual files. When a *new* `.md` is added
  under `templates/packs/foo/`, Cargo doesn’t know to re-run, because no
  tracked file mtime changed. Adding
  `println!("cargo:rerun-if-changed=templates");` near the top would also
  cover directory creation/deletion. Low risk because Askama itself fails the
  build if a referenced template is missing, but the build script can still
  serve stale data on the first incremental build after adding a new pack
  template until something else triggers a rebuild.
- `entry.expect("template directory entry")` will panic on a transient I/O
  error during build; acceptable in a build script.

### `src/cli.rs`

- `--blueprint` is `Option<String>`, parsed in `answers.rs`. Could be
  `Option<Blueprint>` via a clap `value_parser`/`ValueEnum`, which would push
  validation into clap and produce nicer errors automatically. **Nit.**
- `--type` and `--scale` are kept as hidden `Option<String>` flags so the
  retired-flag rejector in `answers.rs` can produce a specific message instead
  of clap’s "unknown argument". Good UX move.
- `--pack` (renamed from `packs`) is `Vec<String>` — repeatable. Good.

### `src/init/mod.rs`

- Module visibility split: `answers`, `command`, `scaffold` private; the rest
  public. Public surface includes `compose`, `manifest`, `profile`, `template`,
  `blueprints`, `packs`, `collision`, `sources`. Wider than necessary —
  `template` and `profile` could be `pub(crate)` since nothing outside `init`
  uses them. **Nit.**

### `src/init/command.rs`

- Two-line wrapper: parse answers, scaffold. Clean.

### `src/init/answers.rs`

- Clear separation between non-interactive and interactive flows; clones are
  explicit and minimal.
- **Should-fix:** `parse_choice_name` round-trips through
  `format!("{} - {}", name, description)` and splits on `" - "`. Pack/blueprint
  descriptions today don’t contain `" - "`, but the contract isn’t enforced
  anywhere. Consider either (a) adding a debug-assert or test that
  `description().contains(" - ")` is false for every variant, or (b) using
  `inquire`’s typed `Select<T>`/`MultiSelect<T>` which avoids string parsing
  entirely.
- `from_args` non-interactive path returns `packs: parse_cli_packs(...)?`,
  which can be `None` (→ resolve blueprint defaults later). The interactive
  path always returns `Some(...)`. This `None`-vs-`Some([])` distinction is
  load-bearing — `compose::compose` and the test
  `explicit_empty_pack_selection_does_not_fall_back_to_blueprint_defaults`
  rely on it. Worth a one-line comment in `Answers` so the next reader doesn’t
  "simplify" it.
- `reject_retired_flags` is a nice migration affordance.

### `src/init/blueprints.rs`

- Self-contained, exhaustive, well-tested. No issues. The Serialize/Deserialize
  derives aren’t used today (the manifest serializes via `Display`/string), but
  they’re cheap and future-proof against a TOML-driven blueprint format.

### `src/init/packs.rs`

- Largest file (415 lines). The bulk is mechanical mapping between the `Pack`
  enum and static catalogs (`folders`, `doc_types`, `status_vocab`) plus 20
  per-pack Askama unit-struct templates declared via the `fragment_template!`
  macro.
- **Should-fix (clarity):** `agents_fragment` and `guidelines_fragment` return
  `Result<Option<String>>` but always return `Some(_)`. Either change the
  signature to `Result<String>` (and let callers wrap with `Some` if they
  want), or genuinely return `None` for packs without a fragment (e.g. a pack
  with empty content). The current shape forces every caller to do
  `if let Some(fragment) = ...` for no reason.
- Pack templates are unit structs (no fields). Askama is being used here only
  for compile-time validation and a uniform render path — semantically these
  could be `include_str!`s. Keeping Askama is fine for consistency with the
  base templates that *do* have fields, but worth knowing if the macro
  expansion ever feels heavy.
- `render_fragment` does `format!("{}\n", rendered.trim())` — normalizes
  fragments to "no leading/trailing whitespace, exactly one trailing newline".
  Good, predictable contract.
- `status_vocab` collapses `Pack::Ops` and `Pack::OpsLite` to the same
  `OPS_STATUS` constant. Combined with `dedupe_status_vocab` keying only on
  `document_class`, this is fine *today*; if the two ever diverge, the second
  pack’s entries would silently lose. **Nit:** key the dedupe on
  `(document_class, statuses)` (compare slices) and either error or merge if
  there’s a genuine conflict.

### `src/init/compose.rs`

- The heart of the change. Pure function: `RenderPlan -> Result<InitOutput>`,
  no I/O. Excellent for testing.
- `RenderPlan.packs: Option<Vec<Pack>>` semantics are correct (see notes on
  `answers.rs`).
- `dedupe_packs` uses `BTreeSet`, which means **the resolved pack order is
  alphabetical, not user-supplied**. The manifest test
  `init_manifest_records_resolved_blueprint_packs` actually depends on this:
  it asserts the recorded order is `[ml, data, research]` — but
  `Pack::default_packs(MlResearch)` returns them in that order *and* it
  happens to be the `BTreeSet` sort order under `Ord` (the enum variants are
  declared in that order). **Should-fix:** either order is fine, but pick one
  intentionally. Today the test passes by coincidence of variant declaration
  order matching the human-meaningful order. If someone reorders the enum
  variants for readability, the manifest test will start failing in a way
  that suggests a real bug. One fix: make `dedupe_packs` preserve insertion
  order via `IndexSet` or a manual `seen` HashSet + Vec. Another: write an
  explicit comment that `Pack::ALL` order is the canonical sort order and
  must be kept stable.
- `agent_catalog_fragments` (lines 147-163) and `catalog_fragments` (lines
  165-198) duplicate the doc-type table generation. **Should-fix:** factor a
  helper `doc_types_table(packs) -> Option<String>` and call it from both.
  The status-vocab block remains only in the guidelines version.
- The two table builders use raw `format!` + `push_str`. Fine, but if the
  output ever grows, an Askama template would be more consistent with the
  rest of the file. **Nit.**
- `Utc::now().date_naive()` is called in `index_md` and `log_md`; the test
  filter `\d{4}-\d{2}-\d{2}` neutralizes this for snapshots. Determinism for
  tests is fine.
- Tests in this file cover: blueprint default resolution, explicit override,
  dedupe, and explicit-empty-vs-defaults. Strong coverage for a pure
  function.

### `src/init/template.rs`

- `ProjectGuidelinesTemplate` and `AgentsTemplate` use Askama with
  `escape = "none"`. Correct for markdown.
- **Should-fix:** `compact_blank_lines` is applied to project_guidelines but
  *not* to AGENTS.md. The baseline snapshot shows two trailing blank lines on
  AGENTS.md (no packs → `{% for fragment %}` loop yields just whitespace).
  Either apply `compact_blank_lines` to both, or — better — add Askama
  whitespace control (`{%- for ... -%}`, `{%- if ... -%}`) and stop relying on
  a post-pass. The current rendered output still leaks blank lines into the
  middle of bullet lists and code blocks when ML/AI conditionals are off
  (visible in `init_baseline.snap` lines 191–192, 207–209, 230–238). These
  aren’t bugs — markdown still renders — but they hurt legibility of the
  file the user’s agents are about to read every day.
- `ml_ai_types: if profile.include_ml_ai { ", experiment, eval" } else { "" }`
  embeds presentation logic in Rust. **Nit:** push this into the template
  with `{% if include_ml_ai %}, experiment, eval{% endif %}` for symmetry
  with the rest of the template’s conditionals.

### `src/init/scaffold.rs`

- **Should-fix (small ordering issue):** `fs::create_dir_all(path)` runs
  *before* `refuse_framework_collision`. If the user passes a path that
  doesn’t exist and the collision check then bails (only possible if some of
  the listed artifacts are inside the freshly-created directory, which can’t
  happen), the freshly-created empty directory remains. Today this is
  harmless because the collision check requires pre-existing artifacts that
  can only exist in a pre-existing directory. But if the order ever changes
  (e.g. someone adds a check that *can* fail on a new directory), we’ll be
  leaking empty directories. Trivial swap: collision check first, then
  `create_dir_all`.
- File ordering: spine folders → files → initial sources → manifest. Manifest
  is written last, which is the right marker for "init succeeded".
- **Should-fix (safety / recovery):** the current write order leaves a real
  stranded-project path when `--initial-sources` fails. `create_project`
  writes folders and top-level files first, then calls `copy_initial_sources`.
  If one source is missing or unreadable, init exits with an error *after*
  `AGENTS.md`, `project_guidelines.md`, `wiki/`, etc. were already created,
  but *before* `.llm_wiki/init.toml` is written. A subsequent re-run then
  fails the framework-collision check and cannot recover in place. This is
  more than a cosmetic lack of rollback: it turns one bad source path into a
  manual cleanup requirement. Fix options: pre-validate all initial sources
  before writing anything, or stage source-copy planning ahead of scaffold,
  or explicitly support repair/retry when the manifest is absent.
- The literal `"See @AGENTS.md.\n"` for `CLAUDE.md` would be cleaner as a
  named constant.

### `src/init/sources.rs`

- Copies `--initial-sources` into `raw/initial/<timestamp>/sources/` plus a
  `manifest.md`.
- **Safety, low priority:** symlink handling is permissive in ways that are
  worth documenting or tightening. At the top level, `source.is_dir()`
  follows directory symlinks, so a symlinked directory passed via
  `--initial-sources` is traversed. For files, `fs::copy` copies the target
  contents rather than preserving the symlink. Inside recursive traversal,
  `entry.file_type()?.is_dir()` does *not* recurse into symlinked directories,
  so the behavior is also asymmetric. This is user-opt-in rather than a
  vulnerability, but `symlink_metadata`-based handling would make the policy
  explicit and easier to reason about.
- `Utc::now()` is called twice (directory name vs `Copied At`) — would be
  one second off if straddling a boundary. Cosmetic.
- `unreachable!("unbounded retry loop always returns")` on `for attempt in
  2..` — technically reachable on `usize::MAX` overflow. Practically fine.

### `src/init/collision.rs`

- Hardcoded list of framework artifacts; correct and exhaustive for what
  init writes.
- Note: also blocks `--existing` against a directory that already happens to
  contain a `wiki/` directory unrelated to the framework. Today this is the
  intended behavior (per the related decision doc — refuse to clobber);
  worth a one-line doc comment so the policy is explicit.

### `src/init/manifest.rs`

- Trivial, correct. `framework_version` from `env!("CARGO_PKG_VERSION")`. The
  `Pack` enum’s `#[serde(rename_all = "kebab-case")]` produces stable strings
  in the manifest file.
- **Nit:** the test asserts pack order in the TOML array. As called out
  under `compose.rs`, that ordering currently rides on a `BTreeSet` of the
  enum’s declaration order. Worth making explicit.

### `src/init/profile.rs`

- Three-bool struct. Fine.
- **Nit:** `is_existing` flows in but is currently only read in `compose.rs`
  to decide on `CODE_FOLDERS`. The base template doesn’t see it (no
  `is_existing` template var). The repo-structure code block in
  `project_guidelines.md` always documents `src/`, `tests/`, `scripts/`,
  `infra/` — so an existing-project guidelines doc describes folders that
  weren’t created. Either thread `is_existing` into the template and gate
  those lines, or drop the doc entries (relying on whatever the existing
  project actually has). Visible in `init_is_existing.snap` line 234.

### Templates (`templates/base/*`, `templates/packs/*/*`)

- Base templates are well-organized; the conditionals are the main legibility
  issue (see whitespace note above).
- Pack fragments are short and use a consistent `## <Pack> Pack` heading
  followed by 3-5 bullets. Good.
- No trailing-newline policy is enforced uniformly across the pack templates;
  `render_fragment` normalizes them, so this is fine in practice.

### Tests (`tests/init.rs`, `tests/snapshots/*`)

- Coverage is strong: seven blueprint/pack profiles via snapshots, plus
  collision, retired-flag rejection, manifest contents, AGENTS pack-table
  rendering, and `--initial-sources` copy.
- **Should-fix:** there is no failure-path test for `--initial-sources`.
  Add one where one supplied source is missing or unreadable, and assert the
  intended contract explicitly: either init fails before writing any scaffold,
  or a partially initialized directory remains recoverable by rerunning init.
- `init_profiles_match_snapshots` filters `\d{4}-\d{2}-\d{2}` — covers both
  index/log dates. The HHMMSS timestamp inside `raw/initial/<timestamp>/...`
  is not reached by these snapshots (only the top-level files are read), so
  no flakiness there.
- `find_file_with_contents` does a recursive scan and reads each file —
  acceptable for tiny test trees.
- **Nit:** the snapshot tests run via `assert_cmd::Command::cargo_bin` which
  rebuilds the binary the first time. CI cost rises if the matrix grows.
  Already includes seven cases; consider gating the largest two (`ml_research`,
  `ops_infra`) behind an explicit fixture-update task if iteration speed
  becomes a concern. **Not now.**
- The test that asserts manifest pack ordering should ideally call out
  *why* that order is expected (sorted enum-declaration order vs
  user-input order) — see `compose.rs` note.

---

## Cross-cutting findings

1. **Whitespace artifacts in rendered docs (Should-fix).** Single biggest
   legibility issue. Fix with consistent Askama `{%-`/`-%}` tags or apply
   `compact_blank_lines` to AGENTS too. `init_baseline.snap` is the cleanest
   reproducer.
2. **`Result<Option<String>>` that never returns `None` (Should-fix).** Trim
   to `Result<String>`.
3. **`--initial-sources` can strand a half-initialized project
   (Should-fix).** Core files are written before initial sources are fully
   validated. One bad source path leaves framework artifacts on disk without a
   manifest, and the collision guard then blocks a clean retry. Pre-validate
   sources or define a repair/retry path.
4. **Pack ordering is implicit (Should-fix).** `BTreeSet`-based dedupe
   produces enum-declaration order, and the manifest test depends on it.
   Either preserve insertion order or document/lock the contract.
5. **`scaffold.rs` orders `create_dir_all` before the collision check
   (Should-fix).** Swap them.
6. **Existing-project guidelines documents folders not created (Nit).**
   Thread `is_existing` into the template, or drop the lines.

---

## Verification

Recommended pre-merge checks (none run by this review):

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` (the snapshot tests are the meaningful coverage)
- Add a regression test for invalid `--initial-sources` input and assert the
  desired failure atomicity / rerun behavior.
- Spot-build a project with `--blueprint custom --pack ops --pack security`
  and read the resulting `AGENTS.md` / `project_guidelines.md` end-to-end to
  confirm whitespace looks acceptable.

---

## Verdict

Mergeable. The six Should-fix items are local, well-scoped, and could
reasonably land as a single follow-up commit on `main` rather than blocking
this branch. Nothing here suggests an arbitrary path-traversal escape, but the
current `--initial-sources` flow does create a recoverability problem on
failure and should be tightened.
