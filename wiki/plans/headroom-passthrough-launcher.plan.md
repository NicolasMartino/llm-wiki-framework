# Plan: `llm-wiki headroom` — env-injecting passthrough to the Headroom binary

- Document Class: Plan
- Status: Blocked
- Date: 2026-07-18
- Category: Agent runtime, Headroom, developer ergonomics, CLI
- Revises: the Phase 2 command design in
  `wiki/plans/headroom-wrap-command.plan.md` (Implemented 2026-07-07). That
  command's process model was wrong; this plan corrects it.
- Implements: `wiki/decisions/headroom-single-posture-mcp-first.decision.md`
  (Accepted; amended 2026-07-07) — the "narrow launch convenience" surface.
- Sources:
  - `src/cli.rs` (`HeadroomArgs`, `HeadroomCommand::Wrap`, `HeadroomWrapArgs`)
  - `src/headroom.rs` (current `wrap` / `ChildEnv` / `run_child`)
  - `src/instance.rs` (`mcp_tool_names`, `all_mcp_instances`, `server_name_variants`)
  - `tests/headroom.rs` (integration tests to rewrite)
  - Headroom source read 2026-07-18:
    `headroom/cli/wrap.py` (`_start_proxy`: `proxy_env = os.environ.copy()` →
    `Popen(env=proxy_env)`), `headroom/proxy/server.py:3177`
    (reads `HEADROOM_EXCLUDE_TOOLS` from `os.environ`),
    `headroom/ccr/mcp_server.py:77` (reads `HEADROOM_MCP_READ`, default `off`).
  - Headroom source read 2026-07-20 for installed `headroom-ai 0.32.0`:
    `headroom/proxy/handlers/openai.py` now builds a Responses
    `call_id -> function name` map, computes excluded call ids via
    `is_tool_excluded`, protects matching output slots plus `headroom_retrieve`
    outputs, and uses a 512-byte live Responses unit floor; line numbers drifted
    from older reviews, but the source-level behavior is verified.
- Related:
  - `wiki/checklists/mcp-field-test.checklist.md`
  - `wiki/references/headroom-context-compression.reference.md`
  - `wiki/plans/headroom-mcp-field-test-repair.plan.md`

## Where This Stands (2026-10-06)

Done: the passthrough launcher and its tests, recorded in "Implementation
Evidence (2026-07-18)" below and on master (`src/cli.rs`, `src/headroom.rs`,
`tests/headroom.rs`). Acceptance items 1 to 4 hold.

Still pending, and the reason this plan is Blocked rather than Completed:
acceptance item 5, the Headroom launch smoke
(`wiki/checklists/headroom-launch-smoke.checklist.md`). The first live run did
not pass it ("Live Field-Test Outcome (2026-07-18)" below), and no later run
records it passing.

It waits on `wiki/plans/headroom-mcp-field-test-repair.plan.md` (Active), whose
Phase 7 reruns that smoke. When that rerun records the smoke's gated facts, this
plan completes; the full-payload behavior stays observational here, as
"Acceptance" says.

## Why (the defect this corrects)

The shipped `llm-wiki headroom wrap -- <command>` sets `HEADROOM_MCP_READ` and
`HEADROOM_EXCLUDE_TOOLS` in the environment and then **execs the bare command
directly**. That is a no-op for real Headroom usage:

- Headroom is an **API proxy**. `HEADROOM_EXCLUDE_TOOLS` is read by the
  **proxy process** (`headroom/proxy/server.py:3177`), and the proxy is only
  started by `headroom proxy` / `headroom wrap`. Exec-ing `codex` directly never
  starts a proxy, so nothing consumes the exclude list and Codex talks straight
  to the API — Headroom entirely bypassed.
- Verified live 2026-07-18: no `headroom` process runs after
  `llm-wiki headroom wrap -- codex`; Codex's `~/.codex/config.toml` has no
  `headroom` reference; the vars sit unread.

