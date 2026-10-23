---
plan_id: any_context_blueprint_v03210_isolamento_hermetico_de_buffers_de_chat_e_historico_por_workspace_na_tui_nativa
project: any-context
title: 'Blueprint v0.32.10: Hermetic Isolation of Chat Buffers and History per Workspace in the Native TUI'
version: v0.32.10
status: completed
author: Levi Silveira
created_at: '2026-10-04T13:28:33'
updated_at: '2026-10-04T15:54:22'
session_id: 09953e71-964e-4480-b790-66a5d4797cd7
tags:
- workspace
- chat_isolation
- tui
- ratatui
- view_buffer
- session_store
supersedes: any_context_normalizacao_definitiva_workspace_folders_e_ci_watch
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: Per-Workspace Buffer Structure in App'
  done: false
- name: 'M2: Atomic switch_to_workspace Method in App'
  done: false
- name: 'M3: Integration with clear and reset-memory'
  done: false
- name: 'M4: Automated Test Suite in Rust'
  done: false
- name: 'M5: Dual-Doc Update and Manual Tests'
  done: false
- name: 'M6: Release v0.32.10 and CI Watch'
  done: false
status_log:
- at: '2026-10-04T15:54:22'
  note: Implemented and published in release v0.32.10 with ADR-092 and Scenario 11
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint v0.32.10: Hermetic Isolation of Chat Buffers and History per Workspace in the Native TUI (crates/actx-cli)

## 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.32.10`
- **Status**: Active (`active`)
- **Date**: 2026-10-04
- **Conversation ID**: `09953e71-964e-4480-b790-66a5d4797cd7`
- **Architecture Principle**: Virtual Tab Workspace Isolation pattern (ADR-052 & Hexagonal Ports & Adapters)

---

## 🔍 1. Problem Diagnosis & Root Cause
In the native Rust TUI (`crates/actx-cli/src/tui/app.rs`), the application state kept:
- `pub input_history: HashMap<String, Vec<String>>` (correctly isolated per workspace for Up/Down prompt history navigation).
- **`pub chat_history: Vec<ChatMessageItem>`** (monolithic and shared by every workspace).

When switching workspaces through `/workspace <target>`, `/switch <target>` or the modal menu (`[F1] -> Workspaces`):
1. The previous workspace's `chat_history` was not kept in any map.
2. `self.active_workspace` changed to the new workspace.
3. `load_session_history_for_workspace()` ran and called `self.chat_history.push(...)` without clearing the previous history.
4. If the new workspace had no previous messages (e.g. newly created), no message was added, and the screen kept showing all of the previous workspace's messages, with only a footer notice about the switch.
5. When switching back, the messages were pulled again and duplicated at the end of the single vector.

---

## 🏛️ 2. Solution Architecture (Virtual Tab Workspace Isolation)

```mermaid
flowchart TD
    subgraph TUI_State["Reactive TUI State (crates/actx-cli/src/tui/app.rs)"]
        ActiveWS["self.active_workspace"]
        ScreenBuffer["self.chat_history (Active Viewport)"]
        BufferMap["self.workspace_chat_buffers: HashMap<String, Vec<ChatMessageItem>>"]
        InputHist["self.input_history: HashMap<String, Vec<String>>"]
    end

    subgraph SwitchLifecycle["Switch Cycle: /switch or /workspace (TargetWS)"]
        SaveOld["1. Save self.chat_history into workspace_chat_buffers[OldWS]"]
        UpdateWS["2. self.active_workspace = TargetWS"]
        ResetViewport["3. Reset scroll, stream_buffer and thinking_buffer"]
        CheckCache{"4. TargetWS already in workspace_chat_buffers?"}
        RestoreCache["5a. self.chat_history = workspace_chat_buffers[TargetWS]"]
        QuerySQLite["5b. Query SqliteSessionStore (ws_TargetWS)"]
        LoadSQL{"Has messages in SQLite?"}
        SetSQLMsgs["6a. self.chat_history = SQLite messages"]
        SetWelcome["6b. self.chat_history = TargetWS Welcome Message"]
        RebuildAgent["7. Rebuild actx_agent for TargetWS and reload status"]
    end

    ActiveWS --> SaveOld
    ScreenBuffer --> SaveOld
    SaveOld --> BufferMap
    SaveOld --> UpdateWS
    UpdateWS --> ResetViewport
    ResetViewport --> CheckCache
    CheckCache -- Yes --> RestoreCache
    CheckCache -- No --> QuerySQLite
    QuerySQLite --> LoadSQL
    LoadSQL -- Yes --> SetSQLMsgs
    LoadSQL -- No --> SetWelcome
    RestoreCache --> RebuildAgent
    SetSQLMsgs --> RebuildAgent
    SetWelcome --> RebuildAgent
```

---

## 🎯 3. Execution Milestones

- **M1: Per-Workspace Buffer Structure in `App`**:
  - Add `pub workspace_chat_buffers: std::collections::HashMap<String, Vec<ChatMessageItem>>` to `App`.
  - Create a helper function `create_welcome_message` to standardize per-workspace welcome headers.
- **M2: Atomic `switch_to_workspace` Method in `App`**:
  - Save the source workspace's `chat_history` into `workspace_chat_buffers`.
  - Restore the target workspace's history from the session cache or load it from `SqliteSessionStore` (`ws_<target>`).
  - If the workspace is new, render the workspace's canonical welcome message with its model, grounding and search status.
  - Reset the ephemeral viewport variables (`current_stream_buffer`, `current_thinking_buffer`, `scroll_offset`, `auto_scroll`).
- **M3: Integration with `/clear` and `/reset-memory`**:
  - On `/clear`: clear the on-screen buffer and remove the active workspace's cache entry from `workspace_chat_buffers`.
  - On `/reset-memory`: attach the `CommandAction::ClearChat` action in `engine.rs` so that resetting memory in SQLite also clears the active workspace's screen.
- **M4: Automated Test Suite in Rust**:
  - Create unit tests in `crates/actx-cli/tests/cli_tests.rs` validating that when switching between `WorkspaceA` and `WorkspaceB`:
    - `WorkspaceA` keeps its messages.
    - `WorkspaceB` starts clean or with its own history.
    - Returning to `WorkspaceA` restores exactly its messages, with no leakage or duplication.
- **M5: Dual-Doc Update & Manual Test Scenario**:
  - Sync `TECDOC.md` (ADR-092) and `README.md`.
  - Add Scenario 11 (or extend Scenario 10) in `tests/MANUAL_TESTS.md`, detailing the step-by-step validation of chat screen isolation.
- **M6: Gate Validation & Release `v0.32.10` Preparation**:
  - Version bump in every manifest, binary builds and release follow-up.
