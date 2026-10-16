---
id: 84
title: Universal UI-Agnostic Command Engine (Hexagonal Architecture)
status: accepted
date: '2026-10-02'
version: v0.32.4
subsystems: [commands, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0084: Universal UI-Agnostic Command Engine (Hexagonal Architecture)

## Context

Slash commands and domain operations were tightly coupled to `actx-cli::tui::app::App`, which prevented reusing them in future GUI, web and daemon interfaces.

## Decision

Extract all command evaluation into `any_context_core_rs::commands::CommandEngine`. Use pure input contexts (`ExecutionContext`) and output payloads (`CommandResult`, `CommandAction`, `CommandStateUpdates`). `actx-cli` becomes a thin consumer and adapter.

## Alternatives Considered

Not recorded.

## Consequences

- Command behavior is defined once in the core and shared by every surface.
- Presentation layers only render results and apply the requested actions.
