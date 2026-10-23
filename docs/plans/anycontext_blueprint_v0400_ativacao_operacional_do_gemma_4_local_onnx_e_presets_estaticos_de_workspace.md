---
plan_id: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_e_presets_estaticos_de_workspace
project: any-context
title: 'Blueprint v0.40.0: Operational Activation of Local Gemma 4 ONNX and Static Workspace Presets'
version: v0.40.0
status: superseded
author: Levi Silveira
created_at: '2026-10-14T16:25:55'
updated_at: '2026-10-14T16:38:18'
session_id: 01fd77a2-4853-4967-b572-f0ce69495ea0
tags:
- gemma4
- onnx
- presets
- workspace
- dev_cycle
supersedes: anycontext_blueprint_v0390_ingestao_web_com_jina_reader_rjinaai_rag_fundamentado_puro_e_matriz_de_7_dominios_semanticos
superseded_by: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
parent_plan_id: null
milestones:
- name: 'Milestone 1: Gemma 4 Dispatch in engine.rs'
  done: false
- name: 'Milestone 2: /preset Command in the Core'
  done: false
- name: 'Milestone 3: TUI and Headless Parity'
  done: false
- name: 'Milestone 4: Automated Tests'
  done: false
- name: 'Milestone 5: Dual-Doc MANUAL_TESTS and Release v0.40.0'
  done: false
status_log:
- at: '2026-10-14T16:38:18'
  note: Superseded by anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Engineering Blueprint: AnyContext v0.40.0
## Operational Activation of Local Gemma 4 ONNX & Static Workspace Presets

---

### 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.40.0` (successor of `v0.39.0`)
- **Author**: Levi Silveira & Antigravity Assistant
- **Date**: 2026-10-14
- **Phase**: Master Plan Phase 3 (Model Kit & Local Sovereignty)
- **Status**: Technical Proposal for Approval (Gate 3 - `dev-cycle-protocol`)
- **Components**:
  - `crates/actx-cli`: backend wiring in `resolve_lm_provider` to dispatch local Gemma 4 ONNX, readiness badges in the TUI and headless mode.
  - `crates/any-context-core-rs`: canonical `/preset` command (alias `/kit`) in `CommandEngine` with atomic application of settings to the active workspace.
  - `crates/actx-lm`: validation and integration of `GemmaOnnxProvider` into the real conversation flows.
  - `crates/actx-onnx`: resolution of the canonical models directory `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-125), `tests/MANUAL_TESTS.md` (Scenario 40).

---

## 🏛️ 1. Goal & Architecture Decisions (ADR-125)

### Context and Strategic Alignment
In version `v0.38.0`, we built the Gemma 4 neural infrastructure (`OnnxGemmaEngine` in `actx-onnx` and `GemmaOnnxProvider` in `actx-lm`) and the TUI menus.
However, we identified two crucial opportunities for simplification and operational consolidation:
1. **User Sovereignty vs. Dynamic Inference Routing**:
   - Dynamically switching the inference model (e.g. a local SLM on turn 1 and Claude/GPT-4o on turn 2) breaks the user's predictability, creates risks of authentication failures in restricted environments and violates the operator's deliberate choice of model.
   - **Decision**: the user's model choice is **sovereign and immutable** during the chat. Dynamic routing (`SearchMode::Auto`) works **exclusively on RAG depth** (`Fast RAG` vs `Deep Search`), keeping 100% predictability.
2. **Operational Wiring of Gemma 4 in the CLI (`resolve_lm_provider`)**:
   - Today the `resolve_lm_provider` function in `crates/actx-cli/src/engine.rs` has branches for `claude`, `gpt`, `gemini`, `deepseek`, `groq`, `ollama`, but **no branch for `gemma-4`**.
   - If the user selects `gemma-4-12b` and sends a message, the system falls into the OpenAI fallback (404 error) or the Mock, without triggering `GemmaOnnxProvider`.
   - **Decision**: wire the Gemma 4 dispatch in `engine.rs`, locating the `.onnx` files in the canonical directory `%LOCALAPPDATA%/actx/models/` with the strict guarantee of **ADR-110 (Zero-Breakage Guarantee)**: if the weights are not on disk yet, show friendly guidance and run a graceful fallback without freezing the terminal.
3. **Static Configuration Presets (`/preset` or `/kit`)**:
   - Instead of hidden magic switching during the chat, the `/preset` command acts as a **deterministic workspace configuration shortcut**, applying a cohesive set of flags (`model`, `grounding_mode`, `search_mode`, `web_search_enabled`).

```mermaid
graph TD
    subgraph "1. Sovereign User Selection"
        USER_INPUT["/model gemma-4-12b or /preset offline command"]
        DB["SQLite (workspaces.model = 'gemma-4-12b')"]
    end

    subgraph "2. CLI Dispatcher (crates/actx-cli/src/engine.rs)"
        RESOLVER{"resolve_lm_provider(model)"}
        CHECK_WEIGHTS{"Does the ONNX file exist in<br/>%LOCALAPPDATA%/actx/models/?"}
        ONNX_PROVIDER["GemmaOnnxProvider (actx-lm & actx-onnx)"]
        FALLBACK_PROVIDER["Graceful Fallback (Cloud / Mock) + Friendly Guidance"]
    end

    subgraph "3. Chat Execution"
        CHAT_LOOP["ReActOrchestrator / DeepSearchOrchestrator"]
        DEPTH_ROUTING{"SearchMode::Auto?<br/>Fast RAG vs Deep Search"}
        INFERENCE["Sovereign Local Generation through ONNX Runtime"]
    end

    USER_INPUT --> DB
    DB --> RESOLVER
    RESOLVER --> CHECK_WEIGHTS
    CHECK_WEIGHTS -->|Yes (Weights Present)| ONNX_PROVIDER
    CHECK_WEIGHTS -->|No (No Weights - ADR-110)| FALLBACK_PROVIDER
    ONNX_PROVIDER --> CHAT_LOOP
    FALLBACK_PROVIDER --> CHAT_LOOP
    CHAT_LOOP --> DEPTH_ROUTING
    DEPTH_ROUTING --> INFERENCE
