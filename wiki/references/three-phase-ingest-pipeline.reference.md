# Three-Phase Ingest Pipeline

- Document Class: Reference
- Status: Sourced
- Date: 2026-04-23
- Category: Ingest design
- Scope: Detailed web-sourced explanation of the three-phase ingest pipeline used in LLM Wiki implementations.
- Sources: https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f, https://github.com/NiharShrotri/llm-wiki
- Related: wiki/references/llm-wiki-pattern.reference.md, wiki/references/niharshrotri-llm-wiki.reference.md

## What It Is

The three-phase ingest pipeline decomposes "ingest a source into the wiki"
into three separate jobs:

1. understand the source
2. update durable wiki pages
3. rebuild the navigation and audit surface

Karpathy's original LLM Wiki gist defines the underlying ingest
responsibilities: read a source, extract the important information, update
multiple relevant wiki pages, update `index.md`, and append to `log.md`.
NiharShrotri's implementation makes that decomposition explicit as a
three-pass pipeline:

1. **Extraction**
2. **Page drafting**
3. **Source summary / bookkeeping**

## Why Split Ingest Into Phases

The split exists because ingest has three different kinds of work that do not
benefit from the same prompt shape or model settings.

- Extraction is analysis-heavy. It benefits from structured output and higher
  reasoning effort.
- Page drafting is mutation-heavy. It benefits from smaller, targeted create or
  merge operations against specific pages.
- Bookkeeping is audit-heavy. It benefits from deterministic updates to the
  files that make the wiki navigable and traceable.

This separation reduces prompt overload and creates a cleaner contract between
"what the source says" and "how the wiki changes."

## Phase 1: Extraction

### Purpose

Turn one raw source into a compact, structured working representation before
any wiki page is changed.

### Inputs

- one raw document
- the existing wiki schema or conventions
- enough wiki context to recognize whether findings are new, reinforcing, or
  contradictory

### Typical Outputs

The NiharShrotri implementation describes extraction output as structured data
containing:

- summary
- key takeaways
- named entities
- concepts
- tags

Karpathy's ingest description implies additional categories may also be useful,
such as contradictions, candidate page updates, and questions worth carrying
forward. That is an inference from the responsibilities he assigns to ingest,
not an explicit field list.

### Design Intent

Extraction should answer "what does this source contribute?" without yet
deciding the final page text. In practice this gives the pipeline a review
boundary: the extracted entities, concepts, and summary can be inspected before
the wiki is mutated.

### Failure Modes

- extraction is too shallow and misses concepts that deserve their own page
- extraction is too noisy and creates low-value page candidates
- extraction collapses multiple claims into one vague summary, which weakens
  later merge decisions

## Phase 2: Page Drafting

### Purpose

Apply the extracted findings to the durable wiki layer.

### Core Operation

NiharShrotri's implementation performs one drafting call per entity or concept.
For each unit, the system decides whether to:

- create a new page
- merge into an existing page

### Expected Behaviors

- preserve valid prior content instead of overwriting it wholesale
- update dates or freshness markers where the schema expects them
- append provenance so the page records that this source contributed to it
- add or strengthen cross-links between related pages

This is the phase where the wiki compounds. A source is not merely summarized;
it is distributed into the pages that should remain useful later.

### Why It Is Separate From Extraction

Extraction produces an intermediate representation. Drafting converts that
representation into durable markdown pages. Keeping those tasks separate makes
it easier to:

- review extracted structure before writing
- parallelize or retry page-level updates
- preserve prior content because merges happen against scoped targets
- use different model settings for reasoning vs writing

### Practical Granularity

The README claims a single ingest can create or update a cluster of wiki pages.
That makes drafting the highest-leverage phase operationally: it determines
whether the wiki grows in a coherent graph or turns into a pile of isolated
summaries.

## Phase 3: Source Summary And Bookkeeping

### Purpose

Record what changed, where the source landed, and how the rest of the system
should find it.

### Source Summary

In the NiharShrotri implementation, the third pass writes a
`sources/<slug>.md` page listing every wiki page touched by the source. This is
an audit trail rather than a thematic synthesis page.

The source-summary page answers questions like:

- which pages did this source modify
- what should I inspect if I want to verify the ingest
- where did this source's evidence end up

### Bookkeeping Work After The Three Passes

The same source describes three post-ingest updates:

- rebuild `index.md`
- append to `log.md`
- refresh the QMD search index

Karpathy's original gist independently supports the first two by treating
`index.md` and `log.md` as mandatory navigation files updated during ingest.

### Why This Is Its Own Phase

Bookkeeping is not just cleanup. It makes the ingest queryable, inspectable,
and reversible at the workflow level. Without it, the wiki may contain new
facts but still be hard to navigate or audit.

## End-To-End Data Flow

| Phase | Primary input | Primary output | Main question it answers |
| --- | --- | --- | --- |
| Extraction | raw source | structured findings | What does this source contribute? |
| Page drafting | structured findings + existing pages | created or merged wiki pages | Where should this knowledge live? |
| Source summary / bookkeeping | page-level changes | audit page, updated index/log/search | How do we trace and retrieve the change? |

## Why The Pipeline Scales Better Than One Big Prompt

Compared with a single ingest prompt, the phased design has several
advantages:

- better reasoning/writing separation
- clearer provenance boundaries
- easier human review between understanding and mutation
- less risk of forgetting index, log, or audit updates
- smaller retry surface when one part fails

Karpathy explicitly notes that one source may touch many wiki pages. The
three-phase design handles that fan-out more cleanly by isolating extraction
from page mutation and from bookkeeping.

## What The Sources Leave Open

The sources are strong on the pipeline shape but weaker on exact mechanics.
They do not fully specify:

- the schema of the extraction JSON beyond the example fields
- the scoring rule for deciding create vs merge
- how contradiction resolution should be escalated when sources disagree
- how large batches should coordinate cross-source deduplication

Those details still need to be chosen by each concrete implementation.
