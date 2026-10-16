---
plan_id: any_context_normalizacao_definitiva_workspace_folders_e_ci_watch
project: any-context
title: Normalizacao Definitiva workspace_folders e CI Watch
version: v0.32.0
status: superseded
author: Levi Silveira
created_at: '2026-09-26T11:24:18'
updated_at: '2026-10-04T13:28:33'
session_id: 9f488dbf-db54-4ecb-8ed2-b7a6ac1b2372
tags:
- sqlite
- migration
- workspace_folders
- dev-cycle-protocol
- ci-watch
supersedes: null
superseded_by: any_context_blueprint_v03210_isolamento_hermetico_de_buffers_de_chat_e_historico_por_workspace_na_tui_nativa
parent_plan_id: null
milestones:
- name: 'M1: Dev-Cycle Protocol CI Rule'
  done: true
- name: 'M2: Version Bump v0.32.0'
  done: true
- name: 'M3: SQLite Migration to workspace_folders'
  done: true
- name: 'M4: Drop Legacy paths_json Column'
  done: true
- name: 'M5: Pure Single Table Folder Methods'
  done: true
- name: 'M6: Automated Tests'
  done: true
- name: 'M7: Dual-Doc & Manual Tests'
  done: true
- name: 'M8: Production Release & CI Watch'
  done: true
status_log:
- at: '2026-09-26T13:52:06'
  note: 'Milestone ''M1: Dev-Cycle Protocol CI Rule'': Adicionado Pilar 7 e regras estritas de acompanhamento de CI e SemVer no dev-cycle-protocol'
- at: '2026-09-26T13:53:09'
  note: 'Milestone ''M2: Version Bump v0.32.0'': Todos os manifestos atualizados para v0.32.0 (Cargo.toml, pyproject.toml, version.txt)'
- at: '2026-09-26T13:56:26'
  note: 'Milestone ''M3: SQLite Migration to workspace_folders'': Implementada migração de dados de workspaces.paths_json para workspace_folders'
- at: '2026-09-26T13:56:31'
  note: 'Milestone ''M4: Drop Legacy paths_json Column'': Coluna legada paths_json removida via ALTER TABLE DROP COLUMN'
- at: '2026-09-26T13:56:34'
  note: 'Milestone ''M5: Pure Single Table Folder Methods'': get, add e remove_workspace_folder simplificados para operar exclusivamente na tabela workspace_folders'
- at: '2026-09-26T13:56:38'
  note: 'Milestone ''M6: Automated Tests'': 125 testes unitários e de integração verdes (0 falhas) em cargo test --workspace'
- at: '2026-09-26T13:59:24'
  note: 'Milestone ''M7: Dual-Doc & Manual Tests'': Atualizados TECDOC.md (Seção 78 & ADR-078) e MANUAL_TESTS.md (Cenário 3 v0.32.0)'
- at: '2026-10-02T23:03:46'
  note: 'Milestone ''M8: Production Release & CI Watch'': Release v0.32.9 publicada com sucesso em Levix-Digital/any-context-releases'
- at: '2026-10-04T13:28:33'
  note: Superseded by any_context_blueprint_v03210_isolamento_hermetico_de_buffers_de_chat_e_historico_por_workspace_na_tui_nativa
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint Arquitetural: Migração e Normalização Definitiva de workspace_folders (v0.32.0)

## 1. 🎯 Objetivo & Decisões Arquiteturais (ADR)
- **ADR-032-01: Tabela Relacional `workspace_folders` como Única Fonte da Verdade**:
  - Eliminar a sobrecarga de sincronização dupla e redundância de dados entre a coluna legada `workspaces.paths_json` e a tabela relacional `workspace_folders`.
  - Migrar atomicamente 100% dos registros existentes em `workspaces.paths_json` para a tabela normalizada `workspace_folders`.
  - Executar a remoção física da coluna legada (`ALTER TABLE workspaces DROP COLUMN paths_json`), garantindo integridade relacional estrita e eliminando dívida técnica da transição Python -> Rust.
- **ADR-032-02: Regra Inviolável de CI/CD no `dev-cycle-protocol`**:
  - Incorporar nos Pilares e na Etapa 8 do `dev-cycle-protocol` a exigência de que os workflows de CI e Release no GitHub Actions devem ser acompanhados ininterruptamente até a publicação dos binários e conclusão total.

