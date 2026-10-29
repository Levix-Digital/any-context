# Research: Local multilingual embedding model (spec 002)

Each entry: decision, rationale, alternatives considered.

## R1. Candidate models and their ONNX files

**Decision**: INT8 ONNX exports with a pinned revision:

| Model | Repository and revision | File | Size | Dim | Max tokens | Pooling | Prefixes |
|---|---|---|---|---|---|---|---|
| multilingual-e5-small | `Xenova/multilingual-e5-small` @ `761b726d` | `onnx/model_int8.onnx` + `tokenizer.json` | 118 MB + 17 MB | 384 | 512 | mean, L2 | `query: ` / `passage: ` |
| multilingual-e5-base | `Xenova/multilingual-e5-base` @ `1ec92430` | `onnx/model_int8.onnx` + `tokenizer.json` | 278 MB + 17 MB | 768 | 512 | mean, L2 | `query: ` / `passage: ` |
| BGE-M3 (dense) | `Xenova/bge-m3` @ `4de13258` | `onnx/model_int8.onnx` + `tokenizer.json` | 568 MB + 17 MB | 1024 | 8192 (capped at 512 here) | CLS, L2 | none |

The SHA-256 of each file comes from the Hugging Face LFS metadata and is checked by the existing verified download (`OnnxModelManager::download_model`). Upstream licenses: MIT for the three (the Xenova repositories are ONNX conversions of the MIT originals; the license of the original applies and is recorded in the catalog).

**Rationale**: INT8 keeps the profiles' disk and RAM budgets (the 8 GB profile already carries the reranker and Document AI weights). The exports are single-file graphs (no external `.onnx_data`), which the current loader handles. The same conversion family (Xenova / Optimum) produces the `last_hidden_state` output the pooling needs.

**Alternatives**: the `intfloat` repositories' own ONNX files (`model_qint8_avx512_vnni.onnx` needs AVX-512 VNNI, not portable; `model.onnx` FP32 is 470 MB for the small model); `onnx-community/bge-m3-ONNX` (same size, alternative if the Xenova export fails); FP32 exports, measured only if INT8 loses quality (spec assumption).

## R2. Where the local embedding runs

**Decision**: a new `actx-onnx` module `embedder.rs` with `OnnxEmbedder` (lazy load, `OnceLock` state, batches, pooling and normalization from its config), built like `OnnxReranker`; a thin adapter in the core implements the existing `LmProvider::embed` port so ingestion and retrieval use it exactly like the cloud providers.

**Rationale**: `actx-onnx` already owns ONNX Runtime (`runtime::OnnxSession`), tokenizers and device selection; the reranker proves the loading, error recording and dynamic library pattern (`load-dynamic`, `ORT_DYLIB_PATH`). Reusing the `LmProvider::embed` port keeps the orchestrator, the pipeline and `embed_with_retry` unchanged in shape.

**Alternatives**: a new `EmbeddingPort` trait in the core (cleaner, but every caller already speaks `LmProvider`; deferred, Simplicity First); `fastembed` crate (adds a second ONNX Runtime binding and its own model download, against FR-014).

## R3. Pooling and inputs

**Decision**: inputs `input_ids` and `attention_mask` (plus `token_type_ids` only when the graph declares it); output `last_hidden_state`; mean pooling over the attention mask (e5) or the first token (BGE-M3), then L2 normalization. Truncation at the catalog's max tokens (512), counting truncated inputs.

**Rationale**: these are the reference poolings of the model cards (sentence-transformers configs). Verification during implementation: for a fixed set of pt/en sentences, the Rust vectors must match a Python reference computed with `onnxruntime` + `tokenizers` on the same file to within 1e-4 cosine (test fixture committed as expected vectors for a few short sentences).

## R4. The embedding setting and the vector-space identity

**Decision**: an application-wide setting `embedding_model` in `settings.db` holding a catalog id; the `EmbeddingTarget` gains `dimension` and `prefix_scheme` (a short revision tag of the model's prefixes in the catalog) and its key becomes `provider/model@dim[+prefix-rev]`, e.g. `local/multilingual-e5-small@384+p1`, `gemini/gemini-embedding-2@1536+p1`, `openai/text-embedding-3-small@1536`. `EmbeddingTarget::parse` keeps reading the old `provider/model` keys (recorded before this feature) and maps them to their legacy identity.

**Rationale**: the reindex rule (ADR-0138) compares keys; putting the dimension and prefixes in the key makes any change of space trigger it (FR-008) without a new mechanism. Old keys stay parseable so the upgrade can adopt them (FR-006).

**Alternatives**: commit `4042b89` used `<model>#<dimension>` (no provider, no prefixes; a prefix change would go unnoticed); a separate "space revision" column (more storage, same effect).

## R5. Resolution without fallbacks

**Decision**: `resolve_embedding(db) -> Result<Option<ResolvedEmbedding>, EmbeddingUnavailable>`:
- setting present: build that model or return `EmbeddingUnavailable { model, cause, fix }` (missing key, missing or unverified file, ONNX Runtime not loadable);
- setting absent: the active profile's default local model (every profile has one; `Lite` uses e5-small);
- before building: the single `is_model_allowed(id)` check (FR-019), which allows the whole catalog in this release;
- never another model, never the mock (FR-010, FR-011). The mock is built only by test code through an explicit constructor (`embedding::testing::mock()`), and by the benchmark's lexical-only mode, which asks for no embedding at all.

The query side keeps ADR-0138's rule (the workspace's recorded target answers until the reindex), but with no fallback to "current" when the recorded one cannot be built: then dense search is skipped for that question and the failure is logged (spec story 3, scenario 3).