```

---

## 🏛️ 2. Surface Parity Mapping

| Surface | v0.40.0 Capability / Behavior |
| :--- | :--- |
| **Terminal CLI TUI** | Shows a `[Local ONNX: Gemma 4 ...]` badge in the header and status bar. Full `/preset` support, opening a visual submenu if called without arguments. |
| **One-Shot CLI (Headless)** | Allows `actx "question" --model gemma-4-12b`, running inference locally through ONNX without opening the TUI. |
| **`/model` Command** | Lists the Gemma 4 models and, on selection, checks whether the ONNX file exists on disk, reporting the status immediately. |
| **`/preset` Command (alias `/kit`)** | Applies predefined profiles (`offline`, `balanced`, `frontier`, `privacy`) transparently and atomically in SQLite. |
| **Slash Commands Help** | Formally adds `/profile` and `/preset` to the `DEFAULT_SLASH_COMMANDS` table with autocomplete and contextual help. |

---

## 📁 3. Affected Files and Lines

1. **`crates/actx-cli/src/engine.rs`**:
   - Implement the `else if raw_model.starts_with("gemma-4") || raw_model.starts_with("gemma")` branch in `resolve_lm_provider`.
   - Add a `resolve_gemma_model_path(tier)` helper pointing to `%LOCALAPPDATA%/actx/models/<filename>`.
   - Implement the construction of `GemmaOnnxProvider` with a safety fallback.
2. **`crates/any-context-core-rs/src/commands/engine.rs`**:
   - Implement the `preset` command (alias `kit`):
     - `offline`: model `gemma-4-12b`, mode `strict`, web search `off`.
     - `balanced`: default cloud model (`gpt-4o-mini` or similar), mode `hybrid`, web search `on`.
     - `frontier`: frontier model (`claude-3-7-sonnet` or `gpt-4o`), mode `proactive`, search `deep`.
     - `privacy`: model `gemma-4-12b`, mode `strict`, web search `off`.
3. **`crates/actx-cli/src/commands/registry.rs`**:
   - Include `/profile` and `/preset` (alias `/kit`) in the `DEFAULT_SLASH_COMMANDS` slice.
4. **`crates/actx-cli/src/tui/app.rs` & `menu.rs`**:
   - Adjust the TUI badges to show `Local ONNX (Active)` when the active model is Gemma 4.
5. **Crate Manifests (`Cargo.toml`)**:
   - Version bump to `0.40.0` in the 6 member crates and in `version.txt`.
6. **Dual-Doc & Manual Tests**:
   - `TECDOC.md` (ADR-125), `README.md` (Offline Guide), `tests/MANUAL_TESTS.md` (Scenario 40).

---

## 📚 4. Dual-Doc Update Plan (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - New README section: **"🧠 Full Sovereignty: 100% Offline Execution with Local Gemma 4 ONNX"**.
  - Documentation of the `/preset [offline|balanced|frontier|privacy]` command with a table of its effects on the workspace.
- **`TecDoc` (`TECDOC.md`)**:
  - Record of **ADR-125: Operational Activation of Local Gemma 4 ONNX in the CLI Engine and Static Workspace Presets**.
  - Mermaid diagram of the `GemmaOnnxProvider` life cycle and resolution.
  - Failure behavior matrix (ADR-110).

---

## 🧪 5. Automated Test Strategy & Regression Protection

1. **Unit Tests in `crates/actx-cli/src/engine.rs`**:
   - `test_resolve_lm_provider_gemma_family`: tests the resolution of `gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`.
   - `test_resolve_lm_provider_gemma_fallback_safe`: guarantees that without the downloaded weights, the return is graceful and causes no panic.
2. **Integration Tests in `crates/any-context-core-rs/tests/command_tests.rs`**:
   - `test_preset_command_execution`: validates `/preset offline`, `/preset balanced`, `/preset frontier`, `/preset privacy` and checks the atomic persistence in SQLite (`NativeConfigDb`).
3. **Full Suite**:
   - Run `cargo test --workspace`, guaranteeing zero regressions.

---

## 📝 6. Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)

- **Scenario 40: Operational Execution of Local Gemma 4 ONNX and Workspace Presets (v0.40.0)**
  - Added strictly cumulatively (keeping scenarios 1 to 39 intact).
  - Detailed steps:
    1. Switch to the offline preset: `/preset offline`.
    2. Check the applied settings through `/info` and `/status`.
    3. Select a specific model through `/model gemma-4-12b`.
    4. Send a chat message and validate the `[Local ONNX]` badge and the answer.
    5. Test headless execution: `actx "Explain what RAG is" --model gemma-4-compact`.

---

## 🔒 7. Secret Hygiene, Database Migration and Rollback Plan

- **Secrets**: zero secrets or tokens involved.
- **Database**: reuses columns that already exist in the `workspaces` table (`model`, `grounding_mode`, `web_search_enabled`). No destructive schema migration.
- **Rollback**: in case of a CI regression, `git checkout dev && git revert` restores tag `v0.39.0` immediately.

---

## ⏱️ 8. Step-by-Step Execution (Milestones 1 to 5)

| Milestone | Description | Deliverables |
| :--- | :--- | :--- |
| **Milestone 1** | Gemma 4 dispatch in `engine.rs` | Canonical resolution of ONNX paths and instantiation of `GemmaOnnxProvider` |
| **Milestone 2** | `/preset` command in the Core | `/preset` logic and registration of `/profile` and `/preset` in the slash commands |
| **Milestone 3** | TUI and Headless Parity | Badges and fallback handling in the TUI and the One-Shot CLI |
| **Milestone 4** | Automated Tests | Resolution unit tests and `/preset` tests in `command_tests.rs` |
| **Milestone 5** | Dual-Doc, MANUAL_TESTS & Release | ADR-125 in `TECDOC.md`, `README.md`, Scenario 40, `cargo test --workspace` and release `v0.40.0` |
