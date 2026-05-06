# In-Depth Review: LLM Wiki Framework

- Reviewer: Claude (Opus 4.7, 1M context)
- Date: 2026-05-06
- Scope: Project intent, objectives, architecture, deliverables, and self-consistency
- Method: Read of `CLAUDE.md`, `project_guidelines.template.md`, `wiki/index.md`, all decisions, the V1 roadmap, the documentation-model spec, the init-project spec, the proposal that was active at review time, and `wiki/log.md`.

## 1. Executive Verdict

The intent and objectives are **clear, well-articulated, and internally consistent**. The project knows what it is (a self-managing project-management framework based on the LLM Wiki pattern), why it exists (to make agent-maintained knowledge bases viable for software projects), and how it will know it succeeded (seven sequenced deliverables, each with a falsifiable Proof).

Three things are unusually strong for a project at this stage:

1. **The self-referential design is deliberate, not accidental.** The framework manages its own development, and that recursion is stated as both a goal and a forcing function. Every pain point in the meta-project is, by construction, a bug in the product.
2. **Ownership is unambiguous.** The agent owns `wiki/`; humans curate `raw/`. One rule, applied consistently, eliminates an entire class of "who edits what" ambiguity that plagues docs-as-code systems.
3. **The typed-document system carries semantic load.** Spec / Decision / Proposal / Roadmap / Plan / Experiment / Eval / Checklist / Reference are not folder cosmetics — each carries a distinct truth-relationship, which makes "what we believe" structurally separable from "what we want to believe."

The main weaknesses are not in clarity — they are in **falsifiability of the later deliverables (D4–D7)** and in a handful of unstated product-strategy questions that will matter before "self-replicating" is declared done.

## 2. Stated Intent and Objectives

From `wiki/roadmaps/framework-v1.roadmap.md:9-17`, the project aims to prove that an LLM-Wiki-based project management framework can:

1. manage its own development (self-referential dogfooding),
2. scale to real project complexity,
3. spawn new projects where an agent is immediately productive,
4. maintain knowledge integrity through ingest / query / lint cycles.

These are translated into seven deliverables:

| ID | Deliverable | Status | Proof shape |
| --- | --- | --- | --- |
| D1 | Bootstrap | Completed | Wiki exists, navigable from index alone |
| D2 | Ingest cycle | Completed | A non-trivial source compiled end-to-end |
| D3 | Lint operation | Completed | At least one real issue found and fixed |
| D4 | Query produces durable knowledge | Draft | New wiki page filed from a synthesis |
| D5 | Spawns a new project | Draft | Fresh agent in fresh repo is productive |
| D6 | Scale test | Draft | 50+ pages, index navigation still works |
| D7 | Self-replicating framework | Draft | Two distinct domains bootstrapped |

The progression is sound: bootstrap → operate → scale → replicate. Each completed deliverable is currently backed by observable artifacts in `wiki/`.

## 3. What is Clear and Well-Formed

### 3.1 Architecture

The three-layer split (`raw/` immutable + `wiki/` agent-owned + `CLAUDE.md` schema, see `wiki/decisions/three-layer-architecture.decision.md:11-45`) is justified against three alternatives — legacy three-lane docs, pure flat Karpathy wiki, and a hybrid lane-inside-wiki. Each rejection has a stated reason. This is exactly the level of rigor a foundational decision deserves.

### 3.2 Ownership Boundary

`wiki/decisions/agent-owns-wiki.decision.md` makes the boundary explicit: humans interact through raw curation and queries; the agent owns wiki accuracy and consistency. Crucially, it also names the failure mode of the alternative ("Agent suggests, human approves every change … the wiki would lag behind reality the same way human-maintained docs do"). The decision is not stylistic — it is load-bearing.

### 3.3 Typed Documents

`wiki/decisions/typed-documents.decision.md` keeps nine types from the legacy framework. The justification ("preventing the conflation of what is true with what we want to be true") names a specific failure mode that the type system structurally prevents. The "Revisit When" clause ("a document type is consistently unused across multiple projects") is appropriately humble.

