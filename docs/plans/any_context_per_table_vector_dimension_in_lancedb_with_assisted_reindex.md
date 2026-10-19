---
plan_id: any_context_per_table_vector_dimension_in_lancedb_with_assisted_reindex
project: any-context
title: Per-table vector dimension in LanceDB with assisted reindex
kind: feature
version: null
status: draft
author: Claude
created_at: '2026-10-19T15:43:49'
updated_at: '2026-10-19T15:47:19'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-004
milestones: []
status_log:
- at: '2026-10-19T15:43:49'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-19T15:47:19'
  by: Claude
  status: draft
  note: Plan updated
---

# Per-table vector dimension in LanceDB with assisted reindex

Plan for the backlog items below (`dev-cycle-protocol` Phases 1 and 2). Replace every instruction paragraph with the plan.

## Backlog items

### IMP-004: Per-table vector dimension in LanceDB with assisted reindex
- **Priority**: high
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Description**: storage/lancedb.rs fixes DEFAULT_VECTOR_DIM = 1536 and ingestion writes every table with it, while Gemini (768), Ollama nomic-embed-text (768) and local ONNX embedders (384/768) produce other sizes. Store model and dimension per workspace/table, detect divergence and offer assisted reindex. First LanceDB schema change since C0: needs migration and rollback; combine with the Frente D parent_id change (roadmap 2.2 goal 4).

## Discovery (2026-10-19)

Scope decided by the user: IMP-004 alone, not combined with the Frente D `parent_id` change.

### What already exists (ADR-0138, v0.45.1)
- `workspaces.embedding_target` records the `provider/model` that built each workspace's vectors; `decide_reindex` triggers an automatic full reindex when the resolved target changes, and `pending_reindex_notice` warns from `/model` and `/sync`.
- `resolve_query_embedding_provider` embeds queries with the recorded target, so the "store the model per workspace, detect divergence, reindex" part of IMP-004 is done. What is left is the **dimension**.

### Current dimension handling
- One shared table, `workspace_chunks`, with a fixed `FixedSizeList(1536)` vector column (`storage/lancedb.rs:21`, `DEFAULT_VECTOR_DIM`). Every row of every workspace (and `Global`) lives in it, separated by the `workspace` column.
- `build_record_batch` silently zero-pads shorter vectors and truncates longer ones to 1536 (`storage/lancedb.rs`, `r.vector.resize(d, 0.0)`).
- Writers hard-code 1536: `ingestion/orchestrator.rs` (`upsert_records(..., Some(DEFAULT_VECTOR_DIM))`, offline fallback vectors), `actx-cli/src/engine.rs` (`GLOBAL_VECTOR_DIM`, which rejects any other size for `Global`), and Gemini requests `outputDimensionality: 1536` (`GEMINI_EMBEDDING_DIM`) to fit the table.
- Providers: OpenAI `text-embedding-3-small` = 1536, Gemini `gemini-embedding-2` = 1536 (requested), Ollama `nomic-embed-text` = 768, mock = 1536. There is no local ONNX embedder yet (Frente C goal 1); 384-dimension models are future work.

### Defect found: dense search never works on Ollama workspaces
- Chunk vectors (768) are zero-padded to 1536 on write, but the query vector is sent to `search_vector_async` unpadded (`retrieval/pipeline.rs`, dense search loop).
- LanceDB 0.39 rejects a query vector whose size matches no vector column ("No vector column found to match with the query vector dimension", its own test in `lancedb-0.39.0/src/query.rs`).
- The pipeline drops the error (`if let Ok(matches) = ...`), so Ollama workspaces are answered by BM25 only while `last_query_embedding()` reports `Dense`. ADR-0138 lists the padding as a known consequence but not the failed search.

### Related behavior (out of scope unless decided otherwise)
- The `Default` workspace searches every row (no workspace filter) and multi-workspace queries reuse the primary workspace's query vector for the other workspaces, so workspaces with different targets are compared across vector spaces. Per-dimension storage would turn that silent mismatch into an explicit skip.

### Options
- **A. Pad the query vector only**: pad or reject the query vector to the table size, record the dimension with the target, fail loudly above 1536. Smallest fix; zero padding is exact for L2 and cosine, but storage stays 1536 for every model and models above 1536 can't be used.
- **B. One table per vector dimension** (recommended): `workspace_chunks` stays the 1536 table, untouched; other sizes get additive tables (for example `workspace_chunks_768`), chosen from the workspace's recorded target. No rewrite of existing data, and an older version still reads every 1536 workspace. Searches and deletes resolve the table from the workspace; a query only meets rows of its own size.
- **C. One table per workspace**: the most literal reading of "per table", but every read, delete and the `Default` all-workspace search must change, and every existing row must be copied: the largest migration.

## 1. Goal and decisions

What changes and why; decisions with alternatives become ADRs (`architecture-docs`).

## 2. Design (`software-design`)

Where each piece lives, interfaces, data flow, error strategy, dependencies, what is deliberately not abstracted.

## 3. Affected surfaces and files

Interfaces in `architecture.surfaces` that change; files and modules.

## 4. Tests (`quality-assurance`)

Automated tests to add, manual tests to add, change or remove, acceptance criteria.

## 5. Documentation (`architecture-docs`, `user-docs`)

Subsystem pages and ADRs, README sections.

## 6. CI, release and observability (`devops-release`)

Workflow changes, version impact, logs, diagnostics and telemetry to add and how they are tested.

## 7. Security, migration and rollback

Secrets, data migrations (backward compatible), how to roll back.

## 8. Execution steps and milestones
