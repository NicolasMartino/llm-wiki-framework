# Plan: The Search Eval Runs In A Temporary Home

- Document Class: Plan
- Status: Draft
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

- **One recipe** (name: the owner's choice 2) that:
  - makes a temporary home and points `HOME` and the XDG variables into it;
  - gives that home the managed models and the accepted licenses from the
    real one, without writing to the real one (how: the owner's choice 1),
    and fails with one plain line naming `llm-wiki install` when the real
    home has none;
  - makes the eval project's own root inside the temporary directory: a copy
    of the checkout's `wiki/`, with its own `.llm_wiki/search.toml` turning
    meaning-based search on, never the checkout itself;
  - registers that root under `llm-wiki-framework-semantic-search` with
    `--no-mcp`, and indexes it;
  - runs the ignored eval test, which writes its report under `target/` as
    today;
  - removes the temporary home on the way out, success or failure.
- **The eval page** names the recipe where it says how to run the eval.

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

## Open For The Owner

1. **The models: linked from the real managed home**, with the accepted
   licenses and search settings copied. No download and no copy; search and
   index only read the models folder (to be rechecked by whoever does the
   work), and the before-and-after checksums of the Done When prove nothing
   was written there. Not chosen: copying them (1.6 GB on the machine this
   plan was written on, at every run), or downloading them into the
   temporary home (slow, needs the network, and the eval would measure
   another copy).
2. **The recipe's name: `just search-eval`.** Not chosen: `just eval`, which
   reads as if it ran `llm-wiki eval`, a different command.

## Out Of Scope

- What the eval measures, its cases and its floors.
- The managed models themselves and how `llm-wiki install` fetches them.
- Running the eval in CI.
