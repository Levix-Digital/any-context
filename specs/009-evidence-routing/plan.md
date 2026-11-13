# Implementation Plan: Automatic mode routes by evidence, not by keywords

**Branch**: `feat/009-evidence-routing` | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-evidence-routing/spec.md`

## Summary

The keyword and length classifier that picks Fast or Deep before the search is removed, with the heuristic query classifier, its adapter and its unused 133 MB weights. Automatic mode always starts Fast and escalates to Deep on evidence: the IMP-046 band trigger (nothing or weak), and a new LLM trigger: in automatic mode the Fast prompt asks the LLM to reply only with a fixed signal when the excerpts do not hold the answer, and the agent, which holds back the first streamed characters, drops that attempt and runs Deep. Pinned modes are respected; live status questions keep their route until FEAT-082.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only; no new crate.

**Storage**: none changed.

**Testing**: `cargo test` per crate (`actx-agent` with mock LLM and retrievers, `any-context-core-rs`, `actx-onnx`, `actx-cli`, `actx-installer`); the CI benchmark runs gate the unchanged retrieval metrics.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: questions the classifier sent to Deep now take one Fast call when one excerpt answers them; normal Fast answers wait at most the signal's length (28 characters) before the first token is shown.

**Constraints**: no reindex; retrieval output unchanged.

**Scale/Scope**: `actx-agent` (`router.rs`, `lib.rs`, `events.rs`, tests), `actx-onnx` (`query_classifier.rs` removed, `engine.rs`, `lib.rs`), core (`ingestion/onnx_adapter.rs`, `ingestion/model_manager.rs`, `hardware/mod.rs`), `actx-cli` (`engine.rs`, TUI `app.rs` and `menu.rs`, `cli/oneshot.rs`), `actx-installer` (`hardware.rs`).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | The route is a pure function of the search mode and the status terms; the signal check is a small streaming filter with unit tests | Pass |
| II. Simplicity First | About 600 lines of classifier and its weights removed; one rule, one signal, one filter added; no configuration | Pass |
| III. Hexagonal | The agent owns the signal and the rule (it decides escalation); the core prompt builders do not change | Pass |
| IV.2 Evidence | Agent tests for every route and trigger with mock LLM and retrievers; filter unit tests; catalog and kit tests | Pass |
| IV.3 Documentation describes today | `agent.md`, `onnx.md`, `model-catalog.md`, `cli.md`, `SYSTEM_KNOWLEDGE.md`, README edited in place; ADR-0161 | Pass |
| IV.6 Security | The signal is matched only at the start of the answer and removed elsewhere; retrieved text stays escaped, so an excerpt cannot inject the rule | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **Route** (`actx-agent/src/router.rs`, rewritten): `pub enum Route { Fast, Deep, Status }` and `pub struct RouteDecision { route, pinned: bool, reason: &'static str }`. `pub fn route(query, search_mode) -> RouteDecision`: status terms → `Status` (any mode); `Deep` pinned → `Deep`; `Fast` pinned or `Auto` → `Fast` (`pinned` false for `Auto`). `is_status_query` and `STATUS_TERMS` stay. `ChatModelRouter`, `ChatRouterConfig`, `ComplexityClassifier`, `DeterministicClassifier`, `QueryComplexity`, `QueryIntent`, `RoutingDecision` are removed, with `AgentBuilder::router` and `Agent::router`.
2. **Agent routes** (`lib.rs`): `run` and `stream` match on `Route`. `Fast` uses the deterministic retriever when there is one, else the tool-calling orchestrator as today; `Deep` uses the Deep engine when there is one. IMP-046's band escalation is unchanged.
3. **Not-enough signal** (`lib.rs`, new `escalation.rs` for the pieces): `NOT_ENOUGH_SIGNAL = "[[ACTX_NEEDS_DEEPER_SEARCH]]"` and `NOT_ENOUGH_RULE`. When the Fast route can escalate (automatic mode, Deep engine present) and the evidence is `Sufficient`, the agent appends the rule to the Fast prompt (after the `<user_query>` block, as a closing instruction). `SignalFilter` takes streamed tokens: it buffers while the trimmed buffer is a prefix of the signal, flushes on the first mismatch, reports `Signal` when the trimmed buffer starts with the signal, and flushes the rest at the end. On `Signal`: no `Delta` was sent; the agent sends `AgentEvent::Escalation { reason: "the quick search excerpts do not answer the question" }`, runs Deep with the same sender and returns its result; nothing is stored for the Fast attempt. `run` checks the full answer the same way. Any later occurrence of the signal is removed from the shown and stored text.
4. **Events and surfaces**: `AgentEvent::RoutingDecision { route: String, pinned: bool, reason: String }`. The TUI and the one-shot output show `⚡ [Quick search]`, `🧠 [Deep search]` or `📊 [Live status]` with `(pinned)` or `(automatic)` and the reason; no domain, no confidence. The escalation line is IMP-046's.
5. **Classifier removal** (`actx-onnx`): `query_classifier.rs` removed with `OnnxQueryClassifier`, `OnnxComplexity`, `OnnxIntent`, `OnnxEngine::query_classifier`, `classify_query` and its summary row. Core `OnnxComplexityClassifierAdapter` removed. `actx-cli/engine.rs` no longer builds a router.
6. **Weights removal**: `bge-small-onnx` removed from `OnnxModelManager::catalog` (13 entries), the `bge`/`query` aliases, `ModelRole::QueryClassifier` if nothing else uses it, the four kits in `hardware/mod.rs`, `actx-installer/src/hardware.rs` and the model menu entry in `tui/menu.rs`. Kit size totals and their tests follow.

## Project Structure

```text
specs/009-evidence-routing/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/actx-agent/src/{router.rs, lib.rs, events.rs, escalation.rs (new)}, tests/{agent_tests.rs, router_tests.rs}
crates/actx-onnx/src/{query_classifier.rs (removed), engine.rs, lib.rs}
crates/any-context-core-rs/src/{ingestion/onnx_adapter.rs, ingestion/model_manager.rs, hardware/mod.rs}
crates/actx-cli/src/{engine.rs, tui/app.rs, tui/menu.rs, cli/oneshot.rs}
crates/actx-installer/src/hardware.rs
docs/adr/0161-*.md, docs/architecture/{agent.md, onnx.md, model-catalog.md, cli.md}, config/SYSTEM_KNOWLEDGE.md, README.md, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-038 (medium), release R1 v1.0.0 Personal, step 2 of the retrieval design, after IMP-046. On completion it is closed. FEAT-082 already owns the removal of the status term list.

### Tests (quality-assurance)

- `router.rs`: every mode and status combination gives the expected route and `pinned`; questions the old classifier sent to Deep route Fast in automatic mode.
- `escalation.rs`: `SignalFilter` with the signal whole, split over tokens, after leading whitespace, a mismatch on the first character, a short answer that ends inside a signal prefix, and the signal in the middle (removed, no escalation).
- Agent (`agent_tests.rs`, mock LLM counting calls and recording prompts): automatic + sufficient + signal → escalation event, Deep answer, no Fast `Delta`, history without the Fast attempt; automatic + sufficient + normal answer → one call, same text; Fast pinned → no rule in the prompt, signal not escalated and removed; automatic without Deep → no rule; Deep pinned → Deep; status → status route in every mode; IMP-046 band escalation unchanged; the same for `run`.
- Catalog and kits: catalog has 13 entries without `bge-small-onnx`; no kit and no installer list names it.
- CLI: TUI and one-shot routing lines.
- Benchmark: CI lexical and dense runs unchanged.
- Manual tests (`tests/MANUAL_TESTS.md`): in `/search auto`, a long comparative question with an answer in one file is answered by the quick search; a question on a covered topic whose fact is missing shows the escalation with "do not answer the question"; `/search fast` never escalates; the model menu has no query classifier.

### Documentation (architecture-docs, user-docs)

- ADR-0161 "Automatic mode routes by evidence: it starts Fast and escalates on the reranker bands or the LLM's not-enough signal; the keyword classifier and its weights are removed" (supersedes the routing part of RFC-042 as described in `agent.md`, amends ADR-0130 and ADR-0118 for the classifier weights).
- `docs/architecture/agent.md` (route, signal, events), `onnx.md` (component list), `model-catalog.md` (catalog table, adapter line), `cli.md` (start-up step 7), `config/SYSTEM_KNOWLEDGE.md` (diagram and kit model lists), README `/search auto` row; TECDOC index regenerated.

### CI, release and observability (devops-release)

- No workflow change; the benchmark runs on these paths.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR. Release note: automatic mode no longer guesses from the question's words; it answers with a quick search and moves to deep research when the quick search finds nothing relevant or the excerpts do not answer the question; hardware kits no longer download the unused 133 MB query classifier.
- Observability: the routing line says whether the mode was pinned; the escalation line gives one of three reasons.

### Security, migration and rollback

- No new input from the user; the signal rule is an instruction in the Fast prompt, and retrieved text stays escaped by the existing formatter. No data migration: downloaded weights are left on disk. Rollback is a revert of the PR.

## Milestones

1. **M1 Route and signal** (T001 to T005): route function, classifier removal, signal filter and escalation, events and surfaces.
2. **M2 Weights removal** (T006): catalog, kits, installer, menu.
3. **M3 Docs and checks** (T007 to T010): ADR-0161, pages, README, manual tests, backlog, full tests.
