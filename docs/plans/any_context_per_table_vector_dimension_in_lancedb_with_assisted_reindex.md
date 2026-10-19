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
