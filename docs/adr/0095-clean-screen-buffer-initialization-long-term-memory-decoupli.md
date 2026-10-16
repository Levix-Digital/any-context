---
id: 95
title: Clean Screen Buffer Initialization, Long-Term Memory Decoupling & Virtual Tab Lifecycle
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0095: Clean Screen Buffer Initialization, Long-Term Memory Decoupling & Virtual Tab Lifecycle

- **Status**: Accepted & Implemented (`v0.32.13`).
- **Context**: Every launch of the application dumped old session messages directly onto the terminal chat viewport, cluttering the screen and disrupting users who wanted a clean start for their new session.
- **Decision**: Decouple the visual screen buffer from the agent's long-term memory: initialize the TUI chat history strictly with the welcome message banner on startup and unvisited workspace switches; preserve all past messages in SQLite (`actx_session_messages`) for agent retrieval; and enhance `/history` to inspect or wipe long-term memory on demand.

---
