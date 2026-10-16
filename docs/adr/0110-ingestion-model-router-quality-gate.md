---
id: 110
title: Ingestion ModelRouter, Deterministic Quality Gate and Spatial Form Reconstruction (RFC-043)
status: accepted
date: '2026-10-10'
version: v0.34.2
subsystems: [ingestion, chunkers]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0110: Ingestion ModelRouter, Deterministic Quality Gate and Spatial Form Reconstruction (RFC-043)

## Context

Documents with dense forms, multi-column tables or scanned pages lost their semantic associations in standard RAG pipelines, while files with repetitive delimiters and loose fragments polluted the BM25 and vector rankings with noise.

## Decision

1. Create `crates/any-context-core-rs/src/ingestion/quality_gate.rs` with strictly deterministic mathematical heuristics that run in under 0.1 ms per chunk, discard low-entropy noise and send dense tables to layout handling.
2. Implement `SpatialFormChunker` in `crates/any-context-core-rs/src/ingestion/chunkers/spatial_form.rs` for domain-agnostic 2D typographic extraction, accurate without relying on industry-specific vocabularies.
3. Create `IngestionModelRouter` in `crates/any-context-core-rs/src/ingestion/model_router.rs`, orchestrating the three-level pipeline with `VisionExecutionMode` (`Disabled`, `LocalSlm`, `CorporateVpc`) and a pure Rust fallback (under 5 MB of RAM, no GPU and no heavy model download).
4. Integrate the model router into `NativeSyncOrchestrator`, extending `SyncResult` with auditable metrics: `chunks_dropped_quality`, `documents_enriched_ai` and `quality_pass_rate`.

Later ADRs refer to this pure Rust fallback as the "ADR-110 Zero-Breakage Guarantee".

## Alternatives Considered

Not recorded.

## Consequences

- Structural noise and degenerate fragments are removed from the LanceDB and Okapi BM25 indexes.
- Complex tabular documents and forms are reconstructed accurately, including the critical cases of the `IKEAShipments` benchmark.
- Very light execution, suitable for modest corporate machines (Intel Core i5/i7, 8-16 GB RAM, integrated graphics).
- Unit and integration tests green across the workspace.
