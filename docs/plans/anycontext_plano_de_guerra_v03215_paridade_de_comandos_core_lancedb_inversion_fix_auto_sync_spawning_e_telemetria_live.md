---
plan_id: anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
project: any-context
title: 'War Plan v0.32.15: Core Command Parity, LanceDB Inversion Fix, Auto-Sync Spawning and Live Telemetry'
version: v0.32.15
status: completed
author: Levi Silveira
created_at: '2026-10-06T16:34:14'
updated_at: '2026-10-06T17:00:12'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags:
- war_plan
- parity
- rust_commands
- auto_sync
- telemetry
- lancedb_fix
- modal_loop_fix
supersedes: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: Critical Fixes and LanceDB'
  done: true
- name: 'M2: Reactive Telemetry and Spinner'
  done: true
- name: 'M3: Hexagonal Parity of Core Commands'
  done: true
- name: 'M4: Automated Test Suite'
  done: true
- name: 'M5: Dual-Doc and Scenario 16'
  done: true
- name: 'M6: Gate 8 Verification and Release v0.32.15'
  done: true
status_log:
- at: '2026-10-06T16:34:32'
  note: 'Milestone ''M1: Critical Fixes and LanceDB'': fixed the LanceDB argument order (count_records), resolved the incremental modal loop and sanitized Windows paths (\\?\\).'
- at: '2026-10-06T16:34:33'
  note: 'Milestone ''M2: Reactive Telemetry and Spinner'': dynamic calculation in db_store.py/sqlite.rs, scanning callbacks in the crawler/ingestor and a 250ms Braille spinner in the TUI.'
- at: '2026-10-06T16:34:34'
  note: 'Milestone ''M3: Hexagonal Parity of Core Commands'': spawn_sync_worker, auto-spawn in /folder and /web, /keys audit/modal, /config dashboard, /purge metadata cleanup, /vision, real /logs tail and /inspect --full.'
- at: '2026-10-06T16:34:35'
  note: 'Milestone ''M4: Automated Test Suite'': 127 Core unit tests, 15 command tests in command_tests.rs, 14 CLI tests in cli_tests.rs, and Python integration tests 100% green.'
- at: '2026-10-06T16:34:37'
  note: 'Milestone ''M5: Dual-Doc and Scenario 16'': complete documentation in README.md, TECDOC.md (ADR-098 and ADR-099) and cumulative, non-truncated Scenario 16.'
- at: '2026-10-06T17:00:12'
  note: 'Milestone ''M6: Gate 8 Verification and Release v0.32.15'': CI/CD on GitHub Actions finished 100% green (Run ID: 37541081744). All 12 binaries and scripts published successfully in Levix-Digital/any-context-releases.'
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.15: Live Scanning Progress Bar, Core Command Parity & TUI Telemetry

## 1. Context & Root Cause
In version `v0.32.14`, three critical production failures were identified after migrating the Core to Rust:
1. **Parameter Inversion in LanceDB**:
   - `NativeLanceStore::count_records` took `workspace` as the first parameter and `table_name` as the second. In the CLI (`crates/actx-cli/src/engine.rs`), the calls passed `count_records(Some("workspace_chunks"), Some(&ws))`, inverting the arguments.
   - Consequence: the system queried a non-existent table and reported "0 chunks", making the AI falsely answer: *"Yes, all pages were scanned and indexed... However, there are currently no indexed chunks available."*, even with more than 45,000 chunks indexed in LanceDB.
2. **Incremental Sync Modal Loop**:
   - In the TUI `/menu` (`crates/actx-cli/src/tui/app.rs`), selecting incremental sync produced the `/sync incremental` command. The dispatcher in `engine.rs` compared strictly against `"--incremental"`, ignoring the command and reopening the menu again and again in a loop without triggering the worker.
3. **Footer Telemetry Freeze (`⚡ Syncing [scanning...] │ `)**:
   - While scanning/crawling large portals (e.g. 2,516 URLs), SQLite kept the initial static string `"[scanning...]"` because `db_store.py` did not compute `progress_bar` when `stage="scanning"`.
   - The reader in `crates/any-context-core-rs/src/storage/sqlite.rs` did not defensively recompute the text when it found the static flag, and the TUI had no tick-based animation.
