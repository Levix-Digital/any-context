---
id: 109
title: Aceleração de CI/CD com Sccache e Otimizações de Perfil de Release
status: accepted
date: '2026-10-09'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0109: Aceleração de CI/CD com Sccache e Otimizações de Perfil de Release

- **Status**: Aprovado & Implementado (`v0.34.1`).
- **Contexto**: A compilação limpa do ecossistema Rust (com LanceDB, Apache Arrow, DataFusion e Tree-sitter em C) demandava ~50 minutos no runner Windows do GitHub Actions, gerando atrito e lentidão no ciclo de release.
- **Decisão**:
  1. Configurar `mozilla-actions/sccache-action@v0.0.11` com `SCCACHE_GHA_ENABLED: "true"` e `RUSTC_WRAPPER: "sccache"` nos workflows `release.yml` e `e2e-tests.yml`.
  2. Ajustar `swatinem/rust-cache` para `cache-targets: "false"`, focando exclusivamente no cache do registro de crates.
  3. Adicionar permissão explícita `actions: write` aos workflows.
  4. Adicionar perfil `[profile.release]` no `Cargo.toml` com `opt-level = 3`, `codegen-units = 16`, `lto = "off"`, `strip = true` e `debug = false`.
- **Consequências**:
  - Eliminação de recompilações desnecessárias de dependências externas após bumps de versão.
  - Redução drástica do tempo de execução dos pipelines no GitHub Actions.
  - Rastreabilidade via telemetria nativa `${SCCACHE_PATH} --show-stats` nos logs da esteira.

---
