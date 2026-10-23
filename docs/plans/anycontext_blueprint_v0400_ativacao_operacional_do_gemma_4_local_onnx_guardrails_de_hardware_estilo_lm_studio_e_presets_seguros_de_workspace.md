---
plan_id: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_guardrails_de_hardware_estilo_lm_studio_e_presets_seguros_de_workspace
project: any-context
title: 'Blueprint v0.40.0: Operational Activation of Local Gemma 4 ONNX, LM Studio-Style Hardware Guardrails and Safe Workspace Presets'
version: v0.40.0
status: superseded
author: Levi Silveira
created_at: '2026-10-14T16:38:18'
updated_at: '2026-10-15T08:22:07'
session_id: 01fd77a2-4853-4967-b572-f0ce69495ea0
tags:
- gemma4
- onnx
- guardrails
- hardware
- lm_studio_style
- presets
- workspace
- dev_cycle
supersedes: anycontext_blueprint_v0400_ativacao_operacional_do_gemma_4_local_onnx_e_presets_estaticos_de_workspace
superseded_by: anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
parent_plan_id: null
milestones:
- name: 'Milestone 1: Hardware Guardrail in the Core'
  done: false
- name: 'Milestone 2: Gemma 4 Dispatch in engine.rs'
  done: false
- name: 'Milestone 3: Smart /preset Command'
  done: false
- name: 'Milestone 4: TUI Parity and Automated Tests'
  done: false
- name: 'Milestone 5: Dual-Doc MANUAL_TESTS and Release v0.40.0'
  done: false
status_log:
- at: '2026-10-15T08:22:07'
  note: Superseded by anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Engineering Blueprint: AnyContext v0.40.0
## Operational Activation of Local Gemma 4 ONNX, LM Studio-Style Hardware Guardrails & Safe Workspace Presets

---

### 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.40.0` (successor of `v0.39.0`)
- **Author**: Levi Silveira & Antigravity Assistant
- **Date**: 2026-10-14
- **Phase**: Master Plan Phase 3 (Model Kit, Local Sovereignty & Hardware Protection)
- **Status**: Consolidated Technical Proposal (Gate 3 - `dev-cycle-protocol`)
- **Components**:
  - `crates/actx-cli`: backend wiring in `resolve_lm_provider` to dispatch local Gemma 4 ONNX, hardware guardrails in `/model` and badges in the TUI menu.
  - `crates/any-context-core-rs`: a smart canonical `/preset` command (alias `/kit`) through `HardwareProfiler` and `CommandEngine` with persistence in SQLite.
  - `crates/actx-lm`: validation and integration of `GemmaOnnxProvider` into the real conversation flows with a graceful fallback.
  - `crates/actx-onnx`: resolution of the canonical models directory `%LOCALAPPDATA%/actx/models/`.
- **Dual-Doc**: `README.md` (UserDoc), `TECDOC.md` (TecDoc - ADR-125), `tests/MANUAL_TESTS.md` (Scenario 40).

---

## 🏛️ 1. Goal & Architecture Decisions (ADR-125)

### Context and Strategic Alignment
In version `v0.38.0`, we built the Gemma 4 neural infrastructure (`OnnxGemmaEngine` in `actx-onnx` and `GemmaOnnxProvider` in `actx-lm`) and the TUI menus.
However, two architecture and usability decisions were consolidated:

1. **User Sovereignty vs. Dynamic Inference Routing**:
   - Dynamically switching the inference model (e.g. a local SLM on turn 1 and Claude/GPT-4o on turn 2) breaks the user's predictability, creates risks of authentication failures in restricted environments and violates the operator's deliberate choice of model.
   - **Decision**: the user's model choice is **sovereign and immutable** during the chat. Dynamic routing (`SearchMode::Auto`) works **exclusively on RAG depth** (`Fast RAG` vs `Deep Search`), keeping 100% predictability.

