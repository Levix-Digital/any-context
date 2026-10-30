# Tasks: Pre-filtered dense and lexical retrievers

**Input**: Design documents from `specs/003-prefiltered-retrievers/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Filter (M1, User Story 1)

- [x] T001 [US1] `crates/any-context-core-rs/src/retrieval/filter.rs`: `RetrievalFilter`, escaping, `lance_predicate`, `matches`; unit tests (dimensions, escaping, sibling prefix, open and inverted ranges, missing date, empty filter)
- [x] T002 [US1] `retrieval/pipeline.rs`: `HybridSearchRequest.filter` (`serde(default)`), passed to the dense search and to the file-name and date metadata lookups
- [x] T003 [US1] `retrieval/bm25.rs`: `search` applies the filter before ranking; unit test that the limit is filled from matching documents
- [x] T004 [US1] Check against a real LanceDB table that the source scope (`starts_with`, after `LIKE … ESCAPE` was refused) and the date comparison work as the unit tests expect
- [x] T005 [US1] Pipeline integration tests: SC-001 scenario, empty result for a filter matching nothing, unfiltered results unchanged

**Checkpoint (M1)**: both retrievers honor the filter.

## Phase 2: Parallel retrievers and candidates (M2, User Story 2)

- [x] T006 [US2] `pipeline.rs`: `bm25_indices` as `Arc<BM25Index>`; callers of `get_or_load_bm25` updated
- [x] T007 [US2] `pipeline.rs`: lexical search in `spawn_blocking` joined with question embedding and dense search; test that the lexical list is produced when the embedding provider fails
- [x] T008 [US2] `retrieval/requests.rs`: `CANDIDATES_PER_LIST = 30` for Fast and Deep; tests updated
- [ ] T009 [US2] Benchmark: lexical and dense runs with 30 and with 50 per list; results and latency in research.md

**Checkpoint (M2)**: one search costs about the slower retriever; defaults measured.

## Phase 3: Dense outcome in the answer (M3, User Story 3)

- [x] T010 [US3] `pipeline.rs`: `RetrievalOutput`, `search_with_outcome`, `retrieve_hybrid_batch_with_outcome`; existing functions delegate; tests per outcome
- [x] T011 [US3] `QueryEmbeddingOutcome::answer_notice` with cause and fix; unit tests
- [x] T012 [US3] `actx-agent`: `notice` on `DeterministicRetrievalOutput`, `DeepSearchRetriever::last_notice`, `AgentEvent::Notice` sent once before `Done`; tests with mock retrievers (Fast, nothing-found, Deep, no notice when dense ran)
- [x] T013 [US3] `actx-cli/src/engine.rs`: adapters fill the notice from the search outcome
- [x] T014 [P] [US3] `tui/app.rs`: dimmed warning line under the answer; test
- [x] T015 [P] [US3] `cli/oneshot.rs`: warning line after the answer (no unit test: the event loop needs a live agent; covered by MT-ANS-009)

**Checkpoint (M3)**: the user sees when semantic search did not run.

## Phase 4: Docs and baselines (M4)

- [ ] T016 ADR-0155 and constitution regenerated if the generator picks it up
- [ ] T017 [P] `docs/architecture/retrieval.md` and `agent.md` edited in place; TECDOC index regenerated
- [x] T018 [P] `tests/MANUAL_TESTS.md`, "Retrieval": warning shown and not shown
- [ ] T019 Benchmark baselines refreshed with the final defaults
- [ ] T020 Backlog: IMP-040 closed; the deferred parts written into IMP-033, FEAT-036, IMP-026 and FEAT-082
