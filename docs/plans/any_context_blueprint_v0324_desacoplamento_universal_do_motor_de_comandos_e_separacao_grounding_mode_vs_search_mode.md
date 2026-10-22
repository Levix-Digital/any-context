---
plan_id: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
project: any-context
title: 'Blueprint v0.32.4: Universal Decoupling of the Command Engine and Grounding Mode vs Search Mode Separation'
version: v0.32.4
status: superseded
author: Levi Silveira
created_at: '2026-10-02T10:27:58'
updated_at: '2026-10-02T15:43:59'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- hexagonal_architecture
- command_engine
- grounding_mode
- search_mode
- decoupled_ui
- ports_and_adapters
supersedes: any_context_blueprint_v0323_estabilizacao_tui_historico_updown_top_header_groundingwebsearch_memoria_de_longo_prazo_e_auto_consciencia_rust_core
superseded_by: any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
parent_plan_id: null
milestones:
- name: 'M1: CommandEngine Module in the Core'
  done: true
- name: 'M2: Grounding vs Search Mode Separation'
  done: true
- name: 'M3: TUI Dispatcher Refactored as an Adapter'
  done: true
- name: 'M4: Reactive Five-Item Top Header'
  done: true
- name: 'M5: Automated Isolation Tests'
  done: true
- name: 'M6: Dual-Doc and Manual Tests'
  done: true
- name: 'M7: Release v0.32.4 and CI Watch'
  done: true
status_log:
- at: '2026-10-02T11:32:44'
  note: 'Milestone ''M1: CommandEngine Module in the Core'': implemented in any-context-core-rs::commands'
- at: '2026-10-02T11:32:51'
  note: 'Milestone ''M2: Grounding vs Search Mode Separation'': GroundingMode and SearchDepthMode isolated with strict validation'
- at: '2026-10-02T11:32:55'
  note: 'Milestone ''M3: TUI Dispatcher Refactored as an Adapter'': dispatcher.rs and App refactored to consume CommandEngine'
- at: '2026-10-02T11:32:58'
  note: 'Milestone ''M4: Reactive Five-Item Top Header'': top bar quint-status WS, Model, Grounding, Search, Web rendered in the TUI'
- at: '2026-10-02T11:33:04'
  note: 'Milestone ''M5: Automated Isolation Tests'': 608 tests passed (152 Rust + 456 Pytest)'
- at: '2026-10-02T11:33:20'
  note: 'Milestone ''M6: Dual-Doc and Manual Tests'': TECDOC Section 81 + ADR-084/085, README.md, cumulative Scenario 6 in MANUAL_TESTS.md'
- at: '2026-10-02T11:34:13'
  note: 'Milestone ''M7: Release v0.32.4 and CI Watch'': tag v0.32.4 published and release workflow running on GitHub Actions'
- at: '2026-10-02T15:43:59'
  note: Superseded by any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Architecture Blueprint v0.32.4: Universal Decoupling of the Command Engine and Grounding Mode vs Search Mode Separation

## 1. Overview and Context
Following the user's architecture guidance and the fundamental pillars of the `dev-cycle-protocol` (Pillar 1: Modularization & Domain Isolation; Pillar 2: Hexagonal Architecture & Multi-Surface Parity):
1. **Strict Conceptual Separation**:
   - `Grounding Mode` (`STRICT`, `HYBRID`, `PROACTIVE`): governs the epistemological rigor and how closely the AI is anchored to the documents indexed in the workspace.
   - `Search Mode` / Search Depth (`AUTO`, `FAST`, `DEEP`): governs the RAG retrieval depth (RFC-042), switching between a fast direct search and multi-step reflective reasoning.
2. **Full Decoupling of Commands and Interfaces**:
   - Removes the direct coupling where command functions received `&mut App` (the Ratatui TUI struct).
   - Introduces a pure `CommandEngine` in the Core, receiving an agnostic `ExecutionContext` and returning a structured `CommandResult`.
   - Any UI (current TUI, future Desktop GUI, REST API, Stdio RPC, extensions) acts exclusively as a *Presentation Adapter*.

---

## 2. Hexagonal Architecture (Ports & Adapters)

