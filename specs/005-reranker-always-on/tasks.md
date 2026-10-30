# Tasks: Reranker always on

**Input**: Design documents from `specs/005-reranker-always-on/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Core (M1, User Stories 1, 2 and 3)

- [x] T001 [US3] `retrieval/pipeline.rs`: `RerankOutcome` with `answer_notice`, `RetrievalOutput::rerank` and `notices()`, process-wide last outcome; unit tests
- [x] T002 [US1] `pipeline.rs`: `rerank_mode` removed (field, setter, getter, request field, `should_rerank`); pipeline tests updated
- [x] T003 [US2] `pipeline.rs`: `neural_rerank_scores` returns the cause; `apply_neural_rerank` returns the outcome; unscored unpinned candidates discarded on `Scored`; tail rescaling removed
- [x] T004 [US2] [US3] Pipeline tests: tail discarded (SC-002), fewer than depth, pinned kept, unavailable keeps fusion order with cause, only pinned → `NotNeeded`, batch of two → one outcome
- [x] T005 [P] [US1] `retrieval/requests.rs`: builders without `rerank_mode`; tests

**Checkpoint (M1)**: the core always reranks and reports when it cannot.

## Phase 2: Surfaces (M2, User Stories 1, 3 and 4)

- [x] T006 [US1] `commands/engine.rs`, `models.rs`: `/rerank` removed, `rerank_mode` removed from context and state updates; `command_tests.rs` updated
- [x] T007 [US3] `/status` reranker line (model, state, depths, last pass); test
- [x] T008 [US1] `actx-cli`: registry entry, `--rerank` (args, oneshot, tui/mod) removed; retrievers pass both notices; `cli_tests.rs` updated
- [x] T009 [P] [US1] TUI: `rerank_mode` removed, badge ready/unavailable; tests
- [x] T010 [P] [US4] `actx-agent`: per-chunk LLM yes/no removed; Deep test without per-chunk calls
- [x] T011 [P] `actx-bench`: no mode; unavailable reranker is an environment error

**Checkpoint (M2)**: no surface can turn the reranker off.

## Phase 3: Docs and checks (M3)

- [x] T012 [P] ADR-0157 (amends ADR-0129 and ADR-0156); `retrieval.md`, `commands.md`, `cli.md`, `tui.md`, `agent.md` edited in place; TECDOC index; `check_docs`
- [x] T013 [P] README: `/rerank` and `--rerank` rows removed, troubleshooting via `/status`; `check_readme`
- [x] T014 [P] `tests/MANUAL_TESTS.md`: `/status` reranker test, missing-weights warning test; `manual_tests validate`
- [ ] T015 Benchmark: CI lexical and dense results in research.md (baselines refreshed only if they change)
- [x] T016 Backlog: IMP-043 closed; `cargo test --workspace`, `check_design`, gate checks; plan milestones marked

**Checkpoint (M3)**: documented, measured, ready for the pull request.
