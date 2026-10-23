---
plan_id: any_context_blueprint_v03214_bm25_safe_deserialization_sync_progress_bar_and_agent_awareness
project: any-context
title: 'Blueprint v0.32.14: Safe BM25 Deserialization, /sync Progress Bar in the TUI and Agent Background Self-Awareness'
version: v0.32.14
status: superseded
author: Levi Silveira
created_at: '2026-10-06T12:53:02'
updated_at: '2026-10-06T14:39:19'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: null
superseded_by: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
parent_plan_id: null
milestones:
- name: 'M1: Bounded, Panic-Safe Deserialization in the Rust Core'
  done: true
- name: 'M2: Index Self-Healing and Local Sanitization'
  done: true
- name: 'M3: Canonical Sync Telemetry Table'
  done: true
- name: 'M4: Reactive Progress Bar in the TUI Footer'
  done: true
- name: 'M5: Native system_status Tool and Agent Self-Awareness'
  done: true
- name: 'M6: Automated Test Suite in Rust and Python'
  done: true
- name: 'M7: Dual-Doc and Cumulative Manual Tests'
  done: true
- name: 'M8: Release v0.32.14 Publication and CI Watch'
  done: true
status_log:
- at: '2026-10-06T13:19:37'
  note: 'Milestone ''M1: Bounded, Panic-Safe Deserialization in the Rust Core'': bincode bounded at 150MB, panic-safe catch_unwind and atomic write through temp-rename implemented'
- at: '2026-10-06T13:19:45'
  note: 'Milestone ''M2: Index Self-Healing and Local Sanitization'': isolation in .corrupt.<timestamp>, in-memory fallback and sanitization of corrupted files implemented'
- at: '2026-10-06T13:19:46'
  note: 'Milestone ''M3: Canonical Sync Telemetry Table'': workspace_sync_status table added to the Rust and Python SQLite, updated by the workers'
- at: '2026-10-06T13:19:47'
  note: 'Milestone ''M4: Reactive Progress Bar in the TUI Footer'': 250ms polling added to the TUI and render_footer with a bar and the up-to-date state'
- at: '2026-10-06T13:19:48'
  note: 'Milestone ''M5: Native system_status Tool and Agent Self-Awareness'': system_status tool and system-status Skill created, instructing the model to report telemetry'
- at: '2026-10-06T13:19:49'
  note: 'Milestone ''M6: Automated Test Suite in Rust and Python'': cargo test --workspace 100% green (127+8+1 tests) and Python unit tests passing'
- at: '2026-10-06T13:19:50'
  note: 'Milestone ''M7: Dual-Doc and Cumulative Manual Tests'': README.md, TECDOC.md (ADR-096 and ADR-097) updated and Scenario 15 added to MANUAL_TESTS.md'
- at: '2026-10-06T13:45:57'
  note: 'Milestone ''M8: Release v0.32.14 Publication and CI Watch'': release v0.32.14 published successfully on GitHub Actions (build 37518214630) with all 12 assets in the Levix-Digital/any-context-releases repository'
- at: '2026-10-06T13:46:01'
  note: Release v0.32.14 finished and published successfully with all 8 milestones met
- at: '2026-10-06T14:39:19'
  note: Superseded by any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.14: Safe BM25 Deserialization, /sync Progress Bar in the TUI and Agent Background Self-Awareness

## 1. Overview & Unified Scope (v0.32.14)

This version covers every requirement and fix raised after v0.32.13:
1. **Pillar 1 (Critical Emergency)**: definitive neutralization of the `memory allocation of 7939688266103746149 bytes failed` crash in BM25 through bounded deserialization (`bincode::DefaultOptions`), panic-safe protection (`catch_unwind`), atomic persistence with a temporary file (`atomic_rename`) and resilient self-healing with quarantine of corrupted indexes.
2. **Pillar 2 (Hexagonal Visual Parity)**: restoration of the reactive, compact `/sync` progress bar in the bottom bar (footer) of the native Rust TUI, fed by the Core's canonical state in real time.
3. **Pillar 3 (System Cognition and Self-Awareness)**: a native `system_status` / self-awareness tool for the agent, allowing it to answer factually when the user asks *"Has everything been indexed?"* or *"What is the status of the background sync?"*.

---

## 2. Architecture Decisions (ADRs)

### ADR-096: Bounded Deserialization, Atomic Persistence and Self-Healing in BM25
- **Bincode with a Strict Limit**: `bincode::DefaultOptions::new().with_limit(100 * 1024 * 1024)`. Any attempt to read anomalous lengths (such as 7.9 exabytes) returns `Err(SizeLimit)` immediately without allocating heap.
- **Panic-Safe Barrier**: `std::panic::catch_unwind` in `BM25Index::load_from_file` to shield the process from allocator aborts.
- **Atomic Write**: `BM25Index::save_to_file` writes to `<path>.tmp.<pid>.<timestamp>` and performs an atomic rename (`std::fs::rename`).
- **Self-Healing Recovery**: if the index is corrupted, it is isolated in `.corrupt.<timestamp>`, a clean index is instantiated and the chunks are rebuilt from the intact LanceDB records.

### ADR-097: Canonical Sync Telemetry in SQLite and Multi-Surface Parity
- **Canonical Table in the Core**: `workspace_sync_status` in `settings.db` (storing workspace, status, percentage, total_items, current_item, stage, item_name, updated_at, error).
- **Surface Parity**: the sync worker (Python or Rust) publishes progress to SQLite.
- **Reactivity in the TUI**: the TUI periodically queries the active workspace's telemetry in SQLite and projects it in the footer (`render_footer`):
  `⚡ Syncing [████░░░░] 50% (15/30 files) │ [F1 / /menu] Menu  ...`
- **Native `system_status` Tool in the Agent**: registered in the agent's ReAct engine, querying the sync state and statistics of the active workspace in SQLite, so the model can answer operational status questions without hallucinating and with zero search token cost.

---

## 3. Execution Milestones

- **M1: Bounded, Panic-Safe Deserialization in the Rust Core (`bm25.rs`)**:
  Allocation limits in bincode, a catch_unwind barrier and atomic writes through a tempfile.
- **M2: Index Self-Healing and Local Sanitization (`pipeline.rs` & `%LOCALAPPDATA%`)**:
  Automatic quarantine of corrupted files and rebuild from LanceDB.
- **M3: Canonical Sync Telemetry Table (`NativeConfigDb` & `ConfigDBStore`)**:
  Creation of the `workspace_sync_status` table and telemetry read/write methods in the Core, in Rust and Python.
- **M4: Reactive Progress Bar in the TUI Footer (`actx-cli/src/tui/ui.rs` & `app.rs`)**:
  Bottom bar update with a dynamic display of background sync progress.
- **M5: Native `system_status` Tool and Agent Self-Awareness (`engine.rs` & `prompt.rs`)**:
  Tool registered in the agent to answer questions such as *"Has everything been indexed?"*.
- **M6: Automated Test Suite in Rust and Python**:
  Tests covering detection of anomalous allocations in BM25, sync telemetry and the `system_status` tool.
- **M7: Dual-Doc (`README.md`, `TECDOC.md`) and Cumulative Manual Tests (`MANUAL_TESTS.md` Scenario 15)**:
  Complete technical documentation and non-truncated acceptance scenarios.
- **M8: Release v0.32.14 Publication & Uninterrupted CI Follow-up**:
  Version bumps, git tags, push and monitoring of the remote workflow until the 12 assets are published in the releases repository.
