# AGENTS.md - Project Schema

This project defines and dogfoods a software project management framework
based on the LLM Wiki pattern. The framework is the product. The wiki
manages its own development.

## Agent Role

You own `wiki/`. You write, update, cross-link, and maintain all wiki content.
Humans curate `raw/` and make judgment calls. You handle the bookkeeping.

## How To Orient

1. Read `templates/base/project_guidelines.md` for the documentation model and rules.
2. Read `wiki/index.md` for the catalog of all project knowledge.
3. Read specific wiki pages identified from the index.
4. Read `raw/` sources only when wiki content is insufficient.

Never browse the filesystem to find information. `wiki/index.md` is your
entry point.

## Operations

### Ingest

When new material appears in `raw/`:

1. Read the raw source fully.
2. Identify facts, entities, relationships, decisions.
3. Write or update wiki pages (use the correct document type).
4. Check for contradictions with existing wiki content.
5. Update `wiki/index.md`.
6. Append to `wiki/log.md`.

### Query

When answering questions:

1. Read `wiki/index.md` to find relevant pages.
2. Read those pages.
3. Synthesize an answer with citations.
4. If the answer is durable new knowledge, file it as a wiki page.

### Lint

Periodically or on request:

1. Scan for contradictions between pages.
2. Find stale statuses or outdated claims.
3. Identify orphan pages not linked from index.
4. Check for missing cross-references.
5. Fix issues directly.
6. Log all changes in `wiki/log.md`.

## Conventions

- Document types: spec, decision, proposal, roadmap, plan, experiment,
  eval, checklist, reference.
- Use the type by role, not convenience. See `templates/base/project_guidelines.md`.
- Every wiki page has a metadata block (Document Class, Status, Date,
  Category, Scope, Sources).
- Filenames: `[slug].type.md` or `[index]-[slug].type.md`.
- Archived documents go to `wiki/archive/`.
- `wiki/log.md` uses format: `## [YYYY-MM-DD] operation | subject`.

## This Project Specifically

This project is self-referential. The framework is both the product and
the management system. When working here:

- Specs in `wiki/specs/` define the framework's own rules.
- Decisions in `wiki/decisions/` record why the framework works this way.
- The roadmap in `wiki/roadmaps/` coordinates the framework's deliverables.
- Every pain point is a bug in the product.
- The ultimate proof: a fresh agent in a fresh project, using only
  these specs, can bootstrap and maintain a self-managing wiki.

## How Work Runs

The owner decides. A coordinating session (the operation manager) turns the
owner's go into roadmap entries, plans, issues and Orca workers, starts each
PR's blind review, lands PRs on the owner's PASS, and keeps the plans and the
board true. It writes no product code.

- **First session here?** If `wiki/checklists/operation-manager.checklist.md`,
  "The Board", says "Not set yet", follow
  `wiki/plans/development-workflow-setup.plan.md` first, with the owner's go.
- **Coordinating:** load `/operations`, then `/operations-start <issue>` to
  start a worker and `/operations-land <PR>` to land its PR. The rules and the
  reason for each are in `wiki/checklists/operation-manager.checklist.md`; what
  each worker is told, and why, in `wiki/checklists/worker-briefs.checklist.md`.
- **The repository is the truth**
  (`wiki/decisions/work-is-recorded-in-the-repository.decision.md`): every
  deliverable is a roadmap entry, a code or tooling deliverable has its plan
  before its worker starts, the plan's `Status` is where the work stands, and
  the GitHub board only shows it. A plan's status changes in the PR that does
  the work: Active with its Branch line in the first push, Completed before
  the PR leaves draft; the coordinator commits no status and moves the board.
- **Work in flight is a branch on `origin`**, not a worktree
  (`wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md`). A plan under
  way names its branch (`- Branch: \`<branch>\``) on that branch, so `develop`
  shows it as Draft until its PR merges, and a plan completed from now on
  says where its proof holds: `Completed (develop)`, `(local)` or `(spike)`.
  `just branch-status` checks both against `origin`.
- **Every PR is reviewed twice**: one blind feedback review that never gives a
  verdict, then the owner's verdict, a PR comment with PASS or FAIL in its
  heading (`wiki/decisions/the-pull-request-is-the-review-surface.decision.md`).
- **Work PRs go into `develop`**, GitHub's default branch, and workers start
  from it; master takes `develop` only through a PR from `develop`, with the
  full CI and the owner's PASS
  (`wiki/decisions/the-pull-request-is-the-review-surface.decision.md`, rule 6).
