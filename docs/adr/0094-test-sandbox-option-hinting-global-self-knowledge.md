---
id: 94
title: Test Isolation Sandbox, Universal Contextual Option Hinting and Skill-Based Self-Knowledge via Global LanceDB Workspace
status: accepted
date: '2026-10-06'
version: v0.32.12
subsystems: [storage, tui, agent]
supersedes: []
superseded_by: []
amends: []
amended_by: [147]
---

# ADR-0094: Test Isolation Sandbox, Universal Contextual Option Hinting and Skill-Based Self-Knowledge via Global LanceDB Workspace

## Context

Automated test runs polluted production databases with test workspaces; users could not discover command options or how to delete a workspace; and the agent could not answer questions about its own operation without bloating the system prompt.

## Decision

- Redirect test storage to an ephemeral temporary sandbox when `ACTX_TEST_MODE` is set.
- Add reactive border titles and inline ghost text to every command that has options.
- Provide interactive menu hints for `/switch --delete`.
- Bootstrap the official documentation into a virtual `"Global"` LanceDB/BM25 workspace, paired with a lean (under 50 tokens) system prompt skill directive.

## Alternatives Considered

Not recorded.

## Consequences

- Tests never touch the user's real data.
- Command options are discoverable while typing.
- The agent answers questions about itself by retrieving from the `Global` workspace instead of carrying the manual in its prompt.
