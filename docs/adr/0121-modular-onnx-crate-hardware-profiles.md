---
id: 121
title: Modular ONNX Crate (actx-onnx), Hardware Pre-Flight Check and Semaphore-Tiered Installation Profiles
status: accepted
date: '2026-10-13'
version: v0.37.0
subsystems: [onnx, model-catalog, installer]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0121: Modular ONNX Crate (actx-onnx), Hardware Pre-Flight Check and Semaphore-Tiered Installation Profiles

## Context

1. AnyContext cataloged canonical ONNX models (MobileNetV4, CLIP-ViT, LayoutLMv3, BGE-Small, Laya) and managed their downloads, but inference was still pure Rust deterministic heuristics without tensor runtime bindings.
2. Installing or selecting models demanded too much from standard users, while power users and security-conscious enterprise teams wanted fine-grained control.
3. Coupling the ONNX runtime directly into `any-context-core-rs` risked longer compile times, a larger binary and deployment friction on constrained platforms.

## Decision

1. **Modular inference crate (`crates/actx-onnx`)**:
   - An independent crate implementing `OnnxQueryClassifier`, `OnnxScanSentinel`, `OnnxLayoutExtractor` and `OnnxVisionEncoder`.
   - Coupled to the core only through Rust traits and inversion of control, behind the Cargo feature `onnx = ["dep:actx-onnx"]`.
   - Keeps the ADR-110 Zero-Breakage Guarantee: missing or corrupted ONNX weights fall back immediately to the pure Rust deterministic heuristics without crashing.
2. **Sub-5 ms hardware pre-flight check**:
   - `HardwareProfiler` probes physical RAM (total and available) and free storage through native OS APIs (Windows `GlobalMemoryStatusEx` / `GetDiskFreeSpaceExW`, Linux `/proc/meminfo`, macOS `sysctl`).
   - Capabilities map to semaphore states (green, yellow, red) across three canonical profiles: **Air-Gapped Power** (16 GB RAM or more): five models (about 740 MB on disk); **Smart Balanced** (8 GB RAM): three models (about 252 MB); **Lite / Fast-Path** (under 8 GB RAM, CI, VMs): no downloads, pure Rust heuristics (under 1 µs cold start).
   - The most powerful green profile is selected silently, and power users can customize the selection.
3. **Multi-surface commands and TUI menu**:
   - `/profile` (alias `/preset`) switches tiers, shows the semaphores and triggers model downloads.
   - `/status` is enriched and an interactive TUI submenu is added (`[F1] -> Hardware Profiles`).
   - `actx-installer` gains the `--profile=<tier>` and `--preflight-only` flags.

## Alternatives Considered

Not recorded.

## Consequences

- No friction for general users, who get the best hardware profile automatically.
- Core business logic is fully decoupled from neural tensor inference runtimes.
- Full air-gapped autonomy on high-end workstations and very light operation in CI and Docker.
