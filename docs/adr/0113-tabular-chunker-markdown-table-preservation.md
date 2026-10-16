---
id: 113
title: Removal of Heuristic Inversion in the Tabular Chunker and Unconditional Preservation of 2D Markdown Formats
status: accepted
date: '2026-10-10'
version: v0.34.5
subsystems: [ingestion, chunkers, retrieval]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0113: Removal of Heuristic Inversion in the Tabular Chunker and Unconditional Preservation of 2D Markdown Formats

## Context

A user reported that forms such as T4 and CMR were indexed with absurd key-value pairs (`58792.60: 7976.60`), which prevented factual RAG answers.

## Decision

1. **Markdown tables preserved by the QualityGate**: table blocks that already have header delimiters are exempt from the `DenseComplexForm` classification.
2. **Conservative pairing in forms**: field detection in tables is restricted to strictly declarative headers (`Key | Value`, `Field | Value`, and so on).
3. **RRF threshold calibration in the CLI engine**: lower `min_score` in `HybridSearchRequest` from `0.02` to `0.005` so multilingual semantic queries pass the RRF ranking cutoff.
4. **Defensive runtime decryption**: BM25 queries and candidate assembly in the hybrid pipeline transparently unwrap `enc::` texts.

## Alternatives Considered

Not recorded.

## Consequences

- RAG accuracy restored on tax documents, invoices and logistics reports.
- The T4 tax form returns the exact values for employment income ($58,792.60) and income tax withheld ($7,976.60).
- Existing databases stay compatible after `actx sync --force`.
