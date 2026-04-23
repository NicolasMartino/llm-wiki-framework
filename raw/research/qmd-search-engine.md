# QMD - On-Device Search Engine for Markdown

Source: https://github.com/tobi/qmd
Author: Tobi Lutke (Shopify CEO)
Retrieved: 2026-04-23

## What It Does

On-device search engine for markdown. Indexes notes, transcripts, docs,
and knowledge bases. Searches via keywords or natural language. Runs
entirely locally using node-llama-cpp with GGUF models.

## Hybrid Search Pipeline

1. Query Expansion — LLM generates query variations from user input
2. Parallel Retrieval — each variant searches both FTS and vector indexes
3. Reciprocal Rank Fusion — score = sum(1/(k+rank+1)) with k=60
4. Re-ranking — LLM evaluates top candidates with yes/no + confidence
5. Position-Aware Blending — final scores blend retrieval + reranker by rank position

## Three Search Modes

- search: BM25 full-text only (fast, keyword-based)
- vsearch: Vector semantic search only
- query: Hybrid with expansion and re-ranking (highest quality)

## Smart Chunking

Documents chunked into ~900-token pieces with 15% overlap. Break point scoring:
- Headings: H1 = 100pts, decreasing with level
- Code block boundaries: 80pts
- Paragraph breaks: 20pts
- Line breaks: 1pt

For code files: tree-sitter AST parsing for function/class boundary breaks.

## Models (auto-downloaded)

1. Embedding: embeddinggemma-300M-Q8_0 (~300MB)
2. Re-ranking: qwen3-reranker-0.6b-q8_0 (~640MB)
3. Query expansion: qmd-query-expansion-1.7B-q4_k_m (~1.1GB)

All cached in ~/.cache/qmd/models/.

## Installation

```bash
npm install -g @tobilu/qmd
```

Requires: Node.js >= 22 or Bun >= 1.0.0

## Usage

```bash
# Collections
qmd collection add ~/notes --name notes
qmd context add qmd://notes "Personal notes and ideas"

# Index
qmd embed                          # Generate embeddings
qmd embed --chunk-strategy auto    # AST-aware for code

# Search
qmd search "project timeline"     # BM25
qmd vsearch "how to deploy"       # Semantic
qmd query "quarterly process"     # Hybrid + rerank

# Retrieve
qmd get "path/to/file.md"         # By path
qmd get "#abc123"                  # By document ID
qmd multi-get "journals/2025-*.md" # Batch
```

## MCP Server

Exposes Model Context Protocol server for LLM integration:

```bash
qmd mcp              # Stdio transport
qmd mcp --http       # HTTP on localhost:8181
qmd mcp --daemon     # Background daemon
```

Claude Desktop config:
```json
{
  "mcpServers": {
    "qmd": {
      "command": "qmd",
      "args": ["mcp"]
    }
  }
}
```

Exposed tools: query, get, multi_get, status.

## TypeScript API

```typescript
import { createStore } from '@tobilu/qmd'

const store = await createStore({
  dbPath: './index.sqlite',
  config: {
    collections: {
      docs: { path: '/path/to/docs', pattern: '**/*.md' },
    },
  },
})

const results = await store.search({ query: "auth flow" })
```

## Data Storage

SQLite at ~/.cache/qmd/index.sqlite. Contains: collections, contexts,
documents, FTS indexes, content vectors, LLM response cache.

## Key Properties

- All-local: no external API calls
- Context-aware: descriptive metadata improves relevance
- JSON/files output for LLM integration
- Configurable chunking (regex or AST-aware)
- Editor integration (VS Code, Cursor, Zed, Sublime)
- Git-aware reindexing
- ~2GB total model footprint
