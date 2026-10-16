---
id: 92
title: Virtual Tab Workspace Chat Buffer Isolation & Viewport Lifecycle
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0092: Virtual Tab Workspace Chat Buffer Isolation & Viewport Lifecycle

- **Status**: Accepted & Implemented (`v0.32.10`).
- **Context**: Navigating across workspaces in the Ratatui TUI leaked in-memory chat buffers and viewport streaming states across workspace boundaries, violating conversational privacy and context isolation.
- **Decision**: Implement a Virtual Tab architecture with `workspace_chat_buffers: HashMap<String, Vec<ChatMessageItem>>` and lifecycle hooks in `switch_to_workspace`. Bind `/reset-memory` to `CommandAction::ClearChat` to enforce immediate frontend/backend synchronization across all workspace transitions and resets.

---