2. **Active Hardware Guardrails on Local Models (LM Studio Style)**:
   - For cloud models (Claude, GPT, Gemini, Groq, DeepSeek), the machine's hardware is irrelevant (it uses only ~15MB of RAM for HTTP streaming). The user has full freedom to switch between them at any time, as long as the API key is configured.
   - For local models (Gemma 4 ONNX), trying to load a model larger than the machine's physical RAM (e.g. Gemma 4 32B on a 16GB machine) causes **catastrophic swap thrashing**, freezing the operating system and killing the process by OOM.
   - **Decision**: the `HardwareProfiler` acts as an **active guardrail**:
     - 🟢 **Green (Compatible)**: instant selection without warnings.
     - 🟡 **Yellow (Swap Risk)**: an explanatory high-memory warning, allowing the user to proceed.
     - 🔴 **Red (Incompatible / Exceeds Total RAM)**: **blocked with an emphatic warning of the freeze risk**. Execution is only released if the user passes the explicit `--force` flag (`/model gemma-4-32b --force`), aware that it is at their own risk.
     - **Auto-Detection on Upgrade**: if the user adds physical memory to the machine, `HardwareProfiler` detects the new capacity instantly ($<5\text{ms}$ through OS APIs), unlocking larger models automatically.

3. **Static, Safe Workspace Presets (`/preset` or `/kit`)**:
   - Instead of a fixed arbitrary model, the `/preset` command queries `HardwareProfiler` and adjusts the settings atomically and safely:
     - `offline`: selects **the largest safe (🟢) local model** the machine supports (Compact for 8GB, 12B for 16GB, 27B for 32GB, 32B for 64GB+), with `/mode strict` and `/web-search off`.
     - `balanced`: selects a cost-effective cloud model (`gpt-4o-mini` or `groq`), `/mode hybrid`, `/search auto` and `/web-search on`.
     - `frontier`: selects a frontier model (`claude-3-7-sonnet` or `gpt-4o`), `/mode proactive` and `/search deep`.
     - `privacy`: selects a safe (🟢) local model, with `/mode strict` and `/web-search off`.

4. **Operational Wiring of Gemma 4 in the CLI (`resolve_lm_provider`)**:
   - Add the branch for `gemma-4` and `gemma` in `crates/actx-cli/src/engine.rs`, locating the `.onnx` files in the canonical directory `%LOCALAPPDATA%/actx/models/`.
   - **ADR-110 (Zero-Breakage Guarantee)**: if the ONNX file has not been downloaded yet, show clear guidance (giving the `/profile <tier>` command or `actx-installer`) and run a graceful fallback without freezing the terminal.

```mermaid
graph TD
    subgraph "1. User Selection & Hardware Guardrail"
        INPUT["/model <name> or /preset offline command"]
        PROFILER["HardwareProfiler (<5ms OS APIs)"]
        CHECK_TIER{"Local Model?<br/>Gemma 4"}
        CLOUD_OK["Cloud: checks the API Key and applies immediately"]
        SEMAPHORE{"RAM Traffic Light:<br/>🟢 Green | 🟡 Yellow | 🔴 Red"}
        LOCAL_OK["🟢 Applies the Local Model in SQLite"]
        LOCAL_BLOCK["🔴 BLOCKS with an Emphatic OOM Warning<br/>(Requires --force to proceed)"]
    end

    subgraph "2. CLI Dispatcher (crates/actx-cli/src/engine.rs)"
        RESOLVER{"resolve_lm_provider(model)"}
        CHECK_FILE{"Does the ONNX file exist on disk?"}
        ONNX_ENGINE["GemmaOnnxProvider (actx-lm)"]
        FALLBACK_SAFE["Graceful Fallback (Cloud / Mock) + Friendly Guidance"]
    end

    subgraph "3. Chat Execution"
        CHAT_LOOP["ReActOrchestrator / DeepSearchOrchestrator"]
        DEPTH_ROUTING{"SearchMode::Auto?<br/>Fast RAG vs Deep Search"}
        INFERENCE["Sovereign, Predictable Inference"]
    end

    INPUT --> CHECK_TIER
    CHECK_TIER -->|No (Cloud)| CLOUD_OK
    CHECK_TIER -->|Yes (Local)| PROFILER
    PROFILER --> SEMAPHORE
    SEMAPHORE -->|🟢 Green| LOCAL_OK
    SEMAPHORE -->|🔴 Red| LOCAL_BLOCK
    LOCAL_BLOCK -->|With --force| LOCAL_OK
    LOCAL_OK --> RESOLVER
    CLOUD_OK --> RESOLVER
    RESOLVER --> CHECK_FILE
    CHECK_FILE -->|Yes| ONNX_ENGINE
    CHECK_FILE -->|No (ADR-110)| FALLBACK_SAFE
    ONNX_ENGINE --> CHAT_LOOP
    FALLBACK_SAFE --> CHAT_LOOP
    CHAT_LOOP --> DEPTH_ROUTING
    DEPTH_ROUTING --> INFERENCE
```

