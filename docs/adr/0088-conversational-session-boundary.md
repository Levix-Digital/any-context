---
id: 88
title: High-Level Conversational Session Boundary and Orphaned Tool Scrubbing
status: accepted
date: '2026-10-02'
version: v0.32.6
subsystems: [agent, storage]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0088: High-Level Conversational Session Boundary and Orphaned Tool Scrubbing

## Context

Persisting intermediate ReAct tool messages in SQLite corrupted multi-turn conversation payloads: OpenAI rejected them with HTTP 400 because of orphaned tool messages.

## Decision

Restrict long-term session persistence to high-level user prompts and final synthesized assistant answers. Keep ReAct tool iterations in ephemeral working memory. Add automatic scrubbing and SQL filtering in `SqliteSessionStore`.

## Alternatives Considered

Not recorded.

## Consequences

- Reloaded history is always a valid alternation of user and assistant messages.
- Tool traces are not kept across sessions.
