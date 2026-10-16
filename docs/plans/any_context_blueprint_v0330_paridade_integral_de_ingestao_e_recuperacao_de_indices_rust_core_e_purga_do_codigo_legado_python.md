---
plan_id: any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
project: any-context
title: 'Blueprint v0.33.0: Paridade Integral de Ingestao e Recuperacao de Indices (Rust Core) e Purga do Codigo Legado Python'
version: v0.33.0
status: superseded
author: Levi Silveira
created_at: '2026-10-09T11:25:02'
updated_at: '2026-10-09T14:08:55'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- rust
- python-purge
- ingestion
- retrieval
- parity
- architecture
supersedes: any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
superseded_by: any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
parent_plan_id: null
milestones:
- name: 'M1: Modelos de Presets RAG em Rust'
  done: true
- name: 'M2: Parser Nativo de Documentos de Escritorio (DOCX e PPTX)'
  done: true
- name: 'M3: ContextualEnricher e Envelopes Semanticos em Rust'
  done: true
- name: 'M4: Web Crawler Recursivo com Sitemaps e Robots.txt'
  done: true
- name: 'M5: Ferramenta Nativa de Web Search no actx-agent'
  done: true
- name: 'M6: Purga Completa do Codigo Python Legado'
  done: true
- name: 'M7: Suite de Testes Rust 100% Verdes'
  done: true
- name: 'M8: Atualizacao Dual-Doc e Release v0.33.0'
  done: true
status_log:
- at: '2026-10-09T11:35:19'
  note: 'Milestone ''M1: Modelos de Presets RAG em Rust'': Implementado RetrievalPreset com Turbo, Balanced e Deep e integrado ao HybridSearchRequest'
- at: '2026-10-09T11:39:22'
  note: 'Milestone ''M2: Parser Nativo de Documentos de Escritorio (DOCX e PPTX)'': Implementado OfficeChunker para .docx e .pptx via OpenXML SAX parsing com zip e quick-xml, integrado ao IngestionRouter'
- at: '2026-10-09T11:47:26'
  note: 'Milestone ''M3: ContextualEnricher e Envelopes Semanticos em Rust'': ContextualEnricher nativo integrado no Orchestrator com persistencia de envelopes semanticos e sync ledger diff no SQLite'
- at: '2026-10-09T11:54:15'
  note: 'Milestone ''M4: Web Crawler Recursivo com Sitemaps e Robots.txt'': NativeWebCrawler implementado com suporte a parsing de sitemaps XML e sitemap index via quick-xml, politica robots.txt RFC 9309, BFS link traversal e integracao no orquestrador sync_web_portals'
- at: '2026-10-09T12:02:17'
  note: 'Milestone ''M5: Ferramenta Nativa de Web Search no actx-agent'': NativeWebSearchEngine implementado com cascata Tavily -> Serper -> DuckDuckGo, registrado como tool nativa web_search no actx_agent via actx-cli builder quando web_search_enabled'
- at: '2026-10-09T12:05:18'
  note: 'Milestone ''M6: Purga Completa do Codigo Python Legado'': Purga completa realizada: deletados src/any_context, main.py, launcher, specs, requirements.txt, pyproject.toml, uv.lock e testes python; fluxos do GitHub Actions migrados para 100% Rust'
- at: '2026-10-09T12:07:16'
  note: 'Milestone ''M7: Suite de Testes Rust 100% Verdes'': cargo test --workspace 100% verde com todos os 170+ testes unitarios e de integracao passando com zero falhas'
- at: '2026-10-09T14:08:55'
  note: Superseded by any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.33.0: Paridade Integral de Ingestao e Recuperacao de Indices (Rust Core) e Purga do Codigo Legado Python

## 1. Objetivo & Decisoes Arquiteturais (ADR)
Migracao exaustiva de todas as capacidades de ingestao, chunking, enriquecimento contextual e recuperacao hibrida de indices do Python para o Rust Core nativo, seguida da remocao e purga completa de toda a arvore de codigo Python legado.

## 2. Paridade de Superficies
CLI, TUI, Stdio RPC Bridge, e ReAct Agent unificados operando sobre o Rust Core e a base canonica SQLite.

## 3. Marcos de Execucao
- M1: Modelos de Presets RAG (Turbo, Balanced, Deep Research) em Rust.
- M2: Parser Nativo de Documentos de Escritorio (DOCX/PPTX via ZIP+XML stream).
- M3: ContextualEnricher e Envelopes Semanticos em Rust com cache SQLite.
- M4: Web Crawler Recursivo com Sitemaps, Robots.txt e extrator limpo de texto em Rust.
- M5: Ferramenta Nativa de Web Search (DuckDuckGo/Tavily via reqwest) no actx-agent.
- M6: Purga Completa do Codigo Python legado e arquivos build/spec obsoletos.
- M7: Suite de Testes Rust 100% Verdes.
- M8: Atualizacao Dual-Doc e Release v0.33.0.
