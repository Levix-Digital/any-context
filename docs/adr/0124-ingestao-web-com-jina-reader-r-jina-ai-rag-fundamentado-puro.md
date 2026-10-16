---
id: 124
title: Ingestão Web com Jina Reader (`r.jina.ai`), RAG Fundamentado Puro & Matriz de 7 Domínios Semânticos
status: accepted
date: '2026-10-14'
version: v0.39.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0124: Ingestão Web com Jina Reader (`r.jina.ai`), RAG Fundamentado Puro & Matriz de 7 Domínios Semânticos

- **Status**: Implemented & Verified
- **Date**: 2026-10-14
- **Context**:
  1. O crawler de websites e documentações no AnyContext operava via extração básica de tags HTML (`strip_html_tags`), que falha criticamente em portais de documentação construídos como SPAs (React, Next.js, Docusaurus, GitBook), resultando em páginas vazias ou texto corrompido sem blocos de código e tabelas.
  2. Injetar conteúdo web em tempo real diretamente na janela de contexto do LLM sem indexação prévia violaria a filosofia fundamental do AnyContext, que é exclusivamente um sistema de **RAG Fundamentado (*Grounded RAG*)**.
  3. A classificação semântica de intenções anterior (4 domínios: `Code`, `Architecture`, `Document`, `General`) não cobria adequadamente áreas críticas da engenharia de software (DevOps/Infra, Banco de Dados/Schemas e Segurança/Auditoria).
- **Decisão**:
  1. **Jina Reader Exclusivo na Ingestão (`crates/any-context-core-rs/src/ingestion/jina.rs`)**:
     - Criação do extrator `NativeJinaExtractor` integrado ao `NativeWebCrawler` (`crawler.rs`).
     - Ao executar `/sync` nas fontes web cadastradas no workspace (`/web add <url>`), o crawler consome o endpoint `https://r.jina.ai/{url}` com headers `Accept: text/markdown`, obtendo Markdown estruturado de altíssima fidelidade com zero consumo de RAM na máquina do operador.
     - Preservação estrita do controle incremental via RFC 7232 (`ETag`, `If-Modified-Since`) e hashing SHA-256.
     - Garantia ADR-110: Em caso de erro HTTP ou ausência de rede, o crawler executa fallback transparente para extração direta de HTML sanitizado sem interromper o sync.
  2. **Preservação Inviolável do RAG Puro**:
     - O modelo de inferência (família Gemma 4 local ou nuvem) lê **única e exclusivamente do índice RAG nativo** (`NativeHybridPipeline`: LanceDB + BM25). Não há consumo de web não indexada no prompt.
  3. **Matriz de 7 Domínios Semânticos (`actx-agent::router`)**:
     - Expansão do enum `QueryIntent` e de `OnnxIntent` para 7 domínios canônicos:
       - `Code`: Sintaxe, algoritmos, bugs, refatoração, panics.
       - `Architecture`: Topologia, diagramas, microsserviços, FSM, ADRs.
       - `DevOpsInfra`: CI/CD, Docker, scripts shell/PowerShell, portas, variáveis de ambiente.
       - `DataSchema`: SQL, SQLite, LanceDB, migrações, tabelas, dados tabulares.
       - `SecurityAudit`: Vulnerabilidades, credenciais, autenticação, sanitização, LGPD.
       - `Document`: Contratos, faturas, relatórios, termos fiscais, conformidade.
       - `General`: Perguntas conceituais abertas, saudações, traduções.
     - Telemetria contextual enriquecida com badges precisas na TUI e no modo headless.
- **Consequências**:
  - Ingestão perfeita de portais SPA de documentação com blocos de código e hierarquia de markdown preservados.
  - Zero risco de alucinação ou perda de fundamentação vetorial.
  - Roteamento semântico altamente granular com telemetria clara para o operador.
