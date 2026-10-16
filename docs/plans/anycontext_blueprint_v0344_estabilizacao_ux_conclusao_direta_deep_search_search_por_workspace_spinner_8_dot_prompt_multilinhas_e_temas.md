---
plan_id: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_direta_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_e_temas
project: any-context
title: 'Blueprint v0.34.4: Estabilizacao UX, Conclusao Direta Deep Search, Search por Workspace, Spinner 8-Dot, Prompt Multilinhas e Temas'
version: v0.34.4
status: superseded
author: Levi Silveira
created_at: '2026-10-10T08:32:37'
updated_at: '2026-10-10T08:34:24'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- ux
- deep_search
- workspace_isolation
- tui_multiline
- braille_spinner
- sqlite
- styling
supersedes: null
superseded_by: anycontext_blueprint_v0344_estabilizacao_ux_conclusao_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_temas_e_auto_consciencia_global
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
- name: 'M7: Suite de Testes Automatizados'
  done: false
- name: 'M8: Dual-Doc e Cenario 27 em MANUAL_TESTS'
  done: false
- name: 'M9: Gate 8 e Release v0.34.4'
  done: false
status_log:
- at: '2026-10-10T08:34:24'
  note: Superseded by anycontext_blueprint_v0344_estabilizacao_ux_conclusao_deep_search_search_por_workspace_spinner_8_dot_prompt_multilinhas_temas_e_auto_consciencia_global
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint AnyContext v0.34.4: Estabilizacao UX, Deep Search Synthesis, Search Mode por Workspace, Spinner 8-Dot, Prompt Multilinha e Temas

1. Objetivo & Decisoes Arquiteturais (ADRs):
- Deep Search: Resposta focada exclusivamente na conclusao e rigorosamente no idioma do usuario, eliminando preambulos e narracao de busca.
- /search por Workspace: Persistencia da profundidade de busca (auto, fast, deep) isolada na tabela workspaces do SQLite.
- Spinner Braille 8-Dot: Substituicao de 3 para 4 pontos de altura com reducao de tick para 125ms (dobro da velocidade).
- /update Seguro: Deprecacao de execucao in-tui para evitar corrupcao de stdout, instruindo 'actx --update'.
- Prompt Expansivel Multilinha: Suporte a Shift+Enter, Ctrl+J, auto-wrap e calculo dinamico de altura (3 a 8 linhas) com cursor 2D.
- Identificacao de Workspace: Badge [AI - <Workspace>] estilizado e tipografia cinza (Color::Gray e Color::DarkGray).

2. Superficies Impactadas: actx-cli (TUI, events, ui, app), any-context-core-rs (sqlite, engine), actx-agent (deep_search).

3. Testes: Unitarios e integracao de isolamento SQLite, parser multilinhas e regressao de comandos.
