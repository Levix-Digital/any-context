---
id: 139
title: Retrieval order follows relevance
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: [76, 116]
amended_by: [156]
---

# ADR-0139: Retrieval order follows relevance

## Context

`NativeHybridPipeline` ranks candidates by relevance: the cross-encoder score, or the Reciprocal Rank Fusion (RRF) score when re-ranking does not run. Two later steps changed that order:

- **Source round-robin** ([ADR-0076](0076-native-hybrid-rag-pipeline.md)): `apply_source_diversification` took the first chunk of every source, then the second of every source, and so on, up to `max_chunks_per_source`. The output followed that pass order, not the score order. `apply_density_budget` cuts the tail of the list, and `ContextFormatter::u_shaped_reorder` places input rank 2 next to the query ([ADR-0128](0128-backend-first-deterministic-dual-rag.md)), so a weak chunk from another source could sit next to the query while the second-best chunk landed in the middle or was cut. The round-robin only reordered candidates that had already been retrieved, so it did not widen document coverage (BUG-004).
- **Recency micro-boost** ([ADR-0116](0116-path-healing-temporal-metadata-change-detection.md)): `0.01 / (1 + days_old / 30)` was added to every RRF score to break ties in favor of newer versions. The best single-list RRF score is 1/61 ≈ 0.0164, so the boost was a ranking signal rather than a tie-breaker. A file modified today at BM25 rank 30 (0.0211) outranked a year-old file at rank 1 (0.0172). It also changed which candidates passed `min_score` and reached the cross-encoder. All chunks of a source share one `last_modified`, so the boost only acted across sources (BUG-005).
- **Deep Search threshold**: Deep Search sent `min_score: 0.02`, above the single-list maximum, so a chunk found only by the dense search or only by BM25 passed only when the boost lifted it (BUG-006). The other callers use 0.005, which no candidate of their pools falls below.

## Decision

The relevance score is the only ranking signal, and retrieval always returns chunks in descending score order. In `crates/any-context-core-rs/src/retrieval/`:

- **Per-source cap in score order**: `diversifier::apply_source_cap(chunks, max_per_source, target_k)` replaces `apply_source_diversification`. It walks the score-ordered candidates and skips a chunk whose source already has `max_per_source` selected chunks. When fewer than `target_k` are selected, the skipped chunks fill the remaining slots in score order. The cap gives other sources priority but never leaves a slot empty. The selection keeps the input order, and `max_per_source = 0` means no cap. Both `search_single_async` and `retrieve_hybrid_batch_async` use it.
- **Recency as a tie-breaker**: the micro-boost is removed, and scores are plain RRF (or cross-encoder) scores. `pipeline::compare_by_relevance` orders by score, then newer `last_modified` first on an exact tie (RFC 3339; missing or invalid dates last), then keeps the existing order. It sorts the fused candidates before `min_score` and the re-ranking cut, the final results of `search_single_async`, and the merged candidates of `retrieve_hybrid_batch_async`.
- **RRF floor**: `pipeline::RRF_MIN_SCORE = 0.005` is the `min_score` of every caller in `actx-cli/src/engine.rs` (the `search_db` tool, the deterministic retriever and Deep Search). It is 1/(60 + 140), so it keeps every candidate of a pool of up to 140 per list and only guards against empty matches. It is not a relevance cutoff. Deep Search builds its request in `deep_search_request`.

## Alternatives Considered

- **Keep the round-robin and re-sort its output by score**: rejected. The order would be right, but a weak chunk from another source would still displace a strong second chunk of the same source.
- **Hard per-source cap**: rejected. A workspace with one or two relevant files would get fewer chunks than `top_k` while relevant candidates were available.
- **Smaller or multiplicative recency boost**: rejected. It would still be an uncalibrated ranking signal, and there is no evaluation set yet (IMP-008) to tune it.
- **No recency at all**: rejected. On exact ties, the newer version of a document should win, as ADR-0116 intended.
- **Remove BUG-005 and keep Deep Search at 0.02**: rejected. Without the boost, Deep Search would drop every single-list hit.

## Consequences

- The density budget always cuts the weakest chunks, and the U-shaped order places the two best chunks at the edges of the context.
- One source can still fill up to `max_chunks_per_source` slots ahead of weaker sources, and more when no other source has candidates. The cap value is revisited once hierarchical chunking (Frente D) and an evaluation (IMP-008) exist.
- Recency no longer favors recent files that are less relevant. Questions about recent content rely on the temporal clauses of `QueryPreprocessor`.
- Deep Search keeps more, weaker candidates before its `top_k` cut. The cross-encoder, the per-source cap and the density budget still bound the context. A calibrated relevance cutoff on cross-encoder scores is IMP-007.
- Verified by the unit tests of `apply_source_cap` and `compare_by_relevance`, the BM25-only ordering and recency tests in `pipeline.rs`, and the Deep Search request test in `engine.rs`.
