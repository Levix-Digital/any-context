---
id: 121
title: Modular ONNX Crate (`crates/actx-onnx`), Hardware Pre-Flight Check & Semaphore-Tiered Installation Profiles
status: accepted
date: '2026-10-13'
version: v0.37.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0121: Modular ONNX Crate (`crates/actx-onnx`), Hardware Pre-Flight Check & Semaphore-Tiered Installation Profiles

- **Status**: Implemented & Verified
- **Date**: 2026-10-13
- **Context**:
  1. AnyContext previously cataloged canonical ONNX models (MobileNetV4, CLIP-ViT, LayoutLMv3, BGE-Small, Laya) and managed downloads, but inference remained gated behind pure Rust deterministic heuristics without tensor runtime bindings.
  2. Installing or selecting models required high cognitive overhead from standard users, while demanding fine-grained governance from power users and security-conscious enterprise teams.
  3. Coupling the ONNX runtime directly into `any-context-core-rs` risked increasing compile times, binary size, and deployment friction on resource-constrained platforms.
- **Decision**:
  1. **Modular Inference Crate (`crates/actx-onnx`)**:
     - Created independent crate `crates/actx-onnx` implementing `OnnxQueryClassifier`, `OnnxScanSentinel`, `OnnxLayoutExtractor`, and `OnnxVisionEncoder`.
     - Coupled to Core strictly via Rust traits and Inversion of Control with Cargo feature flag `onnx = ["dep:actx-onnx"]`.
     - Preserves ADR-110 Zero-Breakage Guarantee: missing or corrupted ONNX weights immediately fall back to pure Rust deterministic heuristics without crash.
  2. **Sub-5ms Hardware Pre-Flight Check**:
     - `HardwareProfiler` probes physical RAM (total/available) and free storage via native OS APIs (Windows `GlobalMemoryStatusEx` / `GetDiskFreeSpaceExW`, Linux `/proc/meminfo`, macOS `sysctl`).
     - Maps capabilities to Semaphore states (🟢 Green / 🟡 Yellow / 🔴 Red) across 3 canonical profiles:
       - **🌌 Air-Gapped Power** ($\ge 16\text{ GB}$ RAM): 5 models (~740 MB disk).
       - **⚡ Smart Balanced** ($8\text{ GB}$ RAM): 3 models (~252 MB disk).
       - **🟢 Lite / Fast-Path** ($< 8\text{ GB}$ RAM, CI/VMs): 0 MB downloads, 100% pure Rust heuristics (<1µs cold start).
     - Auto-selects the most powerful Green profile silently while enabling power users to customize selections.
  3. **Multi-Surface Commands & TUI Menu**:
     - Added `/profile` (alias `/preset`) command to switch tiers, inspect semaphores, and trigger model downloads.
     - Enriched `/status` command and added interactive TUI submenu (`[F1] -> Hardware Profiles`).
     - Added `--profile=<tier>` and `--preflight-only` flags to `actx-installer`.
- **Consequences**:
  - Zero cognitive friction for general users with instant optimal hardware profile selection.
  - Complete architectural decoupling between Core business logic and neural tensor inference runtimes.
  - Full air-gapped autonomy for high-spec workstations and ultra-lightweight efficiency for CI/Docker environments.
