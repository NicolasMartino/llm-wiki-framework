# Ship The Operations Setup With llm-wiki

- Document Class: Proposal
- Status: Proposed
- Date: 2026-10-06
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
    steps, its standard questions, its placeholder list and its kit
  - src/mcp/mod.rs (`prompts_list_result`, `prompt_text`), src/init/packs.rs
    (`Pack`), src/init/scaffold.rs, on develop 9b64345
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
  - a plan for P19, written once this proposal is accepted and the owner's
    condition below is met
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
  There is already an `ops` pack (runbooks, SLOs, on-call, postmortems) and an
  `ops-lite` pack (runbooks only).
- **llm-wiki's MCP prompts** (`src/mcp/mod.rs`): five, `wiki_query`,
  `wiki_ingest`, `wiki_lint`, `wiki_research` and `wiki_init`. Each returns one
  user message of steps, with one optional argument.

## Recommendation

Ship it as two parts, split by what can be decided without looking at the
repository and what cannot.

### The pack holds the files

- A new pack, named `operation-manager` rather than `operations`: next to the
  existing `ops` pack, `operations` would read as the same thing.
- It holds the kit's files as compile-time templates, like every other
  template llm-wiki ships. The placeholders become typed fields, so a missing
  one is a build error rather than a `{{...}}` left in a project's file.
- The fields fall in two groups:
  - filled from what init already knows: the project name, the AGENTS file
    name, the date, and the page type for the operation manager and briefs
    pages (a runbook when the `ops` or `ops-lite` pack is selected, a
    checklist otherwise, which is the kit's own rule);
  - filled from the owner's answers to the standard questions: the repository,
    the owner, the branch PRs target, the gate command, CI or none, public or
    not, the tools folder, the log format, the merge settings and branch
    rules.
- The answers are recorded in `.llm_wiki/init.toml` with the pack, so a rerun
  knows them (the record composable init already keeps for this reason).
- Choices that are yes-or-no in the kit today (no CI, a public repository)
  become conditions in the templates, so the text that does not apply is never
  written instead of being deleted by hand.
- Writing the files is deterministic and refuses to overwrite: a file that
  already exists (`orca.yaml`, `.claude/settings.json`, a commit hook, a
  justfile) is reported, not replaced, and the prompt merges it. A rerun never
  rewrites a kit file the project has since changed; it reports the
  difference, the way rerun init already preserves `wiki/index.md` and
  `wiki/log.md`.

### The prompt holds the steps

- A sixth MCP prompt, `operations_setup`, with an optional `path` argument
  like `wiki_init`. In Claude Code it is the slash command
  `/mcp__llm-wiki__operations_setup`.
- Its steps are the global skill's, in order:
  1. Look at the repository first, read-only, and stop on work in flight
     (uncommitted changes, a run under way).
  2. Ask the standard questions, fitted to what step 1 found, each with its
     default.
  3. Write the kit through the pack with the answers, uncommitted, then do the
     judgements the pack cannot: merge into files that already existed, add
     worker kinds the owner asked for, add the line about the project's output
     folders when it has any.
  4. Test what can be tested in a scratch repository: no placeholder left, the
     scripts parse and are executable, the worktree search script registers
     and cleans up, the commit hook strips the trailers, the `branch-status`
     recipe reads a scratch `origin`.
  5. Have the written files blind-reviewed by a fresh session given only the
     repository and the list of files, and fold in every P1 and P2.
  6. Hand over: what was written where, what was fitted and why, what the
     tests and the review found, and what the project's first coordinating
     session does next (the setup page's "Not set yet").
- The same steps are also served as an MCP resource, so a host that does not
  show prompts can still read them when the owner asks for the setup in plain
  words.

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
  changed its copies). The pack writes them once, like `AGENTS.md`; llm-wiki
  never updates, tracks or removes them, and there is one form, Claude Code's,
  so no projector comes back.
- The decision this proposal promotes to records that exception in those
  words, so it does not grow into shipping skills again.

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

## The Owner's Condition

Build it from the global skill once that skill has been used on one more
project. The kit has been fitted twice so far (the owner's other project and
this repository), and each fit changed it. Recommendation: accept the
proposal now, keep P19 Active, and start its plan after the third fit, with
that fit's fixes folded into the kit first.

## Increments

1. **A third fit** with the global skill on another project; fold its fixes
   into the kit.
2. **The plan**: the pack, the answers record, the prompt and its resource
   copy, the golden test, and the decision recording the skills exception.
   Any list of what it touches is a first list, rechecked by whoever writes
   the plan.
3. **The proof**: on a fresh project, `llm-wiki init` and then
   `/mcp__llm-wiki__operations_setup` in an interactive Claude Code session
   write a kit that passes the global skill's tests and comes back from its
   blind review with no P1 or P2.
4. **Retire the global skill's own kit**: the skill points at the prompt.

## Success Criteria

- The owner accepts this proposal, and P19's plan names the third fit it
  waited for.
- A fresh project gets the kit from llm-wiki alone, through the prompt, with
  no `{{...}}` left and nothing committed or changed on GitHub.
- llm-wiki ships no skill it manages; the three operations skills are written
  once and owned by the project.
- One canonical kit remains, in llm-wiki's templates.
