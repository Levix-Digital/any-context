---
plan_id: anycontext_roadmap_pos_v0450_proximas_frentes
project: any-context
title: 'Post-v0.45.0 Roadmap: Next Development Fronts'
version: v0.46.0
status: active
author: 'Claude (cloud session, PR #328)'
created_at: '2026-10-16T10:45:40'
updated_at: '2026-10-16T10:52:10'
session_id: null
tags:
- roadmap
- frente_c
- frente_d
- backlog
- handoff
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: Front C (v0.46.0)
  done: false
- name: Front D (v0.47.0)
  done: false
status_log:
- at: '2026-10-16T10:45:40'
  by: Claude (cloud session)
  status: active
  note: 'Roadmap handoff added in PR #328'
- at: '2026-10-16T10:52:10'
  by: Levi Silveira
  note: Frontmatter added (plan-memory shared plan format)
---

# 🗺️ Post-v0.45.0 Roadmap: Next Development Fronts

- **Date**: 2026-10-16
- **Starting state**: `v0.45.0` published (tag `v0.45.0` → `1daebe4`, merge of PR #326 into `main`; review fixes in PR #327).
- **Purpose of this document**: a handoff between sessions. It summarizes what is delivered, what comes in each front, where the relevant code points are and which conventions to follow. **It is not a blueprint**: each front gets its own blueprint in `docs/plans/` before implementation.
- **Sources**: ADR-130 (`TECDOC.md`, including 130.10), `docs/plans/anycontext_blueprint_v0450_frente_c0_runtime_onnx_real_e_reranker_cross_encoder.md` (revised roadmap, D14 and "Out of scope"), `docs/plans/anycontext_master_roadmap_v0_30_38_rfc042.md` (Phase 3 "Model Kit").

> Items marked **(to verify)** were not confirmed in the code or an external source when this document was written. Confirm them before turning them into a blueprint (ADR-118 rule: no catalog entry or link without live verification).

---

## 1. Where we are

| Front | Version | State | Delivery |
|---|---|---|---|
| A | v0.43.0 | ✅ | Deterministic 1-hop RAG, U-shaped reordering, XML isolation, Zero Hallucination Gate |
| B | v0.44.0 | ✅ (actually completed by C0) | Two-Stage pipeline, `/rerank`, `--rerank`, badges |
| C0 | v0.45.0 | ✅ published | Real ONNX runtime (`ort` load-dynamic + `tokenizers`), real `mmarco-reranker` cross-encoder on CPU, verified catalog (SHA-256), honesty rectification (Gemma 4, GPU and heuristics marked as planned) |
| **C** | **v0.46.0** | 🔜 next | Local ONNX embeddings + Model Kit |
| **D** | **v0.47.0** | planned | Hierarchical chunking / Parent-Document / AST expansion |
| No version | — | backlog | Local generative Gemma 4, GPU EPs, heuristics → neural, macOS, cleanups |

Technical base C0 left ready for reuse: `actx_onnx::runtime::OnnxSession` (lazy session, dynamic loading, no panic), `actx_onnx::tokenizer` (HF `tokenizers`), a multi-file downloader with SHA-256 (`OnnxModelManager::download_model_with_progress`), provisioning in the installer (`crates/actx-installer/src/provision.rs`) and the ADR-110 graceful fallback pattern.

---

## 2. Front C — v0.46.0: Local ONNX Embeddings + Model Kit

### 2.1 Problem
1. **Dense search depends on an external embedding provider.** Ingestion resolves the model per workspace (`crates/any-context-core-rs/src/ingestion/orchestrator.rs:198`, `resolve_embedding_provider`), but the query uses a fixed model:
   - `crates/any-context-core-rs/src/retrieval/pipeline.rs:536` → `lm.embed("text-embedding-3-small", …)`.
   - **Bug**: if the workspace was indexed with another embedding model, the query vector lands in another space (or has another dimension) and dense search degrades or fails silently. Fix it in Front C, or earlier as a `v0.45.1` patch.
2. **Fixed dimension in LanceDB**: `crates/any-context-core-rs/src/storage/lancedb.rs:19` → `DEFAULT_VECTOR_DIM = 1536`. Compact local embeddings have another dimension (typically 384 or 768).
3. **Two model catalogs** (Core `model_manager.rs` and Installer `hardware.rs`), duplicated, protected only by a parity test.
4. **Hardware profiles** calibrated for Gemma 4 (RAM), which does not run locally. Disk was already fixed in PR #327 (sum of the weights actually installed), but RAM and the tier semantics were not.

### 2.2 Goals
1. **`OnnxEmbedder`** in `crates/actx-onnx` (new `embedder.rs`), reusing `OnnxSession` + `tokenizer`: mean pooling + L2 normalization, batches, `spawn_blocking`, a lazy session and an honest `is_neural_active()` (the same semantics as the reranker, including `recorded_load_error`).
2. **Local embedding provider** in `actx-lm` (or a port in the Core) so ingestion and query use the same model per workspace.
3. **Query and ingestion on the same model**: store the model and dimension used per workspace/table (SQLite settings) and use that model in `pipeline.rs:536` instead of the fixed value.
4. **Dimension migration in LanceDB**: dimension per table; detect divergence (model or dimension) and offer an assisted reindex (`/reindex` or the sync flow), without losing the source data. This is the first schema change since C0, so it needs a migration and rollback plan.
5. **Multilingual default model**, consistent with the Calgary global policy. Candidates **(to verify)**: `intfloat/multilingual-e5-small` (384 dims, requires the `query:`/`passage:` prefixes), `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` (384 dims), `BAAI/bge-m3` (1024 dims, ~2 GB, only for large profiles). Before choosing: confirm the official ONNX exists, the commercial license, the exact size, the SHA-256 and a pinned revision, and measure p50/p95 on CPU.
6. **Model Kit** (Phase 3 of the master roadmap), with presets and auto-detection:
   - **Offline / Edge**: local LLM (Ollama / LM Studio), local ONNX embedding, local ONNX reranker.
   - **Hybrid**: local RAG and preprocessing; synthesis through an API.
   - **Frontier**: top API models for synthesis.
   - **Corporate privacy**: private endpoints only (vLLM / Azure OpenAI).
   - The API model names in the master roadmap are from 2025 and need updating **(to verify)**.
7. **Items C0 explicitly left for Front C**:
   - Recalibrate profile RAM to what actually runs (embedding + reranker + heuristic Document AI).
   - Create `actx profile install` (install or switch the profile after installation, reusing `provision.rs`).
   - Unify the Core/Installer catalogs into a single source (e.g. a shared crate or module consumed by both), removing the duplication.

### 2.3 Surfaces
- `/embed` (status: model, dimension, runtime, latency) and `/embed model <id>`, mirroring `/rerank`.
- TUI badge and the `--embed-model` flag.
- A clear message when the workspace needs reindexing.
- `actx download embedder` as a repair tool, following the reranker's pattern.

### 2.4 Non-regression
- ADR-110: no weights or no runtime ⇒ fall back to the current embedding provider (API) or BM25 only, without breaking.
- Cold start preserved (ADR-122): nothing loads at startup.
- Existing workspaces stay queryable until reindexed.
- Live catalog test (`catalog_urls`) covering the new files.

### 2.5 Documentation and release
- Next ADR: **ADR-131**.
- Manual test scenario: **Scenario 46** at the top of `tests/MANUAL_TESTS.md`.

---

## 3. Front D — v0.47.0: Hierarchical Chunking / Parent-Document / AST Expansion

Defined in C0's revised roadmap by its title only. The content below is the initial proposal for the blueprint.

### 3.1 Idea
- **Retrieve fine, deliver context**: index small chunks (better precision in the embedding and the cross-encoder) and, when handing to the LLM, expand to the parent unit (document section, whole function or class, whole table).
- **AST expansion**: the `ASTCodeChunker` (tree-sitter: Python, TS/JS, Java, Kotlin, Swift, Ruby, PHP, Lua, Dart…) already produces `Context: file > class > def [lines X-Y]` breadcrumbs. Use this hierarchy to climb from a passage to the function or class that contains it, or bring in the referenced sibling functions.
- **Documents and structured data**: the `StructuredDataChunker` (Milestone 3, v0.30.7) already keeps the 2D structure. Parent = table or section; child = row or block.

### 3.2 Points of attention
- **Schema**: the chunk needs a `parent_id` and a level. Combine with Front C's schema migration so LanceDB is not migrated twice.
- **Context budget**: expansion must respect the existing `apply_density_budget` and the U reordering (Front A).
- **Reranking**: decide whether the cross-encoder scores the child (more precise) or the parent, and then deduplicate repeated parents.
- **Provenance**: citations (`[file:lines]`) must point to the passage of the parent actually delivered.

---

## 4. Fronts without a defined version (technical backlog)

1. **Local generative Gemma 4** (facts verified in D14 of the C0 blueprint, and ADR-130 §130.4):
   - The real published family is E2B, E4B, 12B, 26B-A4B (MoE) and 31B. The current catalog tiers (`compact`, `27b`, `32b`) do not exist.
   - There is official ONNX (onnx-community) only for E2B and E4B, in a multi-graph, multi-shard format (`.onnx_data_*`); E2B q4 exceeds 3.6 GB.
   - It requires a multi-shard downloader, a generative loop (tokenization, KV-cache, sampling) and tier recalibration.
   - Today it is `Planned` and always uses the fallback provider.
2. **GPU Execution Providers** (DirectML on Windows, CUDA on Linux): CPU only today. Requires the matching ONNX Runtime package and care with DLL dependencies (see C0's D1: static linking with DirectML prevented `actx.exe` from starting).
3. **Heuristics → neural**: the query classifier (`query_classifier.rs`), the scan sentinel, the layout extractor and the vision encoder run deterministic heuristics, labeled as such. The Document AI weights are already downloaded by the profiles; real inference is missing.
4. **macOS**: there is no binary (the release only builds Linux x86_64 and Windows x86_64).
5. **Cleanups**:
   - Legacy `README.md` passages about ChromaDB/Python.
   - The `implementation_plan_subscription_tiers.md` plan (subscription tiers / `BillingManager`).
   - `BLUEPRINT_PRIORITY_4_CLI_TUI.md` **(to verify what is still pending)**.
   - The state of RFC-042 (5-phase reflective Deep Search) **(to verify: parts such as `retrieve_hybrid_batch`, `/fast` and `/deep` already exist)**.

---

## 5. Project conventions (follow on every front)

- **Blueprint first**: `docs/plans/anycontext_blueprint_v0XY0_<topic>.md` with metadata, discovery with `file:line` references, numbered decisions (D1…), explicit out of scope, tests, migration, rollback and a step by step by milestones (M0…M8).
- **Hexagonal architecture**: business rules in the Core (`crates/any-context-core-rs`), inference in `crates/actx-onnx`, CLI/TUI only as adapters.
- **ADR-110**: no model or runtime failure may break the product; there is always a silent, honest fallback.
- **Technical honesty (ADR-130)**: nothing is labeled neural without real inference; no fictitious telemetry; no catalog link without live verification (ADR-118) and SHA-256.
- **Global policy (Calgary)**: no tax or regional rules; multilingual default models.
- **Dual-Doc**: `README.md` (user) + an ADR in `TECDOC.md` (technical); prepend the scenario in `tests/MANUAL_TESTS.md` without truncating the previous ones.
- **Release**:
  1. Version bump in every crate and in `version.txt`.
  2. Commit to `dev` and a `dev → main` PR; CI (`e2e-tests.yml`) runs on the PR to `main` and on push to `dev`.
  3. Merge and then the **tag `vX.Y.Z` on the merge commit in `main`**, which triggers `release.yml` and publishes to `Levix-Digital/any-context-releases`.
- **Local tests**:
  - They need `protoc` and ONNX Runtime: `bash scripts/fetch_onnxruntime.sh .ort`, then `ORT_DYLIB_PATH=.ort/libonnxruntime.so` (or `onnxruntime.dll`) and `ACTX_REQUIRE_ORT=1 cargo test --workspace`.
  - The `test_router_sub_microsecond_performance` test measures time and can fail if run in parallel with a heavy build.

### Limitations observed in cloud sessions (Claude Code)
- The session can only push to its own `claude/*` branch. Pushing directly to `dev` and creating tags are blocked (Auto mode) or refused by the git proxy (tags, HTTP 403). The flow that works: push to the session branch → PR to `dev` → merge by the maintainer → tag created by the maintainer.
- The test workflow does not run on PRs to `dev`; to validate a branch before merging, use `workflow_dispatch` of `e2e-tests.yml` on it.

---

## 6. Suggested prompt to start the next session

> Read `docs/plans/anycontext_roadmap_pos_v0450_proximas_frentes.md` and ADR-130 in `TECDOC.md`. We are starting **Front C (v0.46.0): local ONNX embeddings + Model Kit**. First do the discovery: confirm the cited code points (`pipeline.rs:536`, `lancedb.rs:19`, `orchestrator.rs:198`) and live-verify the embedding model candidates (ONNX existence, license, size, SHA-256, pinned revision). Then write the blueprint `docs/plans/anycontext_blueprint_v0460_...md` following the format of the v0.45.0 blueprint, before any code.
