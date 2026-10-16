---
id: 82
title: SQLite Session Memory & Canonical Identity Injection
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0082: SQLite Session Memory & Canonical Identity Injection

- **Status**: Accepted & Implemented (`v0.32.3`).
- **Decision**: Connect `SqliteSessionStore` per workspace session (`ws_{name}`). Embed persistent memory directives and environment metadata in the system prompt. Register native `search_db` tool for LanceDB vector search. Upgrade `ToolRegistry` to `std::sync::RwLock` for synchronous agent instantiation.
