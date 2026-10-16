---
id: 90
title: Grounding Parity, Real-Time Token Streaming, and Single Canonical LanceDB Path
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0090: Grounding Parity, Real-Time Token Streaming, and Single Canonical LanceDB Path

- **Status**: Accepted & Implemented (`v0.32.7`).
- **Context**: Discrepancies between Python and Rust system prompts allowed models to answer from weights in STRICT mode. Multiple LanceDB paths led to dataset fragmentation. Token output was delayed until turn completion.
- **Decision**: Embed `config/AGENT.md` into the native binary, strictly enforce autonomous `search_db` invocation, consolidate LanceDB into a single canonical directory, persist grounding modes per-workspace in SQLite, and implement real-time SSE token streaming via `chat_stream()`.

---
