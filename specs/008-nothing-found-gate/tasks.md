# Tasks: Nothing-found and weak-evidence gate by relevance bands

**Input**: Design documents from `specs/008-nothing-found-gate/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Core and agent (M1, User Stories 1 to 4)

- [x] T001 [US1] [US2] [US4] Core `retrieval/gate.rs`: `classify_evidence`, `nothing_found_message`, `WEAK_EVIDENCE_WARNING`, `WEAK_EVIDENCE_RULE`; `evaluate_zero_hallucination` removed; unit tests
- [x] T002 [US2] `prompt.rs`: weak rule in the Fast and Deep prompts; `CanonicalSynthesisFormatter` maps `weak` to the maybe band; tests
- [x] T003 [US1] [US2] [US3] `actx-agent`: `Evidence`, `ContextReady::evidence`, `RetrievedChunk::weak`, `AgentEvent::Escalation` and `Caveat`, escalation in `run` and `stream`, caveat on weak; tests with mock LLM and retrievers
- [x] T004 [US1] [US2] `deep_search.rs`: nothing-found without synthesis, caveat on weak, `DeepSearchRetriever::nothing_found_message`; tests
- [x] T005 [US1] [US4] `actx-cli/engine.rs`: adapters return the evidence and `weak`, batch retriever nothing-found message, `search_db` message
- [x] T006 [P] [US2] [US3] TUI and one-shot output: caveat and escalation lines; tests

**Checkpoint (M1)**: every route reads the bands; no score cut left.

## Phase 2: Benchmark (M2, User Story 5)

- [x] T007 [US5] `actx-bench`: `weak_evidence` per question, the two rates in metrics, summary and compare; tests

## Phase 3: Docs and checks (M3)

- [x] T008 [P] ADR-0160; `docs/architecture/retrieval.md` and `agent.md`; README `/search auto`
- [x] T009 [P] `tests/MANUAL_TESTS.md`: nothing found in Fast, escalation in automatic mode, weak-evidence warning
- [x] T010 Backlog: close IMP-046; `@source` suggestion added to FEAT-088's description
- [ ] T011 `cargo test` per crate, `check_docs`, `manual_tests validate`, `check_design`; plan completed after CI
