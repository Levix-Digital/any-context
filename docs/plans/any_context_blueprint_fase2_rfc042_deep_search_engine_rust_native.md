---
plan_id: any_context_blueprint_fase2_rfc042_deep_search_engine_rust_native
project: any-context
title: 'Blueprint Fase 2: Motor Deep Search Agentico Reflexivo Nativo em Rust (RFC-042 & DeepSearcher Reference)'
version: v0.34.0
status: completed
author: Levi Silveira
created_at: '2026-10-09T18:15:14'
updated_at: '2026-10-09T19:56:16'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- deep-search
- rfc-042
- fase-2
- rust
- agent
- deepsearcher
- rag
supersedes: any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
superseded_by: null
parent_plan_id: anycontext_master_roadmap_v0_30_38_rfc042
milestones:
- name: 'M1: Modulo DeepSearchOrchestrator e Prompts Reflexivos em actx-agent'
  done: true
- name: 'M2: Integracao de Batch Retrieval com NativeHybridPipeline e Reranker'
  done: true
- name: 'M3: Loop Iterativo de Reflexao Analise de Lacunas e Eventos Reativos'
  done: true
- name: 'M4: Sintese com Proveniencia Estruturada de Arquivos e Linhas'
  done: true
- name: 'M5: Superficie CLI/TUI Comandos /deep /fast /search e Arvore de Reflexao'
  done: true
- name: 'M6: Suite Completa de Testes Unitarios e E2E'
  done: true
- name: 'M7: Dual-Doc README TECDOC ADR-108 e Registro Cenario 23 em MANUAL_TESTS'
  done: true
- name: 'M8: Versionamento v0.34.0 Gate 8 Release CI Watch e Publicacao'
  done: true
status_log:
- at: '2026-10-09T19:56:16'
  note: Fase 2 entregue com 100% dos 8 marcos concluidos, Gate 8 aprovado, CI green e v0.34.0 publicado
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint Fase 2: Motor Deep Search Agêntico Reflexivo Nativo em Rust (RFC-042 & DeepSearcher Reference)

## 📌 Metadados
- **Projeto**: `any-context` (`actx`)
- **Versão Alvo**: `v0.34.0`
- **Fase do Master Plan**: Fase 2 (RFC-042 Deep Search & ModelRouter)
- **Referência Técnica**: `https://github.com/zilliztech/deep-searcher`
- **Status**: Em Planejamento / Aguardando Aprovação (Gate 3)

---

## 1. 🔍 Contexto & Aprendizados do Repositório de Referência (`zilliztech/deep-searcher`)

O repositório de referência estabelece os princípios fundamentais de um motor de busca profunda baseada em agentes:
1. **Decomposição Ortogonal (`SUB_QUERY_PROMPT`)**: Quebra de perguntas complexas em até 4 sub-perguntas independentes (ou 1 se simples).
2. **Filtragem / Reranking de Chunks (`RERANK_PROMPT`)**: Validação semântica booleana ("YES" / "NO") de chunks candidatos antes de aceitá-los no corpus de evidências, reduzindo ruído e alucinação.
3. **Avaliação Iterativa de Lacunas (`REFLECT_PROMPT`)**: Inspeção cruzada entre pergunta original, sub-perguntas já exploradas e evidências acumuladas, gerando até 3 novas "gap queries" se restarem pontos cegos, ou encerrando imediatamente se as informações forem suficientes.
4. **Síntese de Conteúdo (`SUMMARY_PROMPT`)**: Geração da resposta final consolidando todos os chunks aceitos.

### 🚀 Vantagens Nativas & Diferenciais do AnyContext:
- **100% Pure Native Rust**: Zero latência de Python, execução do loop em milissegundos com paralelismo real via `tokio::join_all` e `retrieve_hybrid_batch_async`.
- **Hybrid RAG Robusto**: Combinação nativa de vetores no LanceDB + léxico BM25 Okapi em memória + fusão RRF ($k=60$).
- **Proveniência de Código em Nível de Linha**: Cada chunk do AnyContext possui `file_path`, `file_name`, `start_line`, `end_line` e `header_path` (Tree-sitter), permitindo síntese com citações exatas de código (`[crates/core/src/router.rs:45-60]`).
- **Telemetria Reativa para TUI**: Emissão de eventos tipados em tempo real (`AgentEvent::Decomposition`, `AgentEvent::IterationStart`, `AgentEvent::GapAnalysis`) para renderização visual da árvore de reflexão.

---

## 2. 🏛️ Arquitetura do `DeepSearchOrchestrator` (`crates/actx-agent/src/deep_search.rs`)

