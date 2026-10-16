---
id: 88
title: High-Level Conversational Session Boundary & Orphaned Tool Scrubbing
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0088: High-Level Conversational Session Boundary & Orphaned Tool Scrubbing

- **Status**: Accepted & Implemented (`v0.32.6`).
- **Context**: Persisting intermediate ReAct tool messages in SQLite corrupted multi-turn conversation payloads, causing OpenAI HTTP 400 rejection due to orphaned tool messages.
- **Decision**: Restrict long-term session persistence to high-level user prompts and final synthesized assistant responses. Keep ReAct tool iterations in ephemeral working memory. Add automatic scrubbing and SQL filtering in `SqliteSessionStore`.
