# Implementation Plan: Nothing-found and weak-evidence gate by relevance bands

**Branch**: `feat/008-nothing-found-gate` | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/008-nothing-found-gate/spec.md`

## Summary

The answer path reads the evidence of a retrieval from the reranker bands of the delivered chunks: nothing, weak (maybe band only) or sufficient. Nothing gives the nothing-found answer without an LLM call, in Fast and now also in Deep (which today always synthesizes). Weak gives an answer that starts with a fixed warning, and the prompt tells the LLM the excerpts are only possibly related. In automatic mode, a Fast route with nothing or weak evidence escalates to Deep with an indicator. The score cut of 0.005 of the old gate goes away. Without bands (no reranker), only an empty selection is judged.

## Technical Context

**Language/Version**: Rust (stable, edition of the workspace)

**Primary Dependencies**: existing only; no new crate.

**Storage**: none changed.

**Testing**: `cargo test` per crate (`actx-agent` with mock LLM and retrievers, `any-context-core-rs`, `actx-cli`, `actx-bench`); the CI benchmark runs gate the unchanged retrieval metrics.

**Target Platform**: Windows, Linux, macOS desktop; `ubuntu-latest` in CI

**Project Type**: desktop CLI/TUI application with a Rust core library

**Performance Goals**: a nothing-found answer makes no answer LLM call (it gets cheaper); an escalation adds the Deep run that automatic mode would have made for a Deep route.

**Constraints**: no reindex; retrieval output unchanged (benchmark ranking and delivered metrics unchanged).

**Scale/Scope**: `actx-agent` (`lib.rs`, `deep_search.rs`, `events.rs`), core `retrieval/` (new `gate.rs`, `prompt.rs`, `mod.rs`), `actx-cli` (`engine.rs`, TUI `app.rs`, `cli/oneshot.rs`), `actx-bench` (metrics, summary, compare).

## Constitution Check

| Principle | Check | Result |
|---|---|---|
| I. Design: modularity, explicit errors, testability | Evidence classification is a pure function in the core; the agent decides the route from a typed output | Pass |
| II. Simplicity First | Three evidence states, one warning, one message; no new configuration | Pass |
| III. Hexagonal | Bands are read in the core and the CLI adapter; the agent only sees `Evidence` and `RetrievedChunk::weak` | Pass |
| IV.2 Evidence | Agent tests with mock LLM and retrievers for every route; core unit tests; benchmark rates | Pass |
| IV.3 Documentation describes today | `retrieval.md`, `agent.md`, README `/search auto` edited in place; ADR-0160 | Pass |
| IV.6 Security | No new input; fewer LLM calls; retrieved text stays escaped | Pass |
| IV.7 Bugs first | Backlog has no open bugs | Pass |
| IV.9 No AI attribution | Plan, commits and PR authored by the repository identity | Pass |

## Design

1. **Evidence** (`actx-agent`, `lib.rs`): `pub enum Evidence { Sufficient, Weak }`. `DeterministicRetrievalOutput::ContextReady` gains `evidence: Evidence`; `ZeroContext` stays the nothing-found case. `RetrievedChunk` gains `weak: bool` (serde default `false`): the reranker put the chunk in the maybe band. `fn evidence_of(chunks: &[RetrievedChunk]) -> Option<Evidence>` (`None` when empty) in `deep_search.rs`.
2. **Core gate** (new `retrieval/gate.rs`): `classify_evidence(results: &[HybridSearchResult]) -> EvidenceClass { Nothing, Weak, Sufficient }`: weak when non-empty and every chunk has `relevance_band == Some(Maybe)`. `nothing_found_message(workspace, folders, urls, query)` replaces `ContextFormatter::evaluate_zero_hallucination` (no score cut). `WEAK_EVIDENCE_WARNING` (the user-facing line) and `WEAK_EVIDENCE_RULE` (the prompt rule) are constants here.
3. **Prompts** (`prompt.rs`): `format_deterministic_prompt` and `format_deep_search_prompt` add `WEAK_EVIDENCE_RULE` after the isolation rules when `classify_evidence` is weak. `CanonicalSynthesisFormatter` maps `RetrievedChunk::weak` to the maybe band so the synthesis prompt gets the same rule.
4. **CLI adapters** (`engine.rs`): `PipelineDeterministicRetriever` returns `ZeroContext` with `nothing_found_message` on `Nothing`, else `ContextReady` with `Evidence::Weak` or `Sufficient`. `PipelineBatchRetriever` sets `weak` from each result's band and implements the new `DeepSearchRetriever::nothing_found_message(query)` (default: a generic message) with the same text as Fast. The status retriever always returns `Sufficient`. The `search_db` tool uses `nothing_found_message` on an empty result.
5. **Agent routes** (`lib.rs`): after the Fast retrieval, `escalates(output)` is true when the search mode is `Auto`, a Deep orchestrator is configured, the route is not the status route and the output is `ZeroContext` or `Weak`. Then the agent sends `AgentEvent::Escalation { reason }` and runs Deep (in `run` and `stream`). Otherwise `ZeroContext` answers without the LLM as today, and `Weak` sends `AgentEvent::Caveat(WEAK)` before the streamed answer; `run` returns the warning, a blank line and the answer. The session keeps the answer without the warning.
6. **Deep** (`deep_search.rs`): after the turns, empty `all_accepted_chunks` → the retriever's nothing-found message, stored like Fast's, no synthesis call; weak → `Caveat` before the synthesis stream and the warning before the returned answer. The warning text is passed in by the agent crate constant `WEAK_EVIDENCE_WARNING` (the core re-exports it), so both paths say the same.
7. **Events and surfaces**: `AgentEvent::Escalation { reason }` and `AgentEvent::Caveat(String)`. The TUI shows the escalation in the thinking area (`↗ [Escalated to Deep Search]: <reason>`) and the caveat as a system line `⚠ <text>` before the answer; the one-shot CLI prints both before the answer.
8. **Benchmark** (`actx-bench`): `QuestionResult::weak_evidence` (delivered non-empty and all maybe); `Metrics::weak_evidence_rate` (negatives) and `positive_weak_evidence_rate`; summary and compare rows, reported only.

## Project Structure

```text
specs/008-nothing-found-gate/   spec.md, plan.md, research.md, tasks.md, checklists/requirements.md
crates/actx-agent/src/{lib.rs, deep_search.rs, events.rs}, tests/{agent_tests.rs, deep_search_tests.rs}
crates/any-context-core-rs/src/retrieval/{gate.rs (new), prompt.rs, mod.rs}, src/lib.rs
crates/actx-cli/src/{engine.rs, tui/app.rs, cli/oneshot.rs}
crates/actx-bench/src/{metrics.rs, summary, compare.rs}
docs/adr/0160-*.md, docs/architecture/{retrieval.md, agent.md}, README.md, tests/MANUAL_TESTS.md
```

## Complexity Tracking

None.

## Dev Cycle Protocol

### Backlog items and release

IMP-046 (medium), release R1 v1.0.0 Personal, step 7 item 5 of the retrieval design, after IMP-045. On completion it is closed. IMP-038 keeps the removal of the keyword classifier and the LLM-says-insufficient trigger; FEAT-088 adds `@source` to the nothing-found suggestions (noted in its description); IMP-050 moves the warning into the streamed answer.

### Tests (quality-assurance)

- `gate.rs`: empty → nothing; all maybe → weak; one relevant, one pinned (no band) or all band-less → sufficient; message lists sources and suggestions and has no score cut.
- `prompt.rs`: weak rule present only for weak results, in Fast and Deep prompts.
- Agent (`agent_tests.rs`, mock LLM counting calls): Fast pinned nothing → no LLM call; Fast pinned weak → caveat first, one call, history without the warning; auto Fast nothing and weak → escalation event and Deep answer; auto without Deep → Fast answer; status route never escalates; band-less → no caveat, no escalation.
- Deep (`deep_search_tests.rs`): no chunks → nothing-found, no synthesis call; all weak → caveat before the first delta; one strong chunk → no caveat.
- CLI: TUI shows caveat and escalation lines; one-shot prints them.
- Benchmark: weak-evidence rates on fixtures; CI lexical and dense runs unchanged on the gated metrics.
- Manual tests (`tests/MANUAL_TESTS.md`): a question outside the workspace in `/fast` gets the nothing-found answer at once; in `/search auto` it shows the escalation and then the nothing-found answer; a loosely related question shows the weak-evidence warning first.

### Documentation (architecture-docs, user-docs)

- ADR-0160 "The answer path reads the evidence from the reranker bands: nothing found without the LLM, a weak-evidence warning, and automatic Fast escalates to Deep" (amends ADR-0128's Zero Hallucination Gate score cut).
- `docs/architecture/retrieval.md` (gate), `docs/architecture/agent.md` (routes, events), README `/search auto` row and troubleshooting line; TECDOC index regenerated.

### CI, release and observability (devops-release)

- No workflow change; the benchmark runs on these paths.
- Version impact: behavior inside R1 v1.0.0; no bump in this PR. Release note: questions with no relevant excerpt get an honest "nothing found" without spending an LLM call, in Fast and Deep; answers built only on loosely related excerpts start with a warning; automatic mode moves to deep research when a quick search finds nothing relevant.
- Observability: the escalation and caveat events reach the TUI and the one-shot output; the benchmark reports the weak-evidence rates.

### Security, migration and rollback

- No new input; no data migration. `RetrievedChunk::weak` has a serde default, so stored or serialized chunks still load. Rollback is a revert of the PR.

## Milestones

1. **M1 Core and agent** (T001 to T006): gate module, prompts, agent routes, Deep gate, CLI adapters and surfaces.
2. **M2 Benchmark** (T007): weak-evidence rates.
3. **M3 Docs and checks** (T008 to T011): ADR-0160, pages, README, manual tests, backlog, full tests.
