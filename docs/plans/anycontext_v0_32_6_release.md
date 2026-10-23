---
plan_id: anycontext_v0_32_6_release
project: any-context
title: Multi-Turn Session Persistence Fix, TUI Error Surfacing and Search Depth Concealment (v0.32.6)
version: v0.32.6
status: completed
author: Levi Silveira
created_at: '2026-10-02T15:19:59'
updated_at: '2026-10-02T15:20:11'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: Conversational Boundary
  done: true
- name: Sqlite Scrubbing
  done: true
- name: TUI Error Surfacing
  done: true
- name: UI Concealment
  done: true
- name: Dual-Doc
  done: true
- name: CI/CD Release
  done: true
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Engineering & Release Plan: v0.32.6

## 🎯 Goal
Fix the lost answer in multi-turn dialogs after tool execution (HTTP 400 error from orphaned tool messages), remove the silent freeze on errors in the TUI and temporarily hide the `[Search: ...]` indicator and related commands until RFC-042 Deep Search is fully implemented.

## 🏛️ Architecture Decisions
- **ADR-088: High-Level Conversational Session Boundary & Orphaned Tool Scrubbing**:
  - `persisted_new_messages` stores only the high-level dialog between user and assistant (`ChatMessage::user` and `ChatMessage::assistant(&final_content)`). The internal tool scratchpad stays only in `working_messages` while the current turn runs.
  - `SqliteSessionStore`: automatic cleanup on startup and an SQL filter in `get_messages_sync`.
- **ADR-089: TUI Error Surfacing Guarantee & Temporary Search Depth UI Concealment**:
  - Explicit display of `AgentEvent::Error` in `chat_history` as `MessageRole::System` with `❌ Error: {err}`, keeping `AppStatus::Error`.
  - Temporary hiding of `[Search: ...]` from the header, the main menu (`build_main_menu`) and the command palette (`COMMAND_REGISTRY`).

## 🏁 Completed Milestones
1. Conversational Boundary in actx-agent FSM (`crates/actx-agent/src/fsm.rs`)
2. SqliteSessionStore Scrubbing & Filtering (`crates/actx-agent/src/session.rs`)
3. TUI Error Surfacing (`crates/actx-cli/src/tui/app.rs`)
4. UI Concealment of Search Depth (`crates/actx-cli/src/tui/ui.rs`, `menu.rs`, `registry.rs`)
5. Dual-Doc Updates (`TECDOC.md`, `tests/MANUAL_TESTS.md` Scenario 8)
6. CI/CD Build & Publish to `Levix-Digital/any-context-releases` (Tag: `v0.32.6`)
