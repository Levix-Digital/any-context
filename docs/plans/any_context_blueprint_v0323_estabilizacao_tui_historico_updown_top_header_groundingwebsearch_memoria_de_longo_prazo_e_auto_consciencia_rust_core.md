---
plan_id: any_context_blueprint_v0323_estabilizacao_tui_historico_updown_top_header_groundingwebsearch_memoria_de_longo_prazo_e_auto_consciencia_rust_core
project: any-context
title: 'Blueprint v0.32.3: Estabilizacao TUI (Historico Up/Down, Top Header Grounding/WebSearch, Memoria de Longo Prazo e Auto-Consciencia Rust Core)'
version: v0.32.3
status: superseded
author: Levi Silveira
created_at: '2026-10-02T09:43:05'
updated_at: '2026-10-02T10:27:58'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- tui
- prompt-history
- top-header
- grounding
- web-search
- long-term-memory
- sqlite-session-store
- rag
- actx-cli
- v0.32.3
supersedes: any_context_blueprint_prioridade_4_interface_cli_tui_100_nativa_em_rust_actx_cli_actx_tui
superseded_by: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
parent_plan_id: null
milestones:
- name: 'M1: Binario Windows & Auto-Update Real'
  done: true
- name: 'M2: Historico de Prompts Up/Down por Workspace'
  done: true
- name: 'M3: Top Header Reativo com Grounding e WebSearch'
  done: true
- name: 'M4: Memoria de Sessao e Auto-Consciencia Rust Core'
  done: true
- name: 'M5: Testes Automatizados e Dual-Doc'
  done: true
status_log:
- at: '2026-10-02T10:14:14'
  note: 'Milestone ''M1: Binario Windows & Auto-Update Real'': Resolvido e compilado release v0.32.3 em bin/actx.exe'
- at: '2026-10-02T10:14:16'
  note: 'Milestone ''M2: Historico de Prompts Up/Down por Workspace'': Implementado Up/Down readline isolado por workspace'
- at: '2026-10-02T10:14:17'
  note: 'Milestone ''M3: Top Header Reativo com Grounding e WebSearch'': Quad-status WS/Model/Mode/Web implementado e reativo'
- at: '2026-10-02T10:14:18'
  note: 'Milestone ''M4: Memoria de Sessao e Auto-Consciencia Rust Core'': SQLite session store conectado e prompt de auto-consciencia ativo'
- at: '2026-10-02T10:14:19'
  note: 'Milestone ''M5: Testes Automatizados e Dual-Doc'': 581 testes 100% verdes, TECDOC ADR-080..083, README e Scenario 5'
- at: '2026-10-02T10:14:27'
  note: Release v0.32.3 deployed to main and tag pushed
- at: '2026-10-02T10:27:58'
  note: Superseded by any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.3: Estabilizacao TUI, Historico Up/Down, Top Header e Memoria Rust Core

## 1. Objetivo & Decisões Arquiteturais (ADRs)
- ADR-080: Historico de Prompts Isolado por Workspace via Ring Buffer e Teclas Up/Down, reservando PageUp/PageDown e Mouse para Scroll.
- ADR-081: Top Header Reativo com Quad-Status (Workspace, Model, Grounding Mode, Web Search).
- ADR-082: Sessao Continua, Auto-Consciencia e Tool Calling de Contexto no actx-agent com SqliteSessionStore e System Prompt Canonico.
- ADR-083: Repositorio Canonico de Releases Levix-Digital/any-context-releases e Atualizacao Real no actx-cli.

## 2. Paridade de Interfaces
- TUI / CLI: Top Header completo, navegacao de historico de prompts, restauracao de mensagens visuais, comandos /mode e /web-search atualizados em tempo real.
- Core: SqliteSessionStore ativo com session_id ws_{workspace}, RAG nativo via search_db, prompt com auto-consciencia.

## 3. Arquivos Impactados
- crates/actx-cli/src/tui/app.rs
- crates/actx-cli/src/tui/events.rs
- crates/actx-cli/src/tui/ui.rs
- crates/actx-cli/src/engine.rs
- crates/actx-cli/src/cli/oneshot.rs
- crates/actx-cli/src/commands/dispatcher.rs
- Cargo.toml / pyproject.toml / version.txt

## 4. Dual-Doc & Testes Manuais
- UserDoc: README.md com Top Header e navegacao Up/Down.
- TecDoc: TECDOC.md com ADR-080 a ADR-083 e diagrama de persistencia de sessao.
- Manual Tests: Cenario 5 em tests/MANUAL_TESTS.md acumulativo.
