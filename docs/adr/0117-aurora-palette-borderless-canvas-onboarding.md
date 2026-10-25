---
id: 117
title: Aurora Borealis Color Harmonization, Borderless Canvas, ReAct Auto-Scroll, Friendly Update Messages, Graceful Download Cancellation and Neutral Onboarding Wizard
status: accepted
date: '2026-10-10'
version: v0.34.9
subsystems: [tui, installer, model-catalog]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0117: Aurora Borealis Color Harmonization, Borderless Canvas, ReAct Auto-Scroll, Friendly Update Messages, Graceful Download Cancellation and Neutral Onboarding Wizard

## Context

1. The TUI used yellow and red for routine, informative elements (accordion border, active workspace, menu shortcuts), cluttering the Aurora Borealis aesthetic and creating a false sense of urgency.
2. The conversation box had borders and a title that limited the modern full-screen experience (in the style of terminals such as Warp).
3. The ReAct reasoning box (the `<think>` accordion) stayed static once it reached its height limit, without auto-scrolling the reasoning steps.
4. Running `actx --update` right after a new tag showed a raw `404 status code` error, because GitHub Actions was still building the Windows/Linux/macOS binaries.
5. ONNX model downloads started without continuous numeric progress (% and MB) and could not be cancelled gracefully.
6. The ONNX models menu left users unsure whether downloads were needed and what each model did.
7. The first-run flow lacked a guided, neutral onboarding wizard (without a biasing "[Recommended]" marker).

## Decision

1. **Aurora Borealis palette harmonization**:
   - Replace yellow (`theme.warning`) and red (`theme.error`) with Glacial Cyan (`theme.accent`), Cosmic Violet (`theme.reasoning`) and Brand Emerald (`theme.primary`) in all informative elements.
   - Reserve yellow and red strictly for real errors (`AppStatus::Error`, tool errors, disconnected folders).
   - Explicit exception: the `[Esc/Ctrl+C] Exit` shortcut stays Polar Crimson (`theme.error`).
2. **Borderless chat canvas**: use `Block::default().borders(Borders::NONE)` for the conversation area to maximize usable space, and show a subtle "scroll paused, press End for real time" indicator in the footer only when the user has detached auto-scroll.
3. **ReAct and reasoning auto-scroll**: add `reasoning_scroll_offset`, `reasoning_auto_scroll` and `reasoning_max_scroll` to `App`; compute the rendered accordion height and keep it scrolled to the end while thoughts are generated; support `Ctrl+PgUp`/`Ctrl+PgDn` for manual inspection.
4. **Friendly handling of HTTP 404 in `actx-installer`**: detect HTTP 404 in `downloader.rs` and return the sentinel message `ASSET_NOT_YET_AVAILABLE`; show an informative message explaining that CI/CD is still building the release packages, recommending to wait a few minutes and linking the official release.
5. **Model download telemetry and cancellation**: implement `download_model_with_progress` with a progress channel and a cancellation signal through `tokio::sync::watch`; show a live footer line with the model, downloaded and total MB, percentage and an "Esc to cancel" hint; cancel gracefully with `Esc` or from the menu, aborting the stream and deleting partial `.download.tmp` files without disk corruption.
6. **Document AI menu organized by function**: five clear submenus (document ingestion, scan sentinel, question routing, document vision, storage summary), explaining that no model is required for AnyContext to work fully.
7. **Neutral interactive onboarding wizard**: a three-step modal wizard opened with `/onboarding` or `/setup`: step 1, LLM provider and API key; step 2, Document AI profile (LLM vision, native extraction, local models), fully neutral with no `[Recommended]` markers; step 3, an optional initial documents folder.

## Alternatives Considered

Not recorded.

## Consequences

- Cohesive, modern and elegant visual experience in the TUI.
- Full and transparent user control over downloads, updates and onboarding settings.
- No friction and no intimidating error messages.
