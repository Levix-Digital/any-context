---
id: 79
title: Multi-Byte UTF-8 Character Boundary Safety, Visual Cursor Alignment and Universal Release Protocol
status: accepted
date: '2026-09-26'
version: v0.32.2
subsystems: [tui]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0079: Multi-Byte UTF-8 Character Boundary Safety, Visual Cursor Alignment and Universal Release Protocol

## Context

Users typing questions with international characters, accents or emojis hit fatal panics in the prompt input. In addition, once the conversation was taller than the terminal, the streaming answer was pushed off screen because the viewport did not scroll automatically.

## Decision

1. Change every string index mutation in `crates/actx-cli` to use `c.len_utf8()` and `.char_indices()`, separating byte offsets from visual terminal cell coordinates.
2. Implement an automatic trailing viewport based on `Paragraph::line_count(inner_width)`, which pauses when the user scrolls and snaps back to the bottom.
3. Register the project's release repository in `.dev-cycle.json`.

## Alternatives Considered

Not recorded.

## Consequences

- No more UTF-8 character boundary panics.
- The chat view scrolls down smoothly as new messages and tokens arrive.
- Smooth keyboard (`PageUp`/`PageDown`/`Home`/`End`) and mouse wheel scrolling.
