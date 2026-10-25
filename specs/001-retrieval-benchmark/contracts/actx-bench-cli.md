# Contract: `actx-bench` command line (development only)

`actx-bench` is a workspace binary for developers and CI. It is not installed, packaged or documented for end users.

```text
actx-bench retrieval validate
actx-bench retrieval run [--embedding <provider/model>] [--models-dir <dir>]
                         [--out <file>] [--compare <baseline>] [--save-baseline]
                         [--check-determinism] [--summary <markdown file>]
```

| Option | Meaning |
|---|---|
| `validate` | Checks `questions.json` against the corpus (FR-005) and exits. |
| `run` | Indexes the corpus in a temp directory and runs every question (FR-006, FR-007). |
| `--embedding` | Index and query with this embedding target (local only). Default: the mock target, dense skipped. Needs the provider's key in the environment; a missing key or a provider error stops the run (no silent fallback). |
| `--models-dir` | Where the reranker is cached or downloaded. Default: `ACTX_MODELS_DIR`, else a folder under the system temp directory. |
| `--out` | Result file. Default: `benchmarks/retrieval/results/<config_id>.json` (git-ignored). |
| `--compare` | Baseline to compare with. Default: `benchmarks/retrieval/baseline.<config_id>.json` when it exists. |
| `--save-baseline` | Writes the result as `baseline.<config_id>.json` (FR-013). Never implied. |
| `--check-determinism` | Runs the ranking pass twice and fails on any difference (SC-002). |
| `--summary` | Also writes the comparison as Markdown (CI passes `$GITHUB_STEP_SUMMARY`). |

Run from the repository root (`cargo run -p actx-bench --release -- retrieval run`).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Run finished; no regression, or no baseline to compare with locally |
| 1 | Regression: `mrr_at_10` or `hit_rate_at_5` more than 0.02 below the baseline |
| 2 | Invalid labels (validation failed) |
| 3 | Environment error: reranker or ONNX Runtime unavailable, embedding provider missing or failing, I/O error |
| 4 | Not comparable with the baseline, or no baseline when `--compare` was given explicitly (CI) |
| 5 | Non-deterministic results with `--check-determinism` |

## Output

Standard output: one table with the overall metrics and the deltas, then the regressed and improved questions. Errors go to standard error with the question id or the failing step.
