---
plan_id: anycontext_blueprint_v03410_standalone_onnx_models_download_resilience_404_fix
project: any-context
title: 'Blueprint v0.34.10: Standalone ONNX Models, Download Resilience & 404 Fix'
version: v0.34.10
status: superseded
author: Levi Silveira
created_at: '2026-10-10T14:42:45'
updated_at: '2026-10-10T14:59:47'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- onnx
- models
- document_ai
- download
- resilience
- http_404
- fallback
supersedes: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_cancelamento_de_download_onboarding_wizard
superseded_by: anycontext_blueprint_v0350_english_i18n_braille_spinner_point_by_point_release_notes_standard
parent_plan_id: null
milestones:
- name: 'M1: Live Validation of the ONNX URLs'
  done: true
- name: 'M2: Reqwest HTTP Client Configuration'
  done: true
- name: 'M3: TUI Submenu Alignment'
  done: true
- name: 'M4: Automated Tests'
  done: true
- name: 'M5: Dual-Doc ADR-118 and Scenario 33'
  done: true
- name: 'M6: Release and Deploy v0.34.10'
  done: true
status_log:
- at: '2026-10-10T14:43:04'
  note: 'Milestone ''M1: Live Validation of the ONNX URLs'': 5 repositories tested live through HTTP HEAD 200'
- at: '2026-10-10T14:43:05'
  note: 'Milestone ''M2: Reqwest HTTP Client Configuration'': User-Agent and a 10-redirect policy configured'
- at: '2026-10-10T14:43:06'
  note: 'Milestone ''M3: TUI Submenu Alignment'': ingestion with layoutlmv3 and vision with clip-vit'
- at: '2026-10-10T14:43:07'
  note: 'Milestone ''M4: Automated Tests'': 206 unit/integration tests passing 100%'
- at: '2026-10-10T14:43:09'
  note: 'Milestone ''M5: Dual-Doc ADR-118 and Scenario 33'': ADR-118 in TECDOC.md and Scenario 33 in MANUAL_TESTS.md'
- at: '2026-10-10T14:43:10'
  note: 'Milestone ''M6: Release and Deploy v0.34.10'': binary installed locally, tag v0.34.10 created and pushed to origin dev'
- at: '2026-10-10T14:59:47'
  note: Superseded by anycontext_blueprint_v0350_english_i18n_braille_spinner_point_by_point_release_notes_standard
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 🏛️ AnyContext Architecture Blueprint: v0.34.10
## Standalone ONNX Model Catalog, 404 Resolution and Download Resilience

---

### 1. Context and Root Cause
In version `v0.34.9`, the user started downloading the `Laya AI (mmBERT INT8 ONNX)` model from the TUI menu and got an immediate HTTP 404 Not Found error:
```text
❌ [Download Failed] Could not download Laya AI (mmBERT INT8 ONNX): Model repository returned HTTP status 404 Not Found: https://huggingface.co/convaiinnovations/laya/resolve/main/model_int8.onnx.
```

**Detailed Diagnosis:**
1. The canonical upstream repository `convaiinnovations/laya` distributes weights only in the PyTorch `model.safetensors` format, with no ONNX export.
2. Modern multimodal models on Hugging Face often split their weights into compound files (`model.onnx` + `model.onnx_data`), so downloading a single `.onnx` file would lead to silent "external data missing" failures when loading at runtime.
3. Requests against the Hugging Face CDN require an explicit `User-Agent` and automatic following of HTTP 302 redirects to Cloudflare/S3 endpoints.

---

### 2. Verified Canonical Catalog (HTTP 200 Standalone Single-File)

All 5 models were tested and validated live through HTTP HEAD requests and a stream of the first bytes:

| ID | Model Name | Category | Exact Size | Canonical Hugging Face URL | Role |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `laya-int8` | **Laya AI (mmBERT INT8 ONNX)** | Ingestion | 424,348,081 B (~404 MB) | `https://huggingface.co/tozp/laya-onnx/resolve/main/model_int8.onnx` | Dynamic classification of forms and invoices (~95% accuracy, 60ms CPU) |
| `mobilenetv4-rvl-cdip` | **MobileNetV4 RVL-CDIP (ONNX)** | Scans | 3,931,745 B (~3.9 MB) | `https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx` | Visual sentinel for scanned documents and photos (224x224, 8ms CPU) |
| `bge-small-onnx` | **BGE-Small Query Classifier (ONNX)** | Questions | 133,093,490 B (~127 MB) | `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx` | RFC-042 neural classification of question complexity (Fast vs Deep) |
| `layoutlmv3-int8` | **LayoutLMv3 Base (INT8 ONNX)** | Ingestion | 127,001,085 B (~121 MB) | `https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx` | Multimodal extraction of document, form and table layouts |
| `clip-vit-int8` | **CLIP ViT Vision Encoder (INT8 ONNX)** | Vision | 88,648,877 B (~84 MB) | `https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` | Dense visual extraction and air-gapped OCR for schematics and diagrams |

---

### 3. Improvements Implemented

1. **HTTP Client Hardening (`download_model_with_progress`)**:
   - `reqwest::Client::builder()` configured with `user_agent("AnyContext/0.34.10 (Windows; x86_64)")`.
   - `redirect(reqwest::redirect::Policy::limited(10))` to follow CDN and Cloudflare hops transparently.
   - Timeout extended to 600 seconds for large model downloads over connections with varying bandwidth.

2. **TUI Submenu Alignment (`menu.rs`)**:
   - `build_doc_ai_ingestion_menu`: exposes `laya-int8` and `layoutlmv3-int8`.
   - `build_doc_ai_vision_menu`: exposes `clip-vit-int8` and the Multimodal Cloud option.
   - `active_vision_state`: updated to detect `clip-vit-int8` and keep the deterministic 0 MB RAM fallback.

3. **Invariant Architectural Guarantee (Zero Crash Fallback)**:
   - When no model is downloaded, AnyContext stays fully operational using the 2D spatial heuristic in pure Rust (<5MB RAM / <1µs).

---

### 4. Validation & Deliverables
- **Unit and Integration Tests**: all 206 tests of `any-context-core-rs`, 13 tests of `actx-cli`, 7 of `actx-agent`, 8 of `actx-lm` and 9 of `actx-installer` passed with 0 failures.
- **Dual-Doc**:
  - `TECDOC.md`: ADR-118 recorded.
  - `tests/MANUAL_TESTS.md`: Scenario 33 recorded.
- **Deploy**: release binary `actx.exe` v0.34.10 built and installed locally at `%LOCALAPPDATA%\actx\bin\actx.exe`.
- **Git**: commit made, tag `v0.34.10` created and pushed successfully to `origin dev`.
