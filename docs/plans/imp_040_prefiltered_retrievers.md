---
plan_id: imp_040_prefiltered_retrievers
project: any-context
title: Pre-filtered dense and lexical retrievers
kind: feature
version: v1.0.0
status: completed
author: LeviGuilherme
created_at: '2026-10-30T12:57:32'
updated_at: '2026-10-30T15:05:07'
session_id: null
tags:
- retrieval
- filters
- bm25
- lancedb
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-040
spec_dir: specs/003-prefiltered-retrievers
milestones:
- name: M1 Filter
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M2 Parallel retrievers and candidates
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M3 Dense outcome in the answer
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
- name: M4 Docs and baselines
  done: true
  done_at: '2026-10-30'
  by: LeviGuilherme
status_log:
- at: '2026-10-30T12:57:32'
  by: LeviGuilherme
  status: draft
  note: Plan created
- at: '2026-10-30T12:58:12'
  by: LeviGuilherme
  status: planned
  note: Plan updated
- at: '2026-10-30T13:12:38'
  by: LeviGuilherme
  status: in_progress
  note: Approved by Levi (gate 3), 2026-10-30
- at: '2026-10-30T15:04:42'
  by: LeviGuilherme
  note: Milestone 'M1 Filter' completed
- at: '2026-10-30T15:05:02'
  by: LeviGuilherme
  note: Milestone 'M2 Parallel retrievers and candidates' completed
- at: '2026-10-30T15:05:05'
  by: LeviGuilherme
  note: Milestone 'M3 Dense outcome in the answer' completed
- at: '2026-10-30T15:05:07'
  by: LeviGuilherme
  note: Milestone 'M4 Docs and baselines' completed
- at: '2026-10-30T15:05:07'
  by: LeviGuilherme
  status: completed
  note: All milestones completed
---

# Pre-filtered dense and lexical retrievers

Envelope of a Spec Kit feature plan. The detail lives in [`specs/003-prefiltered-retrievers/`](../../specs/003-prefiltered-retrievers/): [spec](../../specs/003-prefiltered-retrievers/spec.md), [plan](../../specs/003-prefiltered-retrievers/plan.md), [research](../../specs/003-prefiltered-retrievers/research.md), [tasks](../../specs/003-prefiltered-retrievers/tasks.md).

## Backlog items

- IMP-040 (medium): [retrieval] Pre-filtered dense and lexical retrievers over child chunks.

Release R1 v1.0.0 Personal.

## Goal

Both retrievers take a request filter (workspace plus optional source scopes, modification date range and content types) inside the search, so a narrowed question never comes back empty because better matches exist elsewhere. Dense and lexical start together for each question and sub-question, with 30 candidates per list. When dense search does not run, the answer ends with one warning line saying why.

## Decisions

1. **Scope**: the core of IMP-040 now; the parts that depend on open items stay with them: per-target tables and the old table during a model switch (IMP-033), child chunks (FEAT-036), weighted lexical fields (IMP-026, FEAT-069), leaving `Global` out of every search (FEAT-082). Chosen by Levi on 2026-10-30.
2. **Filter inside the retrievers**: one `RetrievalFilter` rendered as a LanceDB pre-filter and as a lexical predicate checked before ranking. Alternative rejected: filtering the fused list (can leave zero). ADR-0155.
3. **Warning in the answer**: the search returns its dense outcome with the results; the agent sends one notice per answer. Alternative rejected: status line only (IMP-003, easy to miss).
4. **30 candidates per list for Fast and Deep**, measured against 50 with the benchmark.

## Milestones

1. M1 Filter
2. M2 Parallel retrievers and candidates
3. M3 Dense outcome in the answer
4. M4 Docs and baselines