```mermaid
sequenceDiagram
    autonumber
    actor User as Usuário / TUI (/deep)
    participant DSO as DeepSearchOrchestrator
    participant LM as actx-lm (LmProvider)
    participant NHP as NativeHybridPipeline (LanceDB + BM25)
    participant UI as EventSender (TUI Event Loop)

    User->>DSO: run("Como funciona a ingestão e busca no AnyContext?", max_iter=3)
    DSO->>LM: 1. Decomposição (SUB_QUERY_PROMPT)
    LM-->>DSO: Retorna [sub_q1, sub_q2, sub_q3]
    DSO->>UI: emit(AgentEvent::Decomposition { sub_queries })

    loop Iterações de Reflexão (iter 1 até max_iter)
        DSO->>UI: emit(AgentEvent::IterationStart { iter, max_iter })
        DSO->>NHP: 2. retrieve_hybrid_batch_async([sub_queries])
        NHP-->>DSO: Retorna chunks com deduplicação SHA-256 e score RRF
        
        opt Reranking & Filtragem Rápida
            DSO->>LM: Valida relevância de chunks limítrofes (RERANK_PROMPT)
            LM-->>DSO: Chunks aceitos / rejeitados
        end
        
        DSO->>LM: 3. Análise de Lacunas (REFLECT_PROMPT com evidências acumuladas)
        LM-->>DSO: Gap queries ou lista vazia (suficiente)
        
        alt Informações Suficientes ou Lista Vazia
            DSO->>UI: emit(AgentEvent::GapAnalysis { is_sufficient: true, missing: [] })
            note over DSO: Interrompe o loop (early stopping)
        else Restam Lacunas e iter < max_iter
            DSO->>UI: emit(AgentEvent::GapAnalysis { is_sufficient: false, missing: gap_queries })
            note over DSO: sub_queries = gap_queries para próxima iteração
        end
    end

    DSO->>LM: 4. Síntese Final (SUMMARY_PROMPT + Evidências com Proveniência)
    LM-->>DSO: Resposta final estruturada com citações [arquivo:linhas]
    DSO->>UI: emit(AgentEvent::Delta / Done)
    DSO-->>User: AgentResponse
```

---

## 3. 📋 Marcos de Execução (Milestones)

- **M1: Módulo `DeepSearchOrchestrator` e Prompts Reflexivos em `actx-agent`**:
  - Criação de `crates/actx-agent/src/deep_search.rs`.
  - Implementação dos prompts em inglês padronizados: `SUB_QUERY_PROMPT`, `RERANK_PROMPT`, `REFLECT_PROMPT` e `SUMMARY_PROMPT`.
  - Parser resiliente de respostas (extração de listas JSON/Python mesmo com textos explicativos ou blocos `<think>`).

- **M2: Integração de Batch Retrieval com `NativeHybridPipeline` e Reranker**:
  - Conexão do `DeepSearchOrchestrator` com `retrieve_hybrid_batch_async`.
  - Deduplicação de chunks entre iterações e acumulação de score.
  - Reranker opcional/configurável (comportamento híbrido: aceitação direta para scores altos e filtro semântico para limítrofes).

- **M3: Loop Iterativo de Reflexão, Análise de Lacunas e Eventos Reativos**:
  - Loop determinístico com teto `max_iter` (padrão: 3).
  - Emissão síncrona dos eventos `AgentEvent::Decomposition`, `AgentEvent::IterationStart`, `AgentEvent::GapAnalysis`.
  - Early stopping se `gap_queries` for vazio ou sobreposição for superior a 90%.

- **M4: Síntese com Proveniência Estruturada de Arquivos e Linhas**:
  - Formatação dos chunks com cabeçalhos semânticos e identificação clara de arquivos e intervalos de linhas.
  - Inclusão de referências verificáveis na resposta final gerada pelo modelo.

- **M5: Superfície CLI/TUI: Comandos `/deep`, `/fast`, `/search` e Árvore de Reflexão**:
  - Suporte ao comando `/deep <pergunta>` no dispatcher da CLI/TUI para forçar o Deep Search.
  - Suporte ao comando `/fast <pergunta>` para resposta direta em turno único.
  - Suporte ao comando `/search <auto|fast|deep>` para gravar a política do workspace ativo no banco SQLite.
  - Renderização na TUI do status reativo das sub-perguntas e iterações.

- **M6: Suíte Completa de Testes Unitários e E2E**:
  - Testes unitários em `actx-agent` cobrindo decomposição, parsing de gap queries, filtragem e síntese com `MockLmProvider`.
  - Testes de integração em `tests/` validando o fluxo completo com `NativeHybridPipeline` real em SQLite/LanceDB temporário.
  - 100% de testes verdes em `cargo test --workspace`.

- **M7: Dual-Doc (README + TECDOC com ADR-108) e Cenário 23 em `MANUAL_TESTS.md`**:
  - Atualização do `README.md` documentando o comando `/deep` e o modo Deep Search.
  - Atualização do `TECDOC.md` com a Seção 101 e o **ADR-108 (Motor Deep Search Reflexivo e Roteamento Adaptativo)**.
  - Inclusão do **Cenário 23** em `tests/MANUAL_TESTS.md` de forma não-truncada.

- **M8: Versionamento v0.34.0, Gate 8, Release CI Watch e Publicação**:
  - Bump de versão para `0.34.0`.
  - Verificação de Gate 8 (`verify_gate.py --gate 8`).
  - Commit, tag `v0.34.0`, push e acompanhamento contínuo dos builds no GitHub Actions até a confirmação de publicação em `Levix-Digital/any-context-releases`.
