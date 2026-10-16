---
plan_id: anycontext_blueprint_v0344_estabilizacao_ux_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_auto_consciencia_menu_document_ai_e_telemetria_react_enriquecida
project: any-context
title: 'Blueprint v0.34.4: Estabilizacao UX, Deep Search, Search por Workspace, Spinner 8-Dot, Prompt Multilinhas, Auto-Consciencia, Menu Document AI e Telemetria ReAct Enriquecida'
version: v0.34.4
status: active
author: Levi Silveira
created_at: '2026-10-10T08:43:12'
updated_at: '2026-10-10T08:43:12'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- ux
- deep_search
- workspace_isolation
- tui_multiline
- braille_spinner
- sqlite
- styling
- global_knowledge
- document_ai_menu
- react_telemetry
supersedes: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_temas_e_auto_consciencia_global
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: Deep Search Synthesis Prompt e Idioma'
  done: false
- name: 'M2: Search Mode por Workspace no SQLite'
  done: false
- name: 'M3: Spinner Braille 8-Dot e Timer 125ms'
  done: false
- name: 'M4: Redirecionamento Seguro /update para --update'
  done: false
- name: 'M5: Prompt Expansivel Multilinha com Cursor 2D'
  done: false
- name: 'M6: Badge [AI - Workspace] e Tipografia Cinza'
  done: false
- name: 'M7: Auto-Consciencia e Sync Transparente Global'
  done: false
- name: 'M8: Menu Document AI e Modelos Locais no /menu'
  done: false
- name: 'M9: Telemetria ReAct e Reasoning Enriquecida'
  done: false
- name: 'M10: Suite de Testes Automatizados'
  done: false
- name: 'M11: Dual-Doc e Cenario 27 em MANUAL_TESTS'
  done: false
- name: 'M12: Gate 8 e Release v0.34.4'
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint AnyContext v0.34.4: Estabilizacao UX, Deep Search, Search Mode por Workspace, Spinner 8-Dot, Prompt Multilinha, Auto-Consciencia, Menu Document AI e Telemetria ReAct

1. Objetivo & Decisoes Arquiteturais (ADRs):
- Deep Search: Resposta focada exclusivamente na conclusao e rigorosamente no idioma do usuario, eliminando preambulos e narracao de busca.
- /search por Workspace: Persistencia da profundidade de busca (auto, fast, deep) isolada na tabela workspaces do SQLite.
- Spinner Braille 8-Dot: Substituicao de 3 para 4 pontos de altura com reducao de tick para 125ms (dobro da velocidade).
- /update Seguro: Deprecacao de execucao in-tui para evitar corrupcao de stdout, instruindo 'actx --update'.
- Prompt Expansivel Multilinha: Suporte a Shift+Enter, Ctrl+J, auto-wrap e calculo dinamico de altura (3 a 8 linhas) com cursor 2D.
- Identificacao de Workspace: Badge [AI - <Workspace>] estilizado e tipografia cinza (Color::Gray e Color::DarkGray).
- Auto-Consciencia e Sync Global Transparente: Bootstrap e /sync --force automatico em background para o workspace Global a cada atualizacao de versao, e verificacao/sync incremental a cada inicializacao.
- Menu Document AI & Modelos Locais: Novo submenu no /menu para gestao transparente de classificacao e visao (status atual: Heuristica 2D e ModelRouter <1us) com opcao de download/ativacao de modelos locais.
- Telemetria ReAct & Reasoning Enriquecida: Logs detalhados e estruturados no accordion (triage, grounding, busca hibrida LanceDB+BM25, contagem de chunks, reranking e sintese com badges coloridos).

2. Superficies: actx-cli (TUI, events, ui, app, menu, engine, main), any-context-core-rs (sqlite, engine), actx-agent (deep_search, events, fsm).

3. Testes: Unitarios e integracao de isolamento SQLite, parser multilinhas, menu Document AI, auto-consciencia Global e telemetria ReAct.
