---
plan_id: anycontext_blueprint_v0420_matriz_tri_cross_platform_de_aceleracao_por_hardware_directml_metalcoreml_cudavulkan_no_actx_onnx
project: any-context
title: 'Blueprint v0.42.0: Tri-Cross-Platform Hardware Acceleration Matrix (DirectML, Metal/CoreML, CUDA/Vulkan) in actx-onnx'
version: v0.42.0
status: superseded
author: Levi Silveira
created_at: '2026-10-15T09:17:45'
updated_at: '2026-10-15T10:37:30'
session_id: df47900f-597a-4d3f-be8b-6ceaac73a0f4
tags: []
supersedes: anycontext_blueprint_v0410_downloader_assistido_de_pesos_gemma_4_onnx_na_tui_e_cli_actx_download_tier
superseded_by: anycontext_blueprint_v0430_rag_determinístico_backend_first_u_shaped_reordering_e_isolamento_estrutural_em_xml
parent_plan_id: null
milestones:
- name: 'Milestone 1: Tri-Cross-Platform Profiling in the Core'
  done: false
- name: 'Milestone 2: Execution Provider Abstraction in actx-onnx'
  done: false
- name: 'Milestone 3: Wiring in actx-lm and Tokens/s Telemetry'
  done: false
- name: 'Milestone 4: --device CLI Flag, /device Slash Command and TUI Menu'
  done: false
- name: 'Milestone 5: Automated Tests'
  done: false
- name: 'Milestone 6: Dual-Doc TECDOC and Release v0.42.0'
  done: false
status_log:
- at: '2026-10-15T10:37:30'
  note: Superseded by anycontext_blueprint_v0430_rag_determinístico_backend_first_u_shaped_reordering_e_isolamento_estrutural_em_xml
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 AnyContext Blueprint v0.42.0: Tri-Cross-Platform GPU & Hardware Acceleration Matrix (DirectML, Metal/CoreML, CUDA/Vulkan) in `actx-onnx`

> **Status**: Proposed / In Gate Validation (Gate 3)  
> **Target Version**: `v0.42.0`  
> **Date**: 2026-10-15  
> **Author**: Antigravity (Advanced Agentic Pair-Programming)  
> **Product Origin**: Calgary, Alberta, Canada (100% Global, Region-Agnostic)  

---

## 🎯 1. Goal & Architecture Decisions (ADR-127)

### Context
In version `v0.41.0`, AnyContext has the assisted downloader for Gemma 4 family weights and a verified catalog of perception models. However, all neural computation runs mainly on the CPU with SIMD vectorization.
In production scenarios, local language models (Gemma 4 Compact ~1.85GB, 12B ~6.75GB, 27B ~14.8GB and 32B ~20.2GB) need hardware acceleration to reach human conversational fluency (15 to 50+ tokens/second).
Since AnyContext is a strictly global platform, hardware acceleration must cover with equal excellence the 3 dominant operating ecosystems:

1. **Windows**: **DirectML (DirectX 12 Machine Learning)** — a native, hardware-agnostic solution (NVIDIA GeForce/RTX, AMD Radeon, Intel Arc/Iris Xe, Qualcomm Snapdragon X Elite NPUs).
2. **macOS**: **Metal & CoreML** — takes advantage of the Unified Memory Architecture (UMA) of Apple Silicon chips (M1 to M4) for inference with zero PCIe copies and stunning speed (25 to 50 tok/s).
3. **Linux**: **CUDA / TensorRT (NVIDIA)** and **Vulkan Compute** — the undisputed standard on development workstations and cloud/VPC servers.

### Architecture Decision
1. **Core Domain (`crates/any-context-core-rs/src/hardware/mod.rs`)**:
   - Create `HardwareProfiler::probe_accelerators() -> Vec<GpuDevice>` with a defensive runtime check (<5ms):
     - On Windows: DXGI 1.2 enumeration through direct FFI with zero heavy external dependencies.
     - On macOS: detection of Apple Silicon (M1–M4) and Metal GPUs through `sysctl` and IOKit calls.
     - On Linux: inspection of NVIDIA drivers (`/dev/nvidia*`, `libcuda.so`) and DRM render devices (`/sys/class/drm`).
   - Integrate `SystemMetrics` with the recommended accelerator for the active machine.
2. **Neural Engine (`crates/actx-onnx`)**:
   - Introduce the `ExecutionDevice` enum:
     - `Auto`: automatically selects the best accelerator available on the host operating system.
     - `DirectML { adapter_index: usize }`: forces DirectX 12 on Windows.
     - `MetalCoreML`: forces Metal/CoreML on macOS.
     - `Cuda { device_id: usize }`: forces NVIDIA CUDA on Linux or Windows.
     - `Vulkan { device_id: usize }`: forces cross-platform Vulkan Compute.
     - `Cpu`: forces 100% CPU execution with SIMD multi-threading.
   - Inviolable ADR-110 Guarantee (*Zero-Breakage Guarantee*): if the selected accelerator fails to allocate tensors (e.g. not enough VRAM for 27B/32B models), the engine falls back to the CPU immediately, without a crash or interruption.
