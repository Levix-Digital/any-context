---
plan_id: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_temas_e_auto_consciencia_global
project: any-context
title: 'Blueprint v0.34.4: UX Stabilization, Deep Search Conclusion, Search per Workspace, 8-Dot Spinner, Multiline Prompt, Themes and Global Self-Awareness'
version: v0.34.4
status: superseded
author: Levi Silveira
created_at: '2026-10-10T08:34:24'
updated_at: '2026-10-10T08:43:12'
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
supersedes: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_direta_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_e_temas
superseded_by: anycontext_blueprint_v0344_estabilizacao_ux_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_auto_consciencia_menu_document_ai_e_telemetria_react_enriquecida
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
- name: 'M8: Automated Test Suite'
  done: false
- name: 'M9: Dual-Doc and Scenario 27 in MANUAL_TESTS'
  done: false
- name: 'M10: Gate 8 and Release v0.34.4'
  done: false
status_log:
- at: '2026-10-10T08:43:12'
  note: Superseded by anycontext_blueprint_v0344_estabilizacao_ux_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_auto_consciencia_menu_document_ai_e_telemetria_react_enriquecida
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint AnyContext v0.34.4: UX Stabilization, Deep Search Synthesis, Search Mode per Workspace, 8-Dot Spinner, Multiline Prompt, Themes and Global Self-Awareness

1. Goal & Architecture Decisions (ADRs):
- Deep Search: an answer focused exclusively on the conclusion and strictly in the user's language, removing preambles and search narration.
- /search per Workspace: the search depth (auto, fast, deep) persisted per workspace in the SQLite workspaces table.
- 8-Dot Braille Spinner: from 3 to 4 dots of height, with the tick reduced to 125ms (double speed).
- Safe /update: in-TUI execution deprecated to avoid stdout corruption, pointing to 'actx --update'.
- Expandable Multiline Prompt: support for Shift+Enter, Ctrl+J, auto-wrap and dynamic height (3 to 8 lines) with a 2D cursor.
- Workspace identification: a styled [AI - <Workspace>] badge and gray typography (Color::Gray and Color::DarkGray).
- Self-Awareness and Transparent Global Sync: automatic background bootstrap and /sync --force of the Global workspace on every version update, and an incremental check/sync on every start, so that the agent never loses access to manuals and documentation.

2. Affected surfaces: actx-cli (TUI, events, ui, app, engine, main), any-context-core-rs (sqlite, engine), actx-agent (deep_search).

3. Tests: unit and integration tests for SQLite isolation, the multiline parser, Global self-awareness and command regression.
