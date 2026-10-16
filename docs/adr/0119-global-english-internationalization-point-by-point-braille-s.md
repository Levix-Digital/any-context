---
id: 119
title: Global English Internationalization, Point-by-Point Braille Spinner, Default Collapsed Reasoning Accordion, and Standardized Release Notes
status: accepted
date: '2026-10-10'
version: v0.35.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0119: Global English Internationalization, Point-by-Point Braille Spinner, Default Collapsed Reasoning Accordion, and Standardized Release Notes

- **Context**:
  - AnyContext is a global developer tool and its command-line interface, menus, onboarding, system prompts, error guidance, and telemetry must be uniformly presented in English.
  - The Braille spinner was skipping positions due to a `* 2` multiplier in tick calculation, causing a stuttering visual effect.
  - The ReAct & Reasoning accordion occupied significant vertical real estate by defaulting to open (`accordion_open: true`).
  - GitHub releases did not follow the standard GitHub conventional commit categorisation format (`## What's Changed`, `* <commit> by @<author> in #<pr>`).
  - In headless CI testing, `run_startup_splash_bootstrap` conditionally persisted `global_knowledge_version` only when LanceDB was successfully opened, leading to test non-determinism.
- **Decisions**:
  1. **Comprehensive English Internationalization (i18n)**:
     - Translated all menus, badges, onboarding screens, prompts, tool definitions, and CLI guidance messages to clear, idiomatic English across all crates.
     - Preserved multilingual language consistency in agent responses: the LLM automatically matches the user's prompt language (responding in English if queried in English, in Portuguese if queried in Portuguese).
  2. **Sequential Point-by-Point Braille Orbit**:
     - Removed the `* 2` factor from tick calculation in `ui.rs`, standardizing on canonical 10-point perimeter orbits (`["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]`) indexed strictly by `app.tick_count % len` for smooth point-by-point rotation without stutter or skipped dots.
  3. **Default Collapsed ReAct Accordion**:
     - Initialized `accordion_open: false` in `app.rs`, giving maximum viewport focus to conversation messages by default while retaining on-demand expansion via `Ctrl+T`.
  4. **Standardized GitHub Release Notes Generation**:
     - Added `generate_release_notes: true` to both Linux and Windows release jobs in `.github/workflows/release.yml`.
     - Created `.github/release.yml` with semantic categories (Features & Enhancements, Bug Fixes, Documentation, Maintenance & Performance).
  5. **Deterministic Startup Splash Bootstrap in CI**:
     - Updated `run_startup_splash_bootstrap()` to unconditionally write `global_knowledge_version` to the SQLite settings database, ensuring `is_first_run_or_upgrade()` deterministically returns `false` on subsequent runs in headless environments.
- **Consequences**:
  - Truly global, professional interface matching enterprise standards.
  - Fluid, continuous loading animations across all terminal environments.
  - Clean and focused conversation viewport on startup.
  - Full automated release notes generation adhering to GitHub conventions.
