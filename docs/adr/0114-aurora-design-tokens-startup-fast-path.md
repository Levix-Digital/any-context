---
id: 114
title: Centralização Modular de Design Tokens Aurora Boreal e Startup Fast-Path Híbrido (Opção C)
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0114: Centralização Modular de Design Tokens Aurora Boreal e Startup Fast-Path Híbrido (Opção C)

- **Status**: Aprovado & Implementado (`v0.34.6`).
- **Contexto**: A interface visual carecia da identidade institucional da Levix Digital (cores da Aurora Boreal), e o tempo de inicialização da TUI sofria com atrasos perceptíveis decorrentes de verificações síncronas de indexação de conhecimento do sistema a cada abertura.
- **Decisões**:
  1. **Módulo de Tema no Core (`any_context_core_rs::theme`)**: Definir `RgbColor` e `AuroraTheme` agnósticos a UI no crate de base, suportando serialização `serde` para que qualquer interface futura (Web/WASM, REST, Desktop GUI) herde os mesmos tokens.
  2. **Adaptador de Apresentação no CLI (`actx_cli::theme::UiTheme`)**: Implementar conversões para `ratatui::style::Color::Rgb` e sequências ANSI TrueColor de 24-bits.
  3. **Fast-Path no SQLite**: Pular verificações síncronas de LanceDB e BM25 no caminho crítico se a versão do SQLite bater com `CARGO_PKG_VERSION`.
  4. **Splash Loader Telemetry no Upgrade**: Renderizar splash no console padrão durante primeiro uso ou upgrade de versão para transparência total das operações de I/O de inicialização.
- **Consequências**:
  - Tempo de abertura do `actx` reduzido para menos de 50 milissegundos no regime diário.
  - Identidade visual coerente, sóbria, elegante e de alto contraste em todas as telas da TUI e nos logs de terminal.
---
