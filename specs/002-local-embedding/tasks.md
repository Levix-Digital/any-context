# Tasks: Local multilingual embedding model, independent of the chat model

**Input**: Design documents from `specs/002-local-embedding/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/embedding-cli.md

**Tests**: included (the protocol requires automated evidence; regression tests for the IMP-028 failure cases).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Setup

- [x] T001 Read `4042b89` (`feat/imp-004-embedding-model`) and list what is ported (setting, migration, no-fallback errors) and what is not (file-by-file re-embedding); note it in research.md R5/R6 if anything differs
- [x] T002 [P] Fetch the three candidates' files once, record their SHA-256 and confirm the graph inputs/outputs (`input_ids`, `attention_mask`, optional `token_type_ids`; `last_hidden_state`) in research.md R1/R3

## Phase 2: Foundational (blocks every story)

- [x] T003 `crates/any-context-core-rs/src/ingestion/embedding_catalog.rs`: `EmbeddingModelSpec` (runtime, license, dimension, max tokens, pooling, normalize, prefixes, prefix_rev), the three local and three cloud entries, lookup by id; unit tests for catalog validity
- [x] T004 `ingestion/model_manager.rs`: `OnnxModelCategory::Embedding` and the three local entries with pinned files (model + tokenizer), sizes and SHA-256
- [x] T005 `ingestion/embedding.rs`: `EmbeddingTarget` with `dimension` and `prefix_rev`, key `provider/model@dim[+pN]`, legacy `provider/model` parsing; unit tests (round trip, legacy keys, inequality across dimension and prefix)
- [x] T006 [P] `hardware/mod.rs`: `ProfileTier::default_embedding_model()` (proposal from research R9; e5-small for `Lite`), profiles' `target_models` include it; tests for disk sizes
- [x] T007 [P] `storage/lancedb.rs`: nullable `embedding_target` column added on open (provenance-columns pattern), written by `upsert_records`, filter in `search_vector_async`/`search_vector`; tests: column added once, rows of two targets never mixed in a search
- [x] T008 [P] `storage/sqlite.rs`: `get/set_embedding_model`, migration marker helpers; tests

**Checkpoint (M1)**: catalog, identity, storage ready.

## Phase 3: User Story 1, local model runs (P1)

- [x] T009 [US1] `crates/actx-onnx/src/embedder.rs`: `OnnxEmbedder` config (paths, max tokens, pooling, normalize), lazy load with recorded load errors, batches, truncation counter
- [x] T010 [P] [US1] Pooling and normalization unit tests on hand-built tensors (mean with mask, CLS)
- [x] T011 [US1] Reference vectors: script under `crates/actx-onnx/tests/fixtures/embedder/` (Python `onnxruntime` + `tokenizers`) producing expected vectors for six pt/en sentences for e5-small; Rust test compares (cosine ≥ 0.995 per text; see research R3); skipped with a message when the files are absent, required in CI (`ACTX_REQUIRE_EMBEDDER_TESTS=1`)
- [x] T012 [US1] `ingestion/local_embedding.rs`: `LocalEmbeddingProvider` implementing `LmProvider::embed` over `OnnxEmbedder` (blocking work on the blocking pool); tests: order kept, errors mapped to `LmError`
- [x] T013 [US1] Missing model at sync: verified download with progress through `OnnxModelManager::download_model` (per Levi's answer to clarification 1); offline or checksum failure gives `EmbeddingUnavailable` with the install fix
- [x] T014 [US1] No lexical-only mode: when no model can be made available the sync stops before indexing with model, source and manual-install instructions (FR-013); `Lite` gets e5-small; tests
- [x] T014a [US1] Model source (FR-018): `models_mirror` setting and `ACTX_MODELS_MIRROR`, URL rewrite to `<mirror>/<file_name>`, pre-placed files used after SHA-256 check with no network; tests (mirror only contacted, wrong checksum refused)
- [x] T014b [US3] `is_model_allowed(id)` seam called by the resolution before building any model (FR-019), allowing the catalog in this release; refusal stops the sync with the reason; tests

**Checkpoint (M2)**: a local model embeds text through the product port.

## Phase 4: User Story 3, no silent switch (P1)

- [x] T015 [US3] `ingestion/embedding.rs`: `resolve_embedding(db) -> Result<Option<ResolvedEmbedding>, EmbeddingUnavailable>` (setting, else profile default, else `None`); mock only through `embedding::testing::mock()`; OpenAI and mock fallbacks removed
- [x] T016 [US3] Query side: recorded target answers until reindex; if it cannot be built, dense is skipped for that question and logged, never another model
- [x] T017 [US3] `orchestrator.rs`: uses `resolve_embedding`; stops before embedding on `EmbeddingUnavailable` (index unchanged); per-file failures reported and retried next sync; records the target on every chunk
- [x] T018 [P] [US3] Regression tests: unavailable model writes zero chunks (SC-004); failing provider for one file leaves the others and retries it; product resolution never returns mock
- [x] T019 [US3] Update callers: `actx-cli/src/engine.rs`, `cli/oneshot.rs` (Global bootstrap), `retrieval/pipeline.rs`, `actx-bench/src/runner.rs`; remove `resolve_embedding_provider`'s model-override path

## Phase 5: User Story 2, a setting of its own (P1)

- [x] T020 [US2] `ingestion/embedding_migration.rs`: one-time adoption of the most used real recorded target, legacy keys rewritten, chunk backfill, marker (data-model "Migration"); tests (adopt, ignore mock, ties, idempotent). Done in `ingestion/embedding.rs` (`adopt_existing_index_model`); legacy keys are rewritten by the sync; chunks written before the column keep a null target, which the dense filter accepts, instead of a backfill
- [x] T021 [US2] Core command `embedding show/set` (catalog list, installed state, reindex-affected workspaces, cloud data notice) in `commands/`
- [x] T022 [US2] TUI `/embedding [id]` in `commands/registry.rs` and the TUI handler, with confirmation
- [x] T023 [US2] Headless `actx embedding [id] [--yes]` in `cli/args.rs`/`cli/oneshot.rs` (exit codes per contract)
- [x] T024 [US2] `-m/--model` and `/model` no longer affect embeddings (`SyncOptions::model` not used for embedding; `pending_reindex_notice` driven by the embedding setting); regression test: chat-model change re-embeds zero chunks (SC-003)
- [ ] T025 [P] [US2] `/models` lists embedding models in their category; diagnostics shows model, source and installed state
- [x] T026 [P] [US2] CLI tests for the contract (show, set with and without `--yes`, unknown id, cloud notice)

**Checkpoint (M3)**: the setting drives everything; no fallback anywhere.

## Phase 6: User Story 4, prefixes (P2)

- [x] T027 [US4] `apply_prefix(role, text)` from the catalog; orchestrator applies the document prefix right before embedding (stored text and BM25 untouched)
- [x] T028 [US4] Pipeline applies the query prefix to the question embedding
- [x] T029 [P] [US4] Tests: prefixes per model and role; stored text has no prefix; a `prefix_rev` change marks workspaces for reindex
- [x] T030 [US4] Gemini prefixes (`prefix_rev` 1) and nomic prefixes in the catalog; adapter tests unchanged in behavior otherwise

**Checkpoint (M4)**.

## Phase 7: User Story 5, benchmark, defaults and CI (P2)

- [x] T031 [US5] `actx-bench`: `--embedding local/<id>` through the product resolution; run configuration records the full key; tests
- [x] T032 [US5] CI `workflow_dispatch` input to run a chosen candidate; model files cached by the catalog file
- [x] T033 [US5] Run the three candidates; fill research.md "Results" (MRR@10, HitRate@5, cross-language MRR, indexing time, size, peak RSS)
- [x] T034 [US5] Set the profile defaults from the results (rule in spec assumptions); update `default_embedding_model()` and tests
- [ ] T035 [US5] Dense CI job with the reference default and committed baseline `baseline.dense-local-<id>-rerank.json` (from the CI `RESULT-JSON` line); lexical job unchanged
- [ ] T036 [US5] SC-005 check once: swapped prefixes fail the dense job; record in the PR description
- [x] T037 [US5] Unit/e2e workflows provide the cached reference model so T011 runs in CI

**Checkpoint (M5a)**.

## Phase 8: Polish and documentation

- [x] T038 ADR-0154 (supersedes the chat-model derivation of ADR-0132, amends ADR-0138); `constitution.py generate`
- [x] T039 [P] Architecture pages (ingestion embedding section, retrieval query embedding and benchmark) edited in place; TECDOC regenerated
- [x] T040 [P] README "Configuration": embedding model, `/embedding`, local by default, cloud by choice
- [x] T041 [P] `tests/MANUAL_TESTS.md` section "Embedding model" (five cases from the plan)
- [ ] T042 Full `cargo test --workspace`, clippy, the benchmark gates green; release-notes notes (Gemini reindex once, rollback caveat)

**Checkpoint (M5)**: ready for review.

## Dependencies

Phase 2 blocks all stories. US1 (Phase 3) and US3 (Phase 4) can proceed in parallel after Phase 2; US2 needs US3's resolution (T015); US4 needs T005 and T017/T028 call sites; US5 needs US1, US3 and US4.
