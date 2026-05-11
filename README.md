# LLM Wiki Framework

The LLM Wiki framework is a project management system where `raw/` contains
human-curated source material and `wiki/` contains agent-maintained compiled
knowledge.

## Install

Install the binary from a release, then install the framework skills globally:

```bash
curl -L https://github.com/nicolasmartino/llm-wiki-rs/releases/latest/download/llm-wiki-installer.sh | sh
llm-wiki install
```

Users with a Rust toolchain can install from crates.io after the package is
published:

```bash
cargo install llm-wiki-rs
llm-wiki install
```

`llm-wiki install` copies or verifies the runtime binary at
`~/.llm_wiki/bin/llm-wiki` and renders installed skills to call that managed
path directly. Run `llm-wiki path` for optional shell `PATH` guidance.

## Create a Project

```bash
llm-wiki init /path/to/project --non-interactive \
  --name "My Project" \
  --description "One sentence description." \
  --blueprint web-product
```

Use repeatable `--pack <name>` flags to override a blueprint's default pack
selection. For example, `--blueprint custom --pack ml --pack qmd-rs-scale`
creates an ML-oriented wiki with qmd-rs-backed scale guidance.
For command-line tools and developer utilities, use `--blueprint cli-tool`.

Run `llm-wiki --help` for the full command surface.
