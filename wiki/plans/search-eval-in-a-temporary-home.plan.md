# Plan: The Search Eval Runs In A Temporary Home

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-10-07
- Category: Tooling, search eval
- Scope: Carry out P23 of the framework roadmap: one `just` recipe sets up a
  temporary home, registers and indexes the eval's project there with its
  managed models, and runs the hand-run search eval against it, leaving the
  machine's real llm-wiki setup untouched; the eval page names the recipe.
- Sources:
  - Issue #49, "Tooling: Add a recipe that runs the search eval in a
    temporary home"
  - The gap met in poman's PM1 and PM2 (2026-10-06 and 2026-10-07)
  - `tests/natural_language_search_eval.rs`, `src/paths.rs`,
    `src/instance.rs`, the `justfile` and
    `wiki/evals/natural-language-search.eval.md` at `cad8988`, read for this
    plan
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P23 (this plan)
  - `wiki/evals/natural-language-search.eval.md` (the eval this recipe runs)
  - `wiki/decisions/test-instance-namespaced-binary.decision.md` (the test
    build's own names and folders)
  - `wiki/plans/search-ranking-weights-and-phrase-fallback.plan.md` (P16,
    which asks for a replay of this eval)

## What This Proves

Anyone can run the search eval on any machine with the managed models
installed, with one command, and the machine's registry, indexes and settings,
and the checkout's own files, are the same afterwards as before.

## Where It Stands (2026-10-07, at `cad8988`)

- `natural_language_eval_runs_against_managed_models` in
  `tests/natural_language_search_eval.rs` is ignored by default; its
  `#[ignore]` text says it needs managed model artifacts and a fresh semantic
  index, and to run it with `cargo test --test natural_language_search_eval
  -- --ignored --nocapture`.
- Each case runs the test-built `llm-wiki search --project
  llm-wiki-framework-semantic-search --mode <mode> --format json` in all
  modes. The project id is a constant in the test; nothing in the repository
  registers it, so on a machine without it every case fails with "project id
  ... is not registered".
- The child process inherits the test's environment, so `HOME` and the XDG
  variables set by a recipe reach it.
- On Unix, llm-wiki finds everything from `HOME`: the managed home
  (`~/.llm_wiki`, or the test instance's own name), which holds the models
  (`models/`, with `artifacts.toml`), the accepted licenses and the indexes;
  the cache and data homes follow `XDG_CACHE_HOME` and `XDG_DATA_HOME`, else
  `~/.cache` and `~/.local/share` (`src/paths.rs`).
- The eval page (`wiki/evals/natural-language-search.eval.md`) gives the
  `cargo test` command and the project id, and its older runs index the
  project with `cargo run -- index --project
  llm-wiki-framework-semantic-search --force`, but no page says how to
  register it.
- The `justfile` has no recipe for the eval.
- Two writes land in a project's own folder, not the home:
  - `llm-wiki register <root>` without `--no-mcp` wires `<root>/.mcp.json` to
    the managed binary of the current home (`src/registry/mod.rs`,
    `src/mcp_wiring.rs`); under a temporary `HOME` that binary is deleted
    with the home. `.mcp.json` is not in `.gitignore`.
  - A project's search settings come from `<root>/.llm_wiki/search.toml`
    first, and from the home's `search.toml` only when that file is absent
    (`project_search_profile`, `src/search/commands.rs`). In an Orca
    worktree, `tools/wiki-worktree.sh` writes that file with meaning-based
    search off, and the worktree's own registered project reads it.
  Found by this plan's blind review (PR #59, finding 3).

## Target

- **One recipe**, `just search-eval` (choice 2), that:
  - makes a temporary home and points `HOME` and the XDG variables into it;
  - gives that home copies of the real one's small records: the model
    records (`models/artifacts.toml`), the accepted licenses, the search
    settings, and the install manifest, without which search stops with
    "llm-wiki install is required before search" (found by the first run).
    The model records name the real home's model files by absolute path, so
    index and search read those files where they are (choice 1);
  - stops before building anything, with one plain line naming `llm-wiki
    install --configure-search`, when the real home lacks one of those
    records or has meaning-based search off in its default settings;
  - makes the eval project's own root inside the temporary directory: a copy
    of the checkout's `wiki/` and `AGENTS.MD` (register wants an orientation
    file), with the home's default search settings as its own
    `.llm_wiki/search.toml`, never the checkout itself;
  - registers that root under `llm-wiki-framework-semantic-search` with
    `--no-mcp`, and indexes it;
  - runs the ignored eval test, which writes its report under `target/` as
    today;
  - removes the temporary home on the way out, success or failure.
- **The eval page** names the recipe where it says how to run the eval.
- **A quick test of the recipe's stops** (`tools/search-eval-test.sh`, in
  `just verify` and the fast check), since nothing else runs the script: a
  later edit to its checks would otherwise go unseen until someone ran the
  20-minute eval. Asked by the blind review of PR #67, finding 6.

## Done When

- On a machine where `llm-wiki-framework-semantic-search` is not registered,
  the recipe runs the eval to its end, recorded in this plan with the
  command and the counts it printed.
- The real registry, the real managed home, the real cache, and the
  checkout's `.mcp.json` and `.llm_wiki/` hold the same files with the same
  contents before and after the run (or stay absent), shown by a listing
  with checksums taken before and after, recorded once.
- With no managed models in the real home, the recipe stops with the one
  line of the Target, shown once.
