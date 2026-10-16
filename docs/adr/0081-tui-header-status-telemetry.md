---
id: 81
title: TUI Top Header Quad-Status Telemetry
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0081: TUI Top Header Quad-Status Telemetry

- **Status**: Accepted & Implemented (`v0.32.3`).
- **Decision**: Render Workspace, Model, Grounding Mode, and Web Search status in the top bar on every render pass. Wire slash commands (`/mode`, `/web`, `/model`, `/switch`) to trigger immediate state mutation and agent rebuild.
