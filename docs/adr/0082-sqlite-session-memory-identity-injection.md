---
id: 82
title: SQLite Session Memory and Canonical Identity Injection
status: accepted
date: '2026-10-02'
version: v0.32.3
subsystems: [agent, storage]
supersedes: []
superseded_by: []
amends: []
amended_by: [144]
---

# ADR-0082: SQLite Session Memory and Canonical Identity Injection

## Context

When asked about past sessions or whether it had long-term memory, the agent answered with a generic LLM refusal. It had no persistent session linked to SQLite, no self-awareness directives in its system prompt and no native tool to query the workspace context.

## Decision

Connect a `SqliteSessionStore` to each workspace session (`ws_{name}`). Embed persistent memory directives and environment metadata in the system prompt. Register a native `search_db` tool for LanceDB vector search. Upgrade `ToolRegistry` to `std::sync::RwLock` so the agent can be instantiated synchronously.

## Alternatives Considered

Not recorded.

## Consequences

- Conversations persist per workspace and the agent knows it has long-term memory.
- The agent can search the workspace index on its own through `search_db`.
