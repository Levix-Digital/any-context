---
plan_id: any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries
project: any-context
title: 'Embedding failures are hidden: pseudo-vectors at ingestion and silent BM25-only queries'
kind: feature
version: null
status: draft
author: Claude
created_at: '2026-10-18T19:32:46'
updated_at: '2026-10-18T19:32:46'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-014
- IMP-003
milestones: []
status_log:
- at: '2026-10-18T19:32:46'
  by: Claude
  status: draft
  note: Plan created
---

# Embedding failures are hidden: pseudo-vectors at ingestion and silent BM25-only queries

Plan for the backlog items below (`dev-cycle-protocol` Phases 1 and 2). Replace every instruction paragraph with the plan.

## Backlog items

### BUG-014: Failed embedding calls store pseudo-vectors next to real ones
- **Severity**: medium
- **Found**: 2026-10-17, 2026-10-17, BUG-003 discovery
- **Description**: embed_and_upsert (ingestion/orchestrator.rs) replaces a failed provider.embed call with SHA-256 pseudo-vectors (generate_fallback_vectors) and writes them to the same table as real vectors, without telling the user or marking the file. A transient API error (rate limit, network) leaves those files unreachable by dense search until they change on disk. Decide between failing the file so the next sync retries it, or marking it for re-embedding; keep the ADR-110 no-key fallback. Related to IMP-003.

### IMP-003: Report query embedding failures instead of skipping dense search silently
- **Priority**: medium
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Description**: NativeHybridPipeline::search_single_async discards the query embedding error (retrieval/pipeline.rs, '_ => None'), so a missing key or unsupported provider silently leaves only BM25. Keep the ADR-110 fallback but record the outcome (for example a last-status accessor like last_rerank_latency_ms) and surface it in the planned /embed status.

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
