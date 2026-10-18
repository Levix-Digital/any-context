---
id: 108
title: Reflective Agentic Deep Search Engine and Adaptive Routing (RFC-042)
status: accepted
date: '2026-10-09'
version: v0.34.0
subsystems: [agent, retrieval, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: [141, 143]
---

# ADR-0108: Reflective Agentic Deep Search Engine and Adaptive Routing (RFC-042)

## Context

Complex questions over corporate documentation often fail with single-turn RAG: multi-faceted questions need iterative investigation, term refinement and synthesis of evidence spread across many modules and files.

## Decision

1. Implement `DeepSearchOrchestrator` in `crates/actx-agent/src/deep_search.rs` with four stages: orthogonal decomposition, batched hybrid retrieval, gap analysis (reflection) and final synthesis.
2. Connect the async `DeepSearchRetriever` trait to the existing `NativeHybridPipeline` (LanceDB + Okapi BM25 + RRF with k = 60), keeping content-hash deduplication and exact line provenance.
3. Add typed reactive events (`AgentEvent::Decomposition`, `AgentEvent::IterationStart`, `AgentEvent::GapAnalysis`) so the TUI and CLI show the progress of the investigation in real time.
4. Provide the `/deep` and `/fast` shortcuts and the persisted `/search` setting on every command surface.

## Alternatives Considered

Not recorded.

## Consequences

- Much richer, more precise and better grounded answers to complex architectural questions.
- Fewer hallucinations, by discarding unrelated chunks and citing exact files and line ranges.
- Latency stays low thanks to Rust parallelism and early stopping once the accumulated evidence is sufficient.
- Unit and integration tests green across the workspace.
