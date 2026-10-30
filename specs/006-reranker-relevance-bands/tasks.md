# Tasks: Reranker relevance bands

**Input**: Design documents from `specs/006-reranker-relevance-bands/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Core (M1, User Stories 1 and 2)

- [x] T001 [US2] `retrieval/relevance.rs`: `RelevanceBand`, `RelevanceThresholds::band`, `BandCounts`; unit tests
- [x] T002 [P] [US2] `ingestion/model_manager.rs`: `relevance` on `OnnxModelSpec`, thresholds on both rerankers, `relevance_thresholds(id)`; catalog test
- [x] T003 [US1] `retrieval/pipeline.rs`: `relevance_band` on `HybridSearchResult`; band and cut after a `Scored` pass; `set_relevance_cut`
- [x] T004 [US2] `pipeline.rs`: `RetrievalOutput::bands` and the process-wide last counts
- [x] T005 [US1] [US2] Pipeline tests: irrelevant discarded (SC-001), all irrelevant, pinned, unavailable, unknown model, cut off, batch

**Checkpoint (M1)**: no irrelevant chunk reaches the selection.

## Phase 2: Status and benchmark (M2, User Stories 3 and 4)

- [x] T006 [US3] `commands/engine.rs`: band counts in the `/status` reranker line; test
- [x] T007 [US4] `actx-bench`: `--reranker`, ranking pass without the cut, best score per question, calibration table, `delivered_hit_rate` gated; tests
- [x] T008 [US4] Local calibration runs (mmarco and bge-reranker-v2-m3, lexical and dense) recorded in `research.md`
- [x] T009 [US4] Thresholds fixed in the catalog from T008

**Checkpoint (M2)**: the cut is measured and calibrated.

## Phase 3: Docs and checks (M3)

- [x] T010 [P] ADR-0158 (amends ADR-0157); `retrieval.md`, `commands.md`, benchmark page edited in place; TECDOC index; `check_docs`
- [x] T011 [P] `tests/MANUAL_TESTS.md`: band counts in `/status` for a question with no answer; `manual_tests validate`
- [ ] T012 Benchmark: CI lexical and dense results in `research.md`; baselines refreshed from the CI run
- [x] T013 Backlog: IMP-044 closed; `cargo test --workspace`, `check_design`, gate checks
- [ ] T014 Plan milestones marked, plan completed

**Checkpoint (M3)**: documented, measured, ready for the pull request.
