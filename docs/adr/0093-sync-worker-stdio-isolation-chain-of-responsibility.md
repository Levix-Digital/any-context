---
id: 93
title: Sync Worker Background Stdio Isolation & Chain of Responsibility Ingestion Engine
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0093: Sync Worker Background Stdio Isolation & Chain of Responsibility Ingestion Engine

- **Status**: Accepted & Implemented (`v0.32.11`).
- **Context**: Child background sync processes wrote raw stdout/stderr into the terminal during active TUI rendering, and workspaces with only web portals generated false-positive local folder error alerts.
- **Decision**: Redirect child process stdio to dedicated workspace log files with `CREATE_NO_WINDOW` on Windows, and refactor multi-source ingestion into a Chain of Responsibility pattern where handlers independently validate source presence and skip gracefully with zero error emojis.

---
