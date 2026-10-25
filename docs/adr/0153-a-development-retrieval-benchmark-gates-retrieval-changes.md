---
id: 153
title: A development retrieval benchmark gates retrieval changes
status: accepted
date: '2026-10-25'
version: v1.0.0
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0153: A development retrieval benchmark gates retrieval changes

## Context

The R1 retrieval rework (IMP-040 to IMP-046) and several R1 foundations (token-sized chunks IMP-023, the contextual header IMP-026, the embedding model IMP-027 and FEAT-067, per-target tables IMP-033) change what retrieval returns. `docs/architecture/retrieval-pipeline-v2.md` says their defaults are tuned by an evaluation set, but the per-workspace set (FEAT-073, FEAT-106) only arrives in v1.1.0. Until now no change to retrieval could show its effect on quality (IMP-008). The product's request defaults lived in the CLI adapter (`actx-cli/src/engine.rs`), so a benchmark could not call them without copying the numbers. Equal scores were ordered by hash map iteration in BM25 and in the final sort, so the same question could rank differently between runs.

## Decision

- A development benchmark, the `actx-bench` workspace crate (`publish = false`, not built by `release.yml`), indexes the fixed corpus in `benchmarks/retrieval/corpus` (fictitious Portuguese and English documents, the fictitious CMR form) with the real `NativeSyncOrchestrator` in a sandbox, and runs the labeled questions of `benchmarks/retrieval/questions.json` through `NativeHybridPipeline`.
- The sandbox redirects every default path (`ACTX_SETTINGS_DB`, `ACTX_CONTEXT_DB`, `ACTX_MODELS_DIR`, `ACTX_TEST_MODE`), so a run never reads or writes the developer's workspaces, settings or keys.
- The Fast and Deep requests move into the core as `retrieval::fast_request` and `retrieval::deep_request`; the CLI and the benchmark both call them. Each question runs a ranking pass (the Fast request with `top_k` 10 and no density cut) for HitRate@1/3/5/10, MRR@10 and NDCG@10, and a delivered pass (the Fast request as sent) for the delivered hit rate, the nothing-found rates and latency. `Global` is not searched.
- Results are JSON with the run configuration (corpus and question file hashes, retrievers, models, revision). A run is compared with `benchmarks/retrieval/baseline.<config_id>.json` of the same configuration and fails when MRR@10 or HitRate@5 drops more than 0.02. Latency is reported, never gated. The baseline changes only with `--save-baseline`.
- CI (`retrieval-benchmark.yml`) runs the `lexical-rerank` configuration (mock embedding target, so dense search is skipped, plus the verified cross-encoder) on changes to retrieval, ingestion, storage, the ONNX crate or the benchmark. Dense joins CI with the local embedding model (FEAT-067), as a configuration with its own baseline.
- Ties are broken by chunk id, then file path, in `compare_by_relevance`, and by chunk id in `BM25Index::search`, so equal scores always rank the same.

## Alternatives Considered

- **Wait for the per-workspace evaluation set (FEAT-073)**: rejected; the R1 changes that need measurement would ship before it.
- **Local embedding model (FEAT-067) first, then the benchmark**: rejected by Levi on 2026-10-25; the model must be chosen with the benchmark.
- **An integration test in the core**: rejected; `cargo test --workspace` would download the reranker on every developer's run.
- **Copy the request defaults into the benchmark**: rejected; a change to the product's defaults would not move the benchmark.
- **Commit precomputed embeddings to measure dense in CI now**: rejected; they break whenever chunking changes.

## Consequences

- Each pull request that touches retrieval, ingestion or storage shows the comparison in its CI summary; accepting a quality change means committing the new baseline in the same pull request, where its diff shows the effect.
- Editing the corpus or the questions changes their hash, so the baseline must be saved again.
- Until FEAT-067, cross-language questions mostly fail in CI (no dense retrieval); that is the expected baseline, not a defect of the benchmark.
- Verify with `cargo test -p actx-bench` and `cargo run -p actx-bench --release -- retrieval run`; the procedure is in `docs/architecture/retrieval.md`, section Benchmark.
