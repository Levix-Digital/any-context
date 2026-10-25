---
plan_id: imp_065_retrieval_benchmark
project: any-context
title: Development retrieval benchmark (IMP-065)
kind: feature
version: v1.0.0
status: draft
author: Claude
created_at: '2026-10-25T13:35:53'
updated_at: '2026-10-25T13:35:53'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-065
milestones: []
status_log:
- at: '2026-10-25T13:35:53'
  by: Claude
  status: draft
  note: Plan created
---

# Development retrieval benchmark (IMP-065)

Plan for the backlog items below (`dev-cycle-protocol` Phases 1 and 2). Replace every instruction paragraph with the plan.

## Backlog items

### IMP-065: [retrieval] Development retrieval benchmark on a fixed corpus, before the R1 retrieval rework
- **Priority**: high
- **Found**: 2026-10-25, review of improvement suggestions
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation ("in development, the test corpus runs on every code change"). The R1 retrieval items (IMP-040 to IMP-046) set defaults that the design says are tuned by the evaluation set, but the per-workspace set (FEAT-073, FEAT-106) only arrives in v1.1.0. Before IMP-040, a development-only benchmark: a fixed versioned corpus (starting with the fictitious CMR) and a hand-labeled question file (expected documents and passages, plus negative questions that must get the nothing-found answer); a runner over the real retrieval pipeline that reports HitRate@1/3/5/10, MRR@10, NDCG@10, the nothing-found rate and retrieval latency (mean and p95); results saved as JSON in the repository and compared with a previous run, so each retrieval PR shows its effect. Runs locally and in CI when the retrieval code changes, with a regression threshold. Out of scope: user-facing commands, synthetic question generation (FEAT-073) and answer evaluation (FEAT-102).

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
