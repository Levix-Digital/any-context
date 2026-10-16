---
id: 97
title: Canonical SQLite Sync Telemetry, Live TUI Footer Progress Bar & Native Agent Background Awareness
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0097: Canonical SQLite Sync Telemetry, Live TUI Footer Progress Bar & Native Agent Background Awareness

- **Status**: Accepted & Implemented (`v0.32.14`).
- **Context**: Users lacked visual feedback on background synchronization progress in the TUI footer, and the AI agent was unaware of background indexing tasks, leading to false negative responses when asked about indexing progress.
- **Decision**: Create a shared `workspace_sync_status` table in SQLite; connect a 250ms asynchronous polling loop in the TUI event loop to display live progress in `render_footer`; register a native `system_status` agent tool reading SQLite telemetry and LanceDB chunks; and instruct the agent via a lean system prompt skill to report real-time background status autonomously.

---
