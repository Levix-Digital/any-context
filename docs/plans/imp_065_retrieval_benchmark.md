---
plan_id: imp_065_retrieval_benchmark
project: any-context
title: Development retrieval benchmark (IMP-065)
kind: feature
version: v1.0.0
status: planned
author: Levi Silveira
created_at: '2026-10-25T13:41:55'
updated_at: '2026-10-25T13:41:55'
session_id: null
tags:
- retrieval
- benchmark
- evaluation
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-065
spec_dir: specs/001-retrieval-benchmark
milestones:
- name: M1 Setup and shared request builders
  done: false
- name: M2 Measure
  done: false
- name: M3 Compare and validate
  done: false
- name: M4 Corpus and questions
  done: false
- name: M5 Baseline CI and docs
  done: false
status_log:
- at: '2026-10-25T13:41:55'
  by: Levi Silveira
  status: planned
  note: Plan created
---

# Development retrieval benchmark (IMP-065)

Envelope of a Spec Kit feature plan. The detail lives in [`specs/001-retrieval-benchmark/`](../../specs/001-retrieval-benchmark/): [spec](../../specs/001-retrieval-benchmark/spec.md), [plan](../../specs/001-retrieval-benchmark/plan.md), [research](../../specs/001-retrieval-benchmark/research.md), [data model](../../specs/001-retrieval-benchmark/data-model.md), [contract](../../specs/001-retrieval-benchmark/contracts/actx-bench-cli.md), [quickstart](../../specs/001-retrieval-benchmark/quickstart.md), [tasks](../../specs/001-retrieval-benchmark/tasks.md), [analysis](../../specs/001-retrieval-benchmark/analysis.md).

## Backlog items

- IMP-065 (high): [retrieval] Development retrieval benchmark on a fixed corpus, before the R1 retrieval rework. Release R1 v1.0.0 Personal.

## Goal

Give developers a measurement of retrieval quality before the R1 changes that move it: the foundations (IMP-023, IMP-026, IMP-027, IMP-033), the local embedding model (FEAT-067) and the retrieval rework (IMP-040 to IMP-046), whose defaults the v2 design says are tuned by data. The per-workspace evaluation set only arrives in v1.1.0 (FEAT-073, FEAT-106).

## Decisions

Taken with Levi on 2026-10-25:

1. **Order**: this plan first; then a plan for FEAT-067 with IMP-027 and IMP-029, which picks the local embedding model with this benchmark and turns the dense retriever on in CI; IMP-040 after both. Alternative rejected: FEAT-067 first (its model would be chosen without measurement).
2. **CI without dense until FEAT-067**: CI measures BM25 + the local cross-encoder reranker (the mock embedding target makes the pipeline skip dense, BUG-019). Local runs can include dense with a configured provider; those results are never the gate.
3. **Corpus**: 30 to 40 fictitious pt/en documents (Markdown, text, PDF and the CMR form) with hard distractors; 60 to 80 hand-labeled questions, about 15 negative, reviewed by Levi before the first baseline.
4. **Gate**: fail when MRR@10 or HitRate@5 drops more than 0.02 below the committed baseline of the same configuration; latency is reported only.

Taken in planning (research.md), to confirm at the gate:

5. **Shared request builders**: the Fast and Deep request defaults move from `actx-cli/src/engine.rs` into `retrieval::requests` in the core, so the benchmark measures exactly what the product sends and a change to the defaults shows in the benchmark. No behavior change.
6. **Two passes per question**: a ranking pass (Fast request with `top_k` 10, no density cut) for the @k metrics and the gate, and a delivered pass (the Fast request as is) for the delivered hit rate, nothing-found and latency.
7. **Separate dev crate** `crates/actx-bench` (`publish = false`), a CLI with `validate` and `run`; not part of the release build. A new workflow `retrieval-benchmark.yml` runs it on pull requests that touch retrieval, ingestion, storage, the reranker crate or the benchmark, not a required check at first.
8. **ADR-0153** records the benchmark as the gate for retrieval changes.

## Milestones

1. **M1 Setup and shared request builders** (tasks T001 to T007)
2. **M2 Measure** (T008 to T015)
3. **M3 Compare and validate** (T016 to T020)
4. **M4 Corpus and questions**, with Levi's review (T021 to T025)
5. **M5 Baseline, CI and docs** (T026 to T033)

## Version impact

None: no product binary, command, setting or data changes.
