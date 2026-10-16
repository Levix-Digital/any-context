---
plan_id: any_context_blueprint_v03214_bm25_safe_deserialization_sync_progress_bar_and_agent_awareness
project: any-context
title: 'Blueprint v0.32.14: Deserialização Segura BM25, Barra de Progresso /sync na TUI e Auto-Consciência de Background do Agente'
version: v0.32.14
status: superseded
author: Levi Silveira
created_at: '2026-10-06T12:53:02'
updated_at: '2026-10-06T14:39:19'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: null
superseded_by: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
parent_plan_id: null
milestones:
- name: 'M1: Desserialização Bounded e Panic-Safe no Core Rust'
  done: true
- name: 'M2: Autorrecuperação de Índices e Sanitização Local'
  done: true
- name: 'M3: Tabela Canônica de Telemetria de Sincronização'
  done: true
- name: 'M4: Barra de Progresso Reativa no Rodapé da TUI'
  done: true
- name: 'M5: Ferramenta Nativa system_status e Auto-Consciência do Agente'
  done: true
- name: 'M6: Suíte de Testes Automatizados no Rust e Python'
  done: true
- name: 'M7: Dual-Doc e Testes Manuais Acumulativos'
  done: true
- name: 'M8: Publicação da Release v0.32.14 e CI Watch'
  done: true
status_log:
- at: '2026-10-06T13:19:37'
  note: 'Milestone ''M1: Desserialização Bounded e Panic-Safe no Core Rust'': Implementado bincode bounded a 150MB, panic safe catch_unwind e gravação atômica via temp-rename'
- at: '2026-10-06T13:19:45'
  note: 'Milestone ''M2: Autorrecuperação de Índices e Sanitização Local'': Implementado isolamento em .corrupt.<timestamp>, fallback em memória e saneamento de arquivos corrompidos'
- at: '2026-10-06T13:19:46'
  note: 'Milestone ''M3: Tabela Canônica de Telemetria de Sincronização'': Adicionada tabela workspace_sync_status no SQLite Rust e Python, atualizada pelos workers'
- at: '2026-10-06T13:19:47'
  note: 'Milestone ''M4: Barra de Progresso Reativa no Rodapé da TUI'': Adicionado polling de 250ms na TUI e render_footer com barra e estado up to date'
- at: '2026-10-06T13:19:48'
  note: 'Milestone ''M5: Ferramenta Nativa system_status e Auto-Consciência do Agente'': Criada tool system_status e Skill system-status instruindo modelo a relatar telemetria'
- at: '2026-10-06T13:19:49'
  note: 'Milestone ''M6: Suíte de Testes Automatizados no Rust e Python'': cargo test --workspace 100% verde (127+8+1 testes) e unit tests Python passando'
- at: '2026-10-06T13:19:50'
  note: 'Milestone ''M7: Dual-Doc e Testes Manuais Acumulativos'': Atualizados README.md, TECDOC.md (ADR-096 e ADR-097) e adicionado Cenário 15 em MANUAL_TESTS.md'
- at: '2026-10-06T13:45:57'
  note: 'Milestone ''M8: Publicação da Release v0.32.14 e CI Watch'': Release v0.32.14 publicada com sucesso no GitHub Actions (build 37518214630) com todos os 12 ativos no repositório Levix-Digital/any-context-releases'
- at: '2026-10-06T13:46:01'
  note: Release v0.32.14 finalizada e publicada com sucesso com todos os 8 marcos cumpridos
- at: '2026-10-06T14:39:19'
  note: Superseded by any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.14: Deserialização Segura BM25, Barra de Progresso /sync na TUI e Auto-Consciência de Background do Agente

## 1. Visão Geral & Escopo Unificado (v0.32.14)

Esta versão atende a todos os requisitos e correções levantados pós-v0.32.13:
1. **Pilar 1 (Emergência Crítica)**: Neutralização definitiva do crash `memory allocation of 7939688266103746149 bytes failed` no BM25 através de desserialização bounded (`bincode::DefaultOptions`), proteção panic-safe (`catch_unwind`), persistência atômica com arquivo temporário (`atomic_rename`) e autorrecuperação resiliente com quarentena de índices corrompidos.
2. **Pilar 2 (Paridade Visual Hexagonal)**: Restauração da barra de progresso do `/sync` reativa e compacta na barra inferior (footer) da TUI nativa em Rust, alimentada pelo estado canônico do Core em tempo real.
3. **Pilar 3 (Cognição e Auto-Consciência do Sistema)**: Disponibilização da ferramenta nativa `system_status` / auto-consciência para o agente, permitindo responder com precisão factual quando o usuário perguntar *"Já foi tudo indexado?"* ou *"Qual o status da sincronização em background?"*.

