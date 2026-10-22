---
plan_id: anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
project: any-context
title: 'Blueprint v0.41.0: Assisted Downloader of Gemma 4 ONNX Weights in the TUI and CLI (actx download <tier>)'
version: v0.41.0
status: superseded
author: Levi Silveira
created_at: '2026-10-15T08:22:07'
updated_at: '2026-10-15T09:17:45'
session_id: df47900f-597a-4d3f-be8b-6ceaac73a0f4
tags:
- gemma4
- onnx
- downloader
- cli
- tui
- progress
- hardware_profiler
- kit_de_models
supersedes: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
superseded_by: anycontext_blueprint_v0420_matriz_tri_cross_platform_de_aceleracao_por_hardware_directml_metalcoreml_cudavulkan_no_actx_onnx
parent_plan_id: null
milestones:
- name: 'Milestone 1: Catalog and Pre-Flight in the Core'
  done: true
- name: 'Milestone 2: actx download CLI Subcommand'
  done: true
- name: 'Milestone 3: /download Slash Command in the TUI and Menu'
  done: true
- name: 'Milestone 4: Automated Tests'
  done: true
- name: 'Milestone 5: Dual-Doc MANUAL_TESTS and Release v0.41.0'
  done: false
status_log:
- at: '2026-10-15T08:54:47'
  note: 'Milestone ''Milestone 1: Catalog and Pre-Flight in the Core'': catalog extended with 4 Gemma 4 tiers and disk and RAM pre-flight checks'
- at: '2026-10-15T08:54:53'
  note: 'Milestone ''Milestone 2: actx download CLI Subcommand'': headless actx download subcommand with a visual bar, MB/s and ETA implemented and tested'
- at: '2026-10-15T08:55:02'
  note: 'Milestone ''Milestone 3: /download Slash Command in the TUI and Menu'': /download command, CommandAction::StartDownload action, footer telemetry and auto-download in the [F1] menu'
- at: '2026-10-15T08:55:12'
  note: 'Milestone ''Milestone 4: Automated Tests'': 100% green tests in the workspace (175 core + 24 commands + 35 actx-cli)'
- at: '2026-10-15T09:17:45'
  note: Superseded by anycontext_blueprint_v0420_matriz_tri_cross_platform_de_aceleracao_por_hardware_directml_metalcoreml_cudavulkan_no_actx_onnx
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Engineering Blueprint: AnyContext v0.41.0
## Assisted Downloader of Gemma 4 ONNX Weights in the TUI and CLI (`actx download <tier>`)

---

