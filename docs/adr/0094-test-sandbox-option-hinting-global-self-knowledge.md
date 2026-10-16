---
id: 94
title: Test Isolation Sandbox, Universal Contextual Option Hinting & Skill-Based Self-Knowledge via Global LanceDB Workspace
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0094: Test Isolation Sandbox, Universal Contextual Option Hinting & Skill-Based Self-Knowledge via Global LanceDB Workspace

- **Status**: Accepted & Implemented (`v0.32.12`).
- **Context**: Automated test runs polluted production databases with test workspaces; users lacked discoverability for command options and workspace deletion; and the AI agent could not answer questions about its own operation without risking system prompt token bloat.
- **Decision**: Redirect test storage to an ephemeral temporary sandbox via `ACTX_TEST_MODE`; introduce universal reactive border titles and inline ghost text across all commands with options; provide interactive `/switch --delete` menu hints; and bootstrap official documentation into a virtual `"Global"` LanceDB/BM25 workspace paired with a lean (<50 tokens) system prompt skill directive.

---
