---
id: 84
title: Universal UI-Agnostic Command Engine (Hexagonal Architecture)
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0084: Universal UI-Agnostic Command Engine (Hexagonal Architecture)

- **Status**: Accepted & Implemented (`v0.32.4`).
- **Context**: Slash commands and domain operations were tightly coupled to `actx-cli::tui::app::App`, preventing code reuse across upcoming GUI, Web, and Daemon interfaces.
- **Decision**: Extract all command evaluation into `any_context_core_rs::commands::CommandEngine`. Use pure input contexts (`ExecutionContext`) and output payloads (`CommandResult`, `CommandAction`, `CommandStateUpdates`). `actx-cli` serves as a thin consumer/adapter.
