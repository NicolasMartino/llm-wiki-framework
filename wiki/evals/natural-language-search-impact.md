# Natural-Language Search Impact

- Document Class: Eval
- Status: Active
- Date: 2026-05-12
- Category: Search eval comparison and optimization
- Scope: Durable comparison tables derived from raw natural-language search eval
  data.
- Sources: raw/data/eval/natural-language-search/20260511T205231Z-79135/balanced/manifest.toml, raw/data/eval/natural-language-search/20260511T205231Z-79135/balanced/eval-run.json, raw/data/eval/natural-language-search/20260511T205231Z-79135/balanced/eval-calibration.json
- Related: wiki/evals/natural-language-search.eval.md, wiki/plans/semantic-hybrid-search.plan.md

## Purpose

This page is the ingested, human-readable impact ledger for natural-language
search tuning attempts. Raw machine outputs stay immutable under
`raw/data/eval/natural-language-search/`; this page records the comparison
tables needed to optimize future runs.

## Evidence Flow

`target/evals/<run>/` is scratch output. Selected runs are exported to
`raw/data/eval/<corpus-slug>/<run-id>/<candidate-name>/` with a manifest and
source hashes. The wiki then ingests those raw files into this comparison page
and the main natural-language search eval. A PII preflight found that the
initial uncommitted flat bundles contained absolute home paths, so those files
were replaced before commit with the current redacted corpus/run/candidate
bundle.

## Run Ledger

| Raw bundle | Attempt | Run | Candidate | Status | Current hybrid | Proposed hybrid | Hold-out hybrid | No-match hybrid | Exact-ID hybrid | Changes |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `natural-language-search/20260511T205231Z-79135/balanced` | Path-anchor replay | `20260511T205231Z-79135` | `balanced` | `blocked_no_feasible_threshold` | 25 / 5 | 25 / 5 | 17 / 3 | 4 / 4 | 3 / 4 | 26 |

## Mode Summary

| Attempt | Mode | Current pass | Current fail | Current N/A | Proposed pass | Proposed fail | Proposed N/A | Hold-out pass | Hold-out fail | Hold-out N/A |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Path-anchor replay | lexical | 14 | 12 | 4 | 14 | 12 | 4 | 11 | 6 | 3 |
| Path-anchor replay | semantic | 24 | 5 | 1 | 26 | 3 | 1 | 17 | 3 | 0 |
| Path-anchor replay | hybrid | 25 | 5 | 0 | 25 | 5 | 0 | 17 | 3 | 0 |
| Path-anchor replay | auto | 25 | 5 | 0 | 25 | 5 | 0 | 17 | 3 | 0 |

## Precision Gates

| Attempt | Gate | Mode | Current | Proposed | Impact |
| --- | --- | --- | ---: | ---: | --- |
| Path-anchor replay | No-match precision | semantic | 0 / 4 | 2 / 4 | Improves but still admits H9/H11 semantically. |
| Path-anchor replay | No-match precision | hybrid | 0 / 4 | 4 / 4 | Fixes no-match rows by proposed filtering. |
| Path-anchor replay | No-match precision | auto | 0 / 4 | 4 / 4 | Mirrors hybrid. |
| Path-anchor replay | Exact-ID preservation | lexical | 4 / 4 | 4 / 4 | Preserved. |
| Path-anchor replay | Exact-ID preservation | hybrid | 4 / 4 | 3 / 4 | Regresses one exact-identifier case. |
| Path-anchor replay | Exact-ID preservation | auto | 4 / 4 | 3 / 4 | Mirrors hybrid. |

## Latest Proposal Changes

| ID | Split | Mode | Current | Proposed | Impact |
| --- | --- | --- | --- | --- | --- |
| C2 | Calibration | hybrid | pass | pass | Path-anchor replay preserves the expected match. |
| C2 | Calibration | auto | pass | pass | Mirrors hybrid. |
| C5 | Calibration | hybrid | pass | fail | Expected-match regression. |
| C5 | Calibration | auto | pass | fail | Expected-match regression. |
| C6 | Calibration | hybrid | pass | pass | Path-anchor replay preserves the expected match. |
| C6 | Calibration | auto | pass | pass | Mirrors hybrid. |
| C8 | Calibration | hybrid | pass | fail | Expected-match regression. |
| C8 | Calibration | auto | pass | fail | Expected-match regression. |
| C10 | Calibration | semantic | fail | pass | No-match fixed. |
| C10 | Calibration | hybrid | fail | pass | No-match fixed. |
| C10 | Calibration | auto | fail | pass | No-match fixed. |
| H6 | Hold-out | hybrid | pass | fail | Hold-out expected-match regression. |
| H6 | Hold-out | auto | pass | fail | Hold-out expected-match regression. |
| H9 | Hold-out | hybrid | fail | pass | No-match fixed. |
| H9 | Hold-out | auto | fail | pass | No-match fixed. |
| H10 | Hold-out | hybrid | pass | pass | Path-anchor replay preserves the expected match. |
| H10 | Hold-out | auto | pass | pass | Mirrors hybrid. |
| H11 | Hold-out | hybrid | fail | pass | No-match fixed. |
| H11 | Hold-out | auto | fail | pass | No-match fixed. |
| H17 | Hold-out | hybrid | pass | fail | Hold-out expected-match regression. |
| H17 | Hold-out | auto | pass | fail | Hold-out expected-match regression. |
| H18 | Hold-out | hybrid | pass | pass | Path-anchor replay preserves the expected match. |
| H18 | Hold-out | auto | pass | pass | Mirrors hybrid. |
| H20 | Hold-out | semantic | fail | pass | No-match fixed. |
| H20 | Hold-out | hybrid | fail | pass | No-match fixed. |
| H20 | Hold-out | auto | fail | pass | No-match fixed. |

## Optimization Reading

The path-anchor replay optimization removes several false simulated
regressions: hybrid and auto proposed totals recover from 17 / 13 to 25 / 5,
and hold-out hybrid/auto improves from 12 / 8 to 17 / 3. The candidate is
still not promotable because the hybrid semantic branch floor is infeasible:
the weakest calibration expected evidence is 0.103599 while the C10 no-match
maximum is 0.194490.

The remaining tuning target is narrower. C5 and C8 still regress in
calibration, while H6 and H17 still regress in hold-out. C8 is lexical-heavy
with semantic evidence below the default hybrid final floor, so the next
attempt should evaluate lexical-preservation tuning rather than simply lowering
semantic floors. C5 likely needs richer replay evidence, such as title/snippet
anchor counts, before the simulator can prove whether production search would
preserve it under calibrated thresholds.
