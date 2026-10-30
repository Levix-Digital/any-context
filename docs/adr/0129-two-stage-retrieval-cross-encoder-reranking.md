---
id: 129
title: Two-Stage Neural Retrieval with Local ONNX Cross-Encoder Re-Ranking, Hardware Acceleration and Graceful ADR-110 Fallback
status: accepted
date: '2026-10-15'
version: v0.44.0
subsystems: [retrieval, onnx, cli, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: [130, 156, 157]
---

# ADR-0129: Two-Stage Neural Retrieval with Local ONNX Cross-Encoder Re-Ranking, Hardware Acceleration and Graceful ADR-110 Fallback

## Context

- Single-stage retrieval (dense bi-encoders, or RRF fusion of dense and sparse search) compares query and document representations independently. Bi-encoders scale well and are fast for building the initial pool, but they lose syntactic and semantic granularity because they do not perform cross-attention between each query token and the content of each candidate document.
- Cross-encoder re-ranking models process the `(query, document)` pair jointly, layer by layer, so every query word can attend to every document word. This captures negation, adjacency (bigrams) and word order much more precisely (15% to 25% gains in NDCG@10 and MRR).
- Commercial cloud re-rankers add an external connection dependency, per-million-token costs and extra latency (150-400 ms). Local solutions, on the other hand, must not add heavy Python dependencies or break when the user has not downloaded the weights (ADR-110 guarantee).

## Decision

1. **Two-stage retrieval pipeline**:
   - **Stage 1, hybrid recall with RRF**: `NativeHybridPipeline` runs dense vector search in LanceDB and Okapi BM25 lexical search in parallel, producing an initial pool of candidates (k = 30 to 40) balanced through Reciprocal Rank Fusion (k = 60).
   - **Stage 2, neural precision re-ranking**: the local cross-encoder `bge-reranker-small` (`bge_reranker_small_int8.onnx`, about 45.2 MB) receives the Stage 1 candidates and scores deep relevance through bidirectional token-to-token cross-attention, applying a sigmoid to the logits to produce calibrated scores in [0.0, 1.0].
   - **Position in the pipeline**: re-ranking runs right after candidate fusion and before source diversification and `ContextFormatter::u_shaped_reorder`, so the most precise chunks are selected and placed at the Transformer's attention poles.
2. **Domain isolation in `actx-onnx` (`OnnxReranker`)**: implemented in `crates/actx-onnx/src/reranker.rs`, exposing batch scoring (`score_batch`) and single-pair scoring (`score_pair`), integrated with the `ExecutionDevice` hardware acceleration matrix (`Auto`, `DirectML`, `Metal`, `CUDA`, `Vulkan`, `CPU SIMD`).
3. **ADR-110 zero-breakage invariant and graceful fallback**: when `bge_reranker_small_int8.onnx` is missing from `%LOCALAPPDATA%/actx/models/`, `is_neural_active()` returns `false` and the pipeline skips Stage 2 transparently, keeping the Stage 1 RRF order, with no errors, freezes or remote network calls.
4. **Re-ranking modes (`auto`, `on`, `off`)**:
   - `auto` (default): use neural re-ranking when the weights are installed; fall back silently to RRF when they are not.
   - `on`: force neural re-ranking, with a warning and download instructions (`actx download reranker`) when the weights are missing.
   - `off`: disable Stage 2 on purpose and run only Stage 1 (pure RRF) with no CPU/GPU overhead.
5. **Multi-surface parity**:
   - **Headless CLI**: `--rerank <auto|on|off>` flag with the `--reranker` alias (`actx "how does authentication work?" --rerank on`).
   - **Subcommand**: `actx download reranker` (and `/download reranker`) for an assisted download (about 45.2 MB) with a disk space check and real-time progress.
   - **TUI header**: a reactive badge `[Rerank: ⚡ ONNX: bge-small]`, `[Rerank: ⚡ RRF (Fallback)]` or `[Rerank: 🚫 Off]`.
   - **Slash command**: `/rerank [auto|on|off|status]` with atomic persistence in SQLite.

```mermaid
flowchart TD
    QUERY["User query<br/>(Query Text)"] --> STAGE1["Stage 1: Hybrid recall<br/>(Dense LanceDB + Sparse Okapi BM25)"]
    STAGE1 --> RRF["Reciprocal Rank Fusion<br/>(RRF, k=60) -> pool of 30-40 candidates"]

    RRF --> CHECK_MODE{"Rerank mode?<br/>(auto / on / off)"}

    CHECK_MODE -->|off| RRF_PASSTHROUGH["Keep RRF order<br/>(no neural overhead)"]

    CHECK_MODE -->|auto / on| WEIGHTS{"ONNX weights present?<br/>(bge_reranker_small_int8.onnx)"}

    WEIGHTS -->|No (ADR-110)| FALLBACK["Graceful ADR-110 fallback<br/>Keep RRF ranking ($0.00 / no failures)"]

    WEIGHTS -->|Yes| RERANKER["Stage 2: ONNX neural cross-encoder<br/>(bge-reranker-small / DirectML / Metal / CPU SIMD)"]
    RERANKER --> CROSS_ATTN["Bidirectional cross-attention (Query x Doc)<br/>Token match + bigrams + primacy"]
    CROSS_ATTN --> SIGMOID["Sigmoid normalization -> scores [0.0, 1.0]"]
    SIGMOID --> REORDER["Reorder by descending neural score"]

    REORDER --> DIVERSIFY["Source diversification & density budget"]
    FALLBACK --> DIVERSIFY
    RRF_PASSTHROUGH --> DIVERSIFY

    DIVERSIFY --> U_SHAPE["ContextFormatter::u_shaped_reorder<br/>(Anti 'Lost in the Middle')"]
    U_SHAPE --> XML_ISOLATION["Structural XML tag envelope"]
    XML_ISOLATION --> LLM_INFERENCE["Injection into the LLM / assistant context"]
```

## Alternatives Considered

Commercial cloud re-ranking services were considered in the context and rejected because of the external dependency, per-token cost and added latency.

## Consequences

- Substantially better precision and semantic relevance for queries that must tell apart chunks with overlapping terms.
- No cost or network latency from external re-ranking providers.
- No breakage, in strict compliance with ADR-110.
