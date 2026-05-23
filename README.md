# LLM Wiki Framework

The LLM Wiki framework is a project management system where `raw/` contains
human-curated source material and `wiki/` contains agent-maintained compiled
knowledge.

## Install

Install the binary from a release, then install the framework skills globally:

```bash
curl -L https://github.com/nicolasmartino/llm-wiki-rs/releases/latest/download/llm-wiki-installer.sh | sh
llm-wiki install
```

Users with a Rust toolchain can install from crates.io after the package is
published:

```bash
cargo install llm-wiki-rs
llm-wiki install
```

`llm-wiki install` copies or verifies the runtime binary at
`~/.llm_wiki/bin/llm-wiki` and renders installed skills to call that managed
path directly. Run `llm-wiki path` for optional shell `PATH` guidance.

For automation, choose the search posture explicitly:

```bash
llm-wiki install --non-interactive --disable-llm-search
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

The full enabled command is the portable form for scripts. Shorter enabled
commands are only valid when the current local model and license state no
longer needs the omitted confirmation.

## Create a Project

```bash
llm-wiki init /path/to/project --non-interactive \
  --name "My Project" \
  --description "One sentence description." \
  --blueprint web-product
```

Use repeatable `--pack <name>` flags to override a blueprint's default pack
selection. For example, `--blueprint custom --pack ml --pack qmd-rs-scale`
creates an ML-oriented wiki with qmd-rs-backed scale guidance.
For command-line tools and developer utilities, use `--blueprint cli-tool`.

## Diagnostics

Every `llm-wiki` command accepts global `-v` / `--verbose` before or after the
subcommand. Normal command results stay on stdout; verbose diagnostics go to
stderr and explain resolved paths, selected projects, index state, install
decisions, and other command-specific state.

```bash
llm-wiki -v search "query" --project my-project
llm-wiki index --project my-project -v
```

## Search Modes

`llm-wiki search` defaults to `--mode auto`. Auto selects calibrated hybrid
search only when the install profile, accepted licenses, model artifacts, fresh
semantic index, and scoped thresholds are ready; otherwise it reports the
lexical selection or readiness failure in command metadata.

`--rerank` is currently an opt-in, readiness-gated extension rather than part of
the v1 promoted hybrid baseline. The shipped balanced profile does not configure
a reranker by default, so rerank requests fail closed until a future reranker
profile has a local artifact, accepted-license record, eval run, calibration,
and threshold scope.

## Search Evals

Semantic search candidates can be measured without changing production search
state:

```bash
llm-wiki eval run --project-root /path/to/project \
  --candidate-profile balanced \
  --eval-page wiki/evals/natural-language-search.eval.md
```

`llm-wiki eval calibrate --run-report <target/evals/.../eval-run.json>`
derives candidate-specific threshold proposals from the calibration split. It
does not apply thresholds unless `--apply` is passed, and candidate model
changes require an explicit `--apply-profile <scope>`. When a run contains
multiple candidates, destructive or recording actions must name the intended
proposal with `--select-candidate <name>`.

Eval tables may include an `Applies` column before the expected-target column.
Use `all` for the default or a comma-separated mode list such as
`lexical`, `hybrid`, `auto`; non-applicable modes are reported separately and
are excluded from calibration floor derivation.

Hybrid eval JSON records fused scores, branch evidence (`lexical_rank`,
`lexical_score`, `semantic_rank`, `semantic_score`), and displayed-candidate
anchor matches so calibration can replay production-equivalent
path/title/snippet anchor survival instead of treating rank-fused display
scores as semantic similarity. Reports produced before that branch evidence
exists are intentionally not promotable for hybrid threshold application.
When `eval run --rerank` is passed, hybrid and auto evaluation use the selected
reranker artifact, rewrite the hybrid result order through the same qmd-rs
rerank path as production search, and report `rerank_applied` plus
`rerank_ms`. Missing reranker configuration, artifacts, or license acceptance
remain explicit readiness failures.

Calibration reports also include proposed-threshold diagnostics: proposed pass
summaries, hold-out summaries, verdict changes, no-match precision,
exact-identifier preservation, model artifact bytes, and candidate index bytes.
Hybrid final-floor calibration also accounts for no-match rows that survive
only because path/title/snippet anchor evidence would preserve them in
production; those anchor-leaking rows raise the final semantic floor instead
of being treated as harmless semantic-only misses.
Promotion is gated on hybrid/auto behavior: the calibration split and hold-out
split must pass under proposed thresholds before an applied candidate can be
treated as ready for a post-apply production validation run. Semantic-only
results remain diagnostic when the embedding space cannot separate all
no-match rows.
Applied thresholds are stored as scoped records keyed by project/corpus,
profile, embedding artifact, dimensions, qmd-rs adapter version, and chunking
strategy. Legacy single-record threshold files still read as fallback input,
but once scoped records exist, search/index require the current project scope
to match instead of silently reusing another corpus' floors.
Pass `--export-raw-data` to `eval calibrate` to copy the run report and
calibration report into
`raw/data/eval/<corpus-slug>/<run-id>/<candidate-name>/` with a hash-bearing
`manifest.toml`; wiki eval pages should ingest those raw files into
human-readable comparison tables. Repeated calibration attempts create separate
run directories when needed so their impact can be compared over time.

For repeatable corpus-level comparison, use the committed eval fixtures:

- `tests/fixtures/eval-testbed/` for fast infrastructure checks.
- `tests/fixtures/eval-corpora/electric-cars/` for a richer domain retrieval
  corpus with separate raw provenance, compiled wiki pages, and a hidden eval
  table.

Run `llm-wiki --help` for the full command surface.
