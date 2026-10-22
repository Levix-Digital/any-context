---
plan_id: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_download_guiado_de_modelos
project: any-context
title: 'Blueprint v0.34.9: Aurora Boreal Harmonization, ReAct Auto-Scroll, Update UX & Guided Model Download'
version: v0.34.9
status: superseded
author: Levi Silveira
created_at: '2026-10-10T12:52:04'
updated_at: '2026-10-10T12:57:16'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- tui
- aurora_theme
- borderless_chat
- reasoning_autoscroll
- update_ux
- model_download_progress
- guided_models_menu
supersedes: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_na_tui_auto_scroll_react_e_mensagens_profissionais_de_atualizacao
superseded_by: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_cancelamento_de_download_onboarding_wizard
parent_plan_id: null
milestones:
- name: 'M1: Aurora Color Cleanup'
  done: false
- name: 'M2: Borderless Conversation'
  done: false
- name: 'M3: ReAct Auto-Scroll'
  done: false
- name: 'M4: Friendly actx --update Message'
  done: false
- name: 'M5: Real ONNX Download Progress'
  done: false
- name: 'M6: Submenus Guided by Role'
  done: false
- name: 'M7: Tests and Dual-Doc'
  done: false
status_log:
- at: '2026-10-10T12:57:16'
  note: Superseded by anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_cancelamento_de_download_onboarding_wizard
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint v0.34.9: Aurora Boreal Harmonization, ReAct Auto-Scroll, Update UX & Guided Model Download

**Project**: AnyContext (`actx`)  
**Target Version**: `v0.34.9`  
**Date**: 2026-10-10  
**Status**: 📋 Expanded Plan for Validation  

---

## 🎯 1. Scope of Adjustments

1. **Aurora Boreal Color Harmonization in the TUI**:
   - Remove yellow (`Amber`) and red (`Crimson`) from informational and navigation components (Thinking, Menu Shortcuts, ReAct Border, Active Workspace, Fast RAG, Deep Search).
   - Replace them with the corresponding colors of the Aurora Boreal palette: **Glacial Cyan** (`#00F0FF`), **Brand Emerald** (`#00E5A3`), **Cosmic Violet** (`#A855F7`) and **Arctic Sky Blue** (`#38BDF8`).
   - Strictly restrict yellow and red to:
     - **Red (`#EF4444`)**: the destructive `[Esc/Ctrl+C] Exit` shortcut (requested as an explicit exception), critical execution errors (`[Tool Error]`, `AppStatus::Error`) and unrecoverable offline folders (`⚠️ [Sync: X folder(s) unreachable]`).
     - **Yellow (`#F59E0B`)**: real pending sync warnings (`⚡ [Sync: Changes detected (/sync)]`).

2. **"Conversation" Box without Borders or Title (Minimalist Canvas)**:
   - Run the visual test removing the outer border and the `Conversation` title.
   - Gain +2 lines of usable height and +2 columns of usable width for the chat flow.
   - Keep a minimalist paused-scroll indicator when the user scrolls up the conversation.

3. **Smooth Auto-Scroll in the ReAct & Reasoning Box**:
   - Fix the missing scroll calculation in `render_accordion`.
   - Implement wrapping, height limits and automatic auto-scroll to the end as new tokens and tool calls are injected.

4. **Real-Time Visual Feedback of ONNX Model Downloads**:
   - Solve the "blind" download: today the user only gets the initial message and does not know the progress.
   - Download streaming with percentage, downloaded MB and speed.
   - A dynamic indicator in the TUI bottom status bar:
     `📥 [Download: 45% (94.5/210 MB) - Laya AI]`
   - On completion, post a notification in the chat:
     `✔ [Download Complete] Laya AI (mmBERT INT8 ONNX) installed and active!`
   - On failure (network offline/timeout):
     `❌ [Download Failed] <reason>. AnyContext keeps working through the deterministic fallback.`

5. **Model Navigation Guided by Specific Role in the Menu**:
   - Remove the doubt about "having to download one or all of them".
   - Split the `Document AI & Local Models` menu into clear-purpose categories:
     1. 📂 **Document Classification (Ingestion)** (Instant Heuristic vs Laya AI 210MB)
     2. 👁️ **Scan & Rasterized Page Classification** (2D Heuristic vs MobileNetV4 14MB)
     3. 🔍 **Query Classification (Fast vs Deep)** (Deterministic RFC-042 vs BGE-Small 35MB)
     4. 🖼️ **Document Vision & Visual OCR (Air-Gapped)** (Multimodal Cloud vs SmolVLM 480MB vs Moondream2 1.1GB)
   - Explain in the header of each item:
     - **What it is for**.
     - **Whether a download is needed** (stressing that AnyContext already comes 100% ready and working by default without downloading anything).
     - **Whether the models are alternatives** (e.g. in the Local Vision submenu, choose at most ONE model).

