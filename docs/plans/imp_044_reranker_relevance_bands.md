---
plan_id: imp_044_reranker_relevance_bands
project: any-context
title: Reranker relevance bands
version: v1.0.0
status: planned
author: LeviGuilherme
created_at: '2026-10-30T21:40:05'
updated_at: '2026-10-30T21:40:05'
session_id: null
tags:
- retrieval
- reranker
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-044
spec_dir: specs/006-reranker-relevance-bands
milestones:
- name: M1 Core
  done: false
- name: M2 Status and benchmark
  done: false
- name: M3 Docs and checks
  done: false
status_log:
- at: '2026-10-30T21:40:05'
  by: LeviGuilherme
  status: planned
  note: Plan created
---

# Reranker relevance bands

Envelope of a Spec Kit feature plan. The detail lives in [`specs/006-reranker-relevance-bands/`](../../specs/006-reranker-relevance-bands/): [spec](../../specs/006-reranker-relevance-bands/spec.md), [plan](../../specs/006-reranker-relevance-bands/plan.md), [research](../../specs/006-reranker-relevance-bands/research.md), [tasks](../../specs/006-reranker-relevance-bands/tasks.md).

## Backlog items

- IMP-044 (medium): [retrieval] Calibrated reranker relevance bands per model.

Release R1 v1.0.0 Personal. Proposed on 2026-10-30 as the next step of the reranking design after IMP-043.

## Goal and decisions

- Each catalog reranker carries two thresholds; a scored candidate is relevant, maybe or irrelevant, and irrelevant candidates are discarded before the selection. Pinned file-name matches and passes the reranker could not score are left as they are; an unknown model gets no band.
- The search output and `/status` report the band counts of the pass.
- The benchmark ranks without the cut, measures the cut on the delivered pass, runs with any catalog reranker (`--reranker`), writes a calibration table and gates the delivered hit rate.
- Thresholds are calibrated on the benchmark, favoring keeping relevant documents.
- Out of scope, with their owner items: selection by bands and token budget (IMP-045), nothing-found gate and escalation (IMP-046), per-workspace refinement (IMP-008, step 10), kit reranker on Lite (IMP-055).
- ADR-0158 records the decisions.

## Milestones

1. M1 Core (T001 to T005).
2. M2 Status and benchmark (T006 to T009).
3. M3 Docs and checks (T010 to T014).