## 2. 🏛️ Mapeamento de Paridade de Superfícies
- **Core Domain & Storage (`crates/any-context-core-rs`)**:
  - Migração automática executada no método `ensure_tables()` e `migrate_legacy_paths_json_if_needed()`.
  - Métodos `get_workspace_folders`, `add_workspace_folder` e `remove_workspace_folder` passam a operar exclusivamente sobre `workspace_folders`.
- **Adaptadores de Apresentação (CLI, TUI, Stdio RPC, MCP, REST API)**:
  - Consomem o Core de forma transparente, recebendo as listas de pastas normalizadas sem alteração em seus contratos.
- **Protocolo de Governança (`dev-cycle-protocol`)**:
  - Atualização do arquivo `C:\Users\guilh\.gemini\config\skills\dev-cycle-protocol\SKILL.md` com a regra mandatória de acompanhamento ininterrupto de CI/CD.

## 3. 📁 Arquivos e Linhas Impactadas
- `crates/any-context-core-rs/src/storage/sqlite.rs`: Migração atômica, exclusão da coluna legada `paths_json`, simplificação de `get/add/remove_workspace_folder`.
- `crates/actx-cli/Cargo.toml`, `crates/actx-agent/Cargo.toml`, `crates/actx-installer/Cargo.toml`, `crates/actx-lm/Cargo.toml`, `crates/any-context-core-rs/Cargo.toml`, `crates/any-context-core-rs/pyproject.toml`, `pyproject.toml`, `crates/actx-cli/src/cli/args.rs`, `version.txt`: Bump para `v0.32.0`.
- `C:\Users\guilh\.gemini\config\skills\dev-cycle-protocol\SKILL.md`: Atualização dos Pilares e da Etapa 8.
- `TECDOC.md` e `README.md`: Documentação técnica do novo schema relacional e diagrama ER.
- `tests/MANUAL_TESTS.md`: Novo Cenário detalhado com verificação da tabela única normalizada.

## 4. 📚 Plano de Atualização Dual-Doc (UserDoc + TecDoc)
- **UserDoc (`README.md`)**:
  - Documentação da versão v0.32.0 destacando performance e integridade de fontes.
- **TecDoc (`TECDOC.md`)**:
  - Esquema atualizado do banco de dados SQLite sem colunas legadas e diagrama relacional Mermaid de `workspace_folders`.

## 5. 🧪 Estratégia de Testes Automatizados & Proteção contra Regressão
- Teste unitário em `sqlite.rs` validando que bancos com a coluna legada `paths_json` são migrados automaticamente para `workspace_folders` e a coluna antiga é removida sem perda de dados.
- Execução da suíte completa com `cargo test --workspace` (garantindo 100% verde nos 125+ testes).

## 6. 📝 Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)
- Adição do **Cenário 3 (v0.32.0): Validação de Tabela Única Normalizada workspace_folders e Monitoramento Ininterrupto de CI/CD**:
  - Inspeção via SQL verificando a ausência da coluna `paths_json` na tabela `workspaces`.
  - Consulta à tabela `workspace_folders` confirmando todas as pastas mapeadas em seus respectivos workspaces.
  - Teste de adição e remoção de fontes via `/folder --add` e `/folder --remove`.

## 7. 🔒 Higiene de Segredos, Migração de Banco e Plano de Rollback
- Backup automático do banco `settings.db` antes da execução do `ALTER TABLE DROP COLUMN`.
- Transação atômica (`BEGIN TRANSACTION ... COMMIT`) durante a migração.
- Verificação via `verify_gate.py` (Gates 3 e 8).

## 8. ⏱️ Passo a Passo da Execução
1. Atualizar o `dev-cycle-protocol` com a regra ininterrupta de acompanhamento de CI/CD.
2. Executar bump de versão semântico para `v0.32.0` em todos os manifestos.
3. Implementar a rotina de migração e remoção de `paths_json` em `sqlite.rs`.
4. Simplificar os métodos de manipulação de pastas para usar exclusivamente `workspace_folders`.
5. Executar `cargo test --workspace` e validar 100% de sucesso.
6. Compilar o binário release e implantar no ambiente local.
7. Atualizar `TECDOC.md`, `README.md` e `tests/MANUAL_TESTS.md`.
8. Executar commit, push, PR no GitHub, acompanhar as GitHub Actions até a publicação final da release.
