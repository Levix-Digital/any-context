# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-001: Data indexed before field encryption stays in plaintext
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: Chunks written to LanceDB before field-level encryption on every write (ADR-0131) remain unencrypted on disk until the workspace is re-indexed with actx --sync --force. Nothing tells the user, while the README states that indexed text is encrypted.

### BUG-002: Frontier preset selects an outdated model id
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: /preset frontier sets the model claude-3-7-sonnet, which may no longer be a valid model id for the Anthropic API; the preset could then fail on the first question.

### BUG-003: Changing a workspace chat model mixes embedding spaces on the next sync
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: resolve_embedding_provider (ingestion/orchestrator.rs) derives the embedding model from the workspace chat model. After /model changes it, the next incremental sync embeds new or changed files with another provider/model while unchanged files keep their old vectors, and queries use the new model, so part of the workspace becomes unreachable by dense search. Needs the per-workspace embedding model record planned for Frente C, or a forced reindex when the embedding target changes.

## Features

### FEAT-001: actx profile install command
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Install or switch the hardware profile after installation, reusing crates/actx-installer/src/provision.rs (roadmap 2.2 goal 7).

### FEAT-002: Local generative Gemma 4 (E2B/E4B ONNX)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: The published family is E2B, E4B, 12B, 26B-A4B and 31B; the tiers in actx-onnx gemma.rs (Compact, 27b, 32b) do not exist. Official ONNX only for E2B/E4B, multi-graph and multi-shard (E2B q4 > 3.6 GB). Needs a multi-shard downloader, a generative loop (tokenization, KV-cache, sampling) and tier recalibration; today Planned with the fallback provider (ADR-130, C0 D14).

### FEAT-003: GPU execution providers (DirectML on Windows, CUDA on Linux)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: ONNX inference runs on CPU only. Needs the matching ONNX Runtime package and care with DLL dependencies (C0 D1: static DirectML linking prevented actx.exe from starting).

### FEAT-004: macOS release binary
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: release.yml only builds Linux x86_64 and Windows x86_64.

## Improvements

### IMP-001: Review the Deep Search minimum score
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: Deep Search keeps results with a minimum score of 0.02, which looks low and may let irrelevant chunks into the combined answer.

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

### IMP-003: Report query embedding failures instead of skipping dense search silently
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: NativeHybridPipeline::search_single_async discards the query embedding error (retrieval/pipeline.rs, '_ => None'), so a missing key or unsupported provider silently leaves only BM25. Keep the ADR-110 fallback but record the outcome (for example a last-status accessor like last_rerank_latency_ms) and surface it in the planned /embed status.

### IMP-004: Per-table vector dimension in LanceDB with assisted reindex
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: storage/lancedb.rs fixes DEFAULT_VECTOR_DIM = 1536 and ingestion writes every table with it, while Gemini (768), Ollama nomic-embed-text (768) and local ONNX embedders (384/768) produce other sizes. Store model and dimension per workspace/table, detect divergence and offer assisted reindex. First LanceDB schema change since C0: needs migration and rollback; combine with the Frente D parent_id change (roadmap 2.2 goal 4).

### IMP-005: Single source for the Core and Installer model catalogs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Model catalogs are duplicated in any-context-core-rs ingestion/model_manager.rs and actx-installer hardware.rs, guarded only by catalog_parity_tests. Move them to one shared module consumed by both (roadmap 2.2 goal 7).

### IMP-006: Replace labeled heuristics with real neural inference
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: query_classifier.rs, scan sentinel, layout extractor and vision encoder run deterministic heuristics labeled as such. Document AI weights are already downloaded by the profiles; real inference is missing.

## Adjustments

### ADJ-001: Update the outdated .clinerules
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: .clinerules no longer matches the current project (Rust workspace, architecture-docs layout, role-based development cycle) and can mislead agents that read it.

### ADJ-002: Add a license file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The repository has no LICENSE file; the README tells readers to contact Levix Digital for usage and distribution terms. The license (or the proprietary terms) still has to be chosen and published.

### ADJ-003: Recalibrate hardware profile RAM to what actually runs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Profile RAM and tier semantics are calibrated for local Gemma 4, which does not run locally. Recalibrate for embedding + reranker + heuristic Document AI; disk was already fixed in PR #327 (roadmap 2.2 goal 7).

### ADJ-004: Close out stale planning documents
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Decide the fate of implementation_plan_subscription_tiers.md (BillingManager tiers) and BLUEPRINT_PRIORITY_4_CLI_TUI.md at the repository root, verify the state of RFC-042 (retrieve_hybrid_batch, /fast and /deep already exist), and close milestone M8 Release of the released v0.45.0 C0 plan.

## Closed
