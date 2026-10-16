---
id: 113
title: Eliminação de Inversão Heurística em Chunker Tabular e Preservação Incondicional de Formatos Markdown 2D
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0113: Eliminação de Inversão Heurística em Chunker Tabular e Preservação Incondicional de Formatos Markdown 2D

- **Status**: Aprovado & Implementado (`v0.34.5`).
- **Contexto**: A queixa do usuário apontou que formulários como T4 e CMR eram indexados com pares chave-valor absurdos (`58792.60: 7976.60`), impedindo respostas factuais do RAG.
- **Decisões**:
  1. **Preservação de Tabelas Markdown no QualityGate**: Isentar blocos de tabela que já possuam delimitadores de cabeçalho da marcação de `DenseComplexForm`.
  2. **Pareamento Conservador em Formulários**: Restringir a identificação de campos em tabelas a cabeçalhos estritamente declarativos (`Key | Value`, `Field | Value`, etc.).
  3. **Calibração de Limiar de RRF no CLI Engine**: Ajustar `min_score` de `0.02` para `0.005` no `HybridSearchRequest` para permitir que consultas semânticas multilingues passem pelo corte de rankeamento do RRF.
  4. **Descriptografia Defensiva em Tempo de Execução**: Assegurar que consultas do BM25 e montagem de candidatos no pipeline híbrido desembalem textos `enc::` de forma transparente.
- **Consequências**:
  - Restauração total da acurácia semântica do RAG do AnyContext em documentos fiscais, faturas e relatórios logísticos.
  - O formulário fiscal T4 passa a responder valores exatos para rendimento de emprego ($58,792.60) e imposto retido na fonte ($7,976.60).
  - Preservação da compatibilidade com bases de dados existentes mediante comando `actx sync --force`.

---
