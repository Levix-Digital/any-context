# Quickstart: retrieval benchmark

Needs the ONNX Runtime library (`bash scripts/fetch_onnxruntime.sh .ort`, then `ORT_DYLIB_PATH` as in `e2e-tests.yml`). The first run downloads the verified reranker (about 136 MB).

```bash
# Check the labels after editing the corpus or the questions
cargo run -p actx-bench --release -- retrieval validate

# Measure the current code (lexical + reranker, the CI configuration) against the baseline
cargo run -p actx-bench --release -- retrieval run

# Accept a change: save the new baseline and commit it with the change
cargo run -p actx-bench --release -- retrieval run --save-baseline
git add benchmarks/retrieval/baseline.lexical-rerank.json

# Locally, with the dense retriever on a configured provider (never committed as the gate)
GEMINI_API_KEY=... cargo run -p actx-bench --release -- retrieval run --embedding gemini/gemini-embedding-2
```

Adding a document: put it under `benchmarks/retrieval/corpus/<lang>/`, label the questions that it answers or distracts, run `validate`, then `run --save-baseline` (the corpus version changes, so the old baseline is no longer comparable).

Verifying the gate (SC-003): set the reranker off or the candidate pool to 5 in `retrieval::requests`, run, and check exit code 1 with the regressed questions listed.
