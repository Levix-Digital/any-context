---
id: 120
title: Intelligent Source Naming, Decoupled SQLite Alias, Shared Sources Deprecation & Canonical System Knowledge Manual
status: accepted
date: '2026-10-10'
version: v0.36.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0120: Intelligent Source Naming, Decoupled SQLite Alias, Shared Sources Deprecation & Canonical System Knowledge Manual

- **Status**: Implemented & Verified
- **Date**: 2026-10-10
- **Context**:
  1. Monitored folders and web documentation sources previously used raw filesystem paths and long URLs as identifiers, creating noisy and cluttered UI presentations and LLM citations. Modifying a source identifier previously required rewriting metadata across thousands of LanceDB vector chunks ($O(N)$ write amplification).
  2. The `Shared Sources` library introduced unnecessary cognitive overhead, risk of cross-workspace context bleed during Top-K retrieval, and duplicated the functionality of project-specific workspaces.
  3. System self-knowledge was fragmented across `AGENT.md`, `README.md`, and application help text without a cohesive, authoritative technical manual for semantic retrieval.
- **Decision**:
  1. **Relational Decoupled Alias Projection (`workspace_sources`)**:
     - Introduced `workspace_sources (id, workspace, source_type, raw_target, canonical_slug, display_name, created_at, updated_at)` in SQLite (`NativeConfigDb`).
     - Vector chunks in LanceDB retain immutable keys while user-facing presentation resolves to `display_name` via SQLite in $O(1)$ time ($<1\text{ms}$).
     - Intelligent auto-naming heuristics automatically assign descriptive names:
       - Web URLs: GitHub repos (`"GitHub: repo"`), Docs.rs crates (`"Tokio Docs"`), Crates.io packages (`"Crate: rusqlite"`), and domain/path heuristics.
       - Local Folders: Project manifests (`Cargo.toml`, `package.json`, `pyproject.toml`) or leaf directory names (`"finance-engine"` or `"financial-engine/src"`).
     - User renaming via `/sources rename <number_or_name> <new_name>` and interactive TUI menu with 0 LanceDB chunks rewritten.
  2. **Complete Deprecation of Shared Sources**:
     - Deprecated `/shared` command and removed `Linked shared sources search` from `retrieval/pipeline.rs`, enforcing 100% strict workspace isolation with zero context bleed.
  3. **Canonical `SYSTEM_KNOWLEDGE.md` Technical Manual**:
     - Created comprehensive, narrative 7-chapter technical manual in `config/SYSTEM_KNOWLEDGE.md`.
     - Embedded at compile time and automatically vector-indexed into LanceDB's `Global` workspace and BM25 index on application startup/upgrade.
- **Consequences**:
  - Clean, professional source names in terminal listings and LLM citations.
  - Zero LanceDB write amplification when renaming sources.
  - Pure workspace isolation protecting Top-K RAG precision.
  - Authoritative, rich semantic self-knowledge retrieved by the agent on demand.
