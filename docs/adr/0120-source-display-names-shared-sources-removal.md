---
id: 120
title: Intelligent Source Naming, Decoupled SQLite Alias, Shared Sources Deprecation and Canonical System Knowledge Manual
status: accepted
date: '2026-10-10'
version: v0.36.0
subsystems: [storage, retrieval, commands]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0120: Intelligent Source Naming, Decoupled SQLite Alias, Shared Sources Deprecation and Canonical System Knowledge Manual

## Context

1. Monitored folders and web documentation sources used raw filesystem paths and long URLs as identifiers, which cluttered the UI and LLM citations. Changing a source identifier required rewriting metadata across thousands of LanceDB vector chunks (O(N) write amplification).
2. The `Shared Sources` library added cognitive overhead, risked context bleeding across workspaces during Top-K retrieval, and duplicated what project-specific workspaces already do.
3. System self-knowledge was spread across `AGENT.md`, `README.md` and help text, without one authoritative technical manual for semantic retrieval.

## Decision

1. **Relational decoupled alias projection (`workspace_sources`)**:
   - Add `workspace_sources (id, workspace, source_type, raw_target, canonical_slug, display_name, created_at, updated_at)` to SQLite (`NativeConfigDb`).
   - LanceDB vector chunks keep immutable keys; user-facing presentation resolves `display_name` through SQLite in O(1) time (under 1 ms).
   - Auto-naming heuristics assign descriptive names. Web URLs: GitHub repositories (`"GitHub: repo"`), docs.rs crates (`"Tokio Docs"`), crates.io packages (`"Crate: rusqlite"`) and domain/path heuristics. Local folders: project manifests (`Cargo.toml`, `package.json`, `pyproject.toml`) or leaf directory names (`"finance-engine"` or `"financial-engine/src"`).
   - Users rename sources with `/sources rename <number_or_name> <new_name>` or the interactive TUI menu, with zero LanceDB chunks rewritten.
2. **Complete deprecation of Shared Sources**: deprecate the `/shared` command and remove the linked shared sources search from `retrieval/pipeline.rs`, enforcing strict workspace isolation.
3. **Canonical `SYSTEM_KNOWLEDGE.md` technical manual**: a narrative seven-chapter technical manual in `config/SYSTEM_KNOWLEDGE.md`, embedded at compile time and indexed automatically into LanceDB's `Global` workspace and the BM25 index on startup or upgrade.

## Alternatives Considered

Not recorded.

## Consequences

- Clean, professional source names in terminal listings and LLM citations.
- No LanceDB write amplification when renaming sources.
- Pure workspace isolation protects Top-K RAG precision.
- Authoritative self-knowledge that the agent retrieves on demand.
