---
plan_id: anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
project: any-context
title: 'Plano de Guerra v0.32.15: Paridade de Comandos Core, LanceDB Inversion Fix, Auto-Sync Spawning e Telemetria Live'
version: v0.32.15
status: completed
author: Levi Silveira
created_at: '2026-10-06T16:34:14'
updated_at: '2026-10-06T17:00:12'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags:
- war_plan
- parity
- rust_commands
- auto_sync
- telemetry
- lancedb_fix
- modal_loop_fix
supersedes: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: Correcoes Criticas e LanceDB'
  done: true
- name: 'M2: Telemetria Reativa e Spinner'
  done: true
- name: 'M3: Paridade Hexagonal de Comandos Core'
  done: true
- name: 'M4: Suite de Testes Automatizados'
  done: true
- name: 'M5: Dual-Doc e Cenario 16'
  done: true
- name: 'M6: Verificacao Gate 8 e Release v0.32.15'
  done: true
status_log:
- at: '2026-10-06T16:34:32'
  note: 'Milestone ''M1: Correcoes Criticas e LanceDB'': Corrigida ordem dos argumentos LanceDB (count_records), resolvido loop modal incremental e sanitizacao de caminhos Windows (\\?\\).'
- at: '2026-10-06T16:34:33'
  note: 'Milestone ''M2: Telemetria Reativa e Spinner'': Calculo dinamico em db_store.py/sqlite.rs, callbacks de scanning no crawler/ingestor e spinner Braille a 250ms na TUI.'
- at: '2026-10-06T16:34:34'
  note: 'Milestone ''M3: Paridade Hexagonal de Comandos Core'': Implementacao de spawn_sync_worker, auto-spawn em /folder e /web, /keys audit/modal, /config dashboard, /purge metadata cleanup, /vision, /logs tail real e /inspect --full.'
- at: '2026-10-06T16:34:35'
  note: 'Milestone ''M4: Suite de Testes Automatizados'': 127 testes unitarios do Core, 15 testes de comandos em command_tests.rs, 14 testes CLI em cli_tests.rs, e testes de integracao Python 100% verdes.'
- at: '2026-10-06T16:34:37'
  note: 'Milestone ''M5: Dual-Doc e Cenario 16'': Documentacao completa em README.md, TECDOC.md (ADR-098 e ADR-099) e Cenario 16 acumulativo nao-truncado.'
- at: '2026-10-06T17:00:12'
  note: 'Milestone ''M6: Verificacao Gate 8 e Release v0.32.15'': CI/CD no GitHub Actions completou 100% verde (Run ID: 37541081744). Todos os 12 binarios e scripts publicados com sucesso em Levix-Digital/any-context-releases.'
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.15: Live Scanning Progress Bar, Paridade de Comandos Core & Telemetria TUI

## 1. Contexto & Causa Raiz
Na versão `v0.32.14`, foram identificadas três falhas críticas em produção após a migração do Core para Rust:
1. **Inversão de Parâmetros no LanceDB**:
   - `NativeLanceStore::count_records` recebia `workspace` como primeiro parâmetro e `table_name` como segundo. No CLI (`crates/actx-cli/src/engine.rs`), as chamadas passavam `count_records(Some("workspace_chunks"), Some(&ws))`, invertendo os argumentos.
   - Consequência: O sistema consultava uma tabela inexistente e reportava "0 chunks", fazendo a IA responder falsamente: *"Sim, todas as páginas foram escaneadas e indexadas... No entanto, atualmente não há chunks indexados disponíveis."*, mesmo com mais de 45.000 chunks indexados no LanceDB.
2. **Loop Modal de Sincronização Incremental**:
   - No `/menu` da TUI (`crates/actx-cli/src/tui/app.rs`), selecionar sincronização incremental gerava o comando `/sync incremental`. O despachador em `engine.rs` comparava estritamente com `"--incremental"`, ignorando o comando e reabrindo o menu sucessivamente em loop sem acionar o worker.
3. **Congelamento da Telemetria no Rodapé (`⚡ Syncing [scanning...] │ `)**:
   - Durante o scanning/crawling de portais extensos (ex: 2.516 URLs), o SQLite retinha a string estática inicial `"[scanning...]"` porque `db_store.py` não calculava `progress_bar` quando `stage="scanning"`.
   - O leitor em `crates/any-context-core-rs/src/storage/sqlite.rs` não recalculava o texto defensivamente quando encontrava a flag estática, e a TUI não possuía animação por ticks.
