# Electric Cars Eval Corpus

This directory is a frozen eval fixture copied from a separate LLM Wiki
project. Treat `raw/` as immutable provenance and `wiki/` as the compiled
corpus under test.

Do not write eval outputs here. `llm-wiki eval calibrate --export-raw-data`
should write historical result bundles to the caller repository's
`raw/data/eval/electric-cars/<run-id>/<candidate-name>/` tree.
