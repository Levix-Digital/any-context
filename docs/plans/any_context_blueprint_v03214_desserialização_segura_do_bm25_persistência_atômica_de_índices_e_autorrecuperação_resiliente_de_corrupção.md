---
plan_id: any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
project: any-context
title: 'Blueprint v0.32.14: Desserialização Segura do BM25, Persistência Atômica de Índices e Autorrecuperação Resiliente de Corrupção'
version: v0.32.14
status: superseded
author: Levi Silveira
created_at: '2026-10-06T12:51:14'
updated_at: '2026-10-09T11:25:02'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: any_context_blueprint_v03213_clean_screen_startup_and_long_term_memory_preservation
superseded_by: any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
parent_plan_id: null
milestones:
- name: 'M1: Desserialização Bounded e Panic-Safe no Core Rust'
  done: false
- name: 'M2: Persistência Atômica de Índices Binários'
  done: false
- name: 'M3: Pipeline Resiliente e Quarentena de Índices Corrompidos'
  done: false
- name: 'M4: Sanitização Local e Testes Automatizados'
  done: false
- name: 'M5: Dual-Doc e Testes Manuais Acumulativos'
  done: false
- name: 'M6: Release de Produção v0.32.14 e CI Watch'
  done: false
status_log:
- at: '2026-10-09T11:25:02'
  note: Superseded by any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.14: Deserialização Segura do BM25, Persistência Atômica de Índices e Autorrecuperação Resiliente de Corrupção

## 1. Visão Geral & Contexto da Falha Crítica
Durante a execução de consultas no workspace `IKEAShipments` com a ferramenta `search_db`, a aplicação sofreu um travamento catastrófico (process abort / panic):
```
memory allocation of 7939688266103746149 bytes failed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Análise Forense da Causa Raiz
1. **O Número da Alocação**:
   `7939688266103746149` em hexadecimal é `0x6e2f706968736e65`, que corresponde exatamente aos bytes ASCII `'enship/n'` (fragmento de `citizenship/news` de URLs governamentais indexadas).
2. **Condição de Corrida na Escrita**:
   O arquivo `bm25_index.bin` (com mais de 32.000 documentos e 85 MB) vinha sendo gravado diretamente via `std::fs::File::create(path)` de forma não-atômica.
   Quando o worker de sincronização em segundo plano e a interface TUI/search_db gravaram concorrentemente no mesmo arquivo, os bytes de dois documentos distintos se sobrepuseram no offset `5976850`.
3. **Desserialização Irrestrita (`bincode`)**:
   Na leitura através de `bincode::deserialize_from(reader)` em `BM25Index::load_from_file`:
   - Por padrão, o `bincode` não impõe limites na leitura de tamanhos de coleções/strings.
   - Ao encontrar os 8 bytes corrompidos `b'enship/n'`, ele os interpretou como o tamanho de uma string `workspace`, tentando invocar `Vec::with_capacity(7939688266103746149)`.
   - O alocador global do Rust entrou imediatamente em pânico com aborto imediato do processo, derrubando a TUI e qualquer worker sem possibilidade de interceptação por blocos `try/catch` em Python ou `Result` em Rust.

---

## 2. Decisões Arquiteturais (ADR-096)

1. **Desserialização Defensiva e Limitada (`bincode::Options`)**:
   - Configurar o deserializador `bincode` com limite estrito de alocação: `bincode::DefaultOptions::new().with_limit(100 * 1024 * 1024)`.
   - Caso qualquer campo exija uma alocação superior ao limite (ou absurda como 7.9 exabytes), o `bincode` aborta imediatamente retornando um erro canônico `Err(bincode::ErrorKind::SizeLimit)` sem tentar alocar memória no heap.
   - Envolver a operação em `std::panic::catch_unwind` como proteção de segunda camada contra panics inesperados.

2. **Persistência Atômica via Arquivo Temporário e Rename (`atomic_save`)**:
   - Gravar novos índices BM25 sempre em arquivo temporário único (`<path>.tmp.<pid>.<timestamp>`), dar flush/sync no disco e realizar substituição atômica via renomeação (`std::fs::rename` / replace).
   - Elimina qualquer possibilidade de leitores acessarem arquivos semi-escritos ou gravadores concorrentes corromperem fluxos binários.

3. **Autorrecuperação Resiliente de Índices Corrompidos (*Self-Healing Recovery*)**:
   - No `NativeHybridPipeline::get_or_load_bm25` e `HybridRetrieverEngine::load_bm25_from_file`: se a leitura falhar por corrupção ou erro de limite, a aplicação:
     1. Registra alerta detalhado nos logs.
     2. Move/quarentena o arquivo binário defeituoso (`.corrupt.<timestamp>`).
     3. Instancia um índice BM25 limpo e, havendo registros íntegros no LanceDB (`workspace_chunks`), reconstrói o índice lexical de forma resiliente.
     4. Grava atomicamente o novo índice válido.

---

## 3. Marcos de Execução (Milestones)

- **M1: Desserialização Bounded e Panic-Safe no Core Rust (`bm25.rs`)**:
  Implementar limites estritos no `bincode` e barreira `catch_unwind` em `BM25Index::load_from_file`.
- **M2: Persistência Atômica de Índices Binários (`bm25.rs`)**:
  Implementar escrita em arquivo temporário com rename atômico em `save_to_file`.
- **M3: Pipeline Resiliente e Quarentena de Índices Corrompidos (`pipeline.rs` & `hybrid.rs`)**:
  Integrar detecção de corrupção, quarentena e inicialização/recuperação segura sem travamento.
- **M4: Sanitização da Base Local e Testes Automatizados**:
  Quarentenar o arquivo corrompido atual em `%LOCALAPPDATA%`, adicionar testes automatizados no Rust cobrindo alocações anômalas e testes de roundtrip concorrente.
- **M5: Padrão Dual-Doc & Testes Manuais Acumulativos**:
  Atualizar `README.md`, `TECDOC.md` com ADR-096 e adicionar Cenário 15 em `tests/MANUAL_TESTS.md`.
- **M6: Release de Produção v0.32.14 & Acompanhamento Ininterrupto do CI**:
  Bump de versão para `0.32.14`, commit, tag, push e monitoramento do CI até disponibilização pública dos binários.
