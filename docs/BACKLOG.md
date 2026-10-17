# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-001: Dense query embedding ignores the workspace embedding model
- **Severity**: high
- **Status**: in_progress
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: anycontext_bugfix_v0451_query_embedding_model
- **Description**: retrieval/pipeline.rs:536 always embeds the query with text-embedding-3-small, while ingestion resolves the model per workspace (ingestion/orchestrator.rs:198). A workspace indexed with another model gets query vectors in another space or dimension and dense search silently degrades or fails. Fix in Frente C or earlier as patch v0.45.1.

### BUG-002: Changing a workspace chat model mixes embedding spaces on the next sync
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, BUG-001 discovery (v0.45.0)
- **Plan**: -
- **Description**: Ingestion derives the embedding model from the workspace chat model (orchestrator.rs resolve_embedding_provider). After /model changes it, the next incremental sync embeds new or changed files with another model while unchanged files keep the old vectors, so one workspace holds vectors from two spaces. Needs the per-workspace embedding model record planned for Frente C, or a forced reindex on change.

## Features

### FEAT-001: actx profile install command
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Install or switch the hardware profile after installation, reusing crates/actx-installer/src/provision.rs.

### FEAT-002: Local generative Gemma 4 (E2B/E4B ONNX)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Real family is E2B, E4B, 12B, 26B-A4B, 31B; catalog tiers compact/27b/32b do not exist. Official ONNX only for E2B/E4B, multi-graph and multi-shard (E2B q4 > 3.6 GB). Needs multi-shard downloader, generative loop (tokenization, KV-cache, sampling) and tier recalibration. Today Planned with fallback provider (ADR-130 130.4, C0 D14).

### FEAT-003: GPU execution providers (DirectML on Windows, CUDA on Linux)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: ONNX inference is CPU only. Needs the matching ONNX Runtime package and care with DLL dependencies (C0 D1: static DirectML linking prevented actx.exe from starting).

### FEAT-004: macOS release binary
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Release pipeline only builds Linux x86_64 and Windows x86_64.

## Improvements

### IMP-001: Per-table vector dimension in LanceDB with assisted reindex
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: storage/lancedb.rs:19 fixes DEFAULT_VECTOR_DIM = 1536; local embeddings use 384/768. Store model and dimension per workspace/table, detect divergence and offer assisted reindex. First schema change since C0: needs migration and rollback plan; combine with Frente D parent_id schema change.

### IMP-002: Single source for the Core and Installer model catalogs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Model catalogs are duplicated in Core model_manager.rs and Installer hardware.rs, guarded only by a parity test. Move to one shared crate or module consumed by both.

### IMP-003: Replace labeled heuristics with real neural inference
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: query_classifier.rs, scan sentinel, layout extractor and vision encoder run deterministic heuristics. Document AI weights are already downloaded by the profiles; real inference is missing.

## Adjustments

### ADJ-001: Recalibrate hardware profile RAM to what actually runs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Profile RAM and tier semantics are calibrated for Gemma 4, which does not run locally. Recalibrate for embedding + reranker + heuristic Document AI (disk already fixed in PR #327).

### ADJ-002: Remove legacy ChromaDB/Python passages from README
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: README.md:26 still mentions ChromaDB parallel scanning in the TECDOC pointer; review other legacy ChromaDB/Python passages and version-history sections (user-docs: no version sections).

### ADJ-003: Close out stale planning documents
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0 (roadmap handoff, PR #328)
- **Plan**: -
- **Description**: Decide the fate of implementation_plan_subscription_tiers.md (BillingManager tiers) and BLUEPRINT_PRIORITY_4_CLI_TUI.md (verify what is still pending), verify the state of RFC-042 (retrieve_hybrid_batch, /fast, /deep already exist), and mark the released v0.45.0 C0 plan M8 Release done/completed.

## Closed
