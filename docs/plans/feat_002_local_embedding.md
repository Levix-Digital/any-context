---
plan_id: feat_002_local_embedding
project: any-context
title: Local multilingual embedding model, independent of the chat model
kind: feature
version: v1.0.0
status: completed
author: Levi Silveira
created_at: '2026-10-29T22:43:37'
updated_at: '2026-10-30T00:58:59'
session_id: null
tags:
- embedding
- onnx
- models
- retrieval
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- FEAT-067
- IMP-027
- IMP-028
- IMP-029
spec_dir: specs/002-local-embedding
milestones:
- name: M1 Catalog and identity
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M2 Local embedder
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M3 Setting resolution and migration
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M4 Prefixes
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M5 Benchmark defaults CI and docs
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
status_log:
- at: '2026-10-29T22:43:37'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-29T22:44:07'
  by: Levi Silveira
  status: planned
  note: Plan updated
- at: '2026-10-29T23:11:28'
  by: Levi Silveira
  status: in_progress
  note: Approved by Levi (gate 3), 2026-10-29
- at: '2026-10-30T00:32:58'
  by: LeviGuilherme
  note: Milestone 'M1 Catalog and identity' completed
- at: '2026-10-30T00:33:01'
  by: LeviGuilherme
  note: Milestone 'M2 Local embedder' completed
- at: '2026-10-30T00:33:02'
  by: LeviGuilherme
  note: Milestone 'M3 Setting resolution and migration' completed
- at: '2026-10-30T00:33:04'
  by: LeviGuilherme
  note: Milestone 'M4 Prefixes' completed
- at: '2026-10-30T00:58:59'
  by: LeviGuilherme
  note: Milestone 'M5 Benchmark defaults CI and docs' completed
- at: '2026-10-30T00:58:59'
  by: LeviGuilherme
  status: completed
  note: All milestones completed
---

# Local multilingual embedding model, independent of the chat model

Envelope of a Spec Kit feature plan. The detail lives in [`specs/002-local-embedding/`](../../specs/002-local-embedding/): [spec](../../specs/002-local-embedding/spec.md), [plan](../../specs/002-local-embedding/plan.md), [research](../../specs/002-local-embedding/research.md), [data model](../../specs/002-local-embedding/data-model.md), [contract](../../specs/002-local-embedding/contracts/embedding-cli.md), [quickstart](../../specs/002-local-embedding/quickstart.md), [tasks](../../specs/002-local-embedding/tasks.md).

## Backlog items

- FEAT-067 (medium): [models] Local multilingual ONNX embedding models per profile as the default.
- IMP-027 (medium): [ingestion] Embedding model independent of the chat model, target on each chunk.
- IMP-028 (medium): [ingestion] Embedding never switches model silently; OpenAI and mock fallbacks removed.
- IMP-029 (medium): [ingestion] Query and document prefixes per model from the catalog.

Release R1 v1.0.0 Personal.

## Goal

Semantic search that works out of the box, locally and across languages, with no API key: a local multilingual ONNX embedding model chosen by the hardware profile, an embedding setting separate from the chat model, no silent model switch, and the model's own prefixes on both sides. The benchmark from IMP-065 picks each profile's default and gates dense retrieval in CI.

## Decisions

Taken with Levi on 2026-10-29:

1. **One plan for the four items**: they change the same code and the same vector-space identity.
2. **Candidates**: multilingual-e5-small, multilingual-e5-base and BGE-M3 (MIT, INT8 ONNX). Qwen3-Embedding-0.6B left out (too heavy for CI and the smaller profiles).
3. **IMP-028 included**: without it a failure of the local model would be hidden by the fallbacks.
4. **Per-target tables out of scope**: the single LanceDB table and its zero-padding stay; per-target tables come with IMP-033/IMP-040. The chunk-level target record keeps vector spaces apart until then.
5. **The IMP-004 M1 work (`4042b89`) is ported, not merged**: the old branch predates plan B; the file-by-file re-embedding is not ported.

Taken in planning (research.md), to confirm at the gate:

6. **Local embedder in `actx-onnx`** (`OnnxEmbedder`, like the reranker) behind the existing `LmProvider::embed` port; no new port.
7. **Vector-space identity** `provider/model@dim[+pN]`: dimension and prefix revision in the key, so any change triggers the existing reindex rule (ADR-0138); legacy keys still parse.
8. **Upgrade adopts existing cloud indexes** (no surprise reindex); mock indexes move to the local model on their next sync.
9. **Missing local model is downloaded on first need** through the verified download, from a configurable source (catalog upstream or an internal mirror; pre-placed files are used offline after the SHA-256 check). Decided with Levi on 2026-10-29.
10. **No lexical-only mode** (Levi, 2026-10-29: without semantic search it is not RAG): `Lite` downloads e5-small; without any model the sync stops with model, source and manual-install instructions.
10b. **Governance seam**: one allowed-models check in the resolution (allows the catalog now; organization policy IMP-056 and Admin Hub mirror FEAT-143 plug in at v2.0.0).
11. **Commands**: `/embedding [id]` and `actx embedding [id] [--yes]`; `/model` and `-m` stop affecting embeddings.
12. **CI**: a dense benchmark job with the reference default and its own baseline; the lexical job stays.
13. **ADR-0154** records the decision (supersedes the chat-model derivation of ADR-0132, amends ADR-0138).

## Milestones

1. **M1 Catalog and identity** (tasks T001 to T008)
2. **M2 Local embedder** (T009 to T014)
3. **M3 Setting, resolution and migration** (T015 to T026)
4. **M4 Prefixes** (T027 to T030)
5. **M5 Benchmark, defaults, CI and docs** (T031 to T042)

## Version impact

User-facing: new embedding setting and commands, local semantic search by default, Gemini indexes reindexed once (new prefixes). Part of R1 v1.0.0; no separate bump in this PR.
