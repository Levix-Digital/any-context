# Tasks: Automatic mode routes by evidence, not by keywords

**Input**: Design documents from `specs/009-evidence-routing/`

**Prerequisites**: plan.md, spec.md

**Tests**: included (the protocol requires automated evidence).

Format: `[ID] [P?] [Story] Description`. `[P]` = can run in parallel (different files, no dependency).

## Phase 1: Route and signal (M1, User Stories 1 to 4)

- [ ] T001 [US1] [US4] `actx-agent/src/router.rs`: `Route`, `RouteDecision`, `route()`; classifier, intent, complexity and router config removed; `router_tests.rs` rewritten
- [ ] T002 [US1] [US4] `actx-agent/src/lib.rs`: `run` and `stream` on `Route`; `AgentBuilder::router` removed; IMP-046 band escalation kept; tests
- [ ] T003 [US2] `actx-agent/src/escalation.rs`: `NOT_ENOUGH_SIGNAL`, `NOT_ENOUGH_RULE`, `SignalFilter`; rule appended to the Fast prompt only when it can escalate; escalation on the signal in `run` and `stream`; signal removed elsewhere; unit and agent tests
- [ ] T004 [US1] `actx-onnx` query classifier and core `OnnxComplexityClassifierAdapter` removed; `actx-cli/engine.rs` builds no router
- [ ] T005 [P] [US1] `AgentEvent::RoutingDecision { route, pinned, reason }`; TUI and one-shot routing lines; tests

**Checkpoint (M1)**: no route reads the question's words except the status terms.

## Phase 2: Weights removal (M2, User Story 5)

- [ ] T006 [US5] `bge-small-onnx` out of the catalog, aliases, kits, installer list and model menu; catalog and kit tests

## Phase 3: Docs and checks (M3)

- [ ] T007 [P] ADR-0161; `agent.md`, `onnx.md`, `model-catalog.md`, `cli.md`; `config/SYSTEM_KNOWLEDGE.md`; README `/search auto`
- [ ] T008 [P] `tests/MANUAL_TESTS.md`: quick search for a long question, escalation on "not enough", pinned Fast, live status badge, model menu
- [ ] T009 Backlog: close IMP-038
- [ ] T010 `cargo test` per crate, `check_docs`, `manual_tests validate`, `check_design`; plan completed after CI
