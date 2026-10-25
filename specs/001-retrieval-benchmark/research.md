# Research: Development retrieval benchmark

Decisions taken while planning IMP-065, from reading the current code on `dev` (4a31988).

## R1. Which "real pipeline" the benchmark runs

- **Finding**: the product's retrieval requests are built in the CLI adapter, not in the core. `crates/actx-cli/src/engine.rs` builds the Fast request (`PipelineDeterministicRetriever::search`: `top_k` 6, `candidate_pool_k` 30, `max_density_chars` 14 000, `RRF_MIN_SCORE`, `target_workspaces` `Global`) and the Deep sub-query request (`deep_search_request`: 8, 40, 16 000). `RetrievalPreset` in `retrieval/pipeline.rs` is not used by the product.
- **Decision**: move the two request builders into the core (`retrieval::requests`: `fast_request(query, workspace, rerank_mode)` and `deep_request(...)`), have the CLI call them, and have the benchmark call the same functions. No behavior change; the CLI's existing tests keep passing.
- **Rationale**: FR-007 says a change to the defaults must show up in the benchmark. If the benchmark copied the numbers, an IMP-040 change in `engine.rs` would not move the benchmark. One builder is the single source of truth.
- **Alternatives**: copy the defaults into the benchmark (drifts silently); make the benchmark drive `actx-cli` (pulls the TUI, the agent and the user's settings into a test).

## R2. Measuring @10 when the product returns 6 chunks

- **Finding**: the Fast request returns at most 6 chunks after the density cut, so HitRate@10, MRR@10 and NDCG@10 cannot be measured on it.
- **Decision**: each question runs twice through the same pipeline: (a) a **ranking** run, the Fast request with `top_k` 10 and no density cut, which feeds HitRate@1/3/5/10, MRR@10 and NDCG@10 and the gate; (b) a **delivered** run, the Fast request exactly as the product sends it, which feeds `delivered_hit` (the expected document is among what the LLM would get), the nothing-found rate and the latency. Deep is out of the first version (it rewrites queries with an LLM).
- **Rationale**: the ranking metrics measure order, which is what IMP-040 to IMP-046 change; the delivered figure measures what the user actually gets.
- **Alternatives**: only the delivered run (no @10 metrics, as the backlog asks); only the ranking run (misses the density cut and top_k effects).

## R3. Dense retriever in CI

- **Finding**: there is no local embedding model; `ingestion/embedding.rs` resolves Gemini, OpenAI, Ollama or the mock target. The pipeline already skips dense search when the query target is the mock one (`QueryEmbeddingOutcome`, BUG-019), so indexing with the mock target gives a clean lexical + reranker run.
- **Decision** (with Levi, 2026-10-25): CI indexes with the mock target (dense skipped) and loads the default verified reranker. A local run can pass `--embedding <provider/model>` to index and query with a configured provider; the run records it and is never compared with the CI baseline. FEAT-067 later adds the local-dense configuration and its own baseline.
- **Alternatives**: pull FEAT-067 forward (rejected by Levi's choice, its model must be picked by this benchmark); commit precomputed vectors (break whenever chunking changes).

## R4. Where the benchmark lives

- **Decision**: a new workspace crate `crates/actx-bench` (`publish = false`), a binary plus a small library for the pure parts (question file, validation, metrics, comparison). It depends on `any-context-core-rs` and `actx-onnx` only.
- **Rationale**: it is a driving adapter in hexagonal terms (it drives the core's ingestion and retrieval like the CLI does); it keeps the product binaries untouched (SC-006); release builds and installers only package `actx-cli`.
- **Alternatives**: `examples/` in the core (no place for its own unit tests and dependencies, still compiled with the core's dev-dependencies); an integration test in the core (`cargo test --workspace` would download the reranker on every developer's run).

## R5. Ingestion of the corpus

- **Finding**: `tests/orchestrator_tests.rs` shows the pattern: `NativeConfigDb::open` and `NativeLanceStore::open` on a temp directory, `create_workspace`, `add_workspace_folder`, `NativeSyncOrchestrator::run` with `model: Some("mock")`. The CMR PDF is already ingested by the forms tests.
- **Decision**: the runner copies nothing; it creates a temp directory with a fresh `settings.db` and `lancedb`, registers `benchmarks/retrieval/corpus` as the workspace folder and runs a full sync. `target_workspaces` is empty: the `Global` AnyContext documentation is not part of the benchmark (it changes with every release and IMP-017 replaces it).
- **Isolation**: some core paths still call `open_default()` (the active reranker lookup in `ingestion/model_manager.rs`, `NativeSyncOrchestrator::open_default`), so before touching the core the runner points `ACTX_SETTINGS_DB` and `ACTX_CONTEXT_DB` at the temp directory and `ACTX_MODELS_DIR` at the model cache (a temp or CI cache folder, or the developer's models folder when they pass `--models-dir`). The rerank mode is passed explicitly (`on`), never read from settings. A test checks that a run leaves the default settings path untouched.

## R6. Corpus and question format

- **Decision**: `benchmarks/retrieval/corpus/{pt,en}/...` with Markdown, text and PDF files, plus a copy of the CMR PDF. Questions in `benchmarks/retrieval/questions.json` (JSON because the repository already uses `serde_json`; no new dependency). Documents are referenced by path relative to the corpus root; a passage is an exact substring of the document text.
- **Versions**: corpus version = SHA-256 over the sorted relative paths and contents; question file version = SHA-256 of the file. No manual version to forget.

## R7. Metrics

- Document level: a document counts once, at its best rank. HitRate@k = share of positive questions with a relevant document in the top k. MRR@10 = mean of 1/rank of the first relevant document (0 when not in the top 10). NDCG@10 with graded relevance (grade 1 to 3, gain 2^g − 1, log2 discount), IDCG from the labels.
- Passage HitRate@5: the expected passage text is contained in one of the top 5 chunks (whitespace normalized). Reported only.
- Nothing-found: the delivered run returned no result. `nothing_found_rate` on negatives; `false_nothing_found_rate` on positives. Reported only until IMP-046.
- Latency: wall time of the delivered run per question, mean and p95 (nearest-rank), excluding model load (one warm-up question first).

## R8. Determinism

- **Risk**: equal scores ordered by hash map iteration in `search_single_async` (candidates are collected in a `HashMap`).
- **Decision**: the runner checks SC-002 itself in CI by running the ranking pass twice and failing on any difference; if the pipeline turns out non-deterministic, the fix (a stable tie-break by chunk id after scoring) goes in the core in this plan, with a unit test, because it is also a product defect.

## R9. Regression rule and baseline

- Gate: fail when `mrr_at_10` or `hit_rate_at_5` is more than 0.02 below the baseline of the same configuration (Levi, 2026-10-25). Latency never gates.
- Baseline: `benchmarks/retrieval/baseline.lexical-rerank.json`, written only by `actx-bench run --save-baseline`. CI without a baseline for its configuration fails with the command to create it.

## R10. CI

- New workflow `.github/workflows/retrieval-benchmark.yml`, `ubuntu-latest` only, on pull requests into `dev`/`main` and pushes to `dev`, filtered by paths: `crates/any-context-core-rs/src/{retrieval,ingestion,storage}/**`, `crates/actx-onnx/**`, `crates/actx-bench/**`, `benchmarks/retrieval/**`, the workflow file. ONNX Runtime from `scripts/fetch_onnxruntime.sh` as in `e2e-tests.yml`; the reranker cached with `actions/cache` keyed by its SHA-256 from the catalog. The comparison goes to `$GITHUB_STEP_SUMMARY`, the result JSON is uploaded as an artifact.
- Windows is skipped: the metrics do not depend on the OS and the existing workflow already covers Windows builds.
