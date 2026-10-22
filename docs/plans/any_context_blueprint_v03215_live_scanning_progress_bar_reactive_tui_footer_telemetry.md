---
plan_id: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
project: any-context
title: 'Blueprint v0.32.15: Live Scanning Progress Bar & Reactive TUI Footer Telemetry'
version: v0.32.15
status: superseded
author: Levi Silveira
created_at: '2026-10-06T14:39:19'
updated_at: '2026-10-06T16:34:14'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags:
- sync
- progress_bar
- scanning
- tui
- telemetry
- braille_spinner
supersedes: any_context_blueprint_v03214_bm25_safe_deserialization_sync_progress_bar_and_agent_awareness
superseded_by: anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
parent_plan_id: null
milestones:
- name: 'M1: Dynamic calculation and unfreezing in db_store.py and entrypoint.py'
  done: false
- name: 'M2: Defensive recalculation and discovered items in sqlite.rs'
  done: false
- name: 'M3: Scanning progress emitted by the crawler and the ingestor'
  done: false
- name: 'M4: Dynamic Braille spinner and 250ms animation in the TUI'
  done: false
- name: 'M5: Automated tests in Rust and Python'
  done: false
- name: 'M6: Dual-Doc and Scenario 16 in MANUAL_TESTS.md'
  done: false
- name: 'M7: Gate 8 Verification and Release v0.32.15 Publication'
  done: false
status_log:
- at: '2026-10-06T16:34:14'
  note: Superseded by anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.15: Live Scanning Progress Bar & Reactive TUI Footer Telemetry

## 1. Context & Root Cause
In version `v0.32.14`, the TUI footer telemetry froze in the static state `⚡ Syncing [scanning...] │ ` during the whole sync of large web portals (e.g. 2,516 pages), even when the agent, through `system_status`, correctly reported that 1,440 pages had already been scanned:

### Root Cause Identified:
1. **No Progress Bar Calculation in the Python Worker (`entrypoint.py` / `db_store.py`)**:
   `_worker_progress` called `store.update_sync_status(current, total, stage, item_name)` without providing `progress_bar`.
   In SQLite `update_sync_status`, the SQL ran:
   `progress_bar = COALESCE(excluded.progress_bar, workspace_sync_status.progress_bar)`
   Since `excluded.progress_bar` was `NULL`, SQLite kept the initial value `"[scanning...]"` written by Rust when the worker booted.
2. **Static Evaluation in the Rust Reader (`sqlite.rs`)**:
   In `get_sync_status`, Rust checked `progress_bar.unwrap_or_else(...)`. Since `progress_bar` already held `"[scanning...]"`, it never recalculated the bar from the `current_item` and `total_items` fields.
3. **No Telemetry During Discovery/Scanning**:
   In the initial sweep phase (parsing `sitemap.xml`, BFS over domain links and scanning local files), the system ran without emitting granular callbacks, leaving `total = 0` and `current = 0` with no dynamic indicator.

---

## 2. Architecture Decisions & Solution (ADR-098)
1. **Automatic Calculation & Unfreezing in SQLite (`db_store.py`)**:
   In `update_sync_status`, when `progress_bar` is not provided, it is computed right away through `format_sync_progress_bar(current, total, stage)`.
   In the SQL, `progress_bar = excluded.progress_bar` updates the column on every processed item.
2. **Automatic Recalculation in the Rust Core (`sqlite.rs`)**:
   In `get_sync_status`, when the stored string is static (`"[scanning...]"`, `"[crawling...]"`, `"[calculating...]"`) but `total_items > 0` or `current_item > 0`, Rust dynamically recalculates the Unicode block bar with `format_sync_progress_bar`.
3. **Granular Telemetry in the Scanning/Discovery Phase (`web_crawler.py` & `local_folder_ingestor.py`)**:
   The `discover_site_urls` function now receives `progress_callback` and emits updates as sitemap URLs and page links are discovered (`current > 0`, `total = 0`, `stage = "scanning"`).
   The progress format shows `[scanning... X urls/files found]`.
4. **Dynamic Braille Spinner in the TUI (`app.rs`, `mod.rs`, `ui.rs`)**:
   `App` keeps an asynchronous tick counter (`tick_count: u64`) incremented every 250ms.
   The TUI footer animates a Braille spinner (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`) next to the scanning text, giving the user immediate, continuous visual feedback that the system is alive and processing.

---

## 3. Execution Milestones
- **M1**: Dynamic calculation and unfreezing in `db_store.py` and `entrypoint.py`.
- **M2**: Defensive recalculation and support for discovered items in `sqlite.rs`.
- **M3**: Scanning progress emitted in `web_crawler.py`, `web_scheduler.py` and `local_folder_ingestor.py`.
- **M4**: Dynamic Braille spinner and 250ms animation in the TUI footer (`app.rs`, `mod.rs`, `ui.rs`).
- **M5**: Automated tests in Rust (`cargo test --workspace`) and Python (`unittest`).
- **M6**: Dual-Doc (`README.md`, `TECDOC.md` ADR-098) and Scenario 16 in `MANUAL_TESTS.md`.
- **M7**: Gate 8 verification, `v0.32.15` bump, commit, tag, push and CI/CD monitoring until all 12 binaries are published.
