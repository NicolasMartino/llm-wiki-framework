# Electric Cars Eval Corpus

This fixture is a vendored domain corpus for repeated semantic/hybrid search
model comparison. It is intentionally separate from the framework's live wiki
quality gate and from the small synthetic eval testbed.

## Layout

- `raw/research/2026-05-10-ev-battery-technologies/` contains the frozen raw
  research bundle.
- `wiki/` contains the compiled electric-car battery technology wiki copied
  from that raw bundle.
- `wiki/evals/electric-cars.eval.md` contains the frozen input/output query
  table. The table is hidden from search indexing.

The corpus is useful for realistic domain retrieval, not for proving model
pretraining cleanliness. Contamination control here means the eval answers are
not part of the searchable corpus and historical outputs are written outside
the fixture.

## Deterministic Fixture Run

```bash
LLM_WIKI_TEST_EMBEDDINGS=deterministic \
LLM_WIKI_TEST_QUERY_EXPANSION=deterministic \
cargo run -- eval run \
  --project-root tests/fixtures/eval-corpora/electric-cars \
  --eval-page tests/fixtures/eval-corpora/electric-cars/wiki/evals/electric-cars.eval.md \
  --candidate-profile balanced \
  --candidate-name balanced \
  --output-dir target/evals/electric-cars-balanced
```

## Real Model Comparison

```bash
cargo run -- eval run \
  --project-root tests/fixtures/eval-corpora/electric-cars \
  --eval-page tests/fixtures/eval-corpora/electric-cars/wiki/evals/electric-cars.eval.md \
  --candidate-profile balanced \
  --candidate-profile balanced \
  --output-dir target/evals/electric-cars-comparison \
  --time-budget-warn-ms 5000
```

For process-level parallelism, run one shell per candidate with distinct
`--candidate-name` and `--output-dir` values.

Export a candidate's durable result bundle with:

```bash
cargo run -- eval calibrate \
  --run-report target/evals/electric-cars-balanced/eval-run.json \
  --output-dir target/evals/electric-cars-balanced \
  --export-raw-data
```
