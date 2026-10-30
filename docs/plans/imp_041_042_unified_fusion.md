---
plan_id: imp_041_042_unified_fusion
project: any-context
title: Unified fusion and reranker depth
kind: feature
version: v1.0.0
status: in_progress
author: LeviGuilherme
created_at: '2026-10-30T15:51:21'
updated_at: '2026-10-30T15:54:46'
session_id: null
tags:
- retrieval
- fusion
- rrf
- reranker
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-041
- IMP-042
spec_dir: specs/004-unified-fusion
milestones:
- name: M1 Fusion
  done: false
- name: M2 Reranker input and depth
  done: false
- name: M3 Docs and baselines
  done: false
status_log:
- at: '2026-10-30T15:51:21'
  by: LeviGuilherme
  status: draft
  note: Plan created
- at: '2026-10-30T15:54:46'
  by: LeviGuilherme
  status: planned
  note: Plan updated
- at: '2026-10-30T15:56:34'
  by: LeviGuilherme
  status: in_progress
  note: Approved by Levi (gate 3), 2026-10-30
---

# Unified fusion and reranker depth

Envelope of a Spec Kit feature plan. The detail lives in [`specs/004-unified-fusion/`](../../specs/004-unified-fusion/): [spec](../../specs/004-unified-fusion/spec.md), [plan](../../specs/004-unified-fusion/plan.md), [research](../../specs/004-unified-fusion/research.md), [tasks](../../specs/004-unified-fusion/tasks.md).

## Backlog items

- IMP-041 (medium): [retrieval] Single RRF over clean lists with a per-document cap, no score cut.
- IMP-042 (medium): [retrieval] Reranker depth by time budget and profile.

Release R1 v1.0.0 Personal. Chosen by Levi on 2026-10-30, after IMP-040.

## Goal and decisions

- One search (Fast question or every Deep sub-question) has one RRF (k = 60, equal weights) over clean lists: dense per workspace, lexical, file-name matches and date matches. No RRF score cut; Deep's score summing is removed.
- One reranking pass per search scores each candidate once, against the sub-question whose lists ranked it best (research R3).
- Reranker input: fusion order, at most 3 slots per document, the rest in reserve.
- Depth by mode and hardware profile (Lite 20/30, Smart Balanced and Air-Gapped Power 30/50, Workstation Pro 40/75, Sovereign Ultra 50/100), lowered to a time budget (Fast 1 s, Deep 3 s, floor 10) from the reranking cost measured on the machine. The benchmark uses a fixed depth.
- Out of scope, with their owner items: per-workspace tuning (FEAT-105), reranker per kit (IMP-054, IMP-055), discarding the tail and `/rerank` (IMP-043), file-name and date filters (FEAT-088).
- ADR-0156 records the decisions.

## Milestones

1. M1 Fusion (T001 to T006).
2. M2 Reranker input and depth (T007 to T011).
3. M3 Docs and baselines (T012 to T016).