The env vars **are** real Headroom knobs — they just have to be present in the
environment of the **`headroom` process** that starts the proxy. `_start_proxy`
does `proxy_env = os.environ.copy()` then `Popen(env=proxy_env)`, so a proxy
launched by `headroom` inherits whatever we export. The fix is therefore: make
`llm-wiki headroom` **export the env and then exec the `headroom` binary**,
forwarding all trailing args to it untouched.

## Target interface

```
llm-wiki headroom [--headroom-bin <PATH>] [--unsafe-mcp-read] [--] <headroom args...>
```

- Everything in `<headroom args...>` is forwarded **verbatim** to the `headroom`
  binary. We do not know or care about `wrap` vs `proxy` — Headroom owns those
  subcommands.
- **Separator is optional, both forms supported.** With `trailing_var_arg = true`
  clap does not *require* `--`, so `llm-wiki headroom wrap codex` and
  `llm-wiki headroom -- wrap codex` are both accepted and behave identically. The
  `--` is the recommended, unambiguous form: our flags (`--headroom-bin`,
  `--unsafe-mcp-read`) are only recognized **before** the first forwarded token;
  once forwarding begins, later `--`-prefixed tokens (e.g. Headroom's `--port`)
  pass straight through. Both forms are covered by tests (test 3 below).

Examples:

```bash
llm-wiki headroom -- wrap codex                          # → headroom wrap codex   (+ our env)
llm-wiki headroom -- wrap claude                          # → headroom wrap claude
llm-wiki headroom --headroom-bin /opt/headroom -- wrap codex
llm-wiki headroom --unsafe-mcp-read -- proxy --port 8790
```

Who starts what, after this change:

- **The Headroom proxy** — started by `headroom wrap <tool>` (inherits our env,
  so it reads `HEADROOM_EXCLUDE_TOOLS`).
- **The agent** (Codex/Claude) — launched by `headroom wrap <tool>`.
- **The llm-wiki MCP server** — started by the **agent** from its own config
  (`~/.codex/config.toml [mcp_servers.llm-wiki] → mcp serve`). Not our job;
  confirmed Codex already spawns it on demand. It does **not** read
  `HEADROOM_MCP_READ` — that variable is Headroom's, not llm-wiki's.
- **The Headroom MCP server** (`headroom_retrieve`) — if separately configured
  and launched by the agent, it inherits `HEADROOM_MCP_READ=off` down the process
  tree. This is the process that actually reads that variable
  (`headroom/ccr/mcp_server.py:77`).

## Design

### 1. CLI (`src/cli.rs`)

- Collapse the inner subcommand. `Command::Headroom(HeadroomArgs)` stays; delete
  `HeadroomCommand` and `HeadroomWrapArgs`. `HeadroomArgs` becomes:

  ```rust
  #[command(
      about = "Run the Headroom binary with llm-wiki MCP tools excluded from compression",
    long_about = "Export HEADROOM_EXCLUDE_TOOLS (*llm_wiki* plus all llm-wiki MCP \
                  tools in both namespaces) and HEADROOM_MCP_READ=off, then exec \
                  the Headroom binary with forwarded Headroom args untouched. \
                    Example: llm-wiki headroom -- wrap codex"
  )]
  pub struct HeadroomArgs {
      #[arg(long, value_name = "PATH",
            help = "Path to the headroom binary (default: search $PATH)")]
      pub headroom_bin: Option<PathBuf>,

      #[arg(long, help = "Remove HEADROOM_MCP_READ instead of forcing it off")]
      pub unsafe_mcp_read: bool,

      #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true,
            num_args = 1.., value_name = "HEADROOM_ARGS",
            help = "Arguments forwarded to the headroom binary (recommended form: `-- wrap codex`)")]
      pub args: Vec<OsString>,
  }
  ```

### 2. Binary resolution (`src/headroom.rs`)

Resolution order:

1. `--headroom-bin <PATH>` if given (use as-is; error if it does not resolve to an
   existing regular file).
2. Otherwise search `PATH` for the `headroom` executable.
3. If neither resolves: `bail!` with a message naming `--headroom-bin`, e.g.
   `headroom binary not found on PATH; pass --headroom-bin <path>`.

No dependency on an external `which` crate — a small `#[cfg]`-split resolver
in-module keeps the no-legacy footprint minimal. Cross-platform rules (this repo
has active cross-platform release work, so specify, don't assume):

- **Split**: `env::var_os("PATH")` split with `std::env::split_paths` (handles `:`
  on unix, `;` on Windows) rather than a hand-rolled separator.
- **Candidate names**: unix → `headroom`. Windows → try `headroom` plus each
  extension in `PATHEXT` (default `.COM;.EXE;.BAT;.CMD` when unset), so
  `headroom.exe` resolves. `--headroom-bin` is used as given (no `PATHEXT`
  expansion) but still must point at an existing file.
- **Reject directories**: a candidate matching a directory name is skipped, not
  returned (guard with `metadata().is_file()`).
- **Executability**: unix → require an owner/group/other execute bit
  (`std::os::unix::fs::PermissionsExt`, `mode & 0o111 != 0`). Windows → "is a
  regular file with an allowed extension" is the executability signal (no mode
  bits); do not gate on unix permissions there.

Unit coverage (see test 9) exercises the resolver's file-vs-directory and
name/extension logic across temporary PATH directories with plain files and
directories — **not** a unix shell script — so it runs and asserts on all
platforms.

### 3. Env injection (unchanged computation)

Keep the existing, tested env computation verbatim:

- `exclude_tool_entries()` — emits the broad `*llm_wiki*` glob first, then six
  tool bases × {Production, Test} × {plain, `mcp__<server>__<tool>` for
  hyphenated + underscored server ids}. Explicit entries are sourced from
  `instance::mcp_tool_names()` + `all_mcp_instances()` +
  `server_name_variants()`.
- **`merge_exclude_tools` (invariant to preserve).** `exclude_tools_value()`
  currently calls `merge_exclude_tools(env::var("HEADROOM_EXCLUDE_TOOLS"))`, which
  **merges** the llm-wiki entries into any value the parent already set,
  preserving the parent's entries/order and de-duplicating overlaps
  (`src/headroom.rs:89–118`). This is deliberate — overwriting would silently drop
  a parent's configured exclusions. Keep this function and its two unit tests
  (`merge_preserves_preexisting_entries_alongside_llm_wiki_entries`,
  `merge_dedupes_overlapping_entries`) intact, and add integration coverage
  (test 8 below) so the passthrough rewrite cannot regress it.
- `ChildEnv` / `McpReadEnv`: default `SetOff` (`HEADROOM_MCP_READ=off`);
  `--unsafe-mcp-read` → `Remove` (`env_remove`, so a parent `=on` cannot leak).
  Unchanged.
- `emit_warning`: keep the best-effort / not-a-provenance-boundary framing.

Only the **exec target** changes: instead of `args.command`, we build
`ProcessCommand::new(resolved_headroom_bin).args(&args.args)`, apply the env, and
run it.

### 4. Process model (unchanged shape)

- unix: `exec` the resolved `headroom` argv (replace process).
- non-unix: spawn + wait + `exit(status.code())`.
- Failure context: `execute headroom <args>` (updates the current
  `execute wrapped command …` wording).

### 5. `src/main.rs`

Dispatch simplifies to `Command::Headroom(args) => headroom::run(&args, &context)`
(no inner match).

## Tests (`tests/headroom.rs` — rewrite)

The current tests wrap `env` / `printf` / `sh -c 'exit 7'` directly; those no
longer model the contract (we now exec `headroom`, not an arbitrary command).
Replace them.

**Integration tests** (`tests/headroom.rs`) use a **fake `headroom` binary** the
test controls — a script that prints its `argv` and the relevant env vars — so we
assert the composed argv + env without needing real Headroom. Because the fake is
a shell script, these are `#[cfg(unix)]` (matching the existing suite's
convention); the platform-agnostic resolver logic is covered separately by the
unit test in #9.

1. **Forwarding + env** — point `--headroom-bin` at the fake. Run
   `llm-wiki headroom --headroom-bin <fake> -- wrap codex`. Assert argv is
   `wrap codex`, `HEADROOM_MCP_READ=off`, and `HEADROOM_EXCLUDE_TOOLS` covers prod
   + test (reuse `assert_exclude_tools_cover_prod_and_test`).
2. **`--unsafe-mcp-read`** — with parent `HEADROOM_MCP_READ=on`, assert the fake
   sees `HEADROOM_MCP_READ` **absent** and still gets the exclude set.
3. **Both separator forms + hyphenated passthrough** — assert
   `--headroom-bin <fake> -- wrap codex --model x --port 8790` **and**
   `--headroom-bin <fake> wrap codex --model x --port 8790` (no `--`) both reach
   the fake as `wrap codex --model x --port 8790` (no mangling, `--`-prefixed
   Headroom flags pass through).
4. **`PATH` resolution** — put the fake `headroom` on a `PATH` dir (no
   `--headroom-bin`); assert it is found and invoked.
5. **Not found** — empty `PATH`, no `--headroom-bin`; assert nonzero exit and an
   error naming `--headroom-bin`.
6. **Exit-code forwarding** — fake exits 7; assert code 7.
7. **Empty args rejected** — `llm-wiki headroom` (no forwarded args) fails with
   clap `required`/`HEADROOM_ARGS`.
8. **Parent exclude preserved (merge invariant)** — set parent
   `HEADROOM_EXCLUDE_TOOLS=custom_tool_a,custom_tool_b`; assert the fake sees both
   custom entries **and** the full llm-wiki set, with no duplicates. Guards the
   `merge_exclude_tools` behavior end-to-end (complements the two in-module unit
   tests, which stay).

**Unit test** (`src/headroom.rs`, platform-agnostic — runs on Windows too):

9. **Resolver** — build two temporary PATH directories: the first containing a
   directory named like the executable candidate (which must be skipped), and the
   second containing the executable candidate as a regular file (for example
   `headroom` on unix or `headroom.exe` on Windows). Assert the resolver returns
   the file, skips the directory, honors an explicit `--headroom-bin`, and errors
   when absent. Uses `tempfile` + plain files/directories (no unix shell script),
   so it exercises the file-vs-directory and name/extension logic on all
   platforms.

Keep the `src/headroom.rs` unit tests on `exclude_tool_entries()`,
`merge_exclude_tools` (both cases), and `ChildEnv::new` as-is (env computation is
unchanged).

## Docs & bookkeeping

- `src/cli.rs` help/long_help updated as above (worked `-- wrap codex` example,
  since `llm-wiki headroom --help` no longer documents `wrap`).
- `wiki/plans/headroom-wrap-command.plan.md` — add a top note: the Phase 2
  command process model is superseded by this plan; link here.
- `wiki/decisions/headroom-single-posture-mcp-first.decision.md` — small amend:
  the convenience **execs the `headroom` binary with env injected** (passthrough),
  rather than exec-ing an arbitrary child. Still no proxy start / base-URL
  injection **by us** — Headroom does that.
- `wiki/references/headroom-context-compression.reference.md` — one line on the
  passthrough invocation (`llm-wiki headroom -- wrap codex`).
- **Do not fold Headroom launch into `wiki/checklists/mcp-field-test.checklist.md`.**
  That checklist is the pure-MCP correctness gate (no proxy/launch/exclude
  concepts) and stays that way. Instead add a **separate, clearly optional
  Headroom launch smoke** — a short new doc (e.g.
  `wiki/checklists/headroom-launch-smoke.checklist.md`) that: launches via
  `llm-wiki headroom -- wrap codex`, verifies the **gated** facts below, and notes
  the exclude-list observation as non-gating. The pure-MCP steps are then run
  *inside* that session by reference, not by rewriting the gate.
- Append a `wiki/log.md` entry (fix: passthrough correction + why the old
  exec-bare-command model was a no-op) and add this plan to `wiki/index.md`
  (see Bookkeeping).

## Out of scope

- Starting the Headroom proxy ourselves, base-URL injection, health/teardown.
- Starting the llm-wiki MCP server (the agent does this from its own config).
- Validating that the forwarded subcommand is a real Headroom command (Headroom
  reports its own errors).

## Resolved decision

- Flag name resolved as `--headroom-bin`.

## Acceptance

Gated (these are the pass criteria):

1. `llm-wiki headroom -- wrap codex` execs `headroom wrap codex` with
   `HEADROOM_MCP_READ=off` exported and the full merged `HEADROOM_EXCLUDE_TOOLS`
   set, so the `headroom` process (and the proxy / Headroom MCP server it starts)
   inherits them. Asserted by the fake-binary tests, not by proxy output.
2. `--headroom-bin` overrides resolution; missing binary errors clearly naming the
   flag; the resolver skips directories and honors `PATHEXT` on Windows.
3. `--unsafe-mcp-read` removes `HEADROOM_MCP_READ` even when the parent set `=on`;
   a parent `HEADROOM_EXCLUDE_TOOLS` is preserved (merged), not clobbered.
4. `cargo fmt` / `clippy -D warnings` / `test` / `insta` green;
   `cargo test --test headroom` and `--test identity_lint` green.
5. Headroom launch smoke (the separate optional doc, not the MCP gate) confirms
   the **gated provenance facts**, consistent with
   `headroom-single-posture-mcp-first.decision.md`:
   - `HEADROOM_MCP_READ=off` is inherited by the Headroom process tree; and
   - wiki/raw access in the session routes through `llm_wiki_*` MCP tools.

Observational (recorded, **not** a pass criterion): whether the proxy returns
`llm_wiki_*` output uncompressed under `HEADROOM_EXCLUDE_TOOLS`. Per the governing
decision and reference, the exclude list is best-effort and **not** a provenance
boundary. The behavior is Headroom-version/provider-path dependent: the
2026-07-18 Codex run and inspected `headroom-ai 0.24.0` path failed, while the
installed `headroom-ai 0.32.0` source now appears to honor Responses excludes and
needs a fresh live field test. Treat any "uncompressed" result as
harness-specific evidence, never as the thing that makes wiki/raw provenance
exact.

## Implementation Evidence (2026-07-18)

Implemented the passthrough launcher in code and docs:

- `src/cli.rs`: collapsed the nested `wrap` subcommand into
  `llm-wiki headroom [--headroom-bin <PATH>] [--unsafe-mcp-read] [--]
  <headroom args...>`.
- `src/headroom.rs`: resolves `--headroom-bin` or searches `PATH` for the real
  Headroom binary, preserves parent `HEADROOM_EXCLUDE_TOOLS` entries while
  appending `*llm_wiki*` plus llm-wiki production/test MCP tool names, sets/removes
  `HEADROOM_MCP_READ` as planned, and execs/spawns Headroom with forwarded args.
- `tests/headroom.rs`: rewrote integration coverage around a fake Headroom
  binary for argv forwarding, separator/no-separator forms, hyphenated Headroom
  args, PATH resolution, missing-binary diagnostics, exit-code forwarding,
  unsafe MCP-read removal, and parent-exclude preservation.
- Added `wiki/checklists/headroom-launch-smoke.checklist.md` as the separate
  optional live Headroom launch smoke; the pure MCP field-test checklist was not
  folded into Headroom.

Verification passed:

- `rtk cargo fmt`
- `rtk cargo test --test headroom` — 8 passed.
- `rtk cargo test headroom` — 12 passed, 327 filtered out.
- `rtk cargo test --test identity_lint` — 3 passed.
- `rtk cargo insta test --test init --accept` — 22 passed; 9 snapshots accepted.
- `rtk cargo check`
- `rtk cargo clippy -- -D warnings`
- `rtk cargo test` — 337 passed, 2 ignored.

No live interactive `llm-wiki headroom -- wrap codex` smoke was run in this
implementation pass; the optional checklist now defines how to record that
external-host evidence without changing the pure MCP correctness gate.

Follow-up review repair (2026-07-18):

- Corrected generated/current guidance so exact wiki/raw provenance is attributed
  to `llm_wiki_*` MCP routing, not to `HEADROOM_EXCLUDE_TOOLS`.
- Fixed Windows candidate resolution to try bare `headroom` before PATHEXT names
  and added direct unit coverage for Windows candidate construction, default
  PATHEXT handling, candidate ordering, and directory skipping.
- Updated the stale MCP onboarding decision note that still named
  `llm-wiki headroom wrap` as the live surface.

Additional verification passed: `rtk cargo test headroom` (16 passed, 327
filtered out), `rtk cargo test --test headroom` (8 passed), `rtk cargo
insta test --test init --accept` (22 passed; 9 snapshots accepted), `rtk cargo
check`, `rtk cargo clippy -- -D warnings`, `rtk cargo test` (341 passed, 2
ignored), and `rtk git diff --check`.

## Live Field-Test Outcome (2026-07-18)

The first live Codex + Headroom field test did not pass the Headroom launch
smoke. The local deployment path and passthrough process model worked: the
managed and Cargo-installed binaries reported `llm-wiki 0.2.11`, LLM search was
enabled, model artifacts and licenses were verified, and runtime probes passed.

Inside a session launched via `llm-wiki headroom wrap`, however, MCP payloads
were not preserved:

- `llm_wiki_index` output was replaced by Headroom compression envelopes.
- `llm_wiki_read` returned provenance metadata but the `content` field was a
  `<<ccr:...>>` placeholder instead of real Markdown/source text.
- `llm_wiki_search` returned ready/fresh metadata but omitted `results` arrays.
- Include-filtered `llm_wiki_search_all` reported `result_count` without hits,
  while unfiltered/exclude-filtered `search-all` failed globally on a stale
  registered project with a missing wiki root.

Follow-up source review of installed `headroom-ai 0.24.0` confirmed this was not
a launcher route-key bug for that version/path. Headroom's
Chat-Completions/Anthropic path honored `HEADROOM_EXCLUDE_TOOLS`, but the
Codex/OpenAI-Responses handler compressed tool-output `output` strings without
consulting that exclude list. The launcher still correctly injected env into the
real Headroom process; it just could not force full `llm_wiki_*` payload
preservation on Codex for Headroom 0.24.0.

Current source verification on 2026-07-20 found installed `headroom-ai 0.32.0`,
not 0.24.0. In 0.32.0, the OpenAI-Responses handler appears to implement the
missing exclusion path: it maps Responses call ids to function names, checks
`is_tool_excluded`, and protects excluded tool-output slots, with special
`headroom_retrieve` protection still present. That makes the live outcome above
historical evidence, not a current diagnosis. The next Headroom/Codex gate is a
fresh 0.32.0 field test proving whether the launcher-emitted plain and qualified
llm-wiki tool names match the host's `function_call.name` values and whether
full payloads now survive.

This does not invalidate the implemented CLI passthrough mechanics. It means the
live Headroom/Codex payload-preservation work moves to two tracks: current
Headroom-version retesting and route-key verification first, then
llm-wiki-side compression detection/pagination only if the active path still
requires a small-output mitigation. Headroom-independent `search-all` and
diagnostics repairs remain valid either way. The follow-up repair is tracked in
`wiki/plans/headroom-mcp-field-test-repair.plan.md`.
