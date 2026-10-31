---
plan_id: imp_045_band_selection
project: any-context
title: Selection by relevance bands and the token budget
version: v1.0.0
status: in_progress
author: LeviGuilherme
created_at: '2026-10-31T13:40:41'
updated_at: '2026-10-31T14:49:45'
session_id: null
tags:
- retrieval
- selection
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-045
spec_dir: specs/007-band-selection
milestones:
- name: M1 Core
  done: true
  done_at: '2026-10-31'
  by: LeviGuilherme
- name: M2 Benchmark
  done: true
  done_at: '2026-10-31'
  by: LeviGuilherme
- name: M3 Docs and checks
  done: false
status_log:
- at: '2026-10-31T13:40:41'
  by: LeviGuilherme
  status: active
  note: Plan created
- at: '2026-10-31T14:49:45'
  by: LeviGuilherme
  note: 'Milestone ''M1 Core'' completed: PR #391'
- at: '2026-10-31T14:49:45'
  by: LeviGuilherme
  note: 'Milestone ''M2 Benchmark'' completed: PR #391, local lexical run'
---

# Selection by relevance bands and the token budget

**Backlog**: IMP-045. **Spec folder**: `specs/007-band-selection` (spec, plan, research, tasks).

## Goal and decisions

Replace the fixed selection (top-k, per-file caps with backfill, character ceilings that truncate the last chunk) with a selection by the reranker bands of IMP-044 within a token budget per mode: pinned matches, then the relevant band, then the best of the maybe band, in score order, skipping whole any chunk that does not fit; near-identical chunks enter once; no per-file limit. Budgets: Fast 4,000, Deep 6,000 per batch, tool 3,000 tokens, confirmed by the benchmark. `RetrievalPreset` and the fixed fields are removed. Parents (FEAT-098), current version only (FEAT-027), the model-window bound (IMP-049) and the gate (IMP-046) stay with their items. ADR-0159.

## Milestones

1. M1 Core: selection and dedup, request budget, pipeline step, fixed counts removed.
2. M2 Benchmark: delivered averages, local budget runs, budgets confirmed.
3. M3 Docs and checks: ADR-0159, pages, manual tests, baselines, backlog, full tests.