4. **Lacunas de Paridade Funcional nos Comandos do Core Rust**:
   - Auditoria profunda entre a TUI antiga em Python e a arquitetura hexagonal em Rust revelou comandos com comportamento estático ou ausência de disparo de workers:
     - `/folder add <path>` adicionava o caminho no SQLite mas não disparava o worker de sincronização em background.
     - `/web add <url>` cadastrava a URL mas não iniciava o crawler.
     - Caminhos canônicos do Windows eram salvos com prefixo estendido `\\?\`, quebrando a comparação no `/folder remove`.
     - `/keys` sem argumentos não abria o modal interativo da TUI nem atualizava as variáveis de ambiente do processo.
     - `/config` sem argumentos não exibia o dashboard de configurações gerais.
     - `/purge` não limpava a tabela SQLite `file_metadata`, gerando inconsistências no re-scan.
     - `/logs` retornava mensagem estática em vez de ler as últimas linhas de `sync_<workspace>.log`.
     - `/inspect` não suportava a flag `--full` nem detalhava a contagem total de chunks da base.

---

## 2. Decisões Arquiteturais & Solução (ADR-098 & ADR-099)

### 2.1 Correção Canônica do LanceDB e Sincronização Incremental
- **Assinatura Corrigida no LanceDB**: `count_records(Some(&ws), Some("workspace_chunks"))` em todas as rotas de contagem de vetores no Core e CLI.
- **Normalização de Argumentos no `/sync`**: `execute_sync` aceita `incremental`, `--incremental`, `-i`, e a seleção no menu TUI despacha `["--incremental"]`.

### 2.2 Telemetria Reativa e Spinner Braille Dinâmico
- **Cálculo Automático & Descongelamento no SQLite (`db_store.py`)**: `format_sync_progress_bar` calcula a barra dinâmica Unicode mesmo na fase de descoberta (`[scanning... X urls/files found]`).
- **Auto-Recálculo no Rust Core (`sqlite.rs`)**: Em `get_sync_status`, se a string for estática mas houver contadores, a barra é reconstruída dinamicamente.
- **Spinner Braille Dinâmico na TUI (`app.rs`, `mod.rs`, `ui.rs`)**: Ciclo de ticks a 250ms com animação contínua (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`).

### 2.3 Paridade Hexagonal Total dos Comandos Core (`crates/any-context-core-rs/src/commands/engine.rs`)
- **`spawn_sync_worker`**: Extração de helper utilitário reutilizável que inicializa o worker de sincronização em background e atualiza o estado de telemetria no SQLite.
- **`/folder`**: Spawna automaticamente o sync worker após adicionar diretório; higieniza o prefixo `\\?\` do Windows em `execute_folder` e defensivamente em `remove_workspace_folder`.
- **`/web`**: Spawna automaticamente o crawler worker após adicionar URL.
- **`/keys`**: Sem argumentos abre o modal `keys` na TUI; com `audit` exibe o relatório de chaves configuradas; ao definir chave, persiste no SQLite `api_keys`, define a variável de ambiente via `std::env::set_var` e emite `CommandAction::RebuildAgent`.
- **`/config`**: Sem argumentos renderiza o painel completo de configurações; com chave e valor aplica `app_settings`.
- **`/purge`**: Invoca `clear_workspace_file_metadata` no SQLite além de deletar vetores no LanceDB; reseta o status de sincronização para `idle`.
- **`/vision`**: Configura `enable_vision_llm` em `app_settings`.
- **`/logs`**: Lê dinamicamente as últimas N linhas do arquivo de log real `sync_<workspace>.log` no disco.
- **`/inspect`**: Consulta a taxonomia de documentos e exibe prévia com suporte à flag `--full`.

---

## 3. Marcos de Execução (Milestones)
- **M1: Correções Críticas & LanceDB**: Correção da ordem dos parâmetros LanceDB, resolução do modal loop incremental e sanitização de caminhos Windows (`\\?\`).
- **M2: Telemetria Reativa & Spinner**: Cálculo dinâmico em `db_store.py`/`sqlite.rs`, callbacks granulares de scanning no crawler/ingestor e spinner Braille a 250ms na TUI.
- **M3: Paridade Hexagonal de Comandos Core**: Implementação de `spawn_sync_worker`, auto-spawn em `/folder` e `/web`, `/keys` audit/modal, `/config` dashboard, `/purge` metadata cleanup, `/vision`, `/logs` tail real e `/inspect --full`.
- **M4: Suíte de Testes Automatizados**: 127 testes unitários do Core, 15 testes de comandos em `command_tests.rs`, 14 testes CLI em `cli_tests.rs`, e testes de integração Python 100% verdes.
- **M5: Dual-Doc & Cenário 16 em MANUAL_TESTS.md**: Documentação completa em `README.md`, `TECDOC.md` (ADR-098 e ADR-099) e Cenário 16 acumulativo não-truncado.
- **M6: Verificação Gate 8 & Release v0.32.15**: Validação pelo `verify_gate.py`, commit `80c5a90`, tag `v0.32.15`, push para `dev` e `main`, e CI watch dos 12 binários de release.
