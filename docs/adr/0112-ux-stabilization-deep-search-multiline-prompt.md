---
id: 112
title: UX Stabilization, Deep Search Language Parity, Multi-Line Prompt and Transparent Self-Knowledge
status: accepted
date: '2026-10-10'
version: v0.34.4
subsystems: [agent, tui, storage]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0112: UX Stabilization, Deep Search Language Parity, Multi-Line Prompt and Transparent Self-Knowledge

## Context

With Deep Search and several modes in place, AnyContext needed refinement in human interaction details (keyboard accessibility, answer language fidelity, readable telemetry) and a guarantee that the self-help manuals stay indexed across versions.

## Decision

1. **Deep Search conclusion sanitization**: strengthen the `SUMMARY_PROMPT` instructions in `actx-agent/src/deep_search.rs` and add `sanitize_direct_conclusion` to strip reflective meta-narrative.
2. **Persisted `search_mode`**: add a `search_mode` column to the `workspaces` table with a transparent automatic migration, supported by `/search` and `/switch`.
3. **125 ms TUI event loop and 8-dot spinner**: reduce the loop latency to 125 ms for background sync feedback, using the glyphs `⣾⣽⣻⢿⡿⣟⣯⣷`.
4. **Safe update instruction**: replace the in-process update call with an instruction to run `actx --update` in the OS terminal, avoiding I/O collisions in raw mode.
5. **Flexible multi-line input**: support multiple lines (`\n`) in the input buffer, capture `Shift+Enter` / `Alt+Enter` / `Ctrl+J`, compute the height dynamically (`clamp(3, 8)`) and track a 2D `(x, y)` cursor.
6. **Per-workspace visual identity**: prefix answers with `[AI - <Workspace>]` in the soft accent color `Color::LightCyan` and render the user's text in `Color::Gray`.
7. **Version-aware self-knowledge**: store `global_knowledge_version` in `app_settings` in SQLite and trigger a transparent background sync of the `Global` workspace when the version changes.
8. **Document AI in the menu**: add explanatory and action items for local models and spatial vision in `actx-cli/src/tui/menu.rs`.

## Alternatives Considered

Not recorded.

## Consequences

- Deep Search no longer answers in English to questions asked in Portuguese.
- Users can see which workspace the AI is acting on and what the engine is doing behind the scenes.
- Self-knowledge about commands and behavior is not lost after updates.
