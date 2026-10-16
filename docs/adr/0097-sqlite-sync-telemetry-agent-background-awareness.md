---
id: 97
title: Canonical SQLite Sync Telemetry, Live TUI Footer Progress Bar and Native Agent Background Awareness
status: accepted
date: '2026-10-06'
version: v0.32.14
subsystems: [storage, tui, agent]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0097: Canonical SQLite Sync Telemetry, Live TUI Footer Progress Bar and Native Agent Background Awareness

## Context

Users had no visual feedback on background synchronization in the TUI footer, and the agent did not know about background indexing, so it answered questions about indexing progress with false negatives.

## Decision

- Create a shared `workspace_sync_status` table in SQLite.
- Add a 250 ms asynchronous polling loop to the TUI event loop that shows live progress in `render_footer`.
- Register a native `system_status` agent tool that reads the SQLite telemetry and the LanceDB chunk count.
- Instruct the agent through a lean system prompt skill to report background status on its own.

## Alternatives Considered

Not recorded.

## Consequences

- Sync progress is visible in the footer and to the agent from one source of truth.
