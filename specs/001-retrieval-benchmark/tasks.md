---

description: "Tasks for the development retrieval benchmark (IMP-065)"
---

# Tasks: Development retrieval benchmark on a fixed corpus

**Input**: Design documents from `specs/001-retrieval-benchmark/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/actx-bench-cli.md

**Tests**: required by the constitution (IV.2 Evidence) and listed in plan.md "Tests"; test tasks come before the code they prove.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1 measure, US2 compare, US3 CI gate, US4 grow the corpus safely

---

## Phase 1: Setup

- [x] T001 Add `crates/actx-bench` to the workspace members in `Cargo.toml`; create `crates/actx-bench/Cargo.toml` (`publish = false`, deps: `any-context-core-rs`, `actx-onnx`, `clap`, `serde`, `serde_json`, `sha2`, `tokio`) and an empty `src/main.rs` that builds
- [x] T002 [P] Create `benchmarks/retrieval/` with `corpus/`, an empty `questions.json` (`{"questions": []}`) and `results/` in `.gitignore`

---

## Phase 2: Foundational (blocks every story)

- [x] T003 Unit test in `crates/any-context-core-rs/src/retrieval/requests.rs` asserting today's Fast (6, 30, 14 000, `RRF_MIN_SCORE`, `Global`) and Deep (8, 40, 16 000) values
- [x] T004 Implement `fast_request` and `deep_request` in `crates/any-context-core-rs/src/retrieval/requests.rs`, re-export in `retrieval/mod.rs`
- [x] T005 Switch `crates/actx-cli/src/engine.rs` (`PipelineDeterministicRetriever::search`, `deep_search_request`) to the core builders; `cargo test --workspace` green
- [x] T006 [P] Question file model and parsing in `crates/actx-bench/src/questions.rs` (data-model "Question file")
- [x] T007 [P] Corpus walk and version hash in `crates/actx-bench/src/corpus.rs` (sorted relative paths and contents, SHA-256), with unit tests on a temp folder

**Checkpoint**: shared defaults in the core, the crate parses questions and hashes the corpus

---

## Phase 3: User Story 1 - Measure retrieval quality (P1) 🎯 MVP

**Goal**: one command indexes the corpus in isolation and reports every metric.

**Independent Test**: `actx-bench retrieval run` on a clean checkout prints all metrics and writes the result JSON.

- [x] T008 [P] [US1] Unit tests in `crates/actx-bench/src/metrics.rs`: rank 1, rank 3, not in top 10, graded NDCG, a document in several chunks counted once, fewer than 10 results, negatives excluded from ranking metrics, nothing-found and false-nothing-found rates, p95 nearest-rank, 4-decimal rounding
- [x] T009 [US1] Implement the metrics in `crates/actx-bench/src/metrics.rs` (overall, by language, by category)
- [x] T010 [US1] Isolation test in `crates/actx-bench/src/runner.rs`: with the temp overrides set, a sentinel default settings path is never created or written
- [x] T011 [US1] Implement the runner in `crates/actx-bench/src/runner.rs`: temp dir, `ACTX_SETTINGS_DB`/`ACTX_CONTEXT_DB`/`ACTX_MODELS_DIR`, workspace on `benchmarks/retrieval/corpus`, full sync with the mock target, verified reranker download and load, one warm-up query, ranking pass (`fast_request` with `top_k` 10 and no density cut) and delivered pass (`fast_request` as is), rerank mode `on`, no `Global`
- [x] T012 [US1] Map results to corpus documents and passages (chunk `file_path` relative to the corpus root; passage check on chunk text, whitespace normalized) in `crates/actx-bench/src/runner.rs`
- [x] T013 [US1] Result JSON writer (data-model "Run result", `config_id`, versions, code revision) and the `retrieval run` command with `--out` and the stdout table in `crates/actx-bench/src/main.rs`; exit code 3 for environment errors
- [x] T014 [US1] `--embedding <provider/model>`: key from the environment into the temp settings, index and query with that target, `config_id` `dense-...`; a missing key or provider error exits 3 with no fallback
- [x] T015 [US1] `--check-determinism`: second ranking pass, exit 5 on any difference; if it fails on the real corpus, add a stable tie-break by chunk id in `crates/any-context-core-rs/src/retrieval/pipeline.rs` with a unit test

**Checkpoint**: metrics are reproducible on the current pipeline

---

## Phase 4: User Story 2 - Compare with the previous run (P1)

**Goal**: each run shows its deltas against the committed baseline; the baseline is updated only on request.

**Independent Test**: change the candidate pool, run, and see the deltas and the questions that changed.

- [x] T016 [P] [US2] Unit tests in `crates/actx-bench/src/compare.rs`: comparable and non-comparable configurations (each differing field), deltas, regressed and improved questions, gate at −0.0199, −0.02 and −0.0201 for `mrr_at_10` and `hit_rate_at_5`, latency worse with quality equal passes
- [x] T017 [US2] Implement the comparison, the gate and the Markdown summary in `crates/actx-bench/src/compare.rs`
- [x] T018 [US2] `--compare`, `--save-baseline` and `--summary` in `crates/actx-bench/src/main.rs`; exit codes 1 and 4 per the contract; default baseline `benchmarks/retrieval/baseline.<config_id>.json`

**Checkpoint**: a local developer loop is complete

---

## Phase 5: User Story 4 - Grow the corpus safely (P3, needed before the corpus is written)

**Goal**: broken labels stop the run before indexing.

**Independent Test**: a question pointing to a missing document makes `validate` exit 2 and name it.

- [x] T019 [P] [US4] Unit tests in `crates/actx-bench/src/questions.rs`: missing document, passage not in the text, negative with labels, positive without labels, duplicate id, bad grade, unknown language or category
- [x] T020 [US4] Implement validation (text extraction for passages through the core's extractors for PDF) and the `retrieval validate` command; `run` validates first

---

## Phase 6: Corpus and questions (serves US1 to US4)

- [x] T021 [P] Write the Portuguese documents under `benchmarks/retrieval/corpus/pt/` (about 16 to 20: Markdown, text and at least 2 PDFs), with version, negation, module and date distractors
- [x] T022 [P] Write the English documents under `benchmarks/retrieval/corpus/en/` (about 14 to 18, same mix), including pairs that answer a question asked in the other language
- [x] T023 Copy `crates/any-context-core-rs/tests/fixtures/forms/cmr_single_pickup.pdf` to `benchmarks/retrieval/corpus/forms/` and add form-field questions for it
- [x] T024 Write `benchmarks/retrieval/questions.json`: 60 to 80 questions, at least 20 per language, about 15 negative, every category present; `validate` passes (SC-005)
- [ ] T025 Levi reviews the corpus and the questions (gate inside the milestone; changes go back to T021 to T024)

---

## Phase 7: User Story 3 - CI blocks a regression (P2)

**Goal**: pull requests that touch retrieval, ingestion or storage show the comparison and fail on a regression.

**Independent Test**: a PR with the reranker off fails the job; a docs-only PR skips it.

- [ ] T026 [US3] Run the benchmark, check determinism, commit `benchmarks/retrieval/baseline.lexical-rerank.json`
- [x] T027 [US3] Add `.github/workflows/retrieval-benchmark.yml` (path filter, ubuntu-latest, ONNX Runtime as in `e2e-tests.yml`, reranker cache keyed by its SHA-256, `run --compare --check-determinism --summary "$GITHUB_STEP_SUMMARY"`, upload the result JSON)
- [ ] T028 [US3] Check SC-003 on the branch (reranker off, then pool 5): exit 1 with the regressed questions; record the numbers in the PR description; revert the experiment
- [ ] T029 [US3] Check SC-001 (run time on the CI runner under 5 minutes, excluding the first download) from the workflow logs

---

## Phase 8: Polish and documentation

- [x] T030 [P] ADR-0153 "A development retrieval benchmark gates retrieval changes" in `docs/adr/` (with `architecture-docs` scripts)
- [x] T031 [P] "Benchmark" section in `docs/architecture/retrieval.md`; regenerate `TECDOC.md`
- [x] T032 Regenerate `.specify/memory/constitution.md` (`constitution.py generate`) in the same PR as the ADR
- [ ] T033 Run the quickstart commands from a clean checkout; `cargo test --workspace` and `cargo clippy` green

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phases 3, 4 and 5 (4 needs the result model of T013; 5 is independent of 3 and 4) → Phase 6 (needs 5 for validation) → Phase 7 (needs 3, 4 and 6) → Phase 8.
- T003 before T004 before T005; tests before the code they prove (T008→T009, T010→T011, T016→T017, T019→T020).
- T025 (Levi's review) blocks T026: the first baseline is committed only on the reviewed questions.

## Parallel Opportunities

- T006 and T007; T008, T016 and T019 (different files); T021 and T022; T030 and T031.

## Implementation Strategy

One pull request into `dev` from `feat/001-retrieval-benchmark`, with commits per phase. The MVP is Phases 1 to 4 on a few placeholder questions; the corpus (Phase 6) is the bulk of the work and goes to Levi for review before the baseline.
