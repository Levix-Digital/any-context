---
id: 80
title: Workspace-Isolated Prompt History Navigation
status: accepted
date: '2026-10-02'
version: v0.32.3
subsystems: [tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0080: Workspace-Isolated Prompt History Navigation

## Context

After the migration to the native Rust CLI (`actx-cli`), the `Up` and `Down` arrow keys were intercepted by the chat view auto-scroll logic, which displaced readline-style prompt history navigation. Users had to retype complex prompts and commands.

## Decision

Restrict `Up`/`Down` key events to prompt history navigation; delegate chat viewport scrolling to `PageUp`, `PageDown`, `Home`, `End`, the mouse wheel and `Shift+Up`/`Shift+Down`. Store the history in a `HashMap<String, Vec<String>>` indexed by workspace.

## Alternatives Considered

Not recorded.

## Consequences

- Classic terminal prompt recall is restored while the chat viewport can still be inspected.
- Switching workspaces scopes prompt recall to the selected workspace.
