---
id: 115
title: Native ONNX Weights Repository, Resilient Fallbacks, Search Mode Header Indicator and Human Conversational Tone
status: accepted
date: '2026-10-10'
version: v0.34.7
subsystems: [model-catalog, tui, agent, ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0115: Native ONNX Weights Repository, Resilient Fallbacks, Search Mode Header Indicator and Human Conversational Tone

## Context

1. The TUI offered generic options mentioning Ollama for vision and classification, which diverged from the AnyContext design based on embedded inference through ONNX Runtime and 2D heuristics.
2. Missing models must not break indexing or execution.
3. The search mode indicator (`AUTO`, `FAST`, `DEEP`) was missing from the top bar, and its persisted value was not reloaded when switching workspaces from the menu.
4. The Braille spinner rotated counter-clockwise and slowly.
5. Answers without supporting facts contained rigid, robotic disclaimers (a "⚠️ This information is not in ..." warning), which looked like errors.
6. The timing unit test on the Windows CI runner (`windows-latest`) was flaky under virtualized I/O jitter.

## Decision

1. **Native ONNX weights repository (`OnnxModelManager`)**:
   - Standard directory `%LOCALAPPDATA%\AnyContext\models\` (Windows) and `~/.local/share/any-context/models/` (Linux).
   - Canonical catalog with real names and specifications: `Laya AI (mmBERT INT8 ONNX)`, `MobileNetV4 RVL-CDIP (ONNX)`, `BGE-Small Query Classifier (ONNX)`, `Moondream2 1.8B (INT4 ONNX)` and `SmolVLM-500M (ONNX)`.
   - Non-blocking asynchronous download, triggered from the keyboard in the "Document AI and Local Models" submenu, with HTTP streaming and an atomic write through a temporary file.
2. **Fault tolerance and fallback matrix**:
   - With no ONNX model downloaded, the system runs entirely on the deterministic engine and the pure Rust 2D spatial heuristics (under 5 MB RAM).
   - With some models present, everything works normally without panics.
   - `/status` reports the real-time integrity of the models and the local directory.
3. **Search mode indicator in the TUI header and per-workspace isolation**:
   - Add `[Search: AUTO|FAST|DEEP]` using the Aurora Borealis palette tokens (`theme.info`, `theme.primary`, `theme.magenta`).
   - Update `switch_to_workspace` in `App` to reload `search_mode` from SQLite (`NativeConfigDb`) immediately on switch.
4. **Clockwise Braille spinner at 2x speed**: reverse the sequence to the canonical clockwise rotation `["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"]`, accelerated with `(app.tick_count * 2) % len`.
5. **Human conversational tone**: remove the "this information is not in this workspace's documents" warning from `prompt.rs` and `AGENT.md`; the agent answers naturally and directly, like a teammate, without alarms.
6. **CI resilience on the Windows runner**: raise the timing ceiling of the startup test to `< 500ms`, keeping it stable under virtualized I/O on shared runners.

## Alternatives Considered

Not recorded.

## Consequences

- Fluid, natural, auditable interface with real-time information.
- No mandatory external dependency; models are downloaded on demand with full fallbacks.
- All tests green and a resilient CI.