4. **Functional Parity Gaps in the Rust Core Commands**:
   - A deep audit between the old Python TUI and the Rust hexagonal architecture revealed commands with static behavior or no worker triggering:
     - `/folder add <path>` added the path to SQLite but did not trigger the background sync worker.
     - `/web add <url>` registered the URL but did not start the crawler.
     - Canonical Windows paths were saved with the extended `\\?\` prefix, breaking the comparison in `/folder remove`.
     - `/keys` without arguments did not open the interactive TUI modal nor update the process environment variables.
     - `/config` without arguments did not show the general settings dashboard.
     - `/purge` did not clear the SQLite `file_metadata` table, causing inconsistencies on re-scan.
     - `/logs` returned a static message instead of reading the last lines of `sync_<workspace>.log`.
     - `/inspect` did not support the `--full` flag nor detail the base's total chunk count.

---

## 2. Architecture Decisions & Solution (ADR-098 & ADR-099)

### 2.1 Canonical LanceDB Fix and Incremental Sync
- **Corrected LanceDB Signature**: `count_records(Some(&ws), Some("workspace_chunks"))` in every vector count route in the Core and the CLI.
- **Argument Normalization in `/sync`**: `execute_sync` accepts `incremental`, `--incremental`, `-i`, and the TUI menu selection dispatches `["--incremental"]`.

### 2.2 Reactive Telemetry and Dynamic Braille Spinner
- **Automatic Calculation & Unfreezing in SQLite (`db_store.py`)**: `format_sync_progress_bar` computes the dynamic Unicode bar even in the discovery phase (`[scanning... X urls/files found]`).
- **Automatic Recalculation in the Rust Core (`sqlite.rs`)**: in `get_sync_status`, if the string is static but counters exist, the bar is rebuilt dynamically.
- **Dynamic Braille Spinner in the TUI (`app.rs`, `mod.rs`, `ui.rs`)**: a 250ms tick cycle with continuous animation (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`).

### 2.3 Full Hexagonal Parity of the Core Commands (`crates/any-context-core-rs/src/commands/engine.rs`)
- **`spawn_sync_worker`**: a reusable helper extracted to start the background sync worker and update the telemetry state in SQLite.
- **`/folder`**: automatically spawns the sync worker after adding a directory; sanitizes the Windows `\\?\` prefix in `execute_folder` and defensively in `remove_workspace_folder`.
- **`/web`**: automatically spawns the crawler worker after adding a URL.
- **`/keys`**: without arguments opens the `keys` modal in the TUI; with `audit` shows the report of configured keys; when setting a key, it persists it in SQLite `api_keys`, sets the environment variable through `std::env::set_var` and emits `CommandAction::RebuildAgent`.
- **`/config`**: without arguments renders the full settings panel; with a key and value it applies `app_settings`.
- **`/purge`**: calls `clear_workspace_file_metadata` in SQLite besides deleting vectors in LanceDB; resets the sync status to `idle`.
- **`/vision`**: sets `enable_vision_llm` in `app_settings`.
- **`/logs`**: dynamically reads the last N lines of the real `sync_<workspace>.log` file on disk.
- **`/inspect`**: queries the document taxonomy and shows a preview, supporting the `--full` flag.

---

## 3. Execution Milestones
- **M1: Critical Fixes & LanceDB**: LanceDB parameter order fixed, incremental modal loop resolved and Windows paths sanitized (`\\?\`).
- **M2: Reactive Telemetry & Spinner**: dynamic calculation in `db_store.py`/`sqlite.rs`, granular scanning callbacks in the crawler/ingestor and a 250ms Braille spinner in the TUI.
- **M3: Hexagonal Parity of Core Commands**: `spawn_sync_worker`, auto-spawn in `/folder` and `/web`, `/keys` audit/modal, `/config` dashboard, `/purge` metadata cleanup, `/vision`, real `/logs` tail and `/inspect --full`.
- **M4: Automated Test Suite**: 127 Core unit tests, 15 command tests in `command_tests.rs`, 14 CLI tests in `cli_tests.rs`, and Python integration tests 100% green.
- **M5: Dual-Doc & Scenario 16 in MANUAL_TESTS.md**: complete documentation in `README.md`, `TECDOC.md` (ADR-098 and ADR-099) and cumulative, non-truncated Scenario 16.
- **M6: Gate 8 Verification & Release v0.32.15**: validation through `verify_gate.py`, commit `80c5a90`, tag `v0.32.15`, push to `dev` and `main`, and CI watch of the 12 release binaries.
