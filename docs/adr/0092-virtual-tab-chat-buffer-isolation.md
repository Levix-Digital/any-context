---
id: 92
title: Virtual Tab Workspace Chat Buffer Isolation and Viewport Lifecycle
status: accepted
date: '2026-10-04'
version: v0.32.10
subsystems: [tui, commands]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0092: Virtual Tab Workspace Chat Buffer Isolation and Viewport Lifecycle

## Context

Switching workspaces in the Ratatui TUI leaked in-memory chat buffers and viewport streaming state across workspace boundaries, which broke conversational privacy and context isolation.

## Decision

Implement a Virtual Tab architecture with `workspace_chat_buffers: HashMap<String, Vec<ChatMessageItem>>` and lifecycle hooks in `switch_to_workspace`. Bind `/reset-memory` to `CommandAction::ClearChat` so the frontend and backend stay synchronized across every workspace transition and reset.

## Alternatives Considered

Not recorded.

## Consequences

- Each workspace keeps its own visible conversation while the application runs.
- Resetting memory also clears the visible chat.
