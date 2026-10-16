---
plan_id: anycontext_v0_32_5_release
project: any-context
title: 'Correção Crítica: Tokio Runtime Panic, LanceDB Safe Drop e Release v0.32.5'
version: v0.32.5
status: completed
author: Levi Silveira
created_at: '2026-10-02T13:39:23'
updated_at: '2026-10-02T13:39:42'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: Scoped LanceDB
  done: true
- name: Safe Drop
  done: true
- name: OpenAI ToolCalls
  done: true
- name: Positional Update
  done: true
- name: Dual-Doc
  done: true
- name: Release CI/CD
  done: true
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Plano de Engenharia & Correção: Release v0.32.5

## 🎯 Objetivo
Corrigir pânico crítico de concorrência no Tokio durante execução de ferramentas (`search_db`), garantir destruição segura de instâncias do `NativeLanceStore`, alinhar o protocolo de chamadas de função da OpenAI e unificar os caminhos de atualização no Linux e Windows.

## 🏛️ Arquitetura e Decisões
- **ADR-086: Scoped Non-Blocking LanceDB Invocation and Safe Runtime Destruction**:
  - `NativeLanceStore::safe_block_on`: uso de `tokio::runtime::Handle::try_current()` e `std::thread::scope` com runtime temporário dedicado de thread única para execução síncrona sem conflito com o runtime Tokio pai.
  - Implementação de `Drop for NativeLanceStore` com descarte assíncrono seguro via `std::thread::spawn`.
- **ADR-087: OpenAI Function Tool Call Protocol Alignment**:
  - Serialização de `tool_calls` no payload JSON obedecendo rigorosamente à especificação OpenAI (`id`, `type: "function"`, `function: { name, arguments }`).
- **Positional Command Routing**:
  - `actx update` e `actx check-update` mapeados para o instalador nativo.

## 🏁 Marcos Concluídos
1. Scoped Non-Blocking LanceDB Engine & Safe Drop (`crates/any-context-core-rs/src/storage/lancedb.rs`)
2. Offload LanceDB na TUI via `tokio::task::spawn_blocking` (`crates/actx-cli/src/engine.rs`)
3. Formatação Canônica de `tool_calls` para OpenAI (`crates/actx-lm/src/providers/openai.rs`)
4. Roteamento de comandos posicionais de atualização (`crates/actx-cli/src/cli/oneshot.rs`)
5. Suíte de testes unitários e integração (132 testes aprovados)
6. Atualização Dual-Doc (`TECDOC.md`, `tests/MANUAL_TESTS.md` Cenário 7)
7. CI/CD GitHub Actions: Build cruzado e publicação dos 12 assets no repositório de releases (`Levix-Digital/any-context-releases` tag `v0.32.5`).
