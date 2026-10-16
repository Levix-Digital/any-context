---
plan_id: anycontext_v0_32_6_release
project: any-context
title: Correção Multi-Turn Session Persistence, TUI Error Surfacing e Ocultação Search Depth (v0.32.6)
version: v0.32.6
status: completed
author: Levi Silveira
created_at: '2026-10-02T15:19:59'
updated_at: '2026-10-02T15:20:11'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: Conversational Boundary
  done: true
- name: Sqlite Scrubbing
  done: true
- name: TUI Error Surfacing
  done: true
- name: UI Concealment
  done: true
- name: Dual-Doc
  done: true
- name: CI/CD Release
  done: true
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Plano de Engenharia & Release: v0.32.6

## 🎯 Objetivo
Resolver a perda de resposta em diálogos multi-turn após execução de ferramentas (erro HTTP 400 por mensagens órfãs de ferramentas), eliminar o congelamento silencioso de erros na TUI e ocultar temporariamente o indicador `[Search: ...]` e comandos associados até a implementação completa da RFC-042 Deep Search.

## 🏛️ Decisões Arquiteturais
- **ADR-088: High-Level Conversational Session Boundary & Orphaned Tool Scrubbing**:
  - `persisted_new_messages` armazena exclusivamente o diálogo de alto nível entre usuário e assistente (`ChatMessage::user` e `ChatMessage::assistant(&final_content)`). O scratchpad interno de ferramentas fica apenas em `working_messages` durante a execução do turno atual.
  - `SqliteSessionStore`: Limpeza automática na inicialização e filtro SQL no `get_messages_sync`.
- **ADR-089: TUI Error Surfacing Guarantee & Temporary Search Depth UI Concealment**:
  - Exibição explícita de `AgentEvent::Error` no `chat_history` como `MessageRole::System` com `❌ Error: {err}` e retenção de `AppStatus::Error`.
  - Ocultação temporária de `[Search: ...]` do cabeçalho, do menu principal (`build_main_menu`) e da paleta de comandos (`COMMAND_REGISTRY`).

## 🏁 Marcos Concluídos
1. Conversational Boundary in actx-agent FSM (`crates/actx-agent/src/fsm.rs`)
2. SqliteSessionStore Scrubbing & Filtering (`crates/actx-agent/src/session.rs`)
3. TUI Error Surfacing (`crates/actx-cli/src/tui/app.rs`)
4. UI Concealment of Search Depth (`crates/actx-cli/src/tui/ui.rs`, `menu.rs`, `registry.rs`)
5. Dual-Doc Updates (`TECDOC.md`, `tests/MANUAL_TESTS.md` Cenário 8)
6. CI/CD Build & Publish to `Levix-Digital/any-context-releases` (Tag: `v0.32.6`)
