---
plan_id: any_context_blueprint_v03213_clean_screen_startup_and_long_term_memory_preservation
project: any-context
title: '[any-context] Blueprint v0.32.13: Clean Screen Startup in the TUI and Long-Term Memory Decoupling'
version: v0.32.13
status: superseded
author: Levi Silveira
created_at: '2026-10-06T11:52:26'
updated_at: '2026-10-06T12:51:14'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: any_context_blueprint_v03212_test_isolation_sandbox_universal_option_hinting_skill_based_global_self_knowledge
superseded_by: any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
parent_plan_id: null
milestones:
- name: 'M1: Clean Screen Buffer Initialization'
  done: true
- name: 'M2: Long-Term Memory Decoupling and Persistence'
  done: true
- name: 'M3: Enhanced /history Command Inspectability'
  done: true
- name: 'M4: Automated Tests Suite'
  done: true
- name: 'M5: Dual-Doc and Manual Tests'
  done: true
- name: 'M6: Production Release v0.32.13'
  done: true
status_log:
- at: '2026-10-06T12:04:28'
  note: 'Milestone ''M1: Clean Screen Buffer Initialization'': Removed load_session_history_for_workspace on startup and switch_to_workspace'
- at: '2026-10-06T12:04:37'
  note: 'Milestone ''M2: Long-Term Memory Decoupling and Persistence'': Preserved SQLite actx_session_messages for agent retrieval'
- at: '2026-10-06T12:04:38'
  note: 'Milestone ''M3: Enhanced /history Command Inspectability'': Enhanced /history to inspect SQLite long-term turns and /history --clear to wipe SQLite'
- at: '2026-10-06T12:04:39'
  note: 'Milestone ''M4: Automated Tests Suite'': All tests passing, added test_clean_screen_startup_and_long_term_memory_preservation'
- at: '2026-10-06T12:06:05'
  note: 'Milestone ''M5: Dual-Doc and Manual Tests'': README.md, TECDOC.md Section 89 ADR-095 and tests/MANUAL_TESTS.md Scenario 14 updated'
- at: '2026-10-06T12:35:52'
  note: 'Milestone ''M6: Production Release v0.32.13'': v0.32.13 released and published with 12 assets on GitHub Releases, local binary verified'
- at: '2026-10-06T12:51:14'
  note: Superseded by any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.13: Clean Screen Buffer on Session Startup and Long-Term Memory Preservation

## Context & Problem
On Linux and the other platforms, when starting the application or switching to workspaces not yet visited, the previous session was dumped straight into the screen buffer (chat_history), cluttering the terminal. Each new session must start with a fully clean screen (only the initial welcome banner), while the history stays preserved in SQLite as long-term memory available to the agent.

## Architectural Solution
1. M1: Remove the automatic hydration of past messages from SQLite into chat_history in App::new() and switch_to_workspace(). The screen starts empty with the welcome banner. The in-memory cache (workspace_chat_buffers) keeps the virtual tabs alive strictly during the running session.
2. M2: Keep all messages in SqliteSessionStore (actx_session_messages), so the agent loads the previous conversational context when answering questions about past sessions.
3. M3: Update /history to inspect the SQLite long-term memory without cluttering the screen, and allow /history --clear to wipe it completely from the database.
4. M4: Automated Rust tests covering an empty screen at boot with a populated SQLite.
5. M5: Dual-Doc (README.md, TECDOC.md with ADR-095) and MANUAL_TESTS.md Scenario 14.
6. M6: Release v0.32.13 and CI Watch until publication.
