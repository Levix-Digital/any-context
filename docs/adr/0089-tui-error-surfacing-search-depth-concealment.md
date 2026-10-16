---
id: 89
title: TUI Error Surfacing Guarantee & Temporary Search Depth UI Concealment
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0089: TUI Error Surfacing Guarantee & Temporary Search Depth UI Concealment

- **Status**: Accepted & Implemented (`v0.32.6`).
- **Context**: TUI swallowed `AgentEvent::Error` without rendering, causing silent freezes. Concurrently, displaying `[Search: ...]` prematurely confused users since RFC-042 Deep Search is not yet active.
- **Decision**: Explicitly push error events to `chat_history` with `MessageRole::System` and preserve `AppStatus::Error`. Temporarily conceal search depth UI elements until RFC-042 is fully operational.

---
