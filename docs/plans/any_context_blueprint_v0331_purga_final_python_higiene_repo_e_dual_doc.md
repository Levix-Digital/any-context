---
plan_id: any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
project: any-context
title: 'Blueprint v0.33.1: Purga Final do Legado Python, Higiene de Repositorio e Atualizacao Integral Dual-Doc'
version: v0.33.1
status: superseded
author: Levi Silveira
created_at: '2026-10-09T17:07:42'
updated_at: '2026-10-09T18:15:14'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- rust
- purge
- cleanup
- hygiene
- dual-doc
- v0.33.1
supersedes: null
superseded_by: any_context_blueprint_fase2_rfc042_deep_search_engine_rust_native
parent_plan_id: null
milestones:
- name: 'M1: Remocao de Codigo Python e Artefatos Orfaos'
  done: false
- name: 'M2: Sanitizacao de gitignore e Ambientes'
  done: false
- name: 'M3: Atualizacao Dual-Doc README e TECDOC para 100% Pure Rust'
  done: false
- name: 'M4: Registro do Cenario 22 em MANUAL_TESTS.md'
  done: false
- name: 'M5: Suite de Testes Rust 100% Verdes'
  done: false
- name: 'M6: Versionamento v0.33.1 Release CI Watch e Publicacao'
  done: false
status_log:
- at: '2026-10-09T18:15:14'
  note: Superseded by any_context_blueprint_fase2_rfc042_deep_search_engine_rust_native
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.33.1: Purga Final do Legado Python, Higiene de Repositório e Atualização Integral Dual-Doc

## 1. Contexto e Decisão Arquitetural (ADR)
Com a release v0.33.0 publicada e a remoção completa do PyO3 do any-context-core-rs, o AnyContext opera 100% em Rust nativo. Este plano executa a limpeza final dos últimos arquivos Python órfãos (config/app_settings.py), eliminação de pastas residuais do disco, sanitização do .gitignore e reescrita das seções da documentação (README.md e TECDOC.md) que ainda citavam runtimes Python, PyO3, pip ou pre-extrações de _internal.

## 2. Paridade e Escopo de Arquitetura
- **Zero Python Runtime**: Nenhuma dependência em Python no monorepo de produção.
- **Single Source of Truth**: Configurações geridas unicamente pelo NativeConfigDb em SQLite.
- **Dual-Doc Rigoroso**: Atualização detalhada do README e TECDOC refletindo a arquitetura pure-Rust monorepo.

## 3. Marcos de Execução
- **M1**: Remoção de config/app_settings.py e eliminação de pastas locais residuais (.pytest_cache, .langgraph_api, build/, .coverage).
- **M2**: Sanitização do .gitignore para o perfil exclusivo Cargo/Rust e bancos locais.
- **M3**: Atualização Dual-Doc completa em README.md e TECDOC.md.
- **M4**: Registro acumulativo e não-truncado do Cenário 22 em tests/MANUAL_TESTS.md.
- **M5**: Execução da suíte de testes do workspace (cargo test --workspace) com 100% de sucesso.
- **M6**: Fechamento do ciclo, versionamento v0.33.1, acompanhamento ininterrupto do CI/CD e publicação dos assets.
