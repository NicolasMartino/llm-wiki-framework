# MCP-First Agent Surface

- Document Class: Proposal
- Status: Proposed
- Date: 2026-06-22
- Category: Internal architecture, agent surface, harness independence, complexity reduction
- Scope: Make a single Rust-native stdio MCP server (`llm-wiki mcp serve`, no SDK, single-binary preserved) the framework's primary agent surface. Deterministic operations become harness-independent tools. Procedural workflow guidance moves to the always-on project instruction layer (`AGENTS.md` / harness equivalents) plus MCP resources; MCP prompts are optional unless a host-UX spike proves they are supported and discoverable on both Claude Code and Codex. The intended end state is a no-legacy cutover: generated Claude/Codex skill projection, Codex runtime YAML, dual projectors, and dual install/embed/uninstall paths are removed after ordinary-language cross-harness parity is proven. This proposal does not change the document model, `agent-owns-wiki`, `wiki/`+`raw/` provenance invariants, the `headroom_read` ban, or the single-binary distribution invariant.
- Sources:
  - wiki/proposals/harness-independent-wiki-read-tool.proposal.md
  - wiki/plans/harness-independent-wiki-read-tool.plan.md
  - wiki/plans/mcp-first-agent-surface.plan.md
  - wiki/references/headroom-context-compression.reference.md
  - raw/research/2026-06-11-headroom-llm-wiki-comparison/sources/05-headroom-docs-mcp.mdx
  - raw/research/2026-06-20-codex-tool-surface/research-summary.md
- Related:
  - wiki/decisions/skill-projection-template-engine.decision.md
  - wiki/decisions/llm-wiki-binary-distribution.decision.md
  - wiki/decisions/agent-owns-wiki.decision.md
  - wiki/decisions/composable-project-init.decision.md
- Promotion Target:
  - wiki/plans/mcp-first-agent-surface.plan.md
  - wiki/decisions/skill-projection-template-engine.decision.md (gated cutover)
  - wiki/references/headroom-context-compression.reference.md

## Question

Can the framework reduce its complexity significantly - specifically the
per-harness code that exists only to project the same operations into Claude and
Codex skills - by making one MCP server the primary deterministic surface,
without losing ordinary-language workflow routing or weakening wiki/raw
provenance invariants?

## Observed Problem

The framework maintains parallel runtime surfaces for the same operations:

- `templates/skills/` carries `base.md`, `claude.md`, `codex.md`, and
  `codex_runtime_config.yaml`.
- `crates/llm-wiki-schema/src/projector/{claude,codex,...}` and
  `src/skill_render.rs` own dual projection and idiom conversion.
- `src/install.rs` renders and tracks per-harness materialized skill paths.
- `src/embed.rs` embeds both `skill_md` and `codex_openai` payloads.
- `src/uninstall.rs` mirrors the dual layout for teardown.

The same problem appeared in the retired Headroom integration. Codex has no
native `Read` tool and routes file reads and `llm-wiki search` through
shell-family tools, so any host-tool-name enumeration was fragile to upstream
Codex renames and tended to suppress useful compression. The durable fix is
framework-owned, harness-independent MCP tool names such as `llm_wiki_search`,
`llm_wiki_search_all`, and `llm_wiki_read`.

The original MCP-first framing treated MCP prompts/resources as a direct
replacement for skills. That is not yet proven. Skills and prompts are different
host UX primitives: skills can be intent-routed by description, while MCP
prompts may be user-invoked slash-command surfaces and may not exist in every
MCP client. The plan must prove host behavior before making prompts
load-bearing.

## Proposal

Make a single Rust-native MCP server (`llm-wiki mcp serve`, stdio, no SDK,
single-binary preserved) the primary agent surface in three parts.

### Part 0: Verify host UX before relying on prompts

Before deletion work, run a cheap spike that answers:

