---
plan_id: anycontext_blueprint_v0344_estabilizacao_ux_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_auto_consciencia_menu_document_ai_e_telemetria_react_enriquecida
project: any-context
title: 'Blueprint v0.34.4: UX Stabilization, Deep Search, Search per Workspace, 8-Dot Spinner, Multiline Prompt, Self-Awareness, Document AI Menu and Enriched ReAct Telemetry'
version: v0.34.4
status: superseded
author: Levi Silveira
created_at: '2026-10-10T08:43:12'
updated_at: '2026-10-24T14:16:58'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- ux
- deep_search
- workspace_isolation
- tui_multiline
- braille_spinner
- sqlite
- styling
- global_knowledge
- document_ai_menu
- react_telemetry
supersedes: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_temas_e_auto_consciencia_global
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: Deep Search Synthesis Prompt and Language'
  done: false
- name: 'M2: Search Mode per Workspace in SQLite'
  done: false
- name: 'M3: 8-Dot Braille Spinner and 125ms Timer'
  done: false
- name: 'M4: Safe Redirection of /update to --update'
  done: false
- name: 'M5: Expandable Multiline Prompt with 2D Cursor'
  done: false
- name: 'M6: [AI - Workspace] Badge and Gray Typography'
  done: false
- name: 'M7: Self-Awareness and Transparent Global Sync'
  done: false
- name: 'M8: Document AI Menu and Local Models in /menu'
  done: false
- name: 'M9: Enriched ReAct and Reasoning Telemetry'
  done: false
- name: 'M10: Automated Test Suite'
  done: false
- name: 'M11: Dual-Doc and Scenario 27 in MANUAL_TESTS'
  done: false
- name: 'M12: Gate 8 and Release v0.34.4'
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
- at: '2026-10-24T14:16:58'
  by: Levi Silveira
  status: superseded
  note: Superseded by the architecture v2 redesign and roadmap v2 (docs/ROADMAP.md, docs/plans/anycontext_roadmap_v2.md, decided 2026-10-23); the remaining work was rebuilt as backlog items.
---

# Blueprint AnyContext v0.34.4: UX Stabilization, Deep Search, Search Mode per Workspace, 8-Dot Spinner, Multiline Prompt, Self-Awareness, Document AI Menu and ReAct Telemetry

1. Goal & Architecture Decisions (ADRs):
- Deep Search: an answer focused exclusively on the conclusion and strictly in the user's language, removing preambles and search narration.
- /search per Workspace: the search depth (auto, fast, deep) persisted per workspace in the SQLite workspaces table.
- 8-Dot Braille Spinner: from 3 to 4 dots of height, with the tick reduced to 125ms (double speed).
- Safe /update: in-TUI execution deprecated to avoid stdout corruption, pointing to 'actx --update'.
- Expandable Multiline Prompt: support for Shift+Enter, Ctrl+J, auto-wrap and dynamic height (3 to 8 lines) with a 2D cursor.
- Workspace identification: a styled [AI - <Workspace>] badge and gray typography (Color::Gray and Color::DarkGray).
- Self-Awareness and Transparent Global Sync: automatic background bootstrap and /sync --force of the Global workspace on every version update, and an incremental check/sync on every start.
- Document AI Menu & Local Models: a new /menu submenu for transparent management of classification and vision (current status: 2D heuristic and ModelRouter <1us) with an option to download/activate local models.
- Enriched ReAct & Reasoning Telemetry: detailed, structured logs in the accordion (triage, grounding, LanceDB+BM25 hybrid search, chunk count, reranking and synthesis with colored badges).

2. Surfaces: actx-cli (TUI, events, ui, app, menu, engine, main), any-context-core-rs (sqlite, engine), actx-agent (deep_search, events, fsm).

3. Tests: unit and integration tests for SQLite isolation, the multiline parser, the Document AI menu, Global self-awareness and ReAct telemetry.