### 3.4 Operation Discipline

The framework resists the obvious temptation to add a fourth core operation. `wiki/specs/documentation-model.spec.md:32-35` and the research-intake decision both treat **research as a supporting acquisition step, not a core mutation**. The decision explicitly rejects adding a `knowledge-intake` command on the grounds that it would duplicate `knowledge-research`. This is the kind of restraint that keeps a framework teachable.

### 3.5 Skill Surface

Five skills (`init-project`, `knowledge-ingest`, `knowledge-query`, `knowledge-lint`, `knowledge-research`) map one-to-one onto operations, with a `$knowledge` namespace dispatcher in Codex (`wiki/decisions/knowledge-command-namespace.decision.md`). Dual-target Claude/Codex packaging is decided (`wiki/decisions/project-local-codex-skills.decision.md`) without coupling the framework's semantics to either runtime.

### 3.6 Honesty About Progress

Specs explicitly list limitations (`wiki/specs/documentation-model.spec.md:67-75`): not yet tested on a second project, scale beyond ~50 pages untested, no automated tooling, QMD identified but unimplemented, three-pass ingest pipeline not yet adopted. The `init-project` spec admits the template path is hardcoded and update mode is untested (`wiki/specs/init-project-skill.spec.md:81-87`). This is the right tone for an Active spec — it documents tested truth, not aspirational truth.

## 4. Gaps, Ambiguities, and Risks

### 4.1 The audience question is implicit

The framework is pitched in a self-contained way, but there is no positioning statement about **who it is for**. Three plausible answers:

- (a) a personal framework for the author's own projects;
- (b) an open-source framework for general use;
- (c) a reference implementation to inform a future product.

D7's "self-replicating" wording implies (b). But there is no licensing decision, no distribution plan, no target-user description, no install story for someone outside this repo. This is fine for D1–D5, but it will start to matter before D7. Recommend: a short positioning section in `project_guidelines.template.md` or a `decisions/audience-and-distribution.decision.md` once it firms up.

### 4.2 D4 has a soft proof

D4's proof is "the answer is valuable enough to file as a new wiki page." Compared with D2 ("new wiki page exists with correct metadata and source citations") and D3 ("at least one real issue found and fixed"), D4 is subjective. **What counts as durable?** Suggest tightening to one of:

- a specific question that requires synthesizing ≥3 existing pages;
- a question whose answer changes a spec or decision;
- a measured criterion: the resulting page is later cited by another wiki page within N ingest cycles.

Without this, D4 will be marked "Completed" on vibes.

### 4.3 D6 conflates two scale questions

"50+ pages" measures **navigability** (does the index still work?), while "agent answers 10 questions correctly" measures **retrieval quality** (does the agent find the right pages?). These can fail independently. Recommend splitting into two proofs and gating QMD adoption on a measured threshold (e.g., "if first-try retrieval drops below 80% on a held-out question set, QMD becomes mandatory, not optional").

### 4.4 D5's "immediately productive" is undefined

D5 says a fresh agent in a new repo should be "immediately productive" given only the generated `project_guidelines.md` and `CLAUDE.md`. What does productive mean? Suggest a concrete threshold:

> The fresh agent ingests N raw sources, answers M domain questions citing wiki pages, and runs one lint pass — all without any human edit to `wiki/`.

Without a number, "productive" is unfalsifiable.

### 4.5 Self-reference creates a blind spot

Because the framework manages itself, a flaw in the framework can hide inside its own wiki. The lint operation partially addresses this, but lint is *also* defined by the framework. There is no spec for **external validation** — i.e., a fresh agent in a fresh repo, with no memory, completing a defined task as the falsification test for the framework as a whole.

D5 and D7 hint at this. Promote it to an explicit principle: "The framework is only proven by an agent that has never seen this repo." This protects against a class of failure where the agent uses tribal knowledge from this conversation rather than what is actually written down.

### 4.6 Promotion flow lacks a demotion path

