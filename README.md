# LLM Wiki Framework

The LLM Wiki framework is a project management system where `raw/` contains
human-curated source material and `wiki/` contains agent-maintained compiled
knowledge.

## Install

Install both binaries from a release, `llm-wiki` and `poman` (the project
manager, in an archive of its own), then install the managed MCP surface:

```bash
curl -L https://github.com/nicolasmartino/llm-wiki-rs/releases/latest/download/llm-wiki-rs-installer.sh | sh
curl -L https://github.com/nicolasmartino/llm-wiki-rs/releases/latest/download/poman-installer.sh | sh
llm-wiki install
```

Both installers put their binary in Cargo's bin folder. If you unpack the
release archives by hand instead, unpack `llm-wiki-rs-<target>.tar.xz` and
`poman-<target>.tar.xz` into one folder and run `llm-wiki install` from there.

Users with a Rust toolchain can install from crates.io after the packages are
published; `poman`, the project manager, is installed beside `llm-wiki`:

```bash
cargo install llm-wiki-rs
cargo install poman
llm-wiki install
```

`llm-wiki install` copies or verifies the runtime binary at
`~/.llm_wiki/bin/llm-wiki` and the `poman` beside it at `~/.llm_wiki/bin/poman`,
merges the active instance into Codex MCP config, and writes a staged Claude
project `.mcp.json` under the managed home. It takes `poman` from the folder
its own binary runs from, and only one of its own version; with none there, it
keeps a `poman` it installed before, and otherwise refuses and says how to get
one. Hosts
spawn `llm-wiki mcp serve` over stdio on demand; no background daemon is
installed. The installer no longer renders generated runtime skills.

On upgrade from a pre-MCP install, unchanged manifest-owned generated skills
under `~/.claude/skills/` and `~/.codex/skills/` are removed. User-edited
generated skills are preserved with manual cleanup warnings. Manifest-less old
skill directories are warned about, not deleted.

MCP read and search have different readiness rules: `llm_wiki_read` can read
scoped `wiki/` and `raw/` files from the active project; `llm_wiki_search` and
`llm_wiki_search_all` require registered/indexed project state or return
readiness/failure metadata. Run `llm-wiki path` for optional shell `PATH`
guidance.

For automation, choose the search posture explicitly:

```bash
llm-wiki install --non-interactive --disable-llm-search
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

The full enabled command is the portable form for scripts. Shorter enabled
commands are only valid when the current local model and license state no
longer needs the omitted confirmation.

Enabled LLM-search installs show model download and verification progress on
stderr. Interactive terminals receive a live progress bar with transferred
bytes, rate, percentage, and ETA. Redirected stderr receives bounded milestone
lines instead, so automation logs show that large model transfers are moving
without contaminating stdout.

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

## Commands

`llm-wiki` is MCP-first: hosts drive the wiki through the MCP server, and the
CLI subcommands cover install and local operations.

- `install` — install the managed binary and `poman` and materialize the MCP
  configs.
- `init <path>` — scaffold a new project wiki.
- `headroom [--headroom-bin <PATH>] [--unsafe-mcp-read] [--] <headroom args...>` —
  run Headroom with LLM Wiki environment guards (see below).
- `mcp serve` — run the stdio MCP server (hosts spawn this on demand).
- `read <path>` — read a scoped `wiki/`/`raw/` file through the framework.
- `register` / `forget` / `projects` — manage the project registry.
- `index` / `index-all` — build or refresh semantic indexes.
- `search` / `search-all` — query one or all registered projects.
- `eval run` / `eval calibrate` — measure and tune semantic search candidates.
- `path` / `status` / `doctor` — inspect install and PATH state.
- `uninstall` — remove everything `install` put in place, both binaries
  included; `uninstall --search-artifacts` removes only the search models.

The MCP server exposes these tools: `llm_wiki_read`, `llm_wiki_search`,
`llm_wiki_search_all`, `llm_wiki_index`, `llm_wiki_register`, and
`llm_wiki_status`. Run `llm-wiki --help` for the full surface.

## Headroom

Headroom is an optional external context-compression runtime. `llm-wiki
headroom` exports environment guards, resolves the real `headroom` binary, and
execs it with forwarded Headroom arguments:

```bash
llm-wiki headroom -- wrap codex
```

By default the launcher sets `HEADROOM_MCP_READ=off` in the Headroom environment
so the `headroom_read` tool cannot route `wiki/`/`raw/` content through
compression, and it adds `*llm_wiki*` plus the explicit llm-wiki MCP tool names
to `HEADROOM_EXCLUDE_TOOLS` on a best-effort basis. Pass `--unsafe-mcp-read` to
remove `HEADROOM_MCP_READ` from
Headroom instead of forcing it off; this is only safe when the session routes all
wiki/raw reads through the llm-wiki MCP tools and avoids `headroom_read`.
Known caveat: in `headroom-ai 0.24.0`, Codex/OpenAI-Responses tool outputs did
not consult `HEADROOM_EXCLUDE_TOOLS`, so full llm-wiki MCP payloads could still
be compressed. Inspected `headroom-ai 0.32.0` source appears to honor Responses
excludes, but live runs must record the exact Headroom version/path. Treat CCR
markers, compression envelopes, or omitted payload fields as a failed delivery
check, not exact wiki/raw content.

Current Headroom support is Option A: compact search is supported for discovery,
while exact large `wiki/` and `raw/` reads should be performed outside Headroom.
`llm-wiki` does not add read pagination for Headroom in the current product
posture.

## Search Modes

`llm-wiki search` defaults to `--mode auto`. Hybrid search works out of the box:
the balanced profile ships DEFAULT thresholds, so auto selects hybrid once the
install profile, accepted licenses, model artifacts, and a fresh semantic index
are ready. Calibration is an optional override that tunes those thresholds per
corpus, not a gate that hybrid waits on. When the semantic prerequisites are not
ready, auto reports the lexical selection or readiness failure in command
metadata.

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