**Rationale**: matches design section 8 ("never switches model silently") and keeps today's behavior during a model switch.

## R6. Upgrade path (FR-006 and the edge cases)

**Decision**: on first open after the upgrade, when `embedding_model` is absent, a one-time migration reads the recorded workspace targets: if one or more workspaces hold a real (non-mock) target, the most used one becomes the setting (adopted, no reindex); else the setting stays absent (profile default). Workspaces recorded with the mock target, or with a target different from the adopted one, get the normal reindex on their next sync. The migration is recorded so it never runs twice.

**Rationale**: no surprise reindex of a working cloud index; mock indexes (no meaning) move to the local model. Pending Levi's answer to clarification 3.

## R7. Per-chunk record and dense filtering

**Decision**: a nullable `embedding_target` column in the LanceDB chunks table, added on open like the provenance columns (`ensure_provenance_columns` pattern); written on every upsert; existing rows are backfilled from the workspace's recorded target by the same migration as R6. `search_vector` takes the query target and filters `embedding_target = '<key>'` inside the search.

**Rationale**: FR-009; keeps vector spaces apart in the single shared table until per-target tables (IMP-033). The reindex still runs per workspace (ADR-0138); the file-by-file re-embedding of `4042b89` is not ported (it belongs with per-target tables).

## R8. Prefixes (IMP-029)

**Decision**: the catalog entry carries `query_prefix`, `document_prefix` and `prefix_rev`; a helper `apply_prefix(role, text)` is called by the orchestrator right before `embed_with_retry` and by the pipeline right before the question embedding. Values: e5 `query: ` / `passage: `; `gemini-embedding-2` `task: search result | query: ` / `title: none | text: ` (the old IMP-014 note); `nomic-embed-text` `search_query: ` / `search_document: `; BGE-M3 and OpenAI none.

**Rationale**: one place for the strings, both sides symmetric; the stored text and BM25 never see them (FR-012). Changing Gemini's prefixes changes its key (`+p1`), so existing Gemini workspaces are reindexed once; this is announced in the release notes.

## R9. Profiles and the kit

**Decision**: a new catalog category `Embedding`; each local profile's `target_models` lists its default embedding model (proposal: e5-small for `Lite` and `SmartBalanced`, e5-base for `AirGappedPower`, BGE-M3 for `WorkstationPro` and `SovereignUltra`, final after R11); `ProfileTier::default_embedding_model()` returns it for every profile. A missing model at sync time is downloaded through the verified download with progress (decided with Levi on 2026-10-29).

## R9b. Model source and governance seam (decided with Levi on 2026-10-29)

**Decision**: `OnnxModelManager` resolves each file's URL through a model source: the catalog's pinned upstream URL by default; when a mirror base URL is set (`models_mirror` in `settings.db`, overridden by `ACTX_MODELS_MIRROR`), the URL becomes `<mirror>/<catalog file_name>` and the upstream is never contacted. Files already present in the models directory are used when their SHA-256 matches, with no network. The checksum check is the same in all three cases. Before building any model, the resolution calls `is_model_allowed(id)`, which allows every catalog model now; IMP-056 (organization policy) replaces its body, and FEAT-143 (Admin Hub mirror) sets the mirror.

**Rationale**: a company network may block Hugging Face or require its own approved artifacts; R1 must not hard-code the public host, but the full governance (signed policy, allowed models, Admin Hub) is v2.0.0. Models outside the catalog are not loadable in R1 (the catalog pins every checksum).

**Alternatives**: no mirror in R1 (blocks corporate pilots behind a proxy); free-form model files without checksums (breaks FR-014 and supply-chain safety).

## R10. CLI and TUI surface

**Decision**: slash command `/embedding [id]` (show; or pick with a reindex warning and, for cloud models, the data notice) and headless `actx embedding [id] [--yes]`. `/model` and `-m/--model` stop affecting embeddings; `SyncOptions::model` no longer feeds the embedding resolution. `/models` lists the embedding models with their category.

## R11. Choosing the defaults with the benchmark

**Decision**: `actx-bench retrieval run --embedding local/<id>` uses the product's resolution with that model id (no keys). The developer runs the three candidates (CI `workflow_dispatch` matrix job, models cached by SHA-256), and the comparison table (MRR@10, HitRate@5, cross-language MRR, indexing time, model size, peak RSS) goes into this file under "Results". Rule from the spec: the bigger model wins a profile only with more than 0.01 gain in MRR@10 or HitRate@5 (overall or cross-language).

## R12. CI

**Decision**: `retrieval-benchmark.yml` gains a second job configuration with dense on (default model of `AirGappedPower`, the reference profile), its own baseline `baseline.dense-local-<id>-rerank.json` and the same gate; the lexical-only job stays (it is cheaper and isolates lexical regressions). The model files are cached by the catalog file, like the reranker. Paths watched: unchanged (they already include `crates/actx-onnx/**`).

## Results

To be filled during M5 with the candidate comparison.
