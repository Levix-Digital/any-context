---
id: 89
title: TUI Error Surfacing Guarantee and Temporary Search Depth UI Concealment
status: accepted
date: '2026-10-02'
version: v0.32.6
subsystems: [tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0089: TUI Error Surfacing Guarantee and Temporary Search Depth UI Concealment

## Context

The TUI swallowed `AgentEvent::Error` without rendering it, which looked like a silent freeze. At the same time, showing `[Search: ...]` confused users because RFC-042 Deep Search was not active yet.

## Decision

Explicitly push error events to `chat_history` with `MessageRole::System` and keep `AppStatus::Error`. Temporarily hide the search depth UI elements until RFC-042 is fully operational.

## Alternatives Considered

Not recorded.

## Consequences

- Every agent error is visible to the user.
- The search depth indicator stays hidden until Deep Search is operational.
