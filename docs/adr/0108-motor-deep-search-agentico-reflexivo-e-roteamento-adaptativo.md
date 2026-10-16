---
id: 108
title: Motor Deep Search Agêntico Reflexivo e Roteamento Adaptativo (RFC-042)
status: accepted
date: '2026-10-09'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0108: Motor Deep Search Agêntico Reflexivo e Roteamento Adaptativo (RFC-042)

- **Status**: Aprovado & Implementado (`v0.34.0`).
- **Contexto**: Consultas complexas em bases de documentação corporativas frequentemente falham em abordagens RAG de turno único (*single-turn*), pois perguntas multifacetadas requerem investigação iterativa, refinamento de termos e síntese de evidências dispersas em múltiplos módulos e arquivos.
- **Decisão**:
  1. Implementar o `DeepSearchOrchestrator` em `crates/actx-agent/src/deep_search.rs`, estruturado em 4 etapas (Decomposição Ortogonal, Recuperação Híbrida em Lote, Análise de Lacunas/Reflexão e Síntese Final).
  2. Integrar o trait assíncrono `DeepSearchRetriever` com o `NativeHybridPipeline` existente (`LanceDB` + `BM25 Okapi` + `RRF` $k=60$), preservando deduplicação por hash de conteúdo e proveniência exata de linhas.
  3. Adicionar eventos reativos tipados (`AgentEvent::Decomposition`, `AgentEvent::IterationStart`, `AgentEvent::GapAnalysis`) para que as interfaces (TUI e CLI) informem ao usuário a evolução da investigação em tempo real.
  4. Disponibilizar os comandos de atalho `/deep`, `/fast` e a persistência de configuração `/search` em todas as superfícies de comando.
- **Consequências**:
  - Respostas substancialmente mais ricas, precisas e embasadas para investigações arquiteturais complexas.
  - Eliminação de alucinações através do descarte de chunks não correlatos e citação estrita de arquivos e intervalos de linhas.
  - Preservação da latência ultrarrápida (graças ao paralelismo de Rust e *early stopping* quando as informações acumuladas são suficientes).
  - 100% de cobertura com testes unitários e de integração verdes em todo o workspace.

---