3. **LLM Provider (`crates/actx-lm/src/providers/gemma.rs`)**:
   - `GemmaOnnxProvider` propagates the active accelerator telemetry (`tok/s`, allocated VRAM) to token streaming and the reasoning accordion.
4. **Surface Parity (Headless CLI, Chat TUI and `[F1]` Menu)**:
   - New CLI flag: `actx --device <auto|dml|metal|cuda|cpu>`.
   - New slash command: `/device` (aliases `/gpu`, `/accelerator`) to list adapters and switch in real time.
   - TUI `[F1]` menu: `Hardware Acceleration & GPUs` submenu.
   - TUI header: shows the active accelerator badge (`[🚀 DirectML: RTX 4070]`, `[🚀 Metal: M3 Pro]`, `[🚀 CUDA: RTX 3090]`, or `[💻 CPU: 8 Cores]`).

---

## 🏛️ 2. Surface Parity Mapping

| Surface | Capability in v0.42.0 |
| :--- | :--- |
| **Headless CLI (`actx`)** | `--device <auto\|dml\|metal\|cuda\|cpu>` flag and the `actx --diag` subcommand with a GPU and VRAM report. |
| **Chat TUI (`actx`)** | Interactive `/device` command, tokens-per-second telemetry in the footer/accordion and an accelerator badge in the header. |
| **TUI `[F1]` Menu** | Dedicated `Hardware Acceleration` submenu to switch between GPU and CPU with immediate feedback. |
| **Hardware Guardrails** | Protection against dedicated VRAM overflow on 4GB/6GB/8GB GPUs, preventing OOM with a fallback to CPU. |

---

## 📁 3. Affected Files and Lines

1. **`crates/actx-onnx/src/lib.rs` & `crates/actx-onnx/src/device.rs`**:
   - Add a `device.rs` module with the `ExecutionDevice` enum and the `GpuDevice` struct.
2. **`crates/any-context-core-rs/src/hardware/mod.rs`**:
   - Implement `HardwareProfiler::probe_accelerators()`.
   - Extend `SystemMetrics` with the fields `accelerators: Vec<GpuDevice>` and `recommended_device: ExecutionDevice`.
3. **`crates/actx-onnx/src/session.rs`, `engine.rs` and `gemma.rs`**:
   - Update the `OnnxEngine` and `OnnxGemmaEngine` constructors to receive `ExecutionDevice`.
   - Configure the pipeline with a transparent fallback to CPU.
4. **`crates/actx-lm/src/providers/gemma.rs`**:
   - Inject tokens/s telemetry and the active device into generation.
5. **`crates/any-context-core-rs/src/commands/engine.rs` & `models.rs`**:
   - Add the `/device` route and the `CommandAction::SetDevice(ExecutionDevice)` action.
6. **`crates/actx-cli/src/cli/args.rs` & `tui/app.rs`**:
   - Support for the `--device` flag and rendering of the accelerator badge in the TUI header.
7. **Version Bump to `v0.42.0`**:
   - `version.txt`, `Cargo.toml` in every crate of the monorepo.

---

## 📚 4. Dual-Doc Update Plan (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - New subsection: **"🚀 Native GPU Acceleration (DirectML, Metal/CoreML and CUDA)"**.
  - Documentation of the `/device` command and the `--device` flag.
- **`TecDoc` (`TECDOC.md`)**:
  - Formal record of **ADR-127: Tri-Cross-Platform Hardware Acceleration Matrix in actx-onnx with Graceful Fallback**.
  - Sequence diagram of the handshake and tensor buffer allocation.

---

## 🧪 5. Automated Test Strategy & NFRs

- Unit tests for accelerator enumeration in `crates/any-context-core-rs/tests/hardware_tests.rs`.
- Graceful fallback test: a simulated GPU allocation failure guaranteeing fully working execution on the CPU without a panic.
- Command parsing and dispatch test in `crates/actx-cli/tests/cli_tests.rs`.
- NFR target: accelerator probe time $< 5\text{ms}$ with no impact on AnyContext's instant boot.

---

## 📝 6. Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)

- **Scenario 42** prepended at the top of `tests/MANUAL_TESTS.md`:
  - Step by step to validate `/device` on Windows (DirectML), macOS (Metal) and Linux (CUDA/CPU).
  - Check of the forced graceful fallback (`actx --device cpu`).
  - Inspection of the accelerator badges in the header and tokens-per-second telemetry.

---

## 📐 7. Schedule of the 6 Execution Milestones

- **Milestone 1**: Tri-Cross-Platform Profiling in the Core (`HardwareProfiler::probe_accelerators`).
- **Milestone 2**: Execution Provider Abstraction in `actx-onnx` (`ExecutionDevice`).
- **Milestone 3**: Wiring in `actx-lm` (`GemmaOnnxProvider`) and Tokens/s Telemetry.
- **Milestone 4**: `--device` CLI Flag, `/device` Slash Command and TUI Menu.
- **Milestone 5**: Automated Test Suite (100% green in the workspace).
- **Milestone 6**: Dual-Doc (`README.md`, `TECDOC.md` ADR-127), Scenario 42 and Release `v0.42.0`.
