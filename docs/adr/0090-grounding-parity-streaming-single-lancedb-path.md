---
id: 90
title: Grounding Parity, Real-Time Token Streaming and Single Canonical LanceDB Path
status: accepted
date: '2026-10-02'
version: v0.32.7
subsystems: [agent, lm-providers, storage]
supersedes: []
superseded_by: []
amends: []
amended_by: [144]
---

# ADR-0090: Grounding Parity, Real-Time Token Streaming and Single Canonical LanceDB Path

## Context

Differences between the Python and Rust system prompts let models answer from their weights in STRICT mode. Several LanceDB paths fragmented the dataset. Token output was delayed until the turn completed.

## Decision

Embed `config/AGENT.md` into the native binary, strictly enforce autonomous `search_db` invocation, consolidate LanceDB into a single canonical directory, persist grounding modes per workspace in SQLite, and stream tokens in real time over SSE via `chat_stream()`.

## Alternatives Considered

Not recorded.

## Consequences

- One system prompt and one vector store for every surface.
- Answers appear token by token.