`wiki/specs/documentation-model.spec.md:57` shows the forward flow:

```
research → raw → ingest → proposal/reference → roadmap → plan → evidence → spec/decision → archive
```

The status vocabulary supports `Superseded`, but **what happens when a spec is invalidated by later evidence**? Is the spec demoted, archived, or both? Is a `Superseded By` link required? This is partially covered in the optional metadata (`Supersedes` / `Superseded By`) but not in the flow diagram or as a checklist step. Worth adding a "Demotion" section to the documentation-model spec.

### 4.7 Index-as-orientation has a token-count cliff

`project_guidelines.template.md:90-92` says the index "must fit in a single context window. If it grows beyond ~50,000 tokens, split into a root index with per-type sub-indexes." But the spec also names QMD as the scale solution at >100 pages. Two thresholds (50k tokens and 100 pages) are doing similar work. Recommend reconciling: which fires first, and what is the migration order — sub-indexes, then QMD, or QMD directly?

### 4.8 Lint has no evaluation criteria

Lint is defined as an operation but has no quality bar. How do you know lint is *good*? Possible criteria:

- false-positive rate (lint flags something that is actually fine);
- false-negative rate (lint misses a real contradiction);
- run-to-run determinism (does the same wiki produce the same lint output?).

Without these, lint can degrade silently. This is the same self-reference risk as 4.5.

### 4.9 No story for framework versioning

D7 explicitly excludes "framework versioning or updates to spawned projects." This is reasonable for V1, but the question is large enough to deserve a deferred-decision record. Once N projects are bootstrapped, how do they pick up framework improvements? Without a plan, every spawned project becomes a fork on day one.

## 5. Architectural Tensions Worth Surfacing

### 5.1 Wiki-as-truth vs. code-as-truth

The framework treats `wiki/` as the compiled knowledge base. For framework-level decisions, this works — the framework has no code to speak of. But once spawned into a real software project, the **code is also a source of truth** (often the most reliable one). The spec does not address how wiki claims and code reality are kept aligned. Suggested principle: any spec that claims runtime behavior should cite the file path and function that implements it, and lint should verify the path still exists. This is hinted at in the "Definition of Done" (`project_guidelines.template.md:469-481`) but not enforced.

### 5.2 Ingest is human-triggered

The ingest workflow assumes a human places material in `raw/` and asks the agent to ingest. Fine. But for a long-running project, **the most valuable raw material is conversation** — what was decided in this chat, what the agent learned. There is no recipe for "ingest the conversation" beyond the implicit assumption that the human will write it down. This is a known limitation but worth naming.

### 5.3 The "agent owns wiki" rule has an edge case

If the human wants to record a decision the agent disagrees with, the only path is to add it to `raw/` and let the agent compile it. This is the right design for most cases, but in adversarial or judgment-call situations, the agent might re-compile and effectively soften the decision. The proposal/decision distinction partly covers this (a decision is durable), but there is no rule preventing the agent from editing decision files. Worth a sentence: "Once `Accepted`, decision text is immutable except through `Superseded By`."

## 6. Strengths to Preserve

These are easy to lose under refactoring pressure — flagging them so they are protected:

1. **Three core operations, no more.** Resist the urge to add a fourth.
2. **Index as the sole entry point.** The "agent never browses the filesystem" rule is what makes the framework portable. Do not add fallback navigation patterns that erode this.
3. **Type by role, not by convenience** (`CLAUDE.md:34`). The single most important rule for keeping the typed-document system honest.
4. **Document tested truth, not intended truth** (`project_guidelines.template.md:332`). This is the sentence that makes the framework different from every other docs-as-code system.
5. **The `Revisit When` clause** on every decision. This converts decisions from monuments into reversible commitments, which is exactly what a young framework needs.

## 7. Recommended Next Steps (in priority order)

