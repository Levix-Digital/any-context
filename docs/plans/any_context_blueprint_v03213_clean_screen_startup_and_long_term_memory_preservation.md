---
plan_id: any_context_blueprint_v03213_clean_screen_startup_and_long_term_memory_preservation
project: any-context
title: '[any-context] Blueprint v0.32.13: Inicializacao de Tela Limpa na TUI e Desacoplamento da Memoria de Longo Prazo'
version: v0.32.13
status: superseded
author: Levi Silveira
created_at: '2026-10-06T11:52:26'
updated_at: '2026-10-06T12:51:14'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: any_context_blueprint_v03212_test_isolation_sandbox_universal_option_hinting_skill_based_global_self_knowledge
superseded_by: any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
parent_plan_id: null
milestones:
- name: 'M1: Clean Screen Buffer Initialization'
  done: true
- name: 'M2: Long-Term Memory Decoupling and Persistence'
  done: true
- name: 'M3: Enhanced /history Command Inspectability'
  done: true
- name: 'M4: Automated Tests Suite'
  done: true
- name: 'M5: Dual-Doc and Manual Tests'
  done: true
- name: 'M6: Production Release v0.32.13'
  done: true
status_log:
- at: '2026-10-06T12:04:28'
  note: 'Milestone ''M1: Clean Screen Buffer Initialization'': Removed load_session_history_for_workspace on startup and switch_to_workspace'
- at: '2026-10-06T12:04:37'
  note: 'Milestone ''M2: Long-Term Memory Decoupling and Persistence'': Preserved SQLite actx_session_messages for agent retrieval'
- at: '2026-10-06T12:04:38'
  note: 'Milestone ''M3: Enhanced /history Command Inspectability'': Enhanced /history to inspect SQLite long-term turns and /history --clear to wipe SQLite'
- at: '2026-10-06T12:04:39'
  note: 'Milestone ''M4: Automated Tests Suite'': All tests passing, added test_clean_screen_startup_and_long_term_memory_preservation'
- at: '2026-10-06T12:06:05'
  note: 'Milestone ''M5: Dual-Doc and Manual Tests'': README.md, TECDOC.md Section 89 ADR-095 and tests/MANUAL_TESTS.md Cenario 14 updated'
- at: '2026-10-06T12:35:52'
  note: 'Milestone ''M6: Production Release v0.32.13'': v0.32.13 released and published with 12 assets on GitHub Releases, local binary verified'
- at: '2026-10-06T12:51:14'
  note: Superseded by any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.13: Clean Screen Buffer on Session Startup and Long-Term Memory Preservation

## Contexto & Problema
No Linux e demais plataformas, ao iniciar a aplicacao ou alternar para workspaces nao visitados, a sessao anterior estava sendo despejada diretamente no buffer de tela (chat_history), poluindo o terminal. Cada nova sessao deve iniciar com a tela 100% limpa (apenas com o banner inicial de boas-vindas), enquanto o historico permanece preservado no SQLite como memoria de longo prazo acessivel pelo agente.

## Solucao Arquitetural
1. M1: Remover a hidratacao automatica de mensagens passadas do SQLite para chat_history em App::new() e switch_to_workspace(). A tela inicia zerada com o banner de boas-vindas. O cache em memoria (workspace_chat_buffers) mantem as abas virtuais ativas estritamente durante a sessao em execucao.
2. M2: Preservar 100% das mensagens no SqliteSessionStore (actx_session_messages), garantindo que o agente carregue o contexto conversacional previo ao responder perguntas sobre sessoes passadas.
3. M3: Atualizar /history para inspecionar a memoria de longo prazo do SQLite sem poluir a tela e permitir /history --clear para limpeza completa no banco.
4. M4: Testes automatizados no Rust cobrindo tela zerada no boot com SQLite preenchido.
5. M5: Dual-Doc (README.md, TECDOC.md com ADR-095) e MANUAL_TESTS.md Cenario 14.
6. M6: Release v0.32.13 e CI Watch ate publicacao.
