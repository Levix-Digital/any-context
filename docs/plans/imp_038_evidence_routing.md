---
plan_id: imp_038_evidence_routing
project: any-context
title: Automatic mode routes by evidence, not by keywords
version: v1.0.0
status: in_progress
author: LeviGuilherme
created_at: '2026-11-10T12:51:10'
updated_at: '2026-11-10T17:17:26'
session_id: null
tags:
- retrieval
- routing
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- IMP-038
spec_dir: specs/009-evidence-routing
milestones:
- name: M1 Route and signal
  done: true
  done_at: '2026-11-10'
  by: LeviGuilherme
- name: M2 Weights removal
  done: true
  done_at: '2026-11-10'
  by: LeviGuilherme
- name: M3 Docs and checks
  done: false
status_log:
- at: '2026-11-10T12:51:10'
  by: LeviGuilherme
  status: active
  note: Plan created
- at: '2026-11-10T17:17:21'
  by: LeviGuilherme
  note: 'Milestone ''M1 Route and signal'' completed: PR #406'
- at: '2026-11-10T17:17:26'
  by: LeviGuilherme
  note: 'Milestone ''M2 Weights removal'' completed: PR #406'
---

# Automatic mode routes by evidence, not by keywords

**Backlog**: IMP-038. **Spec folder**: `specs/009-evidence-routing` (spec, plan, research, tasks).

## Goal and decisions

Remove the keyword and length classifier that chooses Fast or Deep before the search (`actx-agent` router, the `actx-onnx` heuristic query classifier and its core adapter), and its unused 133 MB `bge-small-onnx` weights from the catalog, the hardware kits, the installer and the model menu. Automatic mode always starts Fast and escalates to Deep on evidence: the reranker bands (IMP-046, unchanged) or the LLM's not-enough signal, a fixed reply asked for by a rule in the automatic Fast prompt and caught by a streaming filter before any token is shown. Pinned Fast never escalates and carries no rule; pinned Deep always runs Deep. The routing event keeps only the route, whether it was pinned and the reason. Live status terms stay until FEAT-082; tools in Fast and ReAct removal stay with IMP-037. ADR-0161.

## Milestones

1. M1 Route and signal: route function, classifier removal, signal filter and escalation, events and surfaces.
2. M2 Weights removal: catalog, kits, installer, menu.
3. M3 Docs and checks: ADR-0161, pages, README, manual tests, backlog, full tests.
