---
plan_id: any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
project: any-context
title: 'Blueprint v0.33.0: Full Ingestion and Index Retrieval Parity (Rust Core) and Legacy Python Code Purge'
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
- name: 'M1: RAG Preset Models in Rust'
  done: true
- name: 'M2: Native Office Document Parser (DOCX and PPTX)'
  done: true
- name: 'M3: ContextualEnricher and Semantic Envelopes in Rust'
  done: true
- name: 'M4: Recursive Web Crawler with Sitemaps and Robots.txt'
  done: true
- name: 'M5: Native Web Search Tool in actx-agent'
  done: true
- name: 'M6: Complete Purge of the Legacy Python Code'
  done: true
- name: 'M7: Rust Test Suite 100% Green'
  done: true
- name: 'M8: Dual-Doc Update and Release v0.33.0'
  done: true
status_log:
- at: '2026-10-09T11:35:19'
  note: 'Milestone ''M1: RAG Preset Models in Rust'': Implemented RetrievalPreset with Turbo, Balanced and Deep, integrated into HybridSearchRequest'
- at: '2026-10-09T11:39:22'
  note: 'Milestone ''M2: Native Office Document Parser (DOCX and PPTX)'': Implemented OfficeChunker for .docx and .pptx through OpenXML SAX parsing with zip and quick-xml, integrated into IngestionRouter'
- at: '2026-10-09T11:47:26'
  note: 'Milestone ''M3: ContextualEnricher and Semantic Envelopes in Rust'': Native ContextualEnricher integrated into the Orchestrator, persisting semantic envelopes and the sync ledger diff in SQLite'
- at: '2026-10-09T11:54:15'
  note: 'Milestone ''M4: Recursive Web Crawler with Sitemaps and Robots.txt'': NativeWebCrawler implemented with parsing of XML sitemaps and sitemap index through quick-xml, RFC 9309 robots.txt policy, BFS link traversal and integration into the sync_web_portals orchestrator'
- at: '2026-10-09T12:02:17'
  note: 'Milestone ''M5: Native Web Search Tool in actx-agent'': NativeWebSearchEngine implemented with a Tavily -> Serper -> DuckDuckGo cascade, registered as the native web_search tool in actx_agent through the actx-cli builder when web_search_enabled'
- at: '2026-10-09T12:05:18'
  note: 'Milestone ''M6: Complete Purge of the Legacy Python Code'': Complete purge done: deleted src/any_context, main.py, launcher, specs, requirements.txt, pyproject.toml, uv.lock and the Python tests; GitHub Actions workflows migrated to 100% Rust'
- at: '2026-10-09T12:07:16'
  note: 'Milestone ''M7: Rust Test Suite 100% Green'': cargo test --workspace 100% green, with all 170+ unit and integration tests passing and zero failures'
- at: '2026-10-09T14:08:55'
  note: Superseded by any_context_blueprint_v0331_purga_final_python_higiene_repo_e_dual_doc
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.33.0: Full Ingestion and Index Retrieval Parity (Rust Core) and Legacy Python Code Purge

## 1. Goal & Architecture Decisions (ADR)
Exhaustive migration of every ingestion, chunking, contextual enrichment and hybrid index retrieval capability from Python to the native Rust Core, followed by the complete removal and purge of the whole legacy Python code tree.

## 2. Surface Parity
CLI, TUI, Stdio RPC Bridge and ReAct Agent unified, running on the Rust Core and the canonical SQLite database.

## 3. Execution Milestones
- M1: RAG Preset Models (Turbo, Balanced, Deep Research) in Rust.
- M2: Native Office Document Parser (DOCX/PPTX through a ZIP+XML stream).
- M3: ContextualEnricher and Semantic Envelopes in Rust with an SQLite cache.
- M4: Recursive Web Crawler with Sitemaps, Robots.txt and a clean text extractor in Rust.
- M5: Native Web Search Tool (DuckDuckGo/Tavily through reqwest) in actx-agent.
- M6: Complete Purge of the legacy Python code and obsolete build/spec files.
- M7: Rust Test Suite 100% Green.
- M8: Dual-Doc Update and Release v0.33.0.