---

## 2. Decisões Arquiteturais (ADRs)

### ADR-096: Desserialização Bounded, Persistência Atômica e Self-Healing no BM25
- **Bincode com Limite Estrito**: `bincode::DefaultOptions::new().with_limit(100 * 1024 * 1024)`. Qualquer tentativa de ler comprimentos anômalos (como 7.9 exabytes) retorna imediatamente `Err(SizeLimit)` sem alocar heap.
- **Barreira Panic-Safe**: `std::panic::catch_unwind` em `BM25Index::load_from_file` para blindar o processo contra abortos do alocador.
- **Gravação Atômica**: `BM25Index::save_to_file` escreve em `<path>.tmp.<pid>.<timestamp>` e executa renomeação atômica (`std::fs::rename`).
- **Self-Healing Recovery**: Se o índice estiver corrompido, isola em `.corrupt.<timestamp>`, instancia índice limpo e reconstrói chunks a partir dos registros íntegros do LanceDB.

### ADR-097: Telemetria Canônica de Sincronização em SQLite e Paridade Multi-Superfície
- **Tabela Canônica no Core**: `workspace_sync_status` em `settings.db` (armazenando workspace, status, percentual, total_items, current_item, stage, item_name, updated_at, error).
- **Paridade de Superfícies**: O worker de sincronização (Python ou Rust) publica o progresso no SQLite.
- **Reatividade na TUI**: A TUI consulta periodicamente a telemetria do workspace ativo no SQLite e projeta no rodapé (`render_footer`):
  `⚡ Syncing [████░░░░] 50% (15/30 files) │ [F1 / /menu] Menu  ...`
- **Ferramenta Nativa `system_status` no Agente**: Registrada no motor ReAct do agente, consultando o estado de sincronização e estatísticas do workspace ativo no SQLite, permitindo que o modelo responda consultas de status operacional sem alucinar e com zero consumo de tokens de busca.

---

## 3. Marcos de Execução (Milestones)

- **M1: Desserialização Bounded e Panic-Safe no Core Rust (`bm25.rs`)**:
  Limites de alocação no bincode, barreira catch_unwind e gravação atômica via tempfile.
- **M2: Autorrecuperação de Índices e Sanitização Local (`pipeline.rs` & `%LOCALAPPDATA%`)**:
  Quarentena automática de arquivos corrompidos e reconstrução a partir do LanceDB.
- **M3: Tabela Canônica de Telemetria de Sincronização (`NativeConfigDb` & `ConfigDBStore`)**:
  Criação da tabela `workspace_sync_status`, métodos de leitura/escrita de telemetria no Core em Rust e Python.
- **M4: Barra de Progresso Reativa no Rodapé da TUI (`actx-cli/src/tui/ui.rs` & `app.rs`)**:
  Atualização da barra inferior com exibição dinâmica de progresso de sincronização em segundo plano.
- **M5: Ferramenta Nativa `system_status` e Auto-Consciência do Agente (`engine.rs` & `prompt.rs`)**:
  Registro da ferramenta no agente para responder a perguntas como *"Já foi tudo indexado?"*.
- **M6: Suíte de Testes Automatizados no Rust e Python**:
  Testes cobrindo detecção de alocações anômalas no BM25, telemetria de sync e ferramenta `system_status`.
- **M7: Dual-Doc (`README.md`, `TECDOC.md`) e Testes Manuais Acumulativos (`MANUAL_TESTS.md` Cenário 15)**:
  Documentação técnica completa e cenários de homologação não-truncados.
- **M8: Publicação da Release v0.32.14 & Acompanhamento Ininterrupto do CI**:
  Bumps de versão, tags git, push e monitoramento do workflow remoto até publicação dos 12 assets no repositório de releases.