---

## 🏛️ 2. Surface Parity Mapping

| Surface | v0.40.0 Capability / Behavior |
| :--- | :--- |
| **Terminal CLI TUI** | `[F1]` Models Menu with LM Studio-style badges: `🟢 [Compatible]`, `🟡 [Swap Risk]`, `🔴 [Insufficient Hardware - Requires X GB]`. Red models are marked with a warning. |
| **`/model` Command** | Active hardware validation. Blocks 🔴 local models with an explanatory warning detailing total vs required RAM, instructing the use of `--force` if the user wants to take the risk. |
| **`/preset` Command (alias `/kit`)** | Applies atomic workspace shortcuts in SQLite (`offline`, `balanced`, `frontier`, `privacy`), automatically resolving the optimal safe local model. |
| **One-Shot CLI (Headless)** | Allows `actx "question" --model gemma-4-12b`, running inference locally through ONNX without opening the TUI. |
| **Slash Commands Help** | Formally adds `/profile` and `/preset` to the `DEFAULT_SLASH_COMMANDS` table with autocomplete and contextual help. |

---

## 📁 3. Affected Files and Lines

1. **`crates/any-context-core-rs/src/hardware/mod.rs`**:
   - Add a helper `validate_model_safety(model_name: &str, metrics: &HardwareMetrics) -> HardwareSafetyVerdict`:
     - Returns `Safe` (🟢), `Tight` (🟡) or `Unsafe { required_ram_gb: f32, total_ram_gb: f32 }` (🔴).
2. **`crates/actx-cli/src/engine.rs`**:
   - Implement the `else if raw_model.starts_with("gemma-4") || raw_model.starts_with("gemma")` branch in `resolve_lm_provider`.
   - Add a `resolve_gemma_model_path(tier)` helper pointing to `%LOCALAPPDATA%/actx/models/<filename>`.
   - Implement the construction of `GemmaOnnxProvider` with a safety fallback.
3. **`crates/any-context-core-rs/src/commands/engine.rs`**:
   - In the `model` command: validate the hardware guardrail. If `Unsafe` and there is no `--force` flag, return a friendly error with detailed metrics.
   - Implement the `preset` command (alias `kit`):
     - `offline`: selects the largest safe (🟢) local model, mode `strict`, web search `off`.
     - `balanced`: default cloud model (`gpt-4o-mini` or similar), mode `hybrid`, web search `on`.
     - `frontier`: frontier model (`claude-3-7-sonnet` or `gpt-4o`), mode `proactive`, search `deep`.
     - `privacy`: safe (🟢) local model, mode `strict`, web search `off`.
4. **`crates/actx-cli/src/commands/registry.rs`**:
   - Include `/profile` and `/preset` (alias `/kit`) in the `DEFAULT_SLASH_COMMANDS` slice.
5. **`crates/actx-cli/src/tui/app.rs` & `menu.rs`**:
   - Integrate colored hardware traffic-light badges into the local model items in the TUI.
6. **Crate Manifests (`Cargo.toml`)**:
   - Version bump to `0.40.0` in the 6 member crates and in `version.txt`.
7. **Dual-Doc & Manual Tests**:
   - `TECDOC.md` (ADR-125), `README.md` (Offline Guide and Guardrails), `tests/MANUAL_TESTS.md` (Scenario 40).

---

