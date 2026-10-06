# Plan: The Operations Setup Ships With llm-wiki

- Document Class: Plan
- Status: Draft
- Date: 2026-10-06
- Category: Project init, MCP prompts, way of working
- Scope: Carry out P19 of the framework roadmap, as the accepted proposal
  describes it: first, `AGENTS.md`, `CLAUDE.md` and `project_guidelines.md`
  each get one marked block that init owns and refreshes, and init leaves
  everything outside it as it is; then an init pack writes the coordinated way
  of working's files, an `operations_setup` MCP prompt holds the setup steps,
  and a built-in MCP resource carries the same steps.
- Sources:
  - `wiki/proposals/operations-setup-in-llm-wiki.proposal.md`, accepted by the
    owner on 2026-10-06 (#38), and its feedback review and fix round on #38
  - `wiki/roadmaps/framework-v1.roadmap.md`, P19, and issue #40
  - The owner's answers of 2026-10-06 to the proposal's open choices: when to
    build it, "asap"; where the kit's AGENTS sections go, "I think we should
    have a needle, a part of the agents.md file that is dedicated to llm wiki"
  - `src/init/scaffold.rs`, `src/init/compose.rs`, `src/init/template.rs`,
    `src/init/packs.rs`, `src/init/manifest.rs`, `src/init/collision.rs`,
    `src/cli.rs` (`InitArgs`), `src/mcp/mod.rs`, `src/wiki_read/mod.rs`
    (`discover_project_root`), `tests/init.rs`, `tests/snapshots/` and the
    `justfile` at `0c9a240`, read for "Where It Stands"
  - The owner's global operations setup skill (outside this repository): its
    steps, its fitting guide and its kit, which the pack's templates copy
- Related:
  - `wiki/decisions/composable-project-init.decision.md`: the rerun rule this
    plan changes
  - `wiki/specs/wiki-init-skill.spec.md`, "Rerun Behavior"
  - `wiki/plans/init-rerun-pack-drift.plan.md`: the marked Schema Drift
    section the block follows
  - `wiki/proposals/project-update-command.proposal.md` (Proposed): a later
    `update` command over the same framework-owned files
  - `wiki/decisions/skill-projection-template-engine.decision.md`
    (superseded): the no-shipped-skills posture the pack keeps
  - `wiki/evals/mcp-first-host-parity.eval.md`: why the steps are also a
    resource
  - `wiki/checklists/operation-manager.checklist.md`,
    `wiki/checklists/worker-briefs.checklist.md`,
    `wiki/plans/development-workflow-setup.plan.md`: this repository's own
    fitted copy of the kit

## What This Proves

A project gets the coordinated way of working from llm-wiki alone, and keeps
what it writes for itself:

- a fresh project runs `llm-wiki init` and then the `operations_setup` prompt
  in an interactive Claude Code session, and gets the fitted kit, uncommitted,
  passing the setup's own tests and coming back from its blind review with no
  P1 or P2;
- a rerun of `llm-wiki init` (adding a pack) leaves every kit file, and every
  byte outside init's marked block in `AGENTS.md`, `CLAUDE.md` and
  `project_guidelines.md`, as it was;
- a host that shows no MCP prompts can still read the steps, from a built-in
  resource listed in every session;
- `just verify` passes locally and CI's fast check passes on each PR.

## Where It Stands (2026-10-06, `0c9a240`)

Whoever does the work rechecks each point against the commit they start from.

- **What init writes.** Init composes five files: `project_guidelines.md`,
  `AGENTS.md` (the base template with each selected pack's fragment),
  `CLAUDE.md` (one line, `See @AGENTS.md.`), `wiki/index.md` and
  `wiki/log.md` (`compose` in `src/init/compose.rs`). The guidelines carry
  the date of the render.
- **What a rerun writes.** A rerun writes every composed file again except
  `wiki/index.md` and `wiki/log.md`, which it keeps when they exist
  (`preserves_project_knowledge` and the write loop in
  `src/init/scaffold.rs`). So a project's own text in `AGENTS.md`,
  `CLAUDE.md` or `project_guidelines.md` is lost at the next rerun, without a
  word. Init counts as a rerun whenever `.llm_wiki/` exists, even without
  `init.toml`.
- **Init already owns one marked section.** In `wiki/index.md`, the Schema
  Drift section sits between `<!-- llm-wiki:init-schema-drift:start -->` and
  `<!-- llm-wiki:init-schema-drift:end -->`; a rerun replaces only that
  section, keeps the text around it, and appends the section when it is
  missing (`replace_schema_drift_section` in `src/init/scaffold.rs`, with its
  unit tests). The block below follows the same pattern.
- **The file's spelling.** Init writes `AGENTS.md`. A project whose file is
  `AGENTS.MD` (this repository's) would, on a case-sensitive file system, get
  a second file beside it at a rerun. The MCP resources already read either
  spelling (`resource_path_aliases`). The fresh-init collision guard
  (`src/init/collision.rs`) refuses when `wiki`, `raw`, `.llm_wiki`,
  `AGENTS.md`, `CLAUDE.md` or `project_guidelines.md` exists.
- **The answer record.** `.llm_wiki/init.toml` records the framework
  version, the project's name and description, the blueprint, the packs and
  the resolved folders (`src/init/manifest.rs`); nothing else.
- **Init's flags** (`InitArgs` in `src/cli.rs`): the path,
  `--non-interactive`, `--name`, `--description`, `--blueprint`, `--pack`
  (repeated), `--initial-sources`, `--no-register`, `--no-mcp`. No pack takes
  values of its own.
- **Init's tests.** `tests/init.rs` covers reruns and schema drift; nine
  insta snapshots (`tests/snapshots/init__*.snap`) hold each blueprint's
  rendered files. `just verify` checks the snapshots; CI's fast check runs
  `cargo insta test --check`.
- **MCP prompts** (`prompts_list_result`, `prompt_text` in
  `src/mcp/mod.rs`): five, `wiki_query`, `wiki_ingest`, `wiki_lint`,
  `wiki_research`, `wiki_init`, each with one optional argument, each text
  built in the binary.
- **MCP resources** (`resource_specs`, `resource_list`, `read_resource`):
  eleven project files, each with a `llm-wiki://project/<path>` URI and listed
  only when the file exists. `resources/list` fails when the server finds no
  `wiki/index.md` from its folder upward (`discover_current_project_root`).
  An unknown URI gets the not-found code -32002.
- **The kit** is as the proposal's "What Exists Today" describes it. Its
  AGENTS sections ("How Work Runs", "Commits And GitHub") are appended to the
  AGENTS file and refer to its "Operations", "How To Orient" and
  "Conventions" headings, which an init `AGENTS.md` has.

## Target

### The block that init owns

Decided by the owner on 2026-10-06 ("a needle, a part of the agents.md file
that is dedicated to llm wiki"). This changes how init writes the root schema
files for every project, with or without the operations pack.

- **One block per file.** `AGENTS.md`, `CLAUDE.md` and
  `project_guidelines.md` each hold one block between a begin marker and an
  end marker, HTML comments in the form init already uses (recommended:
  `<!-- llm-wiki:managed:start -->` and `<!-- llm-wiki:managed:end -->`, the
  same pair in all three). Everything init renders today for that file goes
  inside it, pack fragments included. The block's first line says that init
  rewrites the text between the markers and that the project's own text goes
  outside them.
- **The same rule for all three files.**
  - `AGENTS.md`: the project's own sections, such as this repository's tuned
    "How Work Runs", go outside the block.
  - `CLAUDE.md`: the block holds the one line pointing at `AGENTS.md`;
    a project's own Claude-only notes, which a rerun drops today, stay
    outside it.
  - `project_guidelines.md`: the block holds the documentation model as
    rendered; its render date changes inside the block only.
  - `wiki/index.md` and `wiki/log.md` keep their rule (kept when they exist).
- **The AGENTS file's spelling.** Init writes its block into the AGENTS file
  that exists, `AGENTS.md` or `AGENTS.MD`, and never writes a second one
  beside it.
- **A fresh init** writes each file as its block alone.
- **A rerun, file with a block:** init replaces the text between the markers
  and leaves every byte before the begin marker and after the end marker as
  it was, line endings and a missing final newline included.
- **A rerun, file missing:** written as on a fresh init.
- **A rerun, file without markers** (every project initialized before this
  change, or a file written by hand). A one-time migration that never loses
  the project's text:
  - if the file is exactly what init renders today for it, it becomes the
    block, with nothing else;
  - otherwise init writes the block at the top, then a heading saying the
    text below was kept from before the block (with the date), then the old
    file unchanged. Init warns, naming the file, that the kept text may
    repeat what the block now says and is the project's to trim. Most
    projects made before this change land here once, since the templates
    have changed since their first render.
- **Edits made inside the block are refreshed, with a warning.** Init
  records in `.llm_wiki/init.toml` a hash of each block it writes. At a
  rerun, a block whose text no longer matches its recorded hash (or that has
  no recorded hash and differs from today's render) was edited: before
  replacing it, init saves the edited block under `.llm_wiki/` in a dated
  file, and warns naming the file, the saved copy, and the edited block's
  lines that the new block does not hold. A block that matches its hash is
  refreshed without a word, even when the templates changed.
- **Broken markers stop the rerun.** A begin marker without its end, an end
  before its begin, or more than one block in a file: init refuses before
  writing any of the three files, naming the file and the line, and says how
  to repair it.
- **What it does not change:** the fresh-init collision guard, folders, the
  schema-drift audit, registration and the MCP wiring.

### The pack

As the proposal's "The pack holds the files" says, with the owner's answer on
the AGENTS sections in place of its three placements:

- **Name:** `operation-manager` until the owner settles it ("Open For The
  Owner").
- **The kit's files are compile-time templates** with typed fields, one per
  placeholder, so a missing one is a build error. The source of each
  placeholder (what init knows, the owner's answers, the prompt's look at the
  repository, worked out by the pack, written by the prompt after the write,
  dropped) and which become conditions are as the proposal lists them.
  Reasons that name the owner's other projects are written without the names.
- **How init gets the values.** The pack is selected with `--pack`; its
  values come from one answers file the prompt writes, given to init by a
  flag. Init refuses before writing anything, naming each missing value, when
  the pack is selected without them and `init.toml` does not already hold
  them. Init records the values in `.llm_wiki/init.toml` with the pack, so a
  rerun reuses them without the prompt. Interactive init does not ask the
  pack's questions: it says the pack is set up through the prompt.
- **Kit files are written only when absent**, on the first run and on every
  rerun. One that exists (`orca.yaml`, `.claude/settings.json`, a commit hook,
  a justfile, `.gitignore`) is left as it is and listed in init's output, so
  the prompt merges it. Scripts are written executable.
- **The kit's AGENTS sections are the pack's fragment inside the `AGENTS.md`
  block**, rendered from the recorded values. A rerun gives them back the
  same; a project's own changes to how work runs go outside the block or in
  the operation manager page, a kit file.
- **One board model**, the repository as the truth, as the proposal says.

### The prompt and its resource

- **A sixth MCP prompt, `operations_setup`**, with an optional `path`
  argument like `wiki_init`. In Claude Code: `/mcp__llm-wiki__operations_setup`.
  Its steps are the proposal's six, in order: look at the repository
  read-only, ask the standard questions, write the kit through the pack and
  do what the pack cannot (merges, extra worker kinds, the output-folder line,
  `ALREADY_DONE`, the kit's pages in `wiki/index.md` and one `wiki/log.md`
  entry), test in a scratch repository, have the result blind-reviewed by a
  fresh session, hand over.
- **A built-in resource with the same text**, a new kind beside the project
  files:
  - its own URI (recommended: `llm-wiki://builtin/operations-setup`);
  - always listed, including when the server finds no project, where
    `resources/list` fails today; the project files are listed as they are
    now when there is a project;
  - served by `resources/read` from the binary, the same text as the
    prompt's (one source for both).

## Phases

Each phase is its own issue and PR, in this order.

1. **The block.** The three root schema files get the block; reruns, the
   migration, the edit warning, broken markers and the AGENTS file's
   spelling as in the Target. The init snapshots are updated, so the review
   sees the markers in each blueprint's files. Tests that prove it, each
   running real init on a temporary project:
   - a rerun with the same answers, and one adding a pack, keep the text
     before and after each block byte for byte (text above and below, CRLF
     lines, no final newline);
   - an unmarked file equal to today's render becomes the block alone; an
     unmarked edited file keeps every one of its bytes below the block, and
     init warns naming it;
   - an edit inside a block is replaced, the saved copy holds the edited
     text, and the warning names the file and the copy; an unedited block
     after a template change is refreshed without a warning;
   - broken markers refuse the rerun and leave all three files unchanged;
   - a project with `AGENTS.MD` gets its block there and no `AGENTS.md`.
2. **The pack.** The templates, the answers file and its record in
   `init.toml`, written-only-when-absent with the list of existing files, the
   AGENTS fragment inside the block. A golden test (an insta snapshot)
   renders the pack with fixed answers on each side of every condition (CI or
   none, public or not, runbook or checklist pages, a justfile or a script),
   and checks no `{{...}}` is left. A rerun test: every kit file, edited after
   the first write, is unchanged after a rerun adding a pack.
3. **The prompt and its resource.** The sixth prompt and the built-in
   resource. Tests: `prompts/list` has six prompts; `prompts/get` for
   `operations_setup` returns the six steps; `resources/list` lists the
   built-in resource in a folder with no project and in a project;
   `resources/read` returns the prompt's text.
4. **The proof.** On a fresh scratch project with a scratch `origin`, in an
   interactive Claude Code session: `llm-wiki init`, then the prompt, run to
   its hand-over. Then `llm-wiki init` again adding the `code` pack. Nothing
   is committed and nothing is changed on GitHub.

## Done When

- **The block:** phase 1's tests pass, and on a project initialized with the
  release before this change and edited by hand, the first rerun keeps the
  old text below the block and warns; the second rerun is silent and changes
  nothing outside the block.
- **The kit through the prompt:** in phase 4, the written kit passes the
  setup's own tests (no `{{...}}` or marker left, the kit's pages in the
  index, the scripts parse and are executable, the worktree search script
  registers and cleans up, the commit hook strips the trailers, the
  branch-status check reads the scratch `origin`), and its blind review finds
  no P1 or P2.
- **The rerun:** the checksums of every kit file and of the text outside the
  three blocks are the same before and after phase 4's second init, and the
  kit's index entries are still there.
- **The resource:** a session in a folder with no project lists the
  built-in resource and reads the steps from it.
- **The gates:** `just verify` passes with nothing skipped on each phase's
  head, and CI's fast check is green on each PR.

### Evidence Recorded

In each PR: the names of the tests that prove each rule, and the snapshot
changes. In phase 4's PR: the commands run, the setup's test output, the
blind review's findings, the checksum listing before and after the second
init, and the resource read from a folder with no project. In this plan, once
each phase lands: "Where It Stands" brought up to date.

### Wiki Pages To Update When Done

Whoever does the work rechecks this list:

- `wiki/decisions/composable-project-init.decision.md`: its rerun rule
  ("refreshes framework-owned root schema files"), amended to say a rerun
  refreshes only the marked block in the three root schema files, and that
  the pack's kit files are written only when absent;
- `wiki/specs/wiki-init-skill.spec.md`, "Rerun Behavior", the same;
- a new decision recording the pack, the prompt and the skills exception
  ("written once when absent, never refreshed, tracked or removed, one
  form"), the proposal's second promotion target; the proposal's
  "Promoted To" line then names it;
- `wiki/proposals/project-update-command.proposal.md`, where it treats
  `AGENTS.md` as a whole framework-owned file;
- any other page that lists the MCP prompts or says a rerun rewrites the
  root schema files (search for "rerun" and "wiki_init");
- `README.md`, where it describes init and the MCP prompts;
- this plan, "Where It Stands".

### What May Be Touched

A first list, to be rechecked by whoever does the work: `src/init/`
(compose, scaffold, template, packs, manifest, answers, command),
`src/cli.rs`, `src/mcp/mod.rs`, `templates/base/agents.md` and a new
`templates/packs/` folder for the pack, `tests/init.rs`, `tests/mcp.rs`, the
init snapshots and new ones for the pack.

### What Closes This Plan

The owner's PASS on the PR that meets "Done When" (phase 4's), merged into
`develop`. That PR also sets P19 Completed.

## Open For The Owner

Asked on 2026-10-06; the work starts once the owner has approved this plan.
When the owner answers, the answer is written here with its date.

1. **The pack's name.** Recommended: `operation-manager`, because next to the
   existing `ops` pack, `operations` reads as the same thing. P19 and this
   plan use it until the owner settles it.

## Out Of Scope

- Migrating a project that already has the kit, this repository included:
  they keep their copies.
- This repository's own `AGENTS.MD`, which init does not manage here.
- Letting a fresh init write its block into an existing `AGENTS.md` or
  `CLAUDE.md`: the collision guard stays as it is.
- The `llm-wiki update` command (`project-update-command.proposal.md`).
- The board-as-truth model, the `ops` and `ops-lite` packs, and the wiki
  operation prompts.
- Anything on GitHub, Orca's own setup, and the owner's global instructions.
- Retiring the global skill's own kit, which is the owner's, once phase 4 has
  passed.
