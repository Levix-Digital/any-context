---
id: 106
title: Paridade Integral de Ingestão/Recuperação em Rust e Purga do Código Legado Python
status: accepted
date: '2026-10-09'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0106: Paridade Integral de Ingestão/Recuperação em Rust e Purga do Código Legado Python

- **Status**: Aprovado & Implementado (`v0.33.0`).
- **Contexto**: A coexistência temporária entre o core em Rust e o código legado em Python criava duplicidade de manutenção, dívida técnica, dependência de empacotadores frágeis (PyInstaller) e risco de descompasso de features.
- **Decisão**:
  1. Portar todas as capacidades restantes (RAG presets, Office docx/pptx chunker, Contextual Retrieval envelopes, sitemap web crawler, web search multiprovedor) para 100% Rust nativo nos crates `any-context-core-rs` e `actx-cli`.
  2. Purgar definitivamente toda e qualquer referência, script, teste ou arquivo Python do repositório.
  3. Alinhar o pipeline de CI/CD para compilação estritamente nativa em Rust.
- **Consequências**:
  - Unificação de 100% da stack em Rust moderno (2021 edition).
  - Redução drástica do tamanho do repositório e do tempo de build no GitHub Actions (< 2 min).
  - Eliminação de 100% das falhas de runtime relacionadas a interpretadores Python ou incompatibilidades de pacotes.
  - Suite de testes do workspace 100% verde com 170+ testes unitários e de integração nativos.

---