## 📚 4. Dual-Doc Update Plan (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - New README section: **"🧠 Local Sovereignty & Hardware Guardrails (LM Studio Style)"**.
  - Documentation of the safety flags and the `/model <name> [--force]` and `/preset [offline|balanced|frontier|privacy]` commands.
- **`TecDoc` (`TECDOC.md`)**:
  - Record of **ADR-125: Operational Activation of Local Gemma 4 ONNX in the CLI Engine, Hardware Guardrails and Static Workspace Presets**.
  - Mermaid diagram of the `GemmaOnnxProvider` life cycle and resolution.
  - A table mapping the RAM and KV-cache requirements of the local models vs the failure behavior (ADR-110).

---

## 🧪 5. Automated Test Strategy & Regression Protection

1. **Unit Tests in `crates/any-context-core-rs/src/hardware/mod.rs`**:
   - `test_validate_model_safety_verdicts`: tests the 🟢, 🟡 and 🔴 verdicts for the 4 Gemma 4 tiers in different RAM simulations (8GB, 16GB, 32GB, 64GB).
2. **Unit Tests in `crates/actx-cli/src/engine.rs`**:
   - `test_resolve_lm_provider_gemma_family`: tests the resolution of `gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b`.
   - `test_resolve_lm_provider_gemma_fallback_safe`: guarantees that without the downloaded weights, the return is graceful and causes no panic.
3. **Integration Tests in `crates/any-context-core-rs/tests/command_tests.rs`**:
   - `test_model_command_hardware_guardrail_block_and_force`: validates that trying to set a 🔴 model without `--force` returns an informative error, and with `--force` applies the setting.
   - `test_preset_command_execution`: validates `/preset offline`, `/preset balanced`, `/preset frontier`, `/preset privacy` and checks the atomic persistence in SQLite (`NativeConfigDb`).
4. **Full Suite**:
   - Run `cargo test --workspace`, guaranteeing zero regressions.

---

## 📝 6. Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)

- **Scenario 40: Operational Execution of Local Gemma 4 ONNX, Hardware Guardrails and Workspace Presets (v0.40.0)**
  - Added strictly cumulatively (keeping scenarios 1 to 39 intact).
  - Detailed steps:
    1. Test the hardware guardrail: try `/model gemma-4-32b` on a machine with $<64\text{ GB}$ of RAM and check the block with a friendly warning.
    2. Test the explicit override: `/model gemma-4-32b --force` and check it is applied.
    3. Switch to the safe offline preset: `/preset offline` and validate that the chosen model is the 🟢 one compatible with the machine.
    4. Check the applied settings through `/info` and `/status`.
    5. Send a chat message and validate the `[Local ONNX]` badge and the answer.
    6. Test headless execution: `actx "Explain what RAG is" --model gemma-4-compact`.

---

## 🔒 7. Secret Hygiene, Database Migration and Rollback Plan

- **Secrets**: zero secrets or tokens involved.
- **Database**: reuses columns that already exist in the `workspaces` table (`model`, `grounding_mode`, `web_search_enabled`). No destructive schema migration.
- **Rollback**: in case of a CI regression, `git checkout dev && git revert` restores tag `v0.39.0` immediately.

---

## ⏱️ 8. Step-by-Step Execution (Milestones 1 to 5)

| Milestone | Description | Deliverables |
| :--- | :--- | :--- |
| **Milestone 1** | Hardware Guardrail in the Core | The `validate_model_safety` function in `HardwareProfiler` and the check in the `/model` command |
| **Milestone 2** | Gemma 4 dispatch in `engine.rs` | Canonical resolution of ONNX paths and instantiation of `GemmaOnnxProvider` |
| **Milestone 3** | Smart `/preset` Command | `/preset` logic selecting the best 🟢 local model and registration in the slash commands |
| **Milestone 4** | TUI Parity and Automated Tests | TUI badges, resolution unit tests and `/preset` and `/model` tests |
| **Milestone 5** | Dual-Doc, MANUAL_TESTS & Release | ADR-125 in `TECDOC.md`, `README.md`, Scenario 40, `cargo test --workspace` and release `v0.40.0` |
