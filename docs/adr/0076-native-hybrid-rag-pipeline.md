---
id: 76
title: Native Rust Hybrid RAG Pipeline and RFC-042 Batch Retrieval Engine
status: accepted
date: '2026-09-25'
version: v0.30.39
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0076: Native Rust Hybrid RAG Pipeline and RFC-042 Batch Retrieval Engine

## Context

RAG retrieval orchestration in Python suffered from GIL contention and multi-stage data marshaling, and it lacked native concurrent multi-query batch execution for RFC-042.

## Decision

Centralize vector search, BM25 retrieval, RRF fusion, cross-query deduplication, source diversification and density budgeting in `NativeHybridPipeline` in Rust, exposing PyO3 bindings for backward compatibility with Python.

## Alternatives Considered

Not recorded.

## Consequences

- Sub-query retrieval latency reduced by up to 5x through concurrent Tokio execution.
- Zero-copy ranking removes intermediate Python allocations and GIL contention.
- Cross-query deduplication and score accumulation give better grounding for multi-faceted questions.
- Lays the foundation for a full Rust migration that removes Python orchestration entirely.
