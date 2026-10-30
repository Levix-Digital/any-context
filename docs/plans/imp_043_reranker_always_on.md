---
plan_id: imp_043_reranker_always_on
project: any-context
title: Reranker always on
kind: feature
version: v1.0.0
status: completed
author: LeviGuilherme
created_at: '2026-10-30T17:44:49'
updated_at: '2026-10-30T18:38:41'
session_id: null
tags:
- retrieval
- reranker
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-043
spec_dir: specs/005-reranker-always-on
milestones:
- name: M1 Core
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M2 Surfaces
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M3 Docs and checks
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
status_log:
- at: '2026-10-30T17:44:49'
  by: LeviGuilherme
  status: planned
  note: Plan created
- at: '2026-10-30T17:48:46'
  by: LeviGuilherme
  status: in_progress
  note: Approved by Levi (gate 3), 2026-10-30
- at: '2026-10-30T18:16:19'
  by: LeviGuilherme
  note: Milestone 'M1 Core' completed
- at: '2026-10-30T18:16:22'
  by: LeviGuilherme
  note: Milestone 'M2 Surfaces' completed
- at: '2026-10-30T18:38:41'
  by: LeviGuilherme
  note: Milestone 'M3 Docs and checks' completed
- at: '2026-10-30T18:38:41'
  by: LeviGuilherme
  status: completed
  note: All milestones completed
---

# Reranker always on

Envelope of a Spec Kit feature plan. The detail lives in [`specs/005-reranker-always-on/`](../../specs/005-reranker-always-on/): [spec](../../specs/005-reranker-always-on/spec.md), [plan](../../specs/005-reranker-always-on/plan.md), [research](../../specs/005-reranker-always-on/research.md), [tasks](../../specs/005-reranker-always-on/tasks.md).

## Backlog items

- IMP-043 (medium): [retrieval] Cross-encoder as the only relevance judge, always on.

Release R1 v1.0.0 Personal. Chosen by Levi on 2026-10-30, after IMP-041 and IMP-042.

## Goal and decisions

- The cross-encoder always runs: `rerank_mode`, `/rerank` (with `model`) and `--rerank` are removed; a stored `rerank_mode` is ignored. `--rerank-model` stays for one run; the stored `reranker_model` is still honored until the hardware kit chooses the model (IMP-055).
- After the pass, unpinned candidates that were not scored are discarded; pinned file-name matches stay on top.
- A reranker that cannot run keeps the fusion order and the answer ends with "Answered without relevance ranking: <cause>. <fix>", next to the dense search warning. `/status` gets a reranker line (model, state, depths, last pass).
- Deep's per-chunk LLM yes/no is removed.
- Out of scope, with their owner items: contextual header (IMP-026), kit model and Model Server (IMP-054, IMP-055), summaries and SQL (FEAT-061, FEAT-093, FEAT-092), relevance bands (IMP-044).
- ADR-0157 records the decisions.

## Milestones

1. M1 Core (T001 to T005).
2. M2 Surfaces (T006 to T011).
3. M3 Docs and checks (T012 to T016).
