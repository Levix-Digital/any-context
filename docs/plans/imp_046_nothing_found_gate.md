---
plan_id: imp_046_nothing_found_gate
project: any-context
title: Nothing-found and weak-evidence gate by relevance bands
version: v1.0.0
status: completed
author: LeviGuilherme
created_at: '2026-11-09T16:45:55'
updated_at: '2026-11-09T17:49:28'
session_id: null
tags:
- retrieval
- gate
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-046
spec_dir: specs/008-nothing-found-gate
milestones:
- name: M1 Core and agent
  done: true
  done_at: '2026-11-09'
  by: LeviGuilherme
- name: M2 Benchmark
  done: true
  done_at: '2026-11-09'
  by: LeviGuilherme
- name: M3 Docs and checks
  done: true
  done_at: '2026-11-09'
  by: LeviGuilherme
status_log:
- at: '2026-11-09T16:45:55'
  by: LeviGuilherme
  status: active
  note: Plan created
- at: '2026-11-09T17:27:43'
  by: LeviGuilherme
  note: 'Milestone ''M1 Core and agent'' completed: PR #403'
- at: '2026-11-09T17:27:45'
  by: LeviGuilherme
  note: 'Milestone ''M2 Benchmark'' completed: PR #403'
- at: '2026-11-09T17:49:28'
  by: LeviGuilherme
  note: 'Milestone ''M3 Docs and checks'' completed: PR #403, CI green (both benchmarks, cargo test)'
- at: '2026-11-09T17:49:28'
  by: LeviGuilherme
  status: completed
  note: All milestones completed
---

# Nothing-found and weak-evidence gate by relevance bands

**Backlog**: IMP-046. **Spec folder**: `specs/008-nothing-found-gate` (spec, plan, research, tasks).

## Goal and decisions

Replace the score cut of the Zero Hallucination Gate (0.005) with a gate on the reranker bands of the delivered chunks: nothing found answers without the LLM, in Fast and now in Deep; maybe-band-only evidence gives an answer that starts with a fixed weak-evidence warning, with a prompt rule; in automatic mode a Fast route with nothing or weak evidence escalates to Deep with an indicator. Band-less chunks (no reranker) are not judged. Removing the keyword classifier stays with IMP-038, `@source` with FEAT-088, the warning inside the streamed answer with IMP-050. ADR-0160.

## Milestones

1. M1 Core and agent: gate module, prompts, agent routes, Deep gate, CLI adapters and surfaces.
2. M2 Benchmark: weak-evidence rates.
3. M3 Docs and checks: ADR-0160, pages, README, manual tests, backlog, full tests.
