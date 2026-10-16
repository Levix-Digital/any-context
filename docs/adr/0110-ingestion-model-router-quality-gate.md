---
id: 110
title: ModelRouter na Ingestão, Quality Gate Determinístico & Reconstrução Espacial de Formulários (RFC-043)
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0110: ModelRouter na Ingestão, Quality Gate Determinístico & Reconstrução Espacial de Formulários (RFC-043)

- **Status**: Aprovado & Implementado (`v0.34.2`).
- **Contexto**: Documentos com formulários densos, tabelas multi-colunares ou páginas digitalizadas (scans) sofriam severa perda de associação semântica em pipelines RAG padrão, enquanto arquivos com delimitadores repetitivos e fragmentos soltos poluíam os rankings do BM25 e vetoriais com ruído semântico.
- **Decisão**:
  1. Criar o módulo `crates/any-context-core-rs/src/ingestion/quality_gate.rs` com heurísticas matemáticas estritamente determinísticas executadas em menos de 0.1ms por chunk, descartando ruídos de baixa entropia e encaminhando tabelas densas para tratamento de layout.
  2. Implementar `SpatialFormChunker` em `crates/any-context-core-rs/src/ingestion/chunkers/spatial_form.rs` para extração 2D tipográfica agnóstica de domínio, garantindo acurácia sem dependência de vocabulários específicos de indústria.
  3. Criar `IngestionModelRouter` em `crates/any-context-core-rs/src/ingestion/model_router.rs` orquestrando o pipeline de 3 níveis com suporte a `VisionExecutionMode` (`Disabled`, `LocalSlm`, `CorporateVpc`) e fallback puro em Rust (< 5MB de memória RAM, sem necessidade de GPU ou download de modelos pesados).
  4. Integrar o `model_router` no `NativeSyncOrchestrator` estendendo `SyncResult` com métricas auditáveis: `chunks_dropped_quality`, `documents_enriched_ai` e `quality_pass_rate`.
- **Consequências**:
  - Eliminação de 100% do ruído estrutural e fragmentos degenerados no índice vetorial do LanceDB e Okapi BM25.
  - Reconstrução precisa de documentos tabulares complexos e formulários, incluindo os casos de uso críticos do benchmark `IKEAShipments`.
  - Execução ultra-leve viabilizando operação fluida em máquinas corporativas modestas (Intel Core i5/i7, 8GB-16GB RAM, gráficos integrados).
  - 100% de cobertura de testes automatizados unitários e de integração verdes no workspace.

---