### 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.41.0` (successor of `v0.40.0`)
- **Author**: Levi Silveira & Antigravity Assistant
- **Date**: 2026-10-15
- **Location / Headquarters**: Calgary, Alberta, Canada (100% Global Product)
- **Phase**: Master Plan Phase 3 (Model Kit, Local Sovereignty & Assisted Downloader)
- **Status**: Consolidated Technical Proposal (Gate 3 - `dev-cycle-protocol`)
- **Affected Components**:
  - `crates/any-context-core-rs`: extension of the `ONNX_CATALOG` catalog with the Gemma 4 family, the `LanguageModel` category, a disk space pre-flight check and a RAM guardrail.
  - `crates/actx-cli`: the headless CLI subcommand `actx download <tier> [--force] [--check]`, a terminal progress bar, the `/download` slash command in the TUI chat, direct activation by click in the `[F1]` menu and graceful cancellation through `Esc`.
  - `crates/actx-onnx`: resolution of paths, metadata and nominal sizes of the `.onnx` files in the canonical directory `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-126), `tests/MANUAL_TESTS.md` (Scenario 41).

---

## 🏛️ 1. Goal & Architecture Decisions (ADR-126)

### Context & Problem
In version `v0.40.0`, we activated the operational dispatch of local Gemma 4 ONNX (`resolve_lm_provider`) with LM Studio-style hardware guardrails (`validate_model_safety`), guaranteeing that AnyContext does not freeze if the user selects local models without the downloaded weights (ADR-110 graceful offline fallback).

However, to get the `.onnx` weights, the user had to download the files externally or use the generic installer, without direct visual feedback in the TUI or convenience in the CLI terminal.

### Fundamental Architecture Decisions (ADR-126)

1. **Hexagonal Download Parity (CLI + TUI + Menu)**:
   - All the logic for cataloging, CDN/Hugging Face URLs, disk prerequisites and streaming channels with cancellation lives in the Core (`OnnxModelManager` in `any-context-core-rs`).
   - The user can trigger the download in three identical ways:
     - **Headless CLI**: `actx download <tier> [--force]` (ideal for automation and pure-terminal setups).
     - **TUI Chat**: the `/download <tier> [--force]` slash command (with real-time telemetry in the footer and cancellation with `Esc`).
     - **TUI `[F1]` Menu**: on navigating to a model with the `(Install)` status and pressing `[Enter]`, the download starts immediately with feedback in the chat.

2. **Active Pre-Flight Checks (Disk Space & Hardware Guardrails)**:
   - **Disk Check**: before downloading heavy files (1.8GB to 20GB), the system checks the free space on the partition of the models directory (`%LOCALAPPDATA%/actx/models/`). If `free_space < model_size + 500MB`, the download is aborted with an explanatory message about the lack of disk space.
   - **RAM Check**: the `HardwareProfiler` measures the machine's physical RAM. If the model is 🔴 `Unsafe` (e.g. trying to download Gemma 4 32B on a 16GB machine), the system blocks the download with an emphatic OOM diagnosis, requiring the explicit `--force` flag (`actx download 32b --force` or `/download 32b --force`).

3. **Atomic Write and Clean Cancellation**:
   - The download streams over chunked HTTP directly into a temporary file (`<filename>.download.tmp`).
   - The file is only renamed to `<filename>` after 100% of the bytes are complete, with a disk flush.
   - Pressing `Esc` in the TUI or `Ctrl+C` in the CLI aborts the HTTP request immediately and deletes the temporary `.download.tmp` file, guaranteeing **zero corrupted or orphaned files on disk**.

4. **Flexible Tier Name Resolution**:
   - Accepts both full names (`gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`) and intuitive aliases (`compact`, `12b`, `27b`, `32b`, `all`).

```mermaid
flowchart TD
    subgraph "1. Triggering the Download"
        CMD_CLI["CLI: actx download <tier> [--force]"]
        CMD_CHAT["TUI Chat: /download <tier> [--force]"]
        CMD_MENU["[F1] Menu: selecting a model '(Install)'"]
    end

    subgraph "2. Pre-Flight Checks (any-context-core-rs)"
        RESOLVE["OnnxModelManager::resolve_spec(tier)"]
        DISK_CHECK{"Enough Disk Space?<br/>(Free >= Size + 500MB)"}
        RAM_CHECK{"HardwareProfiler::validate_model_safety<br/>🟢 Safe | 🟡 Tight | 🔴 Unsafe"}
        FORCE_CHECK{"--force flag present?"}
        BLOCK_DISK["❌ Error: not enough disk space"]
        BLOCK_RAM["❌ Defensive Block: exceeds physical RAM<br/>(Requires --force)"]
    end

    subgraph "3. Atomic Streaming Pipeline"
        TMP_FILE["Creation of <model>.download.tmp"]
        STREAM["Chunked HTTP Streaming (Reqwest)"]
        PROGRESS["Telemetry: MBs, %, MB/s, ETA, Spinner"]
        CANCEL{"Did the user press [Esc] or Ctrl+C?"}
        CLEANUP["🛑 Aborts the request and removes .download.tmp"]
        FINALIZE["Atomic rename .download.tmp -> .onnx"]
        READY["🎉 Model Ready for Local Inference (v0.40.0)"]
    end

    CMD_CLI --> RESOLVE
    CMD_CHAT --> RESOLVE
    CMD_MENU --> RESOLVE
    RESOLVE --> DISK_CHECK
    DISK_CHECK -->|No| BLOCK_DISK
    DISK_CHECK -->|Yes| RAM_CHECK
    RAM_CHECK -->|🟢 Safe / 🟡 Tight| TMP_FILE
    RAM_CHECK -->|🔴 Unsafe| FORCE_CHECK
    FORCE_CHECK -->|No| BLOCK_RAM
    FORCE_CHECK -->|Yes| TMP_FILE

    TMP_FILE --> STREAM
    STREAM --> PROGRESS
    STREAM --> CANCEL
    CANCEL -->|Yes| CLEANUP
    CANCEL -->|No (100%)| FINALIZE
    FINALIZE --> READY
