---
id: 80
title: Workspace-Isolated Prompt History Navigation
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0080: Workspace-Isolated Prompt History Navigation

- **Status**: Accepted & Implemented (`v0.32.3`).
- **Decision**: Restrict `Up`/`Down` key events to prompt history navigation; delegate chat viewport scrolling to `PageUp`, `PageDown`, `Home`, `End`, Mouse Scroll, and `Shift+Up`/`Shift+Down`. Store history in `HashMap<String, Vec<String>>` indexed by workspace.
