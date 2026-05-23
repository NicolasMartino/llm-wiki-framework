## qmd-rs Scale Pack Additions

After reading `wiki/index.md`, query workflows should use
`llm-wiki search --mode auto --format json` for every project query when the
project is registered. `llm-wiki search-all` remains the explicit
cross-project retrieval surface. Both are backed by qmd-rs stores managed by
the llm-wiki binary. Search is a discovery layer over `wiki/`; it does not
replace source citations, typed documents, or index maintenance.
