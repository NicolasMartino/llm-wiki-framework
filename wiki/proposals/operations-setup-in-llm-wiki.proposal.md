# Ship The Operations Setup With llm-wiki

- Document Class: Proposal
- Status: Accepted
- Date: 2026-10-06
- Promoted To: wiki/plans/operations-setup-in-llm-wiki.plan.md (accepted by
  the owner on 2026-10-06, #38)
- Category: Project init, MCP prompts, way of working
- Scope: Ship the coordinated way of working (one coordinating session, Orca
  workers on GitHub issues, plans and roadmaps in the wiki as the record of
  work, one blind review per PR, the owner's verdict) with llm-wiki itself: an
  init pack holds the kit's files and an `operations_setup` MCP prompt holds
  the setup steps, so a host like Claude Code runs it as a slash command next
  to the wiki prompts. Wiki-only: nothing is built until the owner accepts it.
- Sources:
  - GitHub issue #33
  - The owner's decision, 2026-10-06: "I think we should propose to include
    inside llm wiki; I think the pack and mcp make sense; so then we can just
    run in claude with the slash command for example?"
  - The owner's global operations setup skill (outside this repository): its
    steps, its standard questions, its fitting guide with the 31 placeholders,
    and its kit
  - src/mcp/mod.rs (`prompts_list_result`, `prompt_text`, `resource_specs`,
    `resource_list`), src/init/packs.rs (`Pack`), src/init/compose.rs,
    src/init/scaffold.rs (`preserves_project_knowledge`), on develop 9b64345
  - wiki/evals/mcp-first-host-parity.eval.md
- Related:
  - wiki/roadmaps/framework-v1.roadmap.md (P18, P19)
  - wiki/decisions/composable-project-init.decision.md
  - wiki/decisions/skill-projection-template-engine.decision.md (superseded)
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md
  - wiki/proposals/mcp-first-surface.proposal.md
  - wiki/plans/mcp-first-agent-surface.plan.md
  - wiki/checklists/operation-manager.checklist.md
  - wiki/checklists/worker-briefs.checklist.md
  - wiki/plans/development-workflow-setup.plan.md
- Promotion Target:
  - a plan for P19, written once this proposal is accepted (when to start it
    is the owner's choice, in "When To Build It")
  - a decision recording the pack, the prompt and the skills exception

## Question

Can a project get the coordinated way of working from llm-wiki alone, instead
of from a skill that lives only on the owner's machine, without bringing back
the shipped skills llm-wiki retired on 2026-06-22?

## What Exists Today

- **The global setup skill.** The owner runs it in a repository. It looks at
  the repository first (read-only), asks the owner the standard questions
  (where the kit goes, the board model, a public repository, the gates, the log
  format, merge settings and branch rules, releases, the branch PRs target,
  the worker kinds, the question count), writes the fitted kit into the
  working tree uncommitted, tests what can be tested in a scratch repository,
  has the result blind-reviewed, and hands over. It changes nothing on GitHub.
- **The kit.** Three decisions (the repository is the truth, work in flight is
  a pushed branch, the pull request is the review surface); the operation
  manager page, the worker briefs page and the setup page; the three
  operations skills (start a session, start a worker, land a PR) with the
  worker kinds' base texts and the workers' shared rules; sections for the
  AGENTS file; the worktree search script and `orca.yaml`; the
  `branch-status` recipe; the commit hook that strips trailers; Claude Code's
  attribution setting; `.gitignore` lines. About thirty `{{UPPER_CASE}}`
  placeholders and a handful of judgements fit it to a repository.
- **This repository is the kit fitted once**:
  `wiki/checklists/operation-manager.checklist.md`,
  `wiki/checklists/worker-briefs.checklist.md`,
  `wiki/plans/development-workflow-setup.plan.md`, the three decisions, the
  operations skills, `tools/wiki-worktree.sh` and the rest.
- **llm-wiki's packs** (`src/init/packs.rs`) contribute folders, document
  types, status words and fragments of `AGENTS.md` and the project
  guidelines. No pack writes a whole file of its own today; the whole files
  init writes are the root schema files, `wiki/index.md` and `wiki/log.md`.
  A rerun of init writes every one of those files again except
  `wiki/index.md` and `wiki/log.md`, which it keeps when they exist
  (`preserves_project_knowledge` in `src/init/scaffold.rs`;
  `wiki/decisions/composable-project-init.decision.md`). So an edit made by
  hand to `AGENTS.md`, `CLAUDE.md` or the project guidelines, pack fragments
  included, is lost at the next rerun.
  There is already an `ops` pack (runbooks, SLOs, on-call, postmortems) and an
  `ops-lite` pack (runbooks only).
- **llm-wiki's MCP prompts** (`src/mcp/mod.rs`): five, `wiki_query`,
  `wiki_ingest`, `wiki_lint`, `wiki_research` and `wiki_init`. Each returns one
  user message of steps, with one optional argument.
- **llm-wiki's MCP resources** (`src/mcp/mod.rs`, `resource_specs`) are all
  files read from the active project (`wiki/index.md`, `wiki/log.md`, the
  wiki operation specs, `AGENTS.md`, the project guidelines), each listed
  only when its file exists. None holds text that comes from the binary
  itself.

## Recommendation

Ship it as two parts: the pack writes the files from values it is given, and
the prompt does what needs a session, looking at the repository, asking the
owner, and the judgements after the write.

### The pack holds the files

- A new pack, named `operation-manager` rather than `operations`: next to the
  existing `ops` pack, `operations` would read as the same thing.
- It holds the kit's files as compile-time templates, like every other
  template llm-wiki ships. The placeholders become typed fields, so a missing
  one is a build error rather than a `{{...}}` left in a project's file.
- Each of the kit's 31 placeholders has one source. The pack takes the
  first three groups as input and works out the fourth; the last two stay
  outside it.
  - **What init already knows** (10): `PROJECT`, `DATE`, `AGENTS_FILE`, and
    the pages' paths and types, which follow from the packs selected (a
    runbook when the `ops` or `ops-lite` pack is there, a checklist and a plan
    otherwise, the kit's own rule): `OPS_PAGE`, `BRIEFS_PAGE`, `PAGE_CLASS`,
    `SETUP_PAGE`, `SETUP_CLASS`, `SETUP_STATUS`, `SETUP_DONE_STATUS`.
  - **The owner's answers to the standard questions** (12): `REPO` and
    `OWNER` (read from the `origin` remote, confirmed by the owner),
    `MAIN_BRANCH`, `GATES`, `GATES_SETUP`, `CI_CLAUSE`, `CI_DONE_LINE`,
    `PUBLIC_NOTE`, `LOG_FORMAT`, `REPO_SETTINGS`, `BRANCH_RULES`, and
    `SETUP_ANSWERS`, the answers themselves in one clause each.
  - **The prompt's look at the repository, step 1** (6): `TOOLS_DIR` (the
    folder the repository already has, offered as the default answer);
    `BRANCH_STATUS` and `BRANCH_STATUS_RECIPE` (whether a justfile exists and
    whether its `status` name is taken); `BARE_COMPLETED` and
    `COMPLETED_LEGACY_NOTE` (whether existing plans say a bare `Completed`);
    and `FIRST_WORK`, a list of free-text bullets (plans whose status is
    outside the vocabulary, pushed branches with no plan, a missing gate
    script, anything the owner named). The look runs before the write, so the
    prompt passes these to init like the answers.
  - **Worked out by the pack itself** (1): `KIT_PATHS`, every path the pack
    wrote or reported for a merge, which it knows exactly.
  - **Written by the prompt after the write** (1): `ALREADY_DONE`, what the
    session did on the machine (registering the project, the MCP server).
    The pack leaves a marked line in the setup page, and the prompt's step 4
    check fails while the marker is there.
  - **Dropped** (1): `NEXT_NUMERAL`. The owner's question count lives in the
    owner's own memory, outside llm-wiki (see "What Stays Outside llm-wiki").
    The setup page says to continue the owner's count without a number, and
    the prompt's hand-over names the last numeral the setup used.
- Some placeholders become conditions rather than text: `CI_CLAUSE` and
  `CI_DONE_LINE` on CI or none, `PUBLIC_NOTE` on public or not,
  `COMPLETED_LEGACY_NOTE` on `BARE_COMPLETED`, `BRANCH_STATUS_RECIPE` on a
  justfile or a script in the tools folder. The text that does not apply is
  never written instead of being deleted by hand.
- The values given to init, `FIRST_WORK` included, are recorded in
  `.llm_wiki/init.toml` with the pack, so a rerun knows them (the record
  composable init already keeps for this reason). The plan names its fields.
- Writing the files is deterministic and refuses to overwrite: a file that
  already exists (`orca.yaml`, `.claude/settings.json`, a commit hook, a
  justfile) is reported, not replaced, and the prompt merges it.
- **Kit files sit outside the rerun refresh.** A kit file is written only when
  it is absent, on the first run and on every rerun; one that exists is left
  as it is and reported. This joins the rule that already keeps
  `wiki/index.md` and `wiki/log.md`, and changes how a rerun treats the
  root schema files in no other way.
- **The kit's AGENTS sections.** The kit appends sections ("How Work Runs",
  "Commits And GitHub") to the AGENTS file, and each project tunes them
  (this repository did for P18). A rerun rewrites the AGENTS file, so where
  they go is an open choice for the owner:
  - a pack fragment of `AGENTS.md`, like every pack's: simplest, but a rerun
    (to add the `code` pack, say) puts back the pack's text and drops the
    project's edits without a word;
  - a section init keeps: init leaves the text between two markers in the
    existing AGENTS file as it is on a rerun. New init code, and the only
    place the root schema files would hold text init does not own;
  - a short fragment that points at a kit page: `AGENTS.md` gets a few fixed
    lines ("How work runs here: read the operation manager page first"), and
    the sections' text goes in the operation manager page, which is a kit
    file and so written once.
  - Recommendation: the short fragment that points at the page. A rerun
    rewrites only lines the project has no reason to edit, the tuned text
    lives in a file the rerun does not touch, and init needs no new kind of
    section. The fragment says its text is init's, so an edit goes to the
    page.

### The prompt holds the steps

- A sixth MCP prompt, `operations_setup`, with an optional `path` argument
  like `wiki_init`. In Claude Code it is the slash command
  `/mcp__llm-wiki__operations_setup`.
- Its steps are the global skill's, in order:
  1. Look at the repository first, read-only, and stop on work in flight
     (uncommitted changes, a run under way).
  2. Ask the standard questions, fitted to what step 1 found, each with its
     default.
  3. Write the kit through the pack with the answers and what step 1 found
     (`llm-wiki init` again, so a rerun), uncommitted, then do what the pack
     cannot: merge into files that already existed, add worker kinds the
     owner asked for, add the line about the project's output folders when it
     has any, write `ALREADY_DONE` into the setup page, and add the kit's wiki
     pages to `wiki/index.md` in its own entry format with one entry in
     `wiki/log.md` in the wiki's log format. A rerun keeps both files as they
     are, so the pack adds no entries and the prompt must, or lint reports the
     pages as orphans.
  4. Test what can be tested in a scratch repository: no placeholder or
     marker left, the kit's pages listed in the index, the scripts parse and
     are executable, the worktree search script registers and cleans up, the
     commit hook strips the trailers, the `branch-status` recipe reads a
     scratch `origin`.
  5. Have the written files blind-reviewed by a fresh session given only the
     repository and the list of files, and fold in every P1 and P2.
  6. Hand over: what was written where, what was fitted and why, what the
     tests and the review found, and what the project's first coordinating
     session does next (the setup page's "Not set yet").
- The same steps are also served as an MCP resource, so a host that does not
  show prompts can still read them when the owner asks for the setup in plain
  words. That resource is a new kind: every resource today is a file read
  from the project and listed only when it exists, and a project about to be
  set up has no such file. This one is built in: its text comes from the
  binary (the same text as the prompt's), it is always listed, and it has its
  own URI rather than one under `llm-wiki://project/`. The plan adds this kind
  to the resource list and to `resources/read`.

### The board model and other options

- Ship one board model: the repository is the truth (plans' Status, the
  board only shows it). The global skill's alternative, the GitHub board as
  the truth, stays out until a project needs it; it rewrites three pages and
  drops a shared rule, which is too much to keep in step in two forms.
- Codex workers need nothing from the kit (the owner's global instructions
  start them); the coordinating session the kit sets up is a Claude Code one.

## How It Fits The No-Shipped-Skills Posture

- What llm-wiki retired on 2026-06-22
  (`wiki/decisions/skill-projection-template-engine.decision.md`) is managing
  its own operations as generated skills: rendering them for Claude and Codex,
  installing, tracking and removing them. That stays retired. The setup's
  steps are an MCP prompt, the surface the MCP-first plan moved the wiki
  operations to.
- The three operations skills are different: they are the project's own
  files, tuned to it after the first write (this repository has already
  changed its copies). The pack writes them only when they are absent, and
  they sit outside the rerun refresh with the other kit files (above), unlike
  `AGENTS.md`, which a rerun rewrites. llm-wiki never updates, tracks or
  removes them, and there is one form, Claude Code's, so no projector comes
  back.
- The decision this proposal promotes to records that exception: written
  once when absent, never refreshed, tracked or removed, one form. That keeps
  it from growing into shipping skills again.

## What Stays Outside llm-wiki

- **Orca** runs the workers and worktrees; the kit only writes `orca.yaml` and
  the worktree search script it calls. The prompt checks that Orca is there
  and says so when it is not; it does not install it.
- **GitHub and `gh`**: the board, the labels, the merge settings and branch
  rules. The project's first coordinating session does them, following the
  setup page, with the owner's go, as today.
- **The owner's global instructions**: the worker model table, the rule that
  every agent is an Orca worker, the launcher, the question count kept in
  memory. They belong to the owner's machine, not to a project.
- **`just`**, when the project does not use it: the `branch-status` recipe
  becomes a script in the tools folder, as the kit already allows.

## What It Does Not Do

- It changes nothing on GitHub and commits nothing: the kit is left
  uncommitted for the owner to read, as the global skill does.
- It starts no workers and runs no release step.
- It does not migrate a project that already has the kit (this repository,
  and the others the global skill fitted). Those keep their copies.
- It does not change the `ops` and `ops-lite` packs, or any wiki operation
  prompt.
- It does not ship the board-is-the-truth model (above).

## Risks

- **The kit names tools inside a project's files.** Orca, Claude Code, `gh`
  and model tiers appear in the pages and skills; a public project publishes
  them. Recommendation: keep them (the pages are about running those tools),
  keep asking the public-repository question, keep the commit hook and the
  rules that keep tool names out of commits and GitHub, and say in the pack's
  description that it names them. The kit's reasons also cite the owner's
  other projects by name; the shipped templates give the reasons without the
  names.
- **Three copies drift.** The global skill's kit, the pack's templates and
  this repository's own fitted pages. Recommendation: once the pack ships, its
  templates are the one canonical kit and the global skill becomes a pointer
  to the prompt. This repository's pages stay its own fitted copy; a general
  rule changed in them gets the template change in the same PR, or a backlog
  entry, and a golden test renders the pack with fixed answers so a template
  change is seen in review.
- **Hosts show prompts differently.** Claude Code documents MCP prompts as
  `/mcp__<server>__<prompt>` slash commands, but the 2026-06-23 run of
  `wiki/evals/mcp-first-host-parity.eval.md` found them missing from its
  non-interactive slash-command list, and Codex exposed no prompts at all.
  Recommendation: the resource copy of the steps above, and the plan's proof
  is an interactive Claude Code session.
- **The steps keep judgement in them.** Typed fields and conditions remove
  most of the hand edits, not merging into existing files. Recommendation: the
  prompt's test and blind-review steps stay, as in the global skill.

## When To Build It

The owner's decision of 2026-10-06 is to propose it; it sets no condition on
when to build. The kit has been fitted twice so far (the owner's other project
and this repository), and each fit changed it, so when to start the plan is
an open choice for the owner:

- **Start the plan once the proposal is accepted.** The pack is built from
  the kit as it stands, and a later fit's fixes go into the pack's templates.
- **Start it after one more fit with the global skill** on another project,
  with that fit's fixes folded into the kit first.

Recommendation: start the plan once the proposal is accepted. Each fit's
fixes so far were to the kit's text, which the templates carry as they are,
and once the pack ships a fix lands in one place instead of in the global
skill first and the templates after. Waiting would mean a third fit by hand
of a kit that the pack is meant to replace.

## Open Choices For The Owner

- **When to build it** ("When To Build It"). Recommendation: start the plan
  once the proposal is accepted. Answered by the owner on 2026-10-06, "asap":
  the plan comes now, with no wait for another fit of the global skill.
- **Where the kit's AGENTS sections go** ("The pack holds the files").
  Recommendation: a short fragment that points at the operation manager page.
  Answered by the owner on 2026-10-06, "I think we should have a needle, a
  part of the agents.md file that is dedicated to llm wiki": `AGENTS.md` gets
  one marked block that llm-wiki owns, init writes and refreshes only what is
  between its markers and leaves the rest, the project's own, untouched, and
  the kit's sections go inside the block. The plan carries it out as its own
  phase, before the pack.
- **The pack's name.** Recommendation: `operation-manager`, because next to
  the existing `ops` pack, `operations` reads as the same thing. P19 names the
  pack by this proposed name until the owner settles it; the worker who
  writes P19's plan renames it there if the owner picks another. Not answered
  yet on 2026-10-06; the plan lists it in "Open For The Owner".

## Increments

1. **The proposal is accepted** by the owner, with the open choices above
   answered. This is the first step of P19, not its end.
2. **The plan**: the pack, the answers record, the rule that keeps kit files
   out of the rerun refresh, the AGENTS fragment, the prompt and its built-in
   resource, the golden test, and the decision recording the skills
   exception. Any list of what it touches is a first list, rechecked by
   whoever writes the plan.
3. **The proof**: on a fresh project, `llm-wiki init` and then
   `/mcp__llm-wiki__operations_setup` in an interactive Claude Code session
   write a kit that passes the global skill's tests and comes back from its
   blind review with no P1 or P2. A second rerun of `llm-wiki init` (adding a
   pack) leaves every kit file and the project's index entries as they were.
4. **Retire the global skill's own kit**: the skill points at the prompt.

## Success Criteria

- The owner accepts this proposal and answers its open choices.
- A fresh project gets the kit from llm-wiki alone, through the prompt, with
  no `{{...}}` left, the kit's pages in the index and the log, and nothing
  committed or changed on GitHub.
- A rerun of `llm-wiki init` changes no kit file and loses no edit a project
  made to the kit.
- llm-wiki ships no skill it manages; the three operations skills are written
  once and owned by the project.
- One canonical kit remains, in llm-wiki's templates.