1. **Sharpen D4 proof** — pick one of the concrete criteria in §4.2 and update the roadmap.
2. **Define "productive" for D5** — write a numeric threshold into the deliverable.
3. **Split D6 into navigability and retrieval-quality sub-proofs** (§4.3).
4. **Add a Demotion section** to `documentation-model.spec.md` (§4.6).
5. **Decide audience/distribution** before starting D7 (§4.1) — even a one-paragraph decision is enough.
6. **Promote external-validation to an explicit principle** (§4.5) — "the framework is only proven by an agent that has never seen this repo."
7. **Add a code-as-truth alignment rule** to the Definition of Done (§5.1) — for projects that spawn from this framework and have application code.

None of these block current progress. They sharpen the falsification criteria for the deliverables that are still Draft.

## 8. Bottom Line

This project is doing the rare thing of taking its own conventions seriously. The intent is clear, the objectives are sequenced into deliverables with proofs, and the architecture has survived contact with at least one real ingest cycle and one real lint pass. The risks are concentrated in the later deliverables, where the success criteria are still soft, and in a few unstated product-strategy questions that will surface around D7.

If the framework can complete D5 — a fresh agent, a fresh repo, a navigable wiki — without anyone editing `wiki/` by hand, it will have proven the only thing that matters: that the agent-owned knowledge layer is a real engineering primitive, not a stylistic preference.

---

## 9. Codex Follow-Up Review: Rename and Consistency Findings

- Reviewer: Codex
- Date: 2026-05-06
- Scope: Concrete consistency and operability issues found after the project was renamed to `llm_wiki_framework`.
- Method: Index-first review using `project_guidelines.template.md`, `wiki/index.md`, relevant wiki specs/decisions/plans, root agent instructions, local skill files, and global Codex symlink targets.

### 9.1 High Severity: Init project still points at the old project path

The `init-project` skill still hardcodes the old framework path:

```text
/Users/nicolasmartino/Documents/local_llm_wiki/software_project_management
```

That path appears in both:

- `.codex/skills/init-project/SKILL.md`
- `.claude/skills/init-project/SKILL.md`

The current project path is:

```text
/Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework
```

Impact: `init-project` can fail when it tries to read `project_guidelines.template.md` from the old location. This directly threatens D5, because D5 depends on the skill spawning a fresh project end-to-end.

Recommended fix: replace the stale absolute path with the current `llm_wiki_framework` path, or better, teach the skill to resolve the framework root from the skill file location so future renames do not break it again.

### 9.2 High Severity: Global Codex symlinks are dangling after the rename

The documented global Codex skill exposure is currently stale. The symlinks under `~/.codex/skills/` point to:

```text
/Users/nicolasmartino/Documents/local_llm_wiki/software_project_management/.codex/skills/...
```

The old `software_project_management/` directory is gone, so these are dangling symlinks, not silently stale copies. The affected framework symlinks are:

- `~/.codex/skills/init-project`
- `~/.codex/skills/knowledge`
- `~/.codex/skills/knowledge-ingest`
- `~/.codex/skills/knowledge-lint`
- `~/.codex/skills/knowledge-query`
- `~/.codex/skills/knowledge-research`

The unrelated `~/.codex/skills/knlg` symlink points into `es_llm_wiki` and should be left alone unless that separate project is also stale.

Impact: global `$knowledge` or direct framework skill invocation outside this repo will fail to resolve. This is noisy failure rather than silent behavior drift, but it still undermines the `Project-Local Codex Skills` decision and the `$knowledge` namespace story.

Recommended fix: recreate the symlinks so they point to:

```text
/Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework/.codex/skills/<skill>
```

Then update the wiki specs/decisions that claim the global symlinks are valid.

Root cause: §9.1 and §9.2 both come from hardcoding the framework's absolute path. The durable fix is a decision-level rule: framework source paths should be resolved relative to the skill file location, never embedded as rename-sensitive absolute paths.

### 9.3 High Severity: Canonical guidelines claim is inconsistent with the current repo

The documentation model spec says the canonical specification is `project_guidelines.md` at the repository root and lists `project_guidelines.md` as proven to exist. In the current repo, only `project_guidelines.template.md` exists.

Conflicting references:

