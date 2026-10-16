---
id: 81
title: TUI Top Header Quad-Status Telemetry
status: accepted
date: '2026-10-02'
version: v0.32.3
subsystems: [tui, commands]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0081: TUI Top Header Quad-Status Telemetry

## Context

When the active workspace and model names moved to the TUI top header, the Grounding Mode (`Strict`, `Hybrid`, `Proactive`) and Web Search (`ON`/`OFF`) indicators were dropped, so users could not see whether retrieval augmentation or live web search was influencing answers.

## Decision

Render Workspace, Model, Grounding Mode and Web Search status in the top bar on every render pass. Wire the slash commands `/mode`, `/web`, `/model` and `/switch` to trigger an immediate state mutation and agent rebuild.

## Alternatives Considered

Not recorded.

## Consequences

- The operational state that shapes each answer is always visible.
- Changing any of these settings takes effect on the next turn without restarting.
