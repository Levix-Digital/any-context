---
id: 119
title: Global English Internationalization, Point-by-Point Braille Spinner, Default Collapsed Reasoning Accordion and Standardized Release Notes
status: accepted
date: '2026-10-10'
version: v0.35.0
subsystems: [tui, cli, ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0119: Global English Internationalization, Point-by-Point Braille Spinner, Default Collapsed Reasoning Accordion and Standardized Release Notes

## Context

- AnyContext is a global developer tool, so its command-line interface, menus, onboarding, system prompts, error guidance and telemetry must be presented uniformly in English.
- The Braille spinner skipped positions because of a `* 2` multiplier in the tick calculation, which made it stutter.
- The ReAct and reasoning accordion took a lot of vertical space because it was open by default (`accordion_open: true`).
- GitHub releases did not follow the standard GitHub categorization format (`## What's Changed`, `* <commit> by @<author> in #<pr>`).
- In headless CI tests, `run_startup_splash_bootstrap` persisted `global_knowledge_version` only when LanceDB opened successfully, which made tests non-deterministic.

## Decision

1. **English internationalization**: translate all menus, badges, onboarding screens, prompts, tool definitions and CLI guidance messages into clear, idiomatic English across all crates. Agent answers keep matching the language of the user's prompt (English for English questions, Portuguese for Portuguese questions).
2. **Sequential point-by-point Braille orbit**: remove the `* 2` factor from the tick calculation in `ui.rs` and use the canonical 10-point orbit `["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]`, indexed by `app.tick_count % len`, for smooth rotation without skipped dots.
3. **ReAct accordion collapsed by default**: initialize `accordion_open: false` in `app.rs`, keeping expansion on demand with `Ctrl+T`.
4. **Standardized GitHub release notes**: add `generate_release_notes: true` to the Linux and Windows release jobs in `.github/workflows/release.yml`, and create `.github/release.yml` with semantic categories (Features & Enhancements, Bug Fixes, Documentation, Maintenance & Performance).
5. **Deterministic startup splash bootstrap in CI**: `run_startup_splash_bootstrap()` always writes `global_knowledge_version` to the SQLite settings database, so `is_first_run_or_upgrade()` deterministically returns `false` on later runs in headless environments.

## Alternatives Considered

Not recorded.

## Consequences

- A global, professional interface.
- Smooth, continuous loading animations in every terminal.
- A clean, focused conversation viewport on startup.
- Release notes generated automatically following GitHub conventions.