```

---

## 🏛️ 2. Surface Parity Mapping

| Surface | v0.41.0 Capability / Behavior |
| :--- | :--- |
| **Headless CLI (`actx download`)** | Downloads in the terminal stdout with a graphic progress bar, MB/s and ETA. Supports the `--force` and `--check` flags (to inspect the table of installed models). |
| **TUI Chat (`/download`)** | An interactive slash command with a real-time progress bar in the footer, instant cancellation through `[Esc]` and a notification in the chat history. |
| **TUI `[F1]` Menu** | Models that are not installed trigger the download automatically when pressing `[Enter]` in the model list. |
| **Hardware Guardrails** | Blocks 🔴 models for lack of disk or excess RAM on every surface, with clear diagnostics. |

---

## 📁 3. Affected Files and Lines

1. **`crates/any-context-core-rs/src/ingestion/model_manager.rs`**:
   - Add the `OnnxModelCategory::LanguageModel` variant to the `OnnxModelCategory` enum.
   - Add the 4 Gemma 4 models (`gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`) to the `ONNX_CATALOG` constant.
   - Implement the `OnnxModelManager::resolve_spec(name: &str) -> Option<&'static OnnxModelSpec>` method.
   - Implement the `OnnxModelManager::check_download_safety(spec: &OnnxModelSpec, force: bool) -> Result<(), String>` method.
2. **`crates/actx-cli/src/cli/args.rs`**:
   - Add the `CliCommand::Download { tier: Option<String>, force: bool, check: bool }` variant.
3. **`crates/actx-cli/src/cli/oneshot.rs`**:
   - Implement the `handle_download(tier_opt: Option<&str>, force: bool, check: bool)` function with a terminal progress bar, speed telemetry and Ctrl+C support.
4. **`crates/actx-cli/src/commands/registry.rs`**:
   - Register the `/download` command (aliases `dl`, `get-model`) in `DEFAULT_SLASH_COMMANDS`.
5. **`crates/actx-cli/src/tui/app.rs`**:
   - Add handling for `/download [tier] [--force]` in the chat command dispatcher.
   - Integrate `check_download_safety` before instantiating `start_model_download`.
   - Update the completion notification to suggest `/model <tier>` and mark the model as ready to use.
6. **`crates/actx-cli/src/tui/menu.rs`**:
   - In `handle_menu_enter`: when a local model with the `(Install)` status is selected, trigger `start_model_download` for it with friendly feedback.
7. **Version Bump**:
   - Bump to `v0.41.0` in `version.txt`, `crates/actx-cli/Cargo.toml`, `crates/any-context-core-rs/Cargo.toml`, etc.

---

## 📚 4. Dual-Doc Update Plan (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - New subsection: **"📥 Assisted Download and Management of Local Weights"**.
  - Documentation of the `actx download <tier>`, `actx download --check` and `/download <tier>` commands.
- **`TecDoc` (`TECDOC.md`)**:
  - Formal record of **ADR-126: Assisted Downloader of Gemma 4 ONNX Weights in the CLI and TUI with Pre-Flight Checks and Real-Time Telemetry**.
  - Sequence diagram of the asynchronous download with graceful cancellation through Tokio `watch` and `mpsc` channels.

