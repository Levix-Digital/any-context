---
plan_id: any_context_blueprint_v03215_live_scanning_progress_bar_reactive_tui_footer_telemetry
project: any-context
title: 'Blueprint v0.32.15: Live Scanning Progress Bar & Reactive TUI Footer Telemetry'
version: v0.32.15
status: superseded
author: Levi Silveira
created_at: '2026-10-06T14:39:19'
updated_at: '2026-10-06T16:34:14'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags:
- sync
- progress_bar
- scanning
- tui
- telemetry
- braille_spinner
supersedes: any_context_blueprint_v03214_bm25_safe_deserialization_sync_progress_bar_and_agent_awareness
superseded_by: anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
parent_plan_id: null
milestones:
- name: 'M1: Cálculo dinâmico e descongelamento em db_store.py e entrypoint.py'
  done: false
- name: 'M2: Auto-recálculo defensivo e itens descobertos em sqlite.rs'
  done: false
- name: 'M3: Emissão de progresso de scanning no crawler e ingestor'
  done: false
- name: 'M4: Spinner Braille dinâmico e animação a 250ms na TUI'
  done: false
- name: 'M5: Testes automatizados no Rust e Python'
  done: false
- name: 'M6: Dual-Doc e Cenário 16 em MANUAL_TESTS.md'
  done: false
- name: 'M7: Verificação Gate 8 e Publicação da Release v0.32.15'
  done: false
status_log:
- at: '2026-10-06T16:34:14'
  note: Superseded by anycontext_plano_de_guerra_v03215_paridade_de_comandos_core_lancedb_inversion_fix_auto_sync_spawning_e_telemetria_live
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.15: Live Scanning Progress Bar & Reactive TUI Footer Telemetry

## 1. Contexto & Causa Raiz
Na versão `v0.32.14`, a telemetria do rodapé da TUI apresentou congelamento no estado estático `⚡ Syncing [scanning...] │ ` durante toda a sincronização de grandes portais web (ex: 2.516 páginas), mesmo quando o agente via `system_status` reportava corretamente que 1.440 páginas já haviam sido escaneadas:

### Causa Raiz Identificada:
1. **Falta de Cálculo do Progress Bar no Worker Python (`entrypoint.py` / `db_store.py`)**:
   `_worker_progress` chamava `store.update_sync_status(current, total, stage, item_name)` sem fornecer `progress_bar`.
   No SQLite `update_sync_status`, o SQL executava:
   `progress_bar = COALESCE(excluded.progress_bar, workspace_sync_status.progress_bar)`
   Como `excluded.progress_bar` era `NULL`, o SQLite preservava o valor inicial `"[scanning...]"` gravado pelo Rust no boot do worker.
2. **Avaliação Estática no Leitor do Rust (`sqlite.rs`)**:
   Em `get_sync_status`, o Rust verificava `progress_bar.unwrap_or_else(...)`. Como `progress_bar` já continha `"[scanning...]"`, ele nunca recalculava a barra com base nos campos `current_item` e `total_items`.
3. **Ausência de Telemetria Durante a Descoberta/Scanning**:
   Na fase inicial de varredura (parsing de `sitemap.xml`, BFS de links de domínio e scan de arquivos locais), o sistema operava sem emitir callbacks granulares, deixando `total = 0` e `current = 0` sem indicador dinâmico.

---

## 2. Decisões Arquiteturais & Solução (ADR-098)
1. **Cálculo Automático & Descongelamento no SQLite (`db_store.py`)**:
   Em `update_sync_status`, caso `progress_bar` não seja fornecido, ele é calculado imediatamente através de `format_sync_progress_bar(current, total, stage)`.
   No SQL, `progress_bar = excluded.progress_bar` atualiza a coluna a cada item processado.
2. **Auto-Recálculo no Rust Core (`sqlite.rs`)**:
   Em `get_sync_status`, caso a string armazenada seja estática (`"[scanning...]"`, `"[crawling...]"`, `"[calculating...]"`) mas `total_items > 0` ou `current_item > 0`, o Rust recalcula dinamicamente a barra de blocos Unicode `format_sync_progress_bar`.
3. **Telemetria Granular na Fase de Scanning/Descoberta (`web_crawler.py` & `local_folder_ingestor.py`)**:
   A função `discover_site_urls` agora recebe `progress_callback` e emite atualizações conforme URLs de sitemap e links de página são descobertos (`current > 0`, `total = 0`, `stage = "scanning"`).
   A formatação de progresso exibe `[scanning... X urls/files found]`.
4. **Spinner Braille Dinâmico na TUI (`app.rs`, `mod.rs`, `ui.rs`)**:
   O `App` mantém um contador de ticks assíncronos (`tick_count: u64`) incrementado a cada 250ms.
   O rodapé da TUI anima um spinner Braille (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`) ao lado do texto de scanning, oferecendo feedback visual imediato e contínuo ao usuário de que o sistema está ativo e processando.

---

## 3. Marcos de Execução (Milestones)
- **M1**: Cálculo dinâmico e descongelamento em `db_store.py` e `entrypoint.py`.
- **M2**: Auto-recálculo defensivo e suporte a itens descobertos em `sqlite.rs`.
- **M3**: Emissão de progresso de scanning em `web_crawler.py`, `web_scheduler.py` e `local_folder_ingestor.py`.
- **M4**: Spinner Braille dinâmico e animação a 250ms no rodapé da TUI (`app.rs`, `mod.rs`, `ui.rs`).
- **M5**: Testes automatizados no Rust (`cargo test --workspace`) e Python (`unittest`).
- **M6**: Dual-Doc (`README.md`, `TECDOC.md` ADR-098) e Cenário 16 em `MANUAL_TESTS.md`.
- **M7**: Verificação Gate 8, bump `v0.32.15`, commit, tag, push e monitoramento do CI/CD até publicação de todos os 12 binários.