6. **Friendly, Professional Messages in `actx --update` (Benchmark of the Big Players)**:
   - Handle in a friendly, informative way HTTP 404 download failures caused by the natural build latency of the release on GitHub Actions (15-40 min after the tag push).
   - Remove raw network messages (`ureq HTTPS download request failed ... 404`) and confusing fallback URLs.
   - Adopt the professional, empathetic and informative tone of established tools (GitHub CLI, Rustup, Homebrew).

---

## 🗺️ 2. Architecture of the Guided Model Navigation (Proposed Menu)

```mermaid
flowchart TD
    Menu["/menu: Document AI & Models"] --> Sub1["📂 1. Document Classification\n(Identifies the file type at indexing)"]
    Menu --> Sub2["👁️ 2. Scan Classification\n(Detects scanned PDFs / photos)"]
    Menu --> Sub3["🔍 3. Question Routing\n(Classifies complexity Fast vs Deep)"]
    Menu --> Sub4["🖼️ 4. Document Vision (Visual OCR)\n(Extracts visual data from diagrams/tables)"]
    Menu --> Info["🛡️ 5. Storage & Fallback Summary\n(Status of the models on disk)"]

    Sub1 --> S1_Def["⚡ Rust Heuristic (<1µs / 0MB) [Active Default]"]
    Sub1 --> S1_Onnx["📥 Download Laya AI (~210 MB) [Optional - High Precision]"]

    Sub2 --> S2_Def["📐 2D Spatial Heuristic (<5MB) [Active Default]"]
    Sub2 --> S2_Onnx["📥 Download MobileNetV4 (~14 MB) [Optional - Ultralight]"]

    Sub3 --> S3_Def["⚡ Deterministic RFC-042 (<1µs / 0MB) [Active Default]"]
    Sub3 --> S3_Onnx["📥 Download BGE-Small (~35 MB) [Optional - Local Neural]"]

    Sub4 --> S4_Cloud["☁️ Multimodal Cloud (/model) [Recommended - 0MB Download]"]
    Sub4 --> S4_Smol["📥 Download SmolVLM (~480 MB) [Local - Modest PCs]"]
    Sub4 --> S4_Moon["📥 Download Moondream2 (~1.1 GB) [Local - High Precision]"]
```

---

## 📊 3. Download Experience & Real-Time Telemetry (Item 4)

When the user selects a model to download:
1. An asynchronous thread with an `mpsc::channel` reports `bytes_downloaded` and `total_bytes`.
2. The TUI footer bar (next to the workspace status) shows an animated badge:
   ```text
   📥 [Downloading Laya AI: 45% (94.5/210 MB) 4.2 MB/s]
   ```
3. The user keeps chatting with the AI or changing settings normally.
4. On completion (100%):
   ```text
   │[AI - Default] (12:44:10)
   │  ✔ [Model Attached Successfully]
   │  **Laya AI (mmBERT INT8 ONNX)** was installed on disk and enabled for document classification.
   │  From now on, new indexing runs will use this neural model.
   ```

---

## 🌐 4. Professional Update Message (`actx --update`) (Item 3)

```text
[*] Checking for AnyContext updates...
[*] New version detected: v0.34.8
[*] Downloading distribution package 'actx-windows-x86_64.zip'...

[!] The executable packages for version v0.34.8 are still being prepared
    by the build servers (CI/CD) or are temporarily unavailable.

    💡 Tip: building new releases takes a few minutes after launch.
       Please try again shortly with: actx --update
       Follow the publication at: https://github.com/Levix-Digital/any-context-releases/releases/tag/v0.34.8
```

---

## 📋 5. Implementation Milestone Plan

1. **Milestone 1: Aurora Boreal Color Cleanup**:
   - Move informational `theme.warning` to `theme.accent` (Cyan), `theme.reasoning` (Violet) and `theme.primary` (Emerald).
   - Keep red only on `[Esc/Ctrl+C] Exit` and critical alerts.
2. **Milestone 2: Borderless Conversation Canvas**:
   - Remove the chat box borders and title; add a subtle paused-scroll indicator.
3. **Milestone 3: Dynamic ReAct & Reasoning Auto-Scroll**:
   - Add scroll control in `render_accordion` to follow the reasoning in real time.
4. **Milestone 4: Friendly Message in `actx --update`**:
   - Intercept HTTP 404 and show an empathetic, professional explanation in the GitHub CLI/Rustup style.
5. **Milestone 5: Real-Time ONNX Model Download Telemetry**:
   - Byte streaming, an event channel and a percentage progress indicator in the TUI status bar.
6. **Milestone 6: Menu Reorganization into Submenus Guided by Role**:
   - Ingestion, Scans, Queries and Vision submenus with contextual explanations of their purpose.
7. **Milestone 7: Automated Tests, Dual-Doc & Release v0.34.9**:
   - ADR-117 in `TECDOC.md`, Scenario 32 in `MANUAL_TESTS.md` and release build.