- The eval page names the recipe.
- The fast check passes on the PR into `develop`.

## Runs (2026-10-07)

All on one machine where `llm-wiki-framework-semantic-search` was not
registered, before and after each run.

- **No managed models:** `HOME=<an empty folder> just search-eval` stopped
  before building anything, exit 1, with: "search-eval needs the managed search
  models: run `llm-wiki install --configure-search` first
  (<that folder>/.llm_wiki/manifest.json is missing)".
- **First runs:** register refused a root without an
  orientation file, then every search stopped with "llm-wiki install is
  required before search"; the recipe now copies `AGENTS.MD` and the install
  manifest (the Target above).
- **On `develop` (`c6e4995` plus this branch's recipe):** `just search-eval`
  registered the copy in the temporary home, indexed 135 files, and ran all
  30 cases in about 11 minutes (22 minutes with the index). The eval ran to
  its end and failed one floor: semantic 23 of 30 against a floor of 24. Per
  mode, passes over judged cases: lexical 18/26, semantic 23/29, hybrid 23/30,
  auto 23/30. The no-match sentinels C10, H9, H11 and H20 return results in
  meaning-based modes; C5 misses in every mode (lexical included), C8 in hybrid and auto, and
  H12 in the meaning-based modes. Hybrid and
  auto beat lexical, 23 to 18.
- **On PR #58's head (`d8eab20`, the ranking fix before its fix round):** the same
  script, run from a separate checkout, gave lexical 22/26 and the other modes
  exactly as on `develop`. The test stops at the same semantic floor before
  reaching its last assertion, that hybrid and auto beat lexical; from the
  report's counts that assertion holds there by one case, 23 to 22.
- **After the review's fix round (`69caab3`, the models read in place):** the
  same counts as the first `develop` run, mode by mode and case by case, in
  22 minutes. The first two runs had linked the model files into the
  temporary home; nothing read the links (choice 1).
- **On this branch with `develop` merged in (`b038f00`, the ranking fix as
  it landed, `011979d`):** the same counts as on PR #58's head: lexical
  22/26, semantic 23/29, hybrid 23/30, auto 23/30, in 21 minutes. The test
  stops at the semantic floor; hybrid and auto beat lexical by one case.
- **Left as it was, by checksum:** the listing below, taken before and after
  each of the four runs, was the same each time (SHA-256, first 16 digits;
  `~` is the real home):

  | SHA-256 | File |
  | --- | --- |
  | `604c329a31518187` | `~/.llm_wiki/accepted-licenses.toml` |
  | `7997e9302aeacbed` | `~/.llm_wiki/backups/install-<date>/backup-manifest.json` |
  | `771179514816456a` | `~/.llm_wiki/bin/llm-wiki` |
  | `d734d4fd09c6dfd7` | `~/.llm_wiki/external-dependencies.toml` |
  | `104e6fd062656736` | `~/.llm_wiki/manifest.json` |
  | `185e4d52094b24ac` | `~/.llm_wiki/mcp/claude-project.mcp.json` |
  | `48fa8eaf6e0bcad0` | `~/.llm_wiki/models/artifacts.toml` |
  | `b5ce9d77a3fc4b3b` | `~/.llm_wiki/models/embeddinggemma-300m-q8_0/embeddinggemma-300M-Q8_0.gguf` |
  | `000dfb1c06efa6a0` | `~/.llm_wiki/models/qmd-query-expansion-1.7b-q4_k_m/qmd-query-expansion-1.7B-q4_k_m.gguf` |
  | `3cad88be795df555` | `~/.llm_wiki/search-runtime-probes.toml` |
  | `3de702d89dbfe39e` | `~/.llm_wiki/search.toml` |
  | `95f74ae45ed9e8b3` | the checkout's `.llm_wiki/search.toml` |
  | absent | the real cache, `~/.cache/llm-wiki` |
  | absent | the checkout's `.mcp.json` |

  The real registry and the managed home's `indexes/` are the machine's
  shared state, so they were listed by project id and folder name instead:
  around the third run, for example, 91 ids before and 99 after, 14 index folders before
  and 12 after, all the difference being other worktrees made and removed
  by Orca meanwhile and test fixtures named `fixture-project-<n>` registered
  by other test runs on the machine. Neither ever held
  `llm-wiki-framework-semantic-search`, before, during or after any run.

## The Two Choices

Taken by the coordinator on 2026-10-07 while the owner was away, both as this
plan recommended, to be confirmed by the owner's verdict on the PR that does
the work:

1. **The models: read in place from the real managed home**, never copied
   or downloaded, with the model records, accepted licenses and search
   settings copied, and the before-and-after checksums of the Done When
   proving nothing was written there. Recommended and taken as "linked from
   the real managed home"; the work found the links unneeded: index and
   search reach a model only through the absolute path in its record
   (`src/search/commands.rs`, `src/search/semantic.rs`), and only install and
   uninstall look at the models folder itself, so the copied records already
   point at the real files, read-only. Found by the blind review of PR #67,
   finding 1. Not chosen: copying them (1.6 GB on the machine this plan was
   written on, at every run), or downloading them into the temporary home
   (slow, needs the network, and the eval would measure another copy).
2. **The recipe's name: `just search-eval`.** Not chosen: `just eval`, which
   reads as if it ran `llm-wiki eval`, a different command.

## Out Of Scope

- What the eval measures, its cases and its floors.
- The managed models themselves and how `llm-wiki install` fetches them.
- Running the eval in CI.
