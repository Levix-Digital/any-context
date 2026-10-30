# Tasks: Unified fusion and reranker depth

**Input**: Design documents from `specs/004-unified-fusion/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Fusion (M1, User Stories 1 and 2)

- [x] T001 [P] [US1] `retrieval/rrf.rs`: `RankedList`, `FusedCandidate`, `fuse`; remove `reciprocal_rank_fusion` and `RRFScoreResult`; unit tests
- [x] T002 [US2] `retrieval/pipeline.rs`: `collect_lists` with separate lists (dense per workspace, lexical, file-name, date), rows and pinned ids
- [x] T003 [US1] `pipeline.rs`: `fuse_and_rerank` and selection shared by `search_with_outcome` and `retrieve_hybrid_batch_with_outcome`; score-summing deduplication removed; candidates scored against their best question in one pass
- [x] T004 [US2] Remove `min_score`, `RRF_MIN_SCORE`, `RetrievalPreset::min_score` and `apply_threshold`; callers and tests updated
- [x] T005 [US1] Pipeline tests: batch of two → each candidate scored once against its best question; batch of one equals Fast; no RRF summing
- [x] T006 [US2] Pipeline tests: file-name list pinned, date list clean, `Global` list separate, no candidate dropped by RRF score

**Checkpoint (M1)**: one fusion per search.

## Phase 2: Reranker input and depth (M2, User Stories 3 and 4)

- [x] T007 [P] [US4] `retrieval/rerank_depth.rs`: mode, profile depth, budgets, floor, measured cost, `effective_depth`; unit tests (SC-004)
- [x] T008 [US3] `pipeline.rs`: reranker input through `apply_source_cap(…, 3, depth)`; `MAX_RERANK_CANDIDATES` removed; cost recorded after each pass; test SC-002
- [x] T009 [US4] `HybridSearchRequest`: `rerank_depth`, `rerank_budget_ms`; `requests.rs` builders take the profile; `tool_request`
- [x] T010 [US4] `actx-cli/src/engine.rs`: builders with the active profile; inline tool request replaced; `active_profile` public
- [x] T011 [US4] `actx-bench`: standard profile, no budget; tests that the bench follows the product Fast request

**Checkpoint (M2)**: depth follows mode, profile and budget.

## Phase 3: Docs and baselines (M3)

- [x] T012 Benchmark: lexical and dense runs, results and latency in research.md, baselines refreshed
- [x] T013 [P] ADR-0156 (amends ADR-0129 and ADR-0139); `docs/architecture/retrieval.md` edited in place; `/rerank` status text; TECDOC index; `check_docs`
- [x] T014 [P] `tests/MANUAL_TESTS.md`: Deep breadth test; `manual_tests validate`
- [x] T015 Backlog: IMP-041 and IMP-042 closed; per-workspace tuning noted in FEAT-105
- [x] T016 `cargo test --workspace`, `check_design`, gate checks; plan milestones marked

**Checkpoint (M3)**: documented, measured, ready for the pull request.
