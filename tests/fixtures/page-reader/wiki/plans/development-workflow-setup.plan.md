# Development Workflow Setup

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-10-06
- Category: Development process
- Scope: The one-time steps that start this repository's way of working: the
  first commit, the board and its ids, wiki search for this repository, Orca,
  poman's decisions and roadmap, and the first roadmap entries. Done once; afterwards
  this page is the record of how it was done.
- Sources:
  - The owner's decisions of 2026-10-06 on how this repository is worked on
  - The owner's answers of 2026-10-06 in the session that wrote this kit: the
    kit lands in the main checkout uncommitted for the owner to read (L); it is
    tracked although the repository is public, with no machine paths (LI); a
    worker's gates are `just verify` and CI on the PR's head (LII); this wiki
    keeps its own log format (LIII); no GitHub branch rules or merge settings
    yet (LIV); releases stay the owner's (LV)
- Related:
  - `wiki/checklists/operation-manager.checklist.md`
  - `wiki/decisions/work-is-recorded-in-the-repository.decision.md`

## How It Went (2026-10-06)

Done in one day, with these changes to the steps below, each the owner's:

- **Pushes go through `gh`** (step 1): SSH to GitHub failed from the
  coordinating session, so `origin` became HTTPS, which `gh` serves.
- **The board's ids landed early** (step 3), so workers started during step 7
  did not read "Not set yet".
- **The source material for poman's design stays outside this repository**
  (step 7); poman's decisions cite the owner's decisions of 2026-10-06.
- **Work moved onto `develop`** the same day: `develop` became GitHub's default
  branch, with a fast check on PRs into it and the full CI on the way to master
  (`wiki/plans/develop-and-master-ci.plan.md`), and the way of working was
  pointed at it (framework-v1 roadmap, P11).
- **The first PRs landed** through one blind review, a fix round and the
  owner's PASS: the plan statuses (#6), poman's decisions and roadmap (#8),
  the way of working on `develop` (#11) and the develop and master CI (#12).
- **Left for later:** the board's automations are the owner's to switch on in
  the browser (step 3); the other first work is in the framework-v1 roadmap's
  backlog.

## Before You Start

- **Ask the owner for the go** before step 1: steps 1, 3 and 8 publish to
  GitHub, and this repository is public. Number the question as the operation
  manager checklist, "Talking With The Owner", says. Two sessions numbered the
  owner's questions on 2026-10-06 (riseon's reached L, the one that wrote this
  kit LV): carry on from **LVI**, and say so to the owner. (On 2026-10-06
  the owner restarted the count at I.)
- **What is already done** (2026-10-06):
  - the `llm-wiki` MCP server is registered for Claude Code at user scope, so
    `llm_wiki_search` and `llm_wiki_read` work in every Claude session once
    this repository is registered (step 5);
  - the kit is in the working tree, uncommitted: AGENTS.MD's additions,
    `.claude/skills/operations*`, `.claude/settings.json` (Claude Code's
    attribution off), the two checklists, three decisions, this plan,
    `orca.yaml` with `tools/wiki-worktree.sh`, `.githooks/commit-msg`, the
    justfile's `branch-status` recipe, `.gitignore`, and the index and log
    entries.
- **No step mentions AI or tools** in a commit message, an issue, a PR, a
  label or the board.

## 1. The First Commit

Setup commits straight to master (step 1) and to `develop` (step 8): the
review flow this plan starts does not exist until it is done.

```bash
cd <this repository's main checkout>
git config core.hooksPath .githooks       # the commit-msg hook strips AI trailers
git status --short                        # read every path before committing
git add -A
git commit -m "Wiki: Start the development workflow: plans as the record, a board as their view, one blind review per PR"
git push
```

Done 2026-10-06 as `70bc5a6`. SSH to GitHub failed from the coordinating
session ("Host key verification failed"), so, at the owner's suggestion (LVII:
"couldn't we use gh?"), `origin` was switched to HTTPS
(`git remote set-url origin https://github.com/NicolasMartino/llm-wiki-framework.git`),
which `gh` already serves as the machine's credential helper for github.com.
Worktrees share the repository's config, so workers push the same way.

The board's ids (step 3) were committed straight to master as soon as the board
existed, ahead of step 8, so that workers started during step 7 do not read
"Not set yet".

## 2. The Repository's Settings

None yet (LIV): no branch rules, and squash, merge and rebase merges all stay
allowed. `/operations-land` merges with squash, pinned to the reviewed head.
When the owner wants GitHub to enforce it, a public repository gets branch
rules for free: a ruleset on `develop` requiring a PR and CI's fast check,
with squash-only merges and merged branches deleted, and one on master
requiring a PR from `develop` and the full CI, with merge commits.

## 3. The Board

```bash
gh project create --owner NicolasMartino --title "llm-wiki-framework" --format json --jq '{number, id}'
gh project link <P> --owner NicolasMartino --repo NicolasMartino/llm-wiki-framework
gh project field-list <P> --owner NicolasMartino --format json \
  --jq '.fields[] | select(.name=="Status") | .id'

# the Status field's options are the plan statuses (this replaces Todo, In Progress and Done)
gh api graphql -f query='
mutation($field: ID!) {
  updateProjectV2Field(input: {fieldId: $field, singleSelectOptions: [
    {name: "Draft",     color: GRAY,   description: "Planned, not started"},
    {name: "Active",    color: YELLOW, description: "A worker is on it"},
    {name: "Blocked",   color: RED,    description: "Waits on what its plan names"},
    {name: "Completed", color: GREEN,  description: "Merged"}
  ]}) { projectV2Field { ... on ProjectV2SingleSelectField { id options { id name } } } }
}' -f field=<FIELD_ID>

gh label create track --repo NicolasMartino/llm-wiki-framework --color 5319E7 \
  --description "A track's parent issue" --force
```

