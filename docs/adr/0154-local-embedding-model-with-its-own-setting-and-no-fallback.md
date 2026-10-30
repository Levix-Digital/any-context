---
id: 154
title: Local embedding model with its own setting and no fallback
status: accepted
date: '2026-10-30'
version: v1.0.0
subsystems: [ingestion, retrieval, onnx, model-catalog, installer, cli]
supersedes: []
superseded_by: []
amends: [132, 138, 147]
amended_by: []
---

# ADR-0154: Local embedding model with its own setting and no fallback

## Context

Dense search needed a cloud key: without one the sync wrote mock vectors and search fell back to keywords (ADR-0147), which is not RAG and breaks cross-language questions (the benchmark of ADR-0153 measured 0.42 MRR@10 on them). The embedding model followed the chat model (ADR-0132), so `/model` could reindex a workspace, and a provider without a key silently fell back to OpenAI or to the mock. Company machines may sit in a network that cannot reach the public model hosts, and a company may want to choose the model itself. FEAT-067 (local model), IMP-027 (own setting), IMP-028 (no fallback) and IMP-029 (prefixes) answer this; the decisions were taken with Levi on 2026-10-29 (`specs/002-local-embedding/spec.md`, Clarifications).

## Decision

- **Local by default.** `actx-onnx` gets `OnnxEmbedder` (tokenize, `last_hidden_state`, mean or CLS pooling under the attention mask, L2 normalization, batches of 16, truncation counter). The core wraps it in `LocalEmbeddingProvider`, behind the existing `LmProvider::embed` port, so ingestion and retrieval use it like a cloud provider.
- **Catalog.** `ingestion/embedding_catalog.rs` lists every usable model with runtime, dimension, input limit, pooling and prefixes: `multilingual-e5-small`, `multilingual-e5-base` and `bge-m3` (Xenova INT8 exports, MIT, pinned revisions and SHA-256 in `ONNX_CATALOG`), `text-embedding-3-small` (OpenAI), `gemini-embedding-2` (Gemini), `nomic-embed-text` (Ollama), and the test-only mock.
- **Own setting.** `active_embedding_model` resolves a per-run override (tests, benchmark), then the `embedding_model` setting, then, once per installation, the most used real target recorded by the workspaces (an upgrade keeps the cloud model that built the index), then the profile default. `/model` and `-m` never change it. `/embedding [id] [--yes]` and `actx embedding [id] [--yes]` show it and switch it after a confirmation that lists the workspaces to reindex and warns when document text goes to a cloud provider.
- **Profile defaults**, chosen with the benchmark (results in `specs/002-local-embedding/research.md`): `multilingual-e5-small` on Lite, `multilingual-e5-base` on SmartBalanced, `bge-m3` on AirGappedPower, WorkstationPro and SovereignUltra. Each profile provisions its model at install and update; the first sync downloads a missing one. There is no lexical-only mode.
- **No fallback.** When the chosen model cannot run (missing key, failed download, checksum mismatch, missing ONNX Runtime, a model refused by policy), the sync stops before indexing with `EmbeddingUnavailable { model, cause, fix }` and the index stays as it is. Queries use the workspace's recorded target and never another model; when it cannot run, dense search is skipped for the question and the outcome says why (`QueryEmbeddingOutcome::Unavailable`). The pseudo-vector path is removed.
- **Vector-space identity.** `EmbeddingTarget` is provider, model, dimension and prefix revision, keyed `provider/model@dim[+pN]`; legacy `provider/model` keys parse to the space they named, so an equal space is kept. Every chunk records its target in a nullable `embedding_target` column (added like the provenance columns), and dense search filters on it (`embedding_target = key OR embedding_target IS NULL`), so vectors of two models never meet even in the shared 1536 table (per-target tables stay in IMP-033).
- **Prefixes (IMP-029).** The model receives `query: ` / `passage: ` (e5), Google's task prefixes (`gemini-embedding-2`) or `search_query: ` / `search_document: ` (nomic); BGE-M3 and OpenAI take none. Prefixes apply only when the recorded target's prefix revision equals the catalog's, so an index built before them keeps raw queries until its one reindex. Stored text and BM25 never carry them.
- **Model source and governance.** Downloads come only from catalog entries, pinned and verified by SHA-256. `ACTX_MODELS_MIRROR` (or the `models_mirror` setting in the app) replaces the public source with `<mirror>/<file path>` in the app and the installer. Files placed in the models folder are used after their checksum is verified, with no network. `is_model_allowed` is checked before any model is built; it allows the catalog in this release and is the seam for the organization policy (IMP-056, FEAT-143).
- **Verification.** `crates/actx-onnx/tests/embedder_reference.rs` compares the embedder with vectors computed by Python `onnxruntime` 1.31.0 and `tokenizers` on the same file (cosine ≥ 0.995 per text, required in CI). The retrieval benchmark gains a dense configuration with the AirGappedPower default, with its own baseline.

## Alternatives Considered

- **Keep deriving the embedding model from the chat model**: rejected; switching the chat model must never cost a reindex or change where document text goes.
- **Keyword-only mode when no model can run**: rejected by Levi; without semantic search it is not RAG.
- **Fall back to another model or to the mock**: rejected; mixing vector spaces silently degrades answers and can send text to a provider nobody chose.
- **`fastembed` or the `intfloat` ONNX files**: rejected; a second ONNX binding with its own downloads (against the verified catalog), and the `intfloat` INT8 file needs AVX-512 VNNI.
- **Qwen3 embeddings as a candidate**: left out of R1 by Levi.
- **One table per target now (IMP-033)**: deferred; the per-chunk target and the dense filter keep spaces apart without a storage migration.
- **BGE-M3 on every profile above Lite (the benchmark rule read literally)**: not taken for SmartBalanced; its gain over e5-base rests on one or two of six cross-language questions, and on 8 GB machines it costs 2× the disk and indexing time.

## Consequences

- A fresh install answers by meaning with no key and no document text leaving the machine; the first sync of a profile without its model downloads 135 to 586 MB.
- Upgrades keep the cloud model of existing indexes. Gemini indexes are reindexed once (new prefixes); workspaces indexed with the mock are reindexed once with the profile model.
- A sync can now fail before indexing, with a fix in the message, where it used to index with something else.
- INT8 vectors depend slightly on batch composition and CPU kernels (about 0.003 cosine); far below the gaps the benchmark measures.
- CI downloads `multilingual-e5-small` for the workspace tests and `bge-m3` for the dense benchmark, cached by the catalog file.
- Verify with `cargo test --workspace`, the embedder reference test (`ACTX_EMBEDDER_MODELS_DIR`, `ORT_DYLIB_PATH`) and `cargo run -p actx-bench --release -- retrieval run --embedding local/<id>`; manual tests MT-INST-002, MT-SYNC-006, MT-SYNC-007, MT-ANS-010 and MT-MOD-004.
