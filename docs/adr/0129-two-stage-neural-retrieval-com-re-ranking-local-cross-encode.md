---
id: 129
title: Two-Stage Neural Retrieval com Re-Ranking Local Cross-Encoder ONNX, Aceleração por Hardware e Fallback Gracioso ADR-110
status: accepted
date: '2026-10-15'
version: v0.44.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0129: Two-Stage Neural Retrieval com Re-Ranking Local Cross-Encoder ONNX, Aceleração por Hardware e Fallback Gracioso ADR-110

- **Contexto**:
  - Modelos RAG tradicionais baseados em recuperação de estágio único (bi-encoders densos ou fusão RRF densa + esparsa) comparam representações vetoriais de query e documentos de forma independente. Embora escaláveis e rápidos para o pool inicial, os bi-encoders sofrem com perda de granularidade sintática e semântica, uma vez que não realizam atenção cruzada (*cross-attention*) entre cada token da pergunta e o conteúdo específico de cada documento candidato.
  - Modelos de Re-Ranking Cross-Encoder processam conjuntamente a tupla `(query, document)` camada por camada no Transformer, permitindo que cada palavra da pergunta atenda a cada palavra do documento, capturando nuances de negação, relações de adjacência (bigramas) e ordem sintática com precisão substancialmente superior (ganhos de 15% a 25% no NDCG@10 e MRR).
  - No entanto, modelos de rerank comerciais em nuvem introduzem dependência de conexão externa, custos financeiros por milhão de tokens e latência adicional (150-400ms). Em contrapartida, soluções locais não podem introduzir dependências pesadas em Python nem quebrar a execução caso o usuário não tenha baixado os pesos prévios (garantia ADR-110).
- **Decisão Arquitetural**:
  1. **Arquitetura de Recuperação em Dois Estágios (Two-Stage Retrieval Pipeline)**:
     - **Estágio 1 - Recall Híbrido com RRF**: O `NativeHybridPipeline` executa busca vetorial densa paralela no LanceDB e busca lexical Okapi BM25, gerando o pool inicial de candidatos ($k=30$ a $40$) balanceado via Reciprocal Rank Fusion ($k=60$).
     - **Estágio 2 - Precision Neural Re-Ranking**: O cross-encoder neural local `bge-reranker-small` (`bge_reranker_small_int8.onnx`, ~45.2 MB) recebe os candidatos do Estágio 1 e avalia a relevância profunda via atenção cruzada bidirecional token-a-token, aplicando normalização Sigmoid nos logits para produzir scores calibrados no intervalo $[0.0, 1.0]$.
     - **Posicionamento no Pipeline**: O re-ranking atua imediatamente após a fusão de candidatos e antes da diversificação por fonte e do `ContextFormatter::u_shaped_reorder`, garantindo que os chunks mais precisos sejam selecionados e posicionados nos pólos de máxima atenção do Transformer.
  2. **Isolamento de Domínio no `actx-onnx` (`OnnxReranker`)**:
     - Implementado em `crates/actx-onnx/src/reranker.rs`, expondo métodos de scoring em lote (`score_batch`) e par individual (`score_pair`).
     - Integração com a matriz de aceleração de hardware `ExecutionDevice` (`Auto`, `DirectML`, `Metal`, `CUDA`, `Vulkan`, `CPU SIMD`).
  3. **ADR-110 Zero-Breakage Invariant & Fallback Gracioso**:
     - Se o arquivo de pesos `bge_reranker_small_int8.onnx` estiver ausente no diretório `%LOCALAPPDATA%/actx/models/`, `is_neural_active()` retorna `false`.
     - O pipeline ignora o Estágio 2 de forma 100% transparente e preserva intacta a ordenação RRF do Estágio 1, com zero erros, travamentos ou chamadas remotas de rede.
  4. **Modos Operacionais de Re-Ranking (`auto`, `on`, `off`)**:
     - `auto` (Default): Utiliza re-ranking neural se os pesos estiverem instalados; reverte silenciosamente para RRF se ausentes.
     - `on`: Força a ativação do re-ranking neural, emitindo aviso na interface com instrução para download (`actx download reranker`) caso os pesos não estejam presentes.
     - `off`: Desativa intencionalmente o Estágio 2, executando exclusivamente o Estágio 1 (RRF puro) com overhead zero de CPU/GPU.
  5. **Paridade Multi-Superfície**:
     - **Headless CLI**: Flag `--rerank <auto|on|off>` e alias `--reranker` (`actx "como funciona a autenticação?" --rerank on`).
     - **Subcomando**: `actx download reranker` (e `/download reranker`) para download assistido (~45.2 MB) com checagem de espaço e progresso em tempo real.
     - **TUI Header**: Badge reativo em tempo real `[Rerank: ⚡ ONNX: bge-small]`, `[Rerank: ⚡ RRF (Fallback)]` ou `[Rerank: 🚫 Off]`.
     - **Slash Command**: `/rerank [auto|on|off|status]` com persistência atômica no banco SQLite.
- **Diagrama Arquitetural (Two-Stage Neural Retrieval)**:
```mermaid
flowchart TD
    QUERY["Consulta do Usuário<br/>(Query Text)"] --> STAGE1["Estágio 1: Recall Híbrido<br/>(Dense LanceDB + Sparse Okapi BM25)"]
    STAGE1 --> RRF["Fusão de Ranks Recíprocos<br/>(RRF, k=60) -> Pool de 30-40 Candidatos"]
    
    RRF --> CHECK_MODE{"Modo de Rerank?<br/>(auto / on / off)"}
    
    CHECK_MODE -->|off| RRF_PASSTHROUGH["Preserva Ordem RRF<br/>(Zero Overhead Neural)"]
    
    CHECK_MODE -->|auto / on| WEIGHTS{"Pesos ONNX Presentes?<br/>(bge_reranker_small_int8.onnx)"}
    
    WEIGHTS -->|Não (ADR-110)| FALLBACK["Fallback Gracioso ADR-110<br/>Preserva Ranking RRF ($0.00 / Zero Falhas)"]
    
    WEIGHTS -->|Sim| RERANKER["Estágio 2: Cross-Encoder Neural ONNX<br/>(bge-reranker-small / DirectML / Metal / CPU SIMD)"]
    RERANKER --> CROSS_ATTN["Atenção Cruzada Bidirecional (Query x Doc)<br/>Token Match + Bigramas + Primacy"]
    CROSS_ATTN --> SIGMOID["Normalização Sigmoid -> Scores [0.0, 1.0]"]
    SIGMOID --> REORDER["Reordenação por Score Neural Decrescente"]
    
    REORDER --> DIVERSIFY["Diversificação por Fonte & Orçamento de Densidade"]
    FALLBACK --> DIVERSIFY
    RRF_PASSTHROUGH --> DIVERSIFY
    
    DIVERSIFY --> U_SHAPE["ContextFormatter::u_shaped_reorder<br/>(Anti 'Lost in the Middle')"]
    U_SHAPE --> XML_ISOLATION["Envelopamento Estrutural em Tags XML"]
    XML_ISOLATION --> LLM_INFERENCE["Injeção no Contexto do LLM / Assistente"]
```
- **Consequências**:
  - Ganho substancial de precisão e relevância semântica em consultas que exigem diferenciação fina entre chunks com termos sobrepostos.
  - Eliminação de custos e latência de rede de provedores externos de rerank.
  - Zero quebra e conformidade estrita com ADR-110.
