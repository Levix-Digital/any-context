---
plan_id: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
project: any-context
title: 'Blueprint v0.32.4: Desacoplamento Universal do Motor de Comandos e Separacao Grounding Mode vs Search Mode'
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
- name: 'M1: Modulo CommandEngine no Core'
  done: true
- name: 'M2: Separacao Grounding vs Search Mode'
  done: true
- name: 'M3: Refatoracao Dispatcher TUI como Adapter'
  done: true
- name: 'M4: Top Header Quinquenal Reativo'
  done: true
- name: 'M5: Testes Automatizados de Isolamento'
  done: true
- name: 'M6: Dual-Doc e Testes Manuais'
  done: true
- name: 'M7: Release v0.32.4 e CI Watch'
  done: true
status_log:
- at: '2026-10-02T11:32:44'
  note: 'Milestone ''M1: Modulo CommandEngine no Core'': Implementado em any-context-core-rs::commands'
- at: '2026-10-02T11:32:51'
  note: 'Milestone ''M2: Separacao Grounding vs Search Mode'': GroundingMode e SearchDepthMode isolados com validacao estrita'
- at: '2026-10-02T11:32:55'
  note: 'Milestone ''M3: Refatoracao Dispatcher TUI como Adapter'': dispatcher.rs e App refatorados para consumir CommandEngine'
- at: '2026-10-02T11:32:58'
  note: 'Milestone ''M4: Top Header Quinquenal Reativo'': Top bar quint-status WS, Model, Grounding, Search, Web renderizada no TUI'
- at: '2026-10-02T11:33:04'
  note: 'Milestone ''M5: Testes Automatizados de Isolamento'': 608 testes passaram (152 Rust + 456 Pytest)'
- at: '2026-10-02T11:33:20'
  note: 'Milestone ''M6: Dual-Doc e Testes Manuais'': TECDOC Secao 81 + ADR-084/085, README.md, Cenário 6 cumulativo em MANUAL_TESTS.md'
- at: '2026-10-02T11:34:13'
  note: 'Milestone ''M7: Release v0.32.4 e CI Watch'': Tag v0.32.4 publicada e release workflow em execucao no GitHub Actions'
- at: '2026-10-02T15:43:59'
  note: Superseded by any_context_blueprint_v0327_paridade_estrita_de_grounding_no_rust_core_agentmd_diretivas_antialucinação_busca_autônoma_e_resolução_canônica_do_lancedbbm25
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint Arquitetural v0.32.4: Desacoplamento Universal do Motor de Comandos e Separação de Grounding Mode vs Search Mode

## 1. Visão Geral e Contexto
Atendendo às orientações de arquitetura do usuário e aos pilares fundamentais do `dev-cycle-protocol` (Pilar 1: Modularização & Isolamento de Domínio; Pilar 2: Arquitetura Hexagonal & Paridade Multi-Superfície):
1. **Separação Conceitual Rígida**:
   - `Grounding Mode` (`STRICT`, `HYBRID`, `PROACTIVE`): governa o rigor epistemológico e a ancoragem da IA aos documentos indexados no workspace.
   - `Search Mode` / Profundidade de Busca (`AUTO`, `FAST`, `DEEP`): governa a profundidade de recuperação RAG (RFC-042), alternando entre busca rápida direta e raciocínio reflexivo multi-etapa.
2. **Desacoplamento Total de Comandos e Interfaces**:
   - Elimina o acoplamento direto onde as funções de comando recebiam `&mut App` (struct da TUI Ratatui).
   - Introduz o `CommandEngine` puro no Core, recebendo um `ExecutionContext` agnóstico e retornando um `CommandResult` estruturado.
   - Qualquer UI (TUI atual, futura Desktop GUI, REST API, Stdio RPC, extensões) passa a atuar exclusivamente como um *Presentation Adapter*.

---

## 2. Arquitetura Hexagonal (Ports & Adapters)

```mermaid
flowchart TD
    subgraph Presentation Adapters [Camada de Apresentação]
        TUI["TUI (Ratatui / actx-cli)"]
        GUI["Futura Desktop GUI (Tauri / Iced / egui)"]
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

## 3. Especificação dos Modos Operacionais

### A. Grounding Mode (`/mode`, `/grounding`, `/grounding-mode`)
- **Valores válidos**: `strict` (ou `s`), `hybrid` (ou `h`), `proactive` (ou `p`).
- **Comportamento**:
  - `strict`: O agente responde estritamente apoiado em chunks recuperados do workspace ativo.
  - `hybrid`: Prioriza contexto local complementado por conhecimento geral do modelo.
  - `proactive`: Ancoragem com inferência proativa para preenchimento de lacunas.
- **Persistência**: `grounding_mode` em `settings.db`.

### B. Search Mode / Profundidade de Busca (`/search`, `/search-mode`, `/depth`, `/fast`, `/deep`)
- **Valores válidos**: `auto` (ou `a`), `fast` (ou `f`), `deep` (ou `d`).
- **Comportamento**:
  - `auto`: O agente decide autonomamente pela complexidade da query.
  - `fast`: Execução direta de busca híbrida de 1 turno (< 50ms).
  - `deep`: Busca profunda reflexiva com decomposição em sub-queries (RFC-042).
- **Persistência**: `search_mode` em `settings.db`.
- **Configuração do Agente**: Mapeado para `actx_agent::config::SearchMode`.

---

## 4. Estrutura do CommandEngine e CommandResult

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

## 5. Top Header Telemetry na TUI

O cabeçalho superior (`render_header` em `crates/actx-cli/src/tui/ui.rs`) exibirá com destaque todos os 5 parâmetros operacionais:
```text
AnyContext v0.32.4 ─ [WS: Default] ─ [Model: gpt-4o-mini] ─ [Grounding: HYBRID] ─ [Search: AUTO] ─ [Web: OFF] ─ ● IDLE
```

---

## 6. Marcos de Execução (Milestones)
- **M1**: Implementar `CommandResult`, `CommandAction`, `CommandStateUpdates`, `ExecutionContext` e `CommandEngine` em `crates/any-context-core-rs/src/commands/`.
- **M2**: Separar `/mode` (Grounding: strict/hybrid/proactive) e `/search` (Search Mode: auto/fast/deep) no `CommandEngine` e persistir ambas as chaves em `settings.db`.
- **M3**: Refatorar `crates/actx-cli/src/commands/dispatcher.rs` para delegar 100% da execução ao `CommandEngine`, atuando apenas como consumidor de `CommandResult`.
- **M4**: Atualizar o `App` e o TUI Top Header para renderizar `[Grounding: ...]` e `[Search: ...]` de forma reativa.
- **M5**: Suíte de testes unitários e de integração validando o isolamento do CommandEngine e os novos comandos.
- **M6**: Atualização do Dual-Doc (`TECDOC.md`, `README.md`) e acúmulo de testes manuais em `tests/MANUAL_TESTS.md` (Cenário 6).
- **M7**: Verificação Gate 8, bump `v0.32.4`, release e acompanhamento contínuo no CI.