---

## 🧪 5. Automated Test Strategy & Regression Protection

1. **Unit Tests in `crates/any-context-core-rs/src/ingestion/model_manager.rs`**:
   - `test_resolve_spec_aliases`: validates that `"compact"`, `"12b"`, `"gemma-4-27b"` and `"32b"` resolve to the correct specifications.
   - `test_check_download_safety_disk_and_ram`: validates the safety verdict with different disk and RAM simulations.
2. **Unit Tests in `crates/actx-cli/src/cli/args.rs`**:
   - `test_cli_args_download_command_parsing`: validates the parsing of `actx download 12b`, `actx download --check` and `actx download 32b --force`.
3. **Integration Tests in `crates/actx-cli/tests/`**:
   - `test_download_cancellation_cleans_temporary_file`: guarantees that the temporary `.download.tmp` file is deleted immediately on cancellation.
4. **Full Suite**:
   - Run `cargo test --workspace`, guaranteeing 100% green tests.

---

## 📝 6. Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)

- **Scenario 41: Assisted Downloader of Local Gemma 4 ONNX Models, Disk Validation and Download Telemetry (v0.41.0)**
  - Added strictly cumulatively (keeping scenarios 1 to 40 intact).
  - Detailed steps:
    1. **Model Inspection through the CLI (`actx download --check`)**:
       - Run `actx download --check` and validate a formatted table listing the 4 Gemma 4 tiers, nominal size, RAM requirements and status on disk.
    2. **Pre-Flight Guardrail Validation on Download**:
       - Try `actx download 32b` on a machine with $<44\text{ GB}$ of RAM without `--force` and check the safety block.
       - Try `actx download 32b --force` and check that the download starts.
    3. **Progress Bar Test in the Headless CLI**:
       - Run `actx download compact` and validate the visual progress bar, percentage and MB/s.
    4. **Download Test in the TUI Chat with Cancellation through `Esc`**:
       - Open the TUI (`actx`) and run `/download 12b`.
       - Check the footer telemetry: spinner, model name, transferred MBs and percentage.
       - Press `Esc` during the download and confirm the chat message `🛑 [Download Cancelled]` and the removal of the temporary `.tmp` file.
    5. **Download Test through the `[F1]` Menu**:
       - Open the `[F1]` menu -> `AI Models`, navigate to a model with the `(Install)` status and press `[Enter]`.
       - Check the automatic start of the download with a notification in the chat.

---

## 🔒 7. Secret Hygiene, Database Migration and Rollback Plan

- **Secrets**: zero tokens or secrets involved (public downloads over HTTPS endpoints).
- **Database**: no structural change in SQLite. The files are managed directly in the filesystem at `%LOCALAPPDATA%/actx/models/`.
- **Rollback**: if a regression occurs, the commit on the `dev` branch can be reverted immediately without impact on version `v0.40.0`.

---

## ⏱️ 8. Step-by-Step Execution (Milestones 1 to 5)

| Milestone | Description | Deliverables |
| :--- | :--- | :--- |
| **Milestone 1** | Gemma 4 Catalog & Pre-Flight Checks in the Core | `ONNX_CATALOG` extension, the `LanguageModel` enum, `resolve_spec` and `check_download_safety` |
| **Milestone 2** | `actx download` CLI Subcommand & Progress Bar | `CliCommand::Download` in `args.rs` and `handle_download` in `oneshot.rs` |
| **Milestone 3** | `/download` Slash Command in the TUI & `[F1]` Menu Shortcut | Registration in `registry.rs`, dispatcher in `app.rs` and action in `menu.rs` |
| **Milestone 4** | Automated Tests & Suite Validation | Catalog unit tests, argument parsing and cancellation tests |
| **Milestone 5** | Dual-Doc, MANUAL_TESTS (Scenario 41) & Release v0.41.0 | Update of `README.md`, ADR-126 in `TECDOC.md`, `tests/MANUAL_TESTS.md` and release |
