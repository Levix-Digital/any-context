---
id: 95
title: Clean Screen Buffer Initialization, Long-Term Memory Decoupling and Virtual Tab Lifecycle
status: accepted
date: '2026-10-06'
version: v0.32.13
subsystems: [tui, agent, storage]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0095: Clean Screen Buffer Initialization, Long-Term Memory Decoupling and Virtual Tab Lifecycle

## Context

Every launch dumped old session messages onto the chat viewport, cluttering the screen for users who wanted a clean start.

## Decision

Decouple the visual screen buffer from the agent's long-term memory: on startup and on the first visit to a workspace, initialize the TUI chat history with the welcome banner only; keep all past messages in SQLite (`actx_session_messages`) for the agent to use; and extend `/history` to inspect or wipe long-term memory on demand.

## Alternatives Considered

Not recorded.

## Consequences

- Each session starts with a clean screen while the agent keeps its memory.
- Users can inspect or erase that memory explicitly with `/history`.
