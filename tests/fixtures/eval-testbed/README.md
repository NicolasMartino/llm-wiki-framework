# Eval Testbed

This fixture is a small, vendored wiki corpus for eval infrastructure tests.
It is separate from the live project wiki quality gate.

Canonical deterministic invocation:

```bash
LLM_WIKI_TEST_EMBEDDINGS=deterministic \
LLM_WIKI_TEST_QUERY_EXPANSION=deterministic \
cargo run -- eval run \
  --project-root tests/fixtures/eval-testbed \
  --eval-page tests/fixtures/eval-testbed/wiki/evals/testbed.eval.md \
  --candidate-profile balanced \
  --output-dir target/evals/eval-testbed
```

Multi-candidate comparison:

```bash
cargo run -- eval run \
  --project-root tests/fixtures/eval-testbed \
  --eval-page tests/fixtures/eval-testbed/wiki/evals/testbed.eval.md \
  --candidate-profile balanced \
  --candidate-profile balanced \
  --output-dir target/evals/eval-testbed
```

Process-level parallelism for larger comparisons should use distinct
`--candidate-name` and `--output-dir` values per shell.
