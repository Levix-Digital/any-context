# Tasks: Selection by relevance bands and the token budget

**Input**: Design documents from `specs/007-band-selection/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Core (M1, User Stories 1, 2, 3 and 5)

- [x] T001 [US1] [US2] [US3] `retrieval/selection.rs`: `select`, near-duplicate check, `SelectionCounts`; unit tests
- [x] T002 [US4] `retrieval/requests.rs`: `token_budget` per builder (Fast 4,000, Deep 6,000, tool 3,000); tests
- [x] T003 [US1] `retrieval/pipeline.rs`: `token_budget` on `HybridSearchRequest`, selection step in `fuse_and_rerank`, `RetrievalOutput::selection`
- [x] T004 [US1] Remove `RetrievalPreset`, `top_k`, `max_chunks_per_source`, `max_density_chars`, `apply_density_budget` and their re-exports; `apply_source_cap` documented as reranker input slots
- [x] T005 [US1] [US5] Pipeline tests: band order and budget, no condensed marker, reranker unavailable, batch

**Checkpoint (M1)**: no fixed count, cap or character ceiling in the selection.

## Phase 2: Benchmark (M2, User Story 4)

- [x] T006 [US4] `actx-bench`: ranking request with an unbounded budget and its own top 10; delivered chunks and tokens averages in the summary; tests
- [x] T007 [US4] Local runs (mmarco, lexical and dense) with Fast budgets of 3,000, 4,000 and 6,000 recorded in `research.md`
- [x] T008 [US4] Budgets fixed from T007

**Checkpoint (M2)**: the budgets are measured.

## Phase 3: Docs and checks (M3)

- [x] T009 [P] ADR-0159; `docs/architecture/retrieval.md` selection step; benchmark page
- [x] T010 [P] `tests/MANUAL_TESTS.md`: one long document, copied file
- [x] T011 Backlog: close IMP-045; "current version only" added to FEAT-027's description
- [x] T012 `cargo test --workspace`, `check_docs`, `manual_tests validate`, `check_design`
- [ ] T013 Baselines refreshed from the CI RESULT-JSON; plan completed
