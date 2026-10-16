---
id: 77
title: Native Rust CLI and Full-Screen Interactive TUI (actx-cli with Ratatui)
status: accepted
date: '2026-09-25'
version: v0.31.0
subsystems: [cli, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0077: Native Rust CLI and Full-Screen Interactive TUI (actx-cli with Ratatui)

## Context

Relying on Python wrappers or Bun sidecars (OpenTUI) caused binary bloat (over 150 MB), slow startup (over 800 ms) and fragile cross-platform process orchestration.

## Decision

Adopt Ratatui 0.29 and Crossterm 0.28 as the primary interface of AnyContext in `crates/actx-cli`, packaging everything into a single static native executable (`actx.exe` / `actx`).

## Alternatives Considered

The Python wrappers and the Bun-based OpenTUI sidecar were the approaches being replaced; no other alternative was recorded.

## Consequences

- Instant cold start (under 10 ms) and a memory footprint under 40 MB RSS.
- No external runtime dependencies (no Python, Bun or Node.js).
- Full feature parity across Windows, macOS and Linux terminals.