- **Every change goes through a PR**, ingest and lint included, the log and
  the backlog too: the coordinator adds one bookkeeping commit to each PR into `develop`,
  holding its own log entry (at the top of `wiki/log.md` in its usual form,
  citing the PR) and any backlog roadmap entries waiting then, just before the
  PR goes ready, or just after the owner's PASS if it touches only the log and
  roadmap entries. No log PRs, and no roadmap entry committed straight to
  `develop` (`wiki/checklists/operation-manager.checklist.md`, "Landing A PR";
  the owner, 2026-10-07). Where "Operations" above says "append to `wiki/log.md`" or "fix
  issues directly", that is how it happens here. The only exception is the
  setup plan's two commits, which went straight to `develop`.
- **A worker** follows its spec and the rules pasted under it; it sets its own
  plan's `Status` and `Branch` lines in its PR, and never edits `wiki/log.md`,
  this file, any other plan's status, or `templates/` unless its spec names
  it, never merges or releases, and never starts workers of its own.
- **Worktrees:** Orca makes them, and `orca.yaml` gives each its own wiki
  search (`tools/wiki-worktree.sh`); remove one with
  `orca worktree rm --run-hooks`.
- `Sources` may also cite an owner's decision with its date, or an issue or PR.

## Commits And GitHub

- **Titles:** an issue in a track `<Track> · <Area>: <what>`, a track issue
  `<Track> track: <what>`, any other issue `<Area>: <what>`. A PR, and so its
  squash commit, takes the pattern of the issue it closes.
- **No mention of AI, models, agents or the tools that do the work** in commit
  messages or anywhere on GitHub, folder names included: no footer, no
  trailer, no path on anyone's machine. This repository is public. Repository
  files may name tools. `.githooks/commit-msg` strips the usual trailers
  (enabled with `git config core.hooksPath .githooks`), and
  `.claude/settings.json` turns Claude Code's attribution off.
- **Backlog means a roadmap entry** and its issue on the board; a line in a
  chat or a plan alone gets lost.

## Read-Only Access Contract For `wiki/` And `raw/`

These rules keep `wiki/` and `raw/` provenance exact, and keep them safe if a
context-compression tool such as Headroom is ever run in front of the model. See
`wiki/references/headroom-context-compression.reference.md` and
`wiki/decisions/headroom-single-posture-mcp-first.decision.md` for the full
posture.

**Route wiki/raw through the MCP tools.** When the host exposes the LLM Wiki MCP
server, read and search `wiki/` and `raw/` through the framework-owned tools —
`llm_wiki_read`, `llm_wiki_search`, and `llm_wiki_search_all` — rather than
shell-family file reads/search (`cat`, `sed`, `grep`). On Claude Code the native
`Read` tool is an acceptable fallback for a single wiki/raw file; on Codex (no
native read tool) the MCP tools are the primary surface. These outputs are
produced by llm-wiki itself, so read this way they are exact at the MCP server
boundary; if an optional host/proxy later replaces content with CCR markers,
compression envelopes, or omitted payload fields, treat that as a failed
delivery check rather than exact content.

**`headroom_read` ban.** If you run Headroom, do not enable `HEADROOM_MCP_READ=on`
and do not call the `headroom_read` MCP tool against any `wiki/` or `raw/` path.
That tool routes file content through CCR with retrieval markers, which would
defeat exact provenance. Headroom is an optional, unmanaged external runtime; the
framework does not bundle or configure Headroom itself (the four-mode proxy
companion was retired 2026-06-30). It does ship one launch convenience,
`llm-wiki headroom [--headroom-bin <PATH>] [--unsafe-mcp-read] [--] <headroom args...>`,
which execs the real `headroom` binary with `HEADROOM_MCP_READ=off` and the
`llm_wiki_*` MCP tools added to `HEADROOM_EXCLUDE_TOOLS`. Exact wiki/raw
provenance still comes from routing reads/search through the `llm_wiki_*` MCP
tools; `HEADROOM_EXCLUDE_TOOLS` is best-effort only and is not a provenance
boundary. Known `headroom-ai 0.24.0` Codex/OpenAI-Responses runs did not consult
that exclude list for normal tool-output compression; inspected
`headroom-ai 0.32.0` source appears to honor Responses excludes, so live
Headroom runs must record the exact version/path and treat CCR markers,
compression envelopes, or omitted payload fields as failed delivery checks.