Then:

- **Write the ids into** `wiki/checklists/operation-manager.checklist.md`, "The
  Board", "How": rewrite the "Not set yet" paragraph with the real values, and
  replace `<P>`, `<PROJECT_ID>` and `<FIELD_ID>` in the commands below it, and
  `<P>` in `.claude/skills/operations/SKILL.md`. Leave the per-command
  placeholders (`<n>`, `<item>`, `<option>`, `<blocker>`) as they are. Check
  with `grep -rn "<P>\|<PROJECT_ID>\|<FIELD_ID>\|Not set yet" wiki .claude`
  (only this page and the "Not set yet" checks in `/operations` and the
  checklist's intro may still match).
- **The owner, in the browser** (the API cannot turn workflows on), on the
  project's Workflows page: "Auto-add to project" for this repository with the
  filter `is:issue`; "Item added to project" sets Status Draft; "Item closed"
  sets Status Completed. Ask the owner, and add issues by hand
  (`gh project item-add`) until it is done.
- **Milestones:** none. The roadmaps' deliverables are the stages
  (`work-is-recorded-in-the-repository.decision.md`, rule 6).

## 4. Nothing To Install

The gates exist already: `just verify` locally, `ci.yml` on every PR. The
justfile's own `status` recipe is the CLI's status; the branch check is
`just branch-status`.

## 5. Wiki Search For This Repository

This repository is not registered with llm-wiki, so `llm_wiki_search` fails
here today. Register it without MCP wiring (Claude Code has the server at user
scope, and wiring would write a machine path into this public repository),
then build its index:

```bash
~/.llm_wiki/bin/llm-wiki register . --id llm-wiki-framework --name llm-wiki-framework --no-mcp
~/.llm_wiki/bin/llm-wiki index --project llm-wiki-framework     # minutes; run it in the background
~/.llm_wiki/bin/llm-wiki projects | grep llm-wiki-framework      # root-ok, index-present
```

Then check `llm_wiki_search` finds this plan. Registration writes nothing into
the repository; a worktree's own `.llm_wiki/` is ignored by `.gitignore`.

## 6. Orca

```bash
orca repo add --path <this repository's main checkout> --json
```

Orca's other settings are machine-wide: worker worktrees go under
`~/orca/workspaces/llm-wiki-framework/`, branches are prefixed with the git
username, and agents start through the owner's launcher, which finds
`wiki/index.md` here and keeps the wiki tools uncompressed.

`orca.yaml` (committed in step 1) gives each new worktree its own wiki search,
takes it away on removal, and makes a worker's agent wait for it (operation
manager checklist, "Worktrees"). Prove it once, on a throwaway worktree:

```bash
orca worktree create --repo path:<main checkout> --name setup-check --base-branch develop --json
~/.llm_wiki/bin/llm-wiki projects | grep -- '--setup-check'     # registered, index-present
(cd ~/orca/workspaces/llm-wiki-framework/setup-check && ~/.llm_wiki/bin/llm-wiki search "operation manager" --limit 1)
orca worktree rm --worktree path:$HOME/orca/workspaces/llm-wiki-framework/setup-check --run-hooks --json
~/.llm_wiki/bin/llm-wiki projects | grep -c -- '--setup-check'  # 0
```

If the worktree lives elsewhere, `orca worktree list --json` shows its path.

## 7. The First Work

Each with the owner's go, each a roadmap entry and an issue on the board:

- **poman's decisions and roadmap**: a wiki worker turns the owner's settled
  decisions for poman (2026-10-06) into decision pages, and their order into a
  roadmap for the owner to approve. Its source material stays outside this
  repository (the owner, 2026-10-06).
- **The work already in flight**: `impl/install-download-progress` (pushed
  2026-10-06) and `impl/sandbox-safe-search-cache-reads` (2026-06-21) each get
  a roadmap entry, a plan if they lack one, an issue and a draft PR into the
  usual landing, or the owner parks them.
- **Four plans whose status is outside the vocabulary**, which
  `just branch-status` reports as STALE (2026-10-06):
  `headroom-mcp-merge-readiness-repair` and
  `macos-installed-binary-codesign-repair` (a metadata block without bullets,
  so no `- Status:` line is found; they say "Implemented - host parity gate
  pending" and "Implemented - native release archive proof pending"),
  `headroom-passthrough-launcher` and `headroom-wrap-command` ("Implemented…"). A wiki PR gives each a vocabulary status, with what is
  still pending written in the body. Step 8 needs this.
- **A pain point found while writing this kit**: `just build-skills` and
  `just build-skills-to` call `cargo run -- build`, a subcommand the CLI no
  longer has.

## 8. Close The Setup

- `just branch-status` runs clean.
- `wiki/log.md` gets one entry at the top for the setup, in this wiki's format.
- This plan becomes `Completed (develop)`, and its index entry says it is the
  record of how the setup was done. The "First time here?" pointers in
  AGENTS.MD and `/operations` look for "Not set yet", which is gone.
- Commit and push it, straight to `develop` (AGENTS.MD's one exception to
  "every change goes through a PR"):

  ```bash
  git add -A && git status --short    # the checklist's ids, the skill's <P>, this plan, the log, the index
  git commit -m "Wiki: Record the board and close the development workflow setup"
  git push
  ```

  Why: without this commit a fresh clone or a worker's worktree still says
  "Not set yet", and the next session would set up a second board.
- Tell the owner in a few lines: what was set, the board's link, and what is
  left for them (the workflows, if not yet done).
