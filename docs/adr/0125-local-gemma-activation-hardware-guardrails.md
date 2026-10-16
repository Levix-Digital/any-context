---
id: 125
title: Operational Activation of Local Gemma 4 ONNX in the CLI Engine, LM Studio-Style Hardware Guardrails and Safe Workspace Presets
status: accepted
date: '2026-10-14'
version: v0.40.0
subsystems: [cli, model-catalog, commands, tui]
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0125: Operational Activation of Local Gemma 4 ONNX in the CLI Engine, LM Studio-Style Hardware Guardrails and Safe Workspace Presets

## Context

1. In `v0.38.0` the local Gemma 4 ONNX infrastructure (`OnnxGemmaEngine` and `GemmaOnnxProvider`) entered the monorepo, but the CLI inference dispatcher (`crates/actx-cli/src/engine.rs::resolve_lm_provider`) did not yet route requests to load the local weights of the requested tier.
2. With local ONNX models (Gemma 4 INT4), loading weights larger than the host's physical RAM (for example Gemma 4 32B on a 16 GB machine) causes aggressive swap thrashing, freezing the interface and killing the process with an out-of-memory error.
3. Switching the inference model arbitrarily per turn breaks predictability and overrides the deliberate choice made by the operator.

## Decision

1. **Gemma 4 wired into the CLI (`crates/actx-cli/src/engine.rs`)**:
   - `resolve_lm_provider` recognizes `gemma-4*` and `gemma*`.
   - The `.onnx` weights file is resolved automatically in the canonical directory `%LOCALAPPDATA%/actx/models/`.
   - ADR-110 Zero-Breakage Guarantee: if the user selects a local model whose weights have not been downloaded by `actx-installer`, the provider falls back gracefully to the offline `MockLmProvider`, with no data sent to the cloud, $0.00 cost and no interruption or panic.
2. **LM Studio-style active hardware guardrails (`crates/any-context-core-rs/src/hardware/mod.rs`)**:
   - Cloud models (OpenAI, Claude, Gemini, DeepSeek, Groq) can always be switched immediately.
   - Local models have their RAM requirements checked at runtime (under 5 ms through OS APIs): **Safe** (approved with physical memory headroom); **Tight** (explanatory memory pressure warning, execution allowed); **Unsafe** (preventive block against out-of-memory: the selection is rejected with a detailed diagnosis of required versus total/available RAM and a recommendation of the best safe model; execution is allowed only with the explicit `--force` flag, as in `/model gemma-4-32b --force`).
3. **Semaphore badges in the TUI (`crates/actx-cli/src/tui/menu.rs`)**: the interactive model menu (`[F1]` or `/model`) shows dynamic semaphore badges and icons: compatible, swap risk, and insufficient hardware with the required GB.
4. **Safe workspace presets (`/preset` or `/kit`)**:
   - `offline`: atomically reconfigures the workspace to the largest local model that is safe for the machine (Compact for 8 GB, 12B for 16 GB, 27B for 32 GB, 32B for 64 GB or more), strict mode, fast search and web search off ($0.00 cost and network isolation).
   - `balanced`: balanced cloud model (`gpt-4o-mini`), hybrid mode, auto search and web search on.
   - `frontier`: analytical frontier model (`claude-3-7-sonnet`), proactive mode, deep search and web search on.
   - `privacy`: safe local model with corporate network isolation.
5. **User sovereignty**: the model chosen by the user is kept on every turn. The `SearchMode::Auto` triage only decides the RAG search depth (Fast RAG or Deep Search).

## Alternatives Considered

Not recorded.

## Consequences

- Protection against machine freezes and out-of-memory errors caused by selecting a model too heavy for the hardware.
- Local Gemma 4 neural inference activated in the headless and interactive CLI.
- Better ergonomics through `/preset` shortcuts.