- Does Codex's MCP client support prompts at all?
- If prompts exist, do Claude Code and Codex expose them in a way ordinary users
  can discover without special instructions?
- Can ordinary-language requests like "query the wiki" or "ingest this folder"
  still reach the right workflow without the user naming a prompt?
- Can `AGENTS.md` / harness project instructions plus MCP tools/resources carry
  the workflow if prompts are unsupported or not auto-routed?

This spike decides whether prompts are part of the portable surface or only a
convenience layer.

### Part 1: Deterministic operations become MCP tools

Host deterministic cores as harness-independent MCP tools, one implementation
serving all supported harnesses:

- `llm_wiki_read` (increment 1, already proposed/planned)
- `llm_wiki_search`
- `llm_wiki_search_all`
- `llm_wiki_index`
- `llm_wiki_register`
- `llm_wiki_status`

This removes duplicated packaging for these operations and turns wiki/raw read
routing into a framework-owned tool-name fact rather than a per-harness
enumeration of host tool names.

### Part 2: Procedural guidance uses project instructions plus MCP resources

Keep always-on role and workflow obligations in `AGENTS.md` and harness-native
project instruction files. Expose canonical wiki documentation, operation
checklists, and project resources through MCP resources. Add MCP prompts only
when the Phase 0 spike proves the host supports them well enough to preserve
ordinary user workflows.

This reframes the procedural layer around confirmed primitives:

- project instructions for automatic role/workflow routing;
- MCP tools for deterministic operations;
- MCP resources for canonical project knowledge;
- MCP prompts as optional convenience, not the foundation.

### Part 3: Retire skill projection after ordinary-language parity

After deterministic tools and the procedural guidance surface pass parity on
both Claude Code and Codex, retire the dual skill-projection machinery:

- Claude and Codex skill markdown renderings.
- Codex runtime YAML rendering.
- Claude/Codex projector-specific runtime output.
- Per-skill dual install/embed/uninstall manifest entries.

This part supersedes
`wiki/decisions/skill-projection-template-engine.decision.md`, but only when
the cutover gate passes. Until then, current skills may exist as migration
scaffolding. They are not an accepted permanent fallback for this proposal.

## Relationship To The Read Tool

`wiki/proposals/harness-independent-wiki-read-tool.proposal.md` and
`wiki/plans/harness-independent-wiki-read-tool.plan.md` are increment 1 of this
surface. They provide the first server scaffold and the first framework-owned
tool name, `llm_wiki_read`.

That plan intentionally ships only the read half; the search MCP tools land
later. The broader MCP-first plan
(`wiki/plans/mcp-first-agent-surface.plan.md`) owns the host-UX spike, search,
project-instruction/resource parity, install cutover, deletion of projection
machinery, and the later Headroom posture change.

## What This Does Not Change

- The document model and typed-document discipline.
- `wiki/index.md` and `wiki/log.md` bookkeeping obligations.
- `agent-owns-wiki`: agents still operate through explicit project rules, not
  free-form raw model memory.
- `AGENTS.md` and harness-native project instructions remain valid always-on
  workflow guidance.
- The `wiki/`+`raw/` provenance invariants.
- The `headroom_read` ban.
- The single-binary distribution invariant.
- The CLI: subcommands remain for humans and scripts; MCP tools reuse the same
  deterministic cores.

## What This Supersedes (Gated)

- `wiki/decisions/skill-projection-template-engine.decision.md` - candidate
  supersession, effective only when the no-legacy cutover gate passes.

Until that gate passes, the accepted decision remains true for the existing
runtime surface.

## Out Of Scope

- Implementing `llm_wiki_search`, `llm_wiki_search_all`, `index`, `register`, or
  `status` in the read-tool plan. Those are sequenced in
  `wiki/plans/mcp-first-agent-surface.plan.md`.
- Deleting any generated skill runtime surface before ordinary-language
  cross-harness parity is proven.
- Changing the Codex Recommended Posture in the Headroom reference before the
  MCP read and search tools are live and verified.
