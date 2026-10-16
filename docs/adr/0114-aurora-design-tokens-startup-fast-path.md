---
id: 114
title: Modular Aurora Borealis Design Tokens and Hybrid Startup Fast-Path (Option C)
status: accepted
date: '2026-10-10'
version: v0.34.6
subsystems: [tui, cli]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0114: Modular Aurora Borealis Design Tokens and Hybrid Startup Fast-Path (Option C)

## Context

The interface lacked the Levix Digital visual identity (Aurora Borealis colors), and TUI startup had noticeable delays caused by synchronous checks of the system knowledge index on every launch.

## Decision

1. **Theme module in the core (`any_context_core_rs::theme`)**: define UI-agnostic `RgbColor` and `AuroraTheme` in the base crate, serializable with `serde`, so any future interface (Web/WASM, REST, desktop GUI) inherits the same tokens.
2. **Presentation adapter in the CLI (`actx_cli::theme::UiTheme`)**: convert the tokens to `ratatui::style::Color::Rgb` and to 24-bit ANSI TrueColor sequences.
3. **SQLite fast-path**: skip the synchronous LanceDB and BM25 checks on the critical path when the version stored in SQLite matches `CARGO_PKG_VERSION`.
4. **Splash loader on upgrade**: render a splash on the standard console on first use or after a version upgrade, so the startup I/O is visible.

## Alternatives Considered

The title records this as "Option C" of the startup alternatives; the other options were not recorded.

## Consequences

- Daily `actx` startup takes under 50 milliseconds.
- Coherent, sober, high-contrast visual identity across all TUI screens and terminal logs.
