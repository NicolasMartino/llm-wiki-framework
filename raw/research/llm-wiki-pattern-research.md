# LLM Wiki Pattern - Knowledge Storing and Retrieval for Software Projects

- Document Class: Reference
- Status: Sourced draft
- Date: 2026-04-23
- Category: Knowledge management architecture
- Scope: Research notes on Karpathy's LLM Wiki pattern and its applicability to software project spec management.

## Sources

1. [Karpathy's LLM Wiki Gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f) - Original pattern description (2026-04-03)
2. [LLM Wiki v2 - rohitg00](https://gist.github.com/rohitg00/2067ab416f7bbe447c1977edaaa681e2) - Extensions from building agentmemory
3. [Karpathy's Pattern in Production - Fulkerson](https://aaronfulkerson.com/2026/04/12/karpathys-pattern-for-an-llm-wiki-in-production/) - Production deployment with 14 MCP servers
4. [LLM Wiki vs RAG for Codebase Memory - MindStudio](https://www.mindstudio.ai/blog/llm-wiki-vs-rag-internal-codebase-memory) - Comparative analysis
5. [VentureBeat coverage](https://venturebeat.com/data/karpathy-shares-llm-knowledge-base-architecture-that-bypasses-rag-with-an) - Industry analysis
6. [Beyond RAG - Level Up Coding](https://levelup.gitconnected.com/beyond-rag-how-andrej-karpathys-llm-wiki-pattern-builds-knowledge-that-actually-compounds-31a08528665e) - Deep technical walkthrough
7. [LLM Wiki vs RAG Enterprise - Atlan](https://atlan.com/know/llm-wiki-vs-rag-knowledge-base/) - Enterprise scale considerations

## 1. The Core Problem That LLM Wiki Solves

Traditional RAG re-derives knowledge from scratch on every query: chunk documents,
embed into vectors, retrieve similar chunks, synthesize an answer, discard.
Nothing compounds. The same question tomorrow repeats the same work.

Karpathy's insight: the LLM should **compile** raw sources into a structured,
interlinked markdown wiki **once at ingest time**, then query against that
pre-organized knowledge. The hard work (understanding, connecting, synthesizing)
happens upfront and persists.

Key quote from the gist:

> "The tedious part of maintaining a knowledge base is not the reading or the
> thinking - it's the bookkeeping."

LLMs handle cross-references, consistency checks, and contradiction flagging
at near-zero cost. The human curates sources and asks questions; the LLM
handles grunt work.

## 2. Three-Layer Architecture

### Layer 1: Raw Sources

Immutable curated documents. Articles, papers, specs, transcripts, data files.
The human decides what enters. Once ingested, sources are not modified.

### Layer 2: The Wiki

LLM-generated markdown files. Summaries, entity pages, concept pages,
comparisons, synthesis. The LLM owns this layer entirely. It writes, updates,
cross-links, and maintains all wiki pages.

### Layer 3: The Schema

A configuration document (e.g., `CLAUDE.md`) specifying:

- wiki structure and conventions
- workflows for ingesting, querying, and maintaining
- entity and relationship types
- quality standards and contradiction handling
- consolidation schedules

The schema is what makes the LLM a disciplined wiki maintainer rather than a
generic chatbot.

## 3. Three Core Operations

### Ingest

Process new sources one at a time. The LLM:

1. reads the source
2. discusses takeaways
3. writes summary pages
4. updates entity and concept pages across the wiki
5. updates `index.md`
6. appends to `log.md`

### Query

Ask questions against the wiki. The LLM:

1. reads `index.md` to orient
2. identifies relevant pages
3. reads those pages
4. synthesizes answer with citations
5. valuable answers get filed back as new wiki pages

### Lint

Periodic health check. The LLM scans for:

- contradictions between pages
- stale claims
- orphan pages with no inbound links
- missing cross-references
- data gaps

## 4. Navigation Files

### index.md

Content-oriented catalog organized by category. Links and one-line summaries
for every wiki page. Updated on every ingest. Designed to fit in a single
context window so the agent can orient itself in one read.

### log.md

Append-only chronological record. Uses consistent prefix format for
parseability:

```
## [2026-04-02] ingest | Title of Source
```

## 5. Scale Characteristics

At ~100 articles and ~400,000 words the index fits in context. No embeddings,
no vector DB, no retrieval pipeline required.

The pattern works reliably up to ~50,000-100,000 tokens in the index. Beyond
that threshold, the index cannot fit in context and a retrieval layer becomes
necessary regardless of storage format.

Queries get faster and cheaper as the wiki matures because synthesis was
already done at ingest time. Query-time work is retrieval plus generation
over pre-organized material.

## 6. LLM Wiki v2 Extensions

The v2 extension by rohitg00 adds several layers the original left abstract.

### Memory Lifecycle

- **Confidence scoring**: each fact carries a numerical confidence reflecting
  source count, recency, and contradictions
- **Supersession**: new information explicitly replaces old claims rather than
  coexisting, with timestamps and links preserving history
- **Forgetting mechanisms**: facts decay in relevance over time unless
  reinforced (modeled as Ebbinghaus forgetting curve)

### Consolidation Tiers

Information advances through tiers as evidence accumulates:

1. Working memory - recent, unprocessed observations
2. Episodic memory - compressed session summaries
3. Semantic memory - cross-session facts
4. Procedural memory - workflows and repeated patterns

### Knowledge Graph

Beyond flat markdown, the system implements:

- **Entity extraction**: typed entities (people, projects, libraries, concepts,
  files, decisions) with attributes and relationships
- **Typed relationships**: connections carry semantic labels like "uses",
  "depends on", "contradicts", "caused", "fixed", "supersedes"
- **Graph traversal**: questions like "what's impacted by upgrading Redis?"
  walk through dependency edges rather than keyword-matching

The graph augments rather than replaces wiki pages. Pages remain human-readable;
the graph enables navigation and discovery.

### Hybrid Search (Post-100 Pages)

Beyond ~100-200 pages, the single `index.md` approach requires supplementation:

1. BM25 (keyword matching with stemming and synonym expansion)
2. Vector search (semantic similarity via embeddings)
3. Graph traversal (entity-aware relationship walking)

Results fuse using reciprocal rank fusion. The `index.md` remains as a human
catalog but is deprecated as the LLM's primary search mechanism past ~100 pages.

### Event-Driven Hooks

Manual operations become event-triggered:

- **On new source**: auto-ingest, extract entities, update graph and index
- **On session start**: load relevant wiki context based on recent activity
- **On session end**: compress session into observations, file insights
- **On query**: file high-quality answers back (quality score threshold)
- **On memory write**: check for contradictions, trigger supersession
- **On schedule**: periodic lint, consolidation, retention decay

### Self-Healing Lint

Lint operations automatically fix orphan pages, stale claims, and broken
references rather than just flagging them.

### Multi-Agent Collaboration

- **Mesh sync**: multiple concurrent agents merge observations using
  last-write-wins with timestamp-based conflict resolution
- **Shared vs private scoping**: personal knowledge (preferences, workflows)
  separates from shared knowledge (architecture, decisions)
- **Work coordination**: lightweight tracking prevents duplicate effort

## 7. Production Deployment Lessons (Fulkerson)

Aaron Fulkerson deployed the pattern in production with 14 MCP servers pulling
live data (calendar, email, CRM). Key findings:

### Critical Additions

- **Post-compact hooks**: re-inject critical context after LLM context
  compression events, preventing knowledge loss during long sessions
- **Learning graduation loops**: daily observation capture, weekly review,
  pattern graduation into permanent rules and skill improvements
- **Skill routing**: 26 natural-language-triggered workflows that read from
  AND write back to the wiki, creating compounding knowledge without dedicated
  maintenance sessions
- **Provenance in YAML frontmatter**: every wiki page tracks its sources
- **Behavioral enforcement hooks**: 8 hook scripts ensure compliance with
  wiki conventions

### Structural Gaps Revealed

Despite production usage, comparing against Karpathy's framework exposed:

- missing vault-wide lint operations
- no formal activity tracking for stale content
- absent source provenance taxonomy
- no centralized entity index

Quote: "every production system is incomplete. stepping back to compare
notes reveals structural gaps that incremental building hides."

## 8. LLM Wiki vs RAG - When to Use Which

| Dimension | LLM Wiki | RAG |
| --- | --- | --- |
| Interpretability | High - human-readable markdown | Low - opaque retrieval |
| Updates | Edit markdown files | Re-embed changed chunks |
| Debugging | Easy - trace to specific pages | Difficult - reverse-engineer retrieval |
| Scalability | Hundreds of files practical | Thousands to millions viable |
| Team alignment | Shared agent-developer knowledge | Potential divergence |
| Infrastructure | Git + filesystem | Vector DB + embedding pipeline |
| Query cost over time | Decreasing (pre-compiled) | Constant (re-derived each time) |

**Choose the wiki for**: bounded codebases, architectural knowledge priority,
team-maintained documentation, interpretability requirements.

**Choose RAG for**: massive codebases, semantic code search, unstructured
content mixing, scale beyond context window limits.

**Hybrid**: use a wiki for architectural knowledge and RAG for code search.
The agent consults the wiki for context and conventions, then uses retrieval
to find specific files.

## 9. Applicability to Software Project Spec Management

### What the Pattern Addresses

The fundamental problem in software project management: project knowledge is
scattered across tools (Jira, Confluence, Slack, code comments, PRDs, meeting
notes). An agent starting a task must piece together understanding from
fragments. Nothing compounds between sessions.

The LLM Wiki pattern offers a single, structured, agent-navigable knowledge
layer where:

- the agent reads `index.md` for full orientation in one pass
- specs are compiled into feature pages with citations to raw sources
- architectural decisions have explicit rationale (ADRs)
- feature state, blockers, and dependencies are tracked per feature page
- contradictory requirements are caught by lint operations
- knowledge persists across sessions without re-derivation

### How It Maps to Software Project Lanes

| Wiki concept | Software project equivalent |
| --- | --- |
| Raw sources | Original PRDs, meeting notes, RFCs, research |
| Wiki feature pages | Compiled feature state with dependencies and status |
| Wiki entity pages | Services, modules, APIs, data models |
| Wiki decision pages | Architecture Decision Records |
| Index | Master catalog of all project knowledge |
| Log | Chronological record of spec changes |
| Schema (CLAUDE.md) | Project documentation conventions and agent workflows |
| Ingest | Processing a new PRD, RFC, or meeting outcome |
| Query | Agent asking "what does feature X depend on?" |
| Lint | Catching spec contradictions, stale requirements, missing links |

### Relationship to Our Existing Documentation Model

Our `project_guidelines.md` already establishes a documentation framework with
clear lanes (`docs/current/`, `docs/next/`, `docs/process/`) and typed documents
(spec, proposal, plan, roadmap, decision, experiment, eval, checklist, reference).

The LLM Wiki pattern is complementary rather than conflicting:

**Alignment points:**

- Both emphasize separating validated truth from future intent
- Both use typed documents with metadata
- Both require explicit status tracking
- Both value cross-referencing and provenance
- The `index.md` concept maps naturally to our folder README responsibilities
- The lint operation maps to our "common failure modes to prevent"

**What LLM Wiki adds:**

- A formal **ingest workflow** where new sources are compiled into existing
  docs rather than just filed
- An **index.md** as an agent-optimized entry point (distinct from human-oriented
  READMEs)
- A **log.md** for chronological mutation tracking
- **Lint as an operation** rather than guidelines about what to avoid
- The concept of **wiki pages that the LLM owns and maintains** (our current
  model assumes human authorship)
- **Compounding knowledge** - each ingest enriches existing pages rather than
  creating isolated documents

**Open questions for integration:**

1. Should the LLM own any pages directly, or remain an assistant to human
   authors? The Karpathy model has the LLM fully owning the wiki layer.
2. How does `index.md` relate to our folder READMEs? Could READMEs serve as
   distributed indexes, or is a single top-level index more effective for
   agent navigation?
3. Should ingest be a formal operation in our workflow? Currently we have
   no explicit process for "new information arrived, update all affected docs."
4. Where does `log.md` fit? Our model has no equivalent of an append-only
   mutation log.
5. How do we handle the scale ceiling? At ~100+ spec/decision/plan documents,
   the single-index approach may need supplementation.
6. Should lint become a periodic automated operation rather than a set of
   guidelines humans try to follow?

### Recommended Investigation Path

1. Evaluate whether `index.md` + `log.md` can be added to the existing
   documentation model without disrupting the lane structure
2. Define a formal ingest workflow: what happens when a new PRD, RFC, or
   meeting outcome arrives
3. Prototype lint as an agent operation: have the LLM scan for contradictions,
   stale specs, orphan documents, and missing cross-references
4. Test the pattern at the current project scale to determine if the single
   index is sufficient or if distributed indexes per lane work better
5. Determine the boundary between LLM-maintained content and human-authored
   content in the context of our documentation model