- `wiki/specs/documentation-model.spec.md` claims `project_guidelines.md` is canonical and exists.
- `CLAUDE.md` and `AGENTS.md` orient agents to `project_guidelines.template.md`, but still say to see `project_guidelines.md` for document type rules.

Impact: a fresh agent can be sent to a missing file while trying to follow the framework's own rules. This is especially risky because this project is self-referential and uses its own docs as proof of correctness.

Recommended fix: decide whether this repo should have a generated `project_guidelines.md` in addition to the template, or whether this self-framework repo intentionally uses `project_guidelines.template.md` as canonical. Then update `CLAUDE.md`, `AGENTS.md`, and `wiki/specs/documentation-model.spec.md` consistently.

### 9.4 Medium Severity: Claude and Codex ingest behavior diverge on URL/web sources

The current spec and Codex ingest skill say URL or web discovery should go through `knowledge-research` first so snapshots are saved under `raw/` before ingest. The Claude ingest skill still says URLs can be fetched directly with `WebFetch`, saved to `raw/`, and then ingested.

Impact: Claude and Codex can produce different provenance shapes for the same user request. This weakens the accepted boundary that research is source acquisition and ingest is wiki compilation.

Recommended fix: update `.claude/skills/knowledge-ingest/SKILL.md` so URL, site, and web requests route to `knowledge-research` first, matching the Codex skill and `knowledge-research` spec.

### 9.5 Medium Severity: Accepted research-intake proposal should be promoted or archived

At review time, `wiki/proposals/knowledge-intake-command.proposal.md` had `Status: Accepted` and contained a `## Decision` section. The implementation plan was already `Completed`, and the accepted behavior was already described in the active `knowledge-research` spec.

Impact: the document type semantics are blurred. A proposal is supposed to represent unaccepted direction, but this proposal now records an accepted choice.

Recommended fix: either create a decision page for the accepted choice and archive the proposal, or update the proposal status/lifecycle convention to explicitly allow accepted proposals to remain in `wiki/proposals/`.

### 9.6 Priority Fix Order

1. Fix the path-resolution model and rebuild global symlinks in one pass: update `.codex/skills/init-project/SKILL.md` and `.claude/skills/init-project/SKILL.md` to resolve the framework root relative to the skill file, then recreate the six framework symlinks under `~/.codex/skills/` so they point to `llm_wiki_framework`.
2. Record the path-resolution principle as a durable decision: framework paths are resolved relative to project-local skill files and are never hardcoded as rename-sensitive absolute paths.
3. Resolve whether this repo's canonical guidelines file is `project_guidelines.md` or `project_guidelines.template.md`, then update all references.
4. Align Claude URL/web ingest behavior with the research-first boundary.
5. Promote or archive the accepted research-intake proposal.

### 9.7 Implementation Status

Implemented on 2026-05-06. The framework now resolves init skill source paths
relative to the skill file, repaired the affected global Codex and Claude
symlinks, treats `project_guidelines.template.md` as the canonical schema for
this framework repo, aligns Claude ingest with the research-first URL/site/web
boundary, and archives the accepted proposal after promoting its choice into a
decision.

## 10. Single-Source Skill Consolidation Status

Repo-local implementation completed on 2026-05-06. The consolidation plan now has a canonical
`skills/` source directory, `skills/build.sh`, rendered Claude and Codex skill
outputs, Claude `knowledge-lint`, and
`wiki/decisions/single-source-skills.decision.md`.

Local verification passed for the gates that can be checked in this workspace:
the build is idempotent, every indexed skill has a canonical source, no skill
body contains an absolute `/Users/...` path, and URL routing is research-first
in both runtimes. Runtime smoke tests in Claude Code and Codex still require
manual invocation in those products.

Follow-up audit correction: the first canonical draft was too compressed and
would have dropped substantial runtime-specific guidance, especially from
Claude `init-project`. The canonical sources were rebuilt from the full staged
runtime baselines before final rendering, preserving prior detail while keeping
the targeted rename and URL-routing fixes.
