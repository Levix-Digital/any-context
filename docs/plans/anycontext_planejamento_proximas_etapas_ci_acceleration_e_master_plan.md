---
plan_id: anycontext_planejamento_proximas_etapas_ci_acceleration_e_master_plan
project: any-context
title: 'Planning the Next Steps: Radical Release CI/CD Acceleration and Master Plan Continuity'
version: v0.34.1
status: active
author: Levi Silveira
created_at: '2026-10-09T19:57:27'
updated_at: '2026-10-09T19:57:27'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- ci-cd
- sccache
- cargo-xwin
- release-acceleration
- modelrouter
- presets
- master-plan
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: sccache and cargo-xwin PoC for ultra-fast CI'
  done: false
- name: 'M2: New release pipeline under 8 min'
  done: false
- name: 'M3: ModelRouter in Ingestion and Quality Gate'
  done: false
- name: 'M4: Native Jina Reader Connector'
  done: false
- name: 'M5: Phase 3 Preset Matrix'
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 🗺️ Planning the Next Steps: CI/CD Acceleration & Master Plan Continuity

## 📌 Context & Motivation
At the end of **Phase 2 (RFC-042: Reflective Agentic Deep Search)** with release `v0.34.0`, the GitHub Actions CI/CD pipeline took **~50 minutes** on the Windows runner (`windows-latest`) to compile monolithic dependencies (LanceDB, Apache Arrow, DataFusion, tantivy).
This bottleneck eats precious engineering time and slows release iteration.

As requested by the user, this critical **CI/CD Pipeline Acceleration** topic is formally added to the immediate roadmap, before or together with the next Master Plan product deliveries.

---

## 🎯 Strategic Agenda for Discussion and Next Steps

### 🚀 Topic I: Radical Release CI/CD Acceleration (DevOps / Infra)
**Target**: cut the binary build and publication time from ~50 minutes to under 8 minutes.

#### Alternatives Under Evaluation:
1. **Option 1: `sccache` (Mozilla Shared Compilation Cache) integrated into GitHub Actions**
   - Use `sccache` with a GitHub Actions Cache backend (`actions/cache` or S3/R2-compatible storage).
   - Downloads the compiled object files (`.obj`/`.o`) of the ~300 dependencies in seconds, without recompiling.
   - Keeps native runners (`ubuntu-latest` and `windows-latest`).
2. **Option 2: Windows Cross-Compilation from Linux (`cargo-xwin`)**
   - Remove compilation on the `windows-latest` runner.
   - Build the Windows binary (`x86_64-pc-windows-msvc`) directly inside the `ubuntu-latest` runner using `cargo-xwin` and Microsoft's MSVC SDKs.
   - Linux compiles much faster thanks to its I/O efficiency compared to Windows.
3. **Option 3: Release Flag Optimization in `Cargo.toml`**
   - Set `codegen-units = 16`, `split-debuginfo`, `debug = false`, `strip = true` in the release profile to parallelize LLVM code generation.

---

### 🧠 Topic II: Rest of Master Plan Phase 2 (Research & Ingestion)
1. **Milestone 2 (Phase 2): ModelRouter in Ingestion (Quality Gate & Vision/OCR)**
   - Evaluation and semantic enrichment of documents at ingestion.
   - Quality gate to detect garbage/noise before indexing in LanceDB/BM25.
   - Support for local/remote multimodal vision models for images in PDFs/Markdown.
2. **Jina Reader Web Connector (`r.jina.ai`)**
   - Clean Markdown extraction of external web pages directly by the agent.

---

### 🔮 Topic III: Master Plan Phase 3 (Definitive Preset Matrix / Model Kit)
1. **100% Offline / Edge Preset (SLM + Local ONNX)**
   - Full operation without an internet connection, using local models (e.g. Qwen 2.5 Coder 7B, Llama 3.2 3B) through Ollama/LM Studio and local fastembed/ONNX embeddings.
2. **Hybrid Preset (Balanced Cost/Benefit)**
   - Local embeddings and fast triage, reasoning and synthesis with fast cloud providers (Gemini Flash, Claude Haiku, GPT-4o-mini).
3. **Frontier Preset (High Cognitive Capacity)**
   - Deep reasoning with top-tier models (Claude 3.5 Sonnet, GPT-4o, Gemini 1.5 Pro).

---

## 📋 Proposed Action Milestones

- **M1: Diagnosis and Proof of Concept with `sccache` / `cargo-xwin`**:
  - Test `sccache` and `cargo-xwin` on a dedicated branch and measure the time reduction on GitHub Actions.
- **M2: Update of the `.github/workflows/release.yml` Workflow**:
  - Implement the winning solution, cutting CI time to < 10 minutes.
- **M3: ModelRouter in Ingestion & Quality Gate**:
  - Native Rust implementation in `any-context-core-rs` and `actx-agent`.
- **M4: Native Jina Reader Connector**:
  - Integration into `actx-agent` as a native deep web fetch tool.
- **M5: Preset Matrix (Phase 3)**:
  - Implementation of the Offline, Hybrid and Frontier presets.
