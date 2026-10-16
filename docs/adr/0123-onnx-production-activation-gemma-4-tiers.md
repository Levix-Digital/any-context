---
id: 123
title: Activation of ONNX Models in Production Flows and Gemma 4 Family Support with Five Hardware Tiers
status: accepted
date: '2026-10-14'
version: v0.38.0
subsystems: [onnx, model-catalog, lm-providers, ingestion]
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0123: Activation of ONNX Models in Production Flows and Gemma 4 Family Support with Five Hardware Tiers

## Context

Phase 3 of the AnyContext Master Plan calls for activating the neural ONNX models in local production without violating ADR-110 (Zero-Breakage and Pure Rust Fallback Guarantee). With the Gemma 4 family (Compact, 12B, 27B and 32B INT4) selected as the local generative model, the following architectural requirements were identified:

1. Separate the perception layer (Document AI: MobileNetV4, LayoutLMv3, BGE-Small, CLIP-ViT, Laya mmBERT, about 740 MB) from the generative layer (Gemma 4 INT4).
2. Strict hardware isolation: Gemma 4 32B INT4 uses about 25.5 GB RAM with KV cache. On a 32 GB machine, adding the operating system (4-6 GB) causes catastrophic swap thrashing. The 32B model therefore needs the Sovereign Ultra tier (64 GB or more), while 27B INT4 (about 18.5 GB RAM) runs on Workstation Pro (32 GB).
3. Expand the hardware matrix from three to five canonical tiers (`SovereignUltra`, `WorkstationPro`, `AirGappedPower`, `SmartBalanced`, `Lite`).
4. Perception adapters in `any-context-core-rs` implementing universal traits (`ScanSentinel`, `DocumentLayoutExtractor`, `DocumentVisionExtractor`) integrated with the `QualityGate` and `IngestionModelRouter`.
5. A `GemmaOnnxProvider` in `actx-lm` supporting non-streaming and SSE streaming chat, with graceful fallback to the cloud.

## Decision

1. **Five-tier hardware matrix**:
   - `SovereignUltra` (64 GB+ RAM, 19.2 GB disk): Gemma 4 32B INT4 (about 25.5 GB peak RAM) + full Document AI suite (740 MB).
   - `WorkstationPro` (32 GB RAM, 15.7 GB disk): Gemma 4 27B INT4 (about 18.5 GB peak RAM) + full Document AI suite (740 MB).
   - `AirGappedPower` (16 GB RAM, 7.9 GB disk): Gemma 4 12B INT4 (about 10.0 GB peak RAM) + full Document AI suite (740 MB).
   - `SmartBalanced` (8 GB RAM, 1.7 GB disk): Gemma 4 Compact INT4 (about 2.5 GB peak RAM) + essential Document AI (MobileNetV4, LayoutLMv3, BGE-Small, about 252 MB).
   - `Lite` (any hardware, 0 MB disk, under 45 MB RAM): pure Rust deterministic heuristics (under 10 ms) + cloud/API providers.
2. **Perception adapters in `any-context-core-rs`**:
   - `OnnxScanSentinelAdapter` wraps `actx_onnx::OnnxScanSentinel` and implements `ScanSentinel` to classify rasterized pages in under 8 ms in the `QualityGate`.
   - `OnnxLayoutExtractorAdapter` wraps `actx_onnx::OnnxLayoutExtractor` and implements `DocumentLayoutExtractor` for 2D tabular reconstruction in the `IngestionModelRouter`.
   - `OnnxVisionEncoderAdapter` wraps `actx_onnx::OnnxVisionEncoder` and implements `DocumentVisionExtractor` for CLIP ViT embeddings of diagrams and visual elements.
3. **Generative runner in `actx-onnx` and `actx-lm`**:
   - `OnnxGemmaEngine` in `actx-onnx::gemma`: manages Gemma 4 ONNX sessions, BPE formatting with the `<start_of_turn>` and `<end_of_turn>` tokens, a local KV cache and output streaming through a callback.
   - `GemmaOnnxProvider` in `actx-lm::providers::gemma`: implements the `LmProvider` trait (`chat_complete` and `chat_stream`). Without weights, it delegates automatically to the configured fallback provider (Claude, Gemini, OpenAI, Groq, Ollama, Mock).
4. **Multi-surface parity**:
   - `/profile` (alias `/preset`) switches between the five tiers with real-time updates in SQLite and the TUI.
   - `/model` and the TUI menu list the four Gemma 4 models with dynamic local readiness badges (`[Local Ready]`, `[Active / ONNX Ready]`, `[Not Installed]`, `[Fallback]`).
   - `actx-installer` shows the pre-flight check with friendly MB and GB figures and selects the best profile transparently.

## Alternatives Considered

Not recorded.

## Consequences

- No risk of panics or failures when ONNX weights are missing or corrupted (ADR-110 preserved).
- Safe memory isolation for corporate and development machines, without swap death.