```mermaid
flowchart TD
    subgraph Presentation Adapters [Presentation Layer]
        TUI["TUI (Ratatui / actx-cli)"]
        GUI["Future Desktop GUI (Tauri / Iced / egui)"]
        REST["REST API Server (actx --serve)"]
        RPC["Stdio RPC Bridge / MCP"]
    end

    subgraph Command Engine [any-context-core-rs::commands]
        Ctx["ExecutionContext (ws, model, grounding, search, web)"]
        Engine["CommandEngine::execute(cmd, args, ctx)"]
        Result["CommandResult (success, message, action, state_updates)"]
    end

    subgraph Core Domain & Storage [any-context-core-rs / actx-agent]
        DB["NativeConfigDb (SQLite)"]
        Lance["NativeLanceStore (Columnar Arrow)"]
        Agent["actx-agent (ReAct FSM / SessionStore)"]
        Scanner["WorkspaceScanner"]
    end

    TUI -->|raw slash / menu| Engine
    GUI -->|buttons / dropdowns / hotkeys| Engine
    REST -->|HTTP endpoints| Engine
    RPC -->|JSON-RPC / tool calls| Engine

    Engine --> Ctx
    Engine --> DB
    Engine --> Lance
    Engine --> Agent
    Engine --> Scanner
    Engine --> Result

    Result -->|apply state_updates & actions| TUI
    Result -->|render GUI state| GUI
    Result -->|HTTP response| REST
    Result -->|RPC response| RPC
```

---

## 3. Operating Mode Specification

### A. Grounding Mode (`/mode`, `/grounding`, `/grounding-mode`)
- **Valid values**: `strict` (or `s`), `hybrid` (or `h`), `proactive` (or `p`).
- **Behavior**:
  - `strict`: the agent answers strictly based on chunks retrieved from the active workspace.
  - `hybrid`: prioritizes local context, complemented by the model's general knowledge.
  - `proactive`: grounding with proactive inference to fill gaps.
- **Persistence**: `grounding_mode` in `settings.db`.

### B. Search Mode / Search Depth (`/search`, `/search-mode`, `/depth`, `/fast`, `/deep`)
- **Valid values**: `auto` (or `a`), `fast` (or `f`), `deep` (or `d`).
- **Behavior**:
  - `auto`: the agent decides autonomously from the query complexity.
  - `fast`: direct single-turn hybrid search (< 50ms).
  - `deep`: deep reflective search with decomposition into sub-queries (RFC-042).
- **Persistence**: `search_mode` in `settings.db`.
- **Agent configuration**: mapped to `actx_agent::config::SearchMode`.

---

## 4. CommandEngine and CommandResult Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub action: CommandAction,
    pub state_updates: CommandStateUpdates,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CommandAction {
    #[default]
    None,
    Exit,
    ClearChat,
    OpenMenu(String),
    SwitchWorkspace(String),
    RebuildAgent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandStateUpdates {
    pub active_workspace: Option<String>,
    pub active_model: Option<String>,
    pub grounding_mode: Option<String>,
    pub search_mode: Option<String>,
    pub web_search_enabled: Option<bool>,
}
```

---

## 5. Top Header Telemetry in the TUI

The top header (`render_header` in `crates/actx-cli/src/tui/ui.rs`) prominently shows all 5 operating parameters:
```text
AnyContext v0.32.4 ─ [WS: Default] ─ [Model: gpt-4o-mini] ─ [Grounding: HYBRID] ─ [Search: AUTO] ─ [Web: OFF] ─ ● IDLE
```

---

## 6. Execution Milestones
- **M1**: Implement `CommandResult`, `CommandAction`, `CommandStateUpdates`, `ExecutionContext` and `CommandEngine` in `crates/any-context-core-rs/src/commands/`.
- **M2**: Separate `/mode` (Grounding: strict/hybrid/proactive) and `/search` (Search Mode: auto/fast/deep) in `CommandEngine` and persist both keys in `settings.db`.
- **M3**: Refactor `crates/actx-cli/src/commands/dispatcher.rs` to delegate all execution to `CommandEngine`, acting only as a consumer of `CommandResult`.
- **M4**: Update `App` and the TUI Top Header to render `[Grounding: ...]` and `[Search: ...]` reactively.
- **M5**: Unit and integration test suite validating the CommandEngine isolation and the new commands.
- **M6**: Dual-Doc update (`TECDOC.md`, `README.md`) and cumulative manual tests in `tests/MANUAL_TESTS.md` (Scenario 6).
- **M7**: Gate 8 verification, `v0.32.4` bump, release and continuous CI follow-up.