- Adopting tokio or an MCP SDK unless the read-tool plan's transport decision is
  reopened with evidence.
- Changing the document model or removing `AGENTS.md` as the always-on project
  instruction layer.
- Keeping generated skill projection as a permanent compatibility layer.

## Increments

Each increment must pay off independently, but the intended completion state is
the no-legacy cutover in `wiki/plans/mcp-first-agent-surface.plan.md`.

1. **Host UX spike** - confirm Codex prompt support, test ordinary-language
   routing on both harnesses, and decide whether prompts are portable or
   optional.
2. **MCP server + `llm_wiki_read`** - execute the read-tool plan. This proves the
   server surface and the framework-owned MCP read tool.
3. **Deterministic MCP tools** - port `search`, `search-all`, `index`,
   `register`, and `status` to the server, with CLI/MCP JSON parity.
4. **Procedural guidance surface** - prove `AGENTS.md` / harness project
   instructions plus MCP tools/resources preserve workflow routing. Add prompts
   only where supported and useful.
5. **No-legacy decision gate** - if ordinary-language parity holds, delete
   runtime skill projection and supersede the skill-projection decision. If
   prompts are unsupported but project instructions plus tools/resources
   preserve routing, the no-legacy cutover can still pass. If ordinary-language
   routing cannot be preserved without generated skills, this proposal blocks
   and requires a new decision.

## Risks And Mitigations

- **Prompt support may not exist or may not be auto-routed.** Mitigation: Phase 0
  verifies Codex prompt support and host UX before prompts become load-bearing.
  The portable baseline is project instructions plus MCP tools/resources.
- **Ordinary-language routing may regress.** Mitigation: the parity eval must
  start from natural user requests, not prompt names. Slash-command-only
  workflows do not count as parity.
- **Tool schemas could diverge from CLI behavior.** Mitigation: deterministic
  cores are shared, and fixture tests compare CLI JSON with MCP tool JSON for
  success, typed errors, readiness, fallback, and zero-result cases.
- **Install migration could orphan old generated skill material.** Mitigation:
  uninstall/update must remove managed legacy output while preserving unmanaged
  user files, and snapshots must cover fresh install, update, and uninstall.
- **No-legacy cutover removes a known-good surface.** Mitigation: deletion waits
  for ordinary-language cross-harness eval evidence, and the plan blocks on
  failures instead of accepting a slash-command-only regression.

## Success Criteria

The proposal is valid when:

1. Phase 0 records whether Codex supports MCP prompts and whether prompts are
   suitable as a portable workflow surface.
2. `llm-wiki mcp serve` hosts `llm_wiki_read`, `llm_wiki_search`,
   `llm_wiki_search_all`, `llm_wiki_index`, `llm_wiki_register`, and
   `llm_wiki_status`.
3. MCP tools share deterministic cores with CLI behavior and have schema-stable
   outputs.
4. `AGENTS.md` / harness project instructions plus MCP tools/resources preserve
   current framework workflows and their citation, provenance, index, and log
   obligations.
5. MCP prompts are implemented only where the host-UX spike proves they are
   supported and useful; prompt support is not required for no-legacy if
   ordinary-language routing works through project instructions.
6. A cross-harness parity eval records that Claude Code and Codex can perform
   representative query, ingest/lint, read, and search workflows from ordinary
   user requests, without the user naming an MCP prompt.
7. Runtime install no longer materializes generated `.claude/skills/` or
   `.codex/` skill trees as a fallback.
8. `wiki/decisions/skill-projection-template-engine.decision.md` is superseded
   or amended with accepted cutover evidence.
9. No regression occurs to the document model, `agent-owns-wiki`,
   `wiki/`+`raw/` provenance invariants, the `headroom_read` ban, or the
   single-binary distribution invariant.
10. The read half ships before the search MCP tools; the cutover does not claim
    search parity until those tools are live and verified.
