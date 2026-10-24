---
plan_id: anycontext_blueprint_v0450_frente_c0_runtime_onnx_real_e_reranker_cross_encoder
project: any-context
title: 'Blueprint v0.45.0 (Front C0): Real ONNX Runtime (ort load-dynamic + tokenizers), Multilingual mmarco Cross-Encoder and Technical Honesty Rectification (ADR-130)'
version: v0.45.0
status: superseded
author: Levi Silveira
created_at: '2026-10-15T21:03:23'
updated_at: '2026-10-24T14:17:00'
session_id: 8abbc3bf-d4c0-43cc-a5ef-84b7f6e30e4a
tags:
- frente_c0
- onnx_runtime
- ort
- load_dynamic
- tokenizers
- reranker
- cross_encoder
- mmarco
- adr_130
- honestidade_tecnica
- catalogo_sha256
- zero_links_quebrados
- gemma4_fatos
- runtime_no_pacote
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: M0 Base runtime
  done: true
- name: M1 Verified catalog
  done: true
- name: M2 Real reranker
  done: true
- name: M3 Rectification in the code
  done: true
- name: M4 Surfaces
  done: true
- name: M5 Tests and NFR
  done: true
- name: M6 Dual-Doc
  done: true
- name: M7 Manual tests
  done: true
- name: M8 Release
  done: false
status_log:
- at: '2026-10-15T21:54:51'
  note: 'Milestone ''M0 Base runtime'': ort 2.0.0-rc.14 load-dynamic + tokenizers 0.23 behind the runtime feature; explicit init_from loading; exe imports identical to v0.44 (commit ff042d4)'
- at: '2026-10-15T21:54:52'
  note: 'Milestone ''M1 Verified catalog'': multi-file catalog with pinned revision, size and SHA-256; Gemma Planned; catalog_urls 13/13 OK'
- at: '2026-10-15T21:54:54'
  note: 'Milestone ''M2 Real reranker'': real mmarco-mMiniLMv2-L12 through ONNX Runtime CPU; spawn_blocking; top-30; p50 298ms/30 candidates'
- at: '2026-10-15T21:54:56'
  note: 'Milestone ''M3 Rectification in the code'': Gemma planned/fallback, heuristics labeled, GPU planned, installer with verified URLs and reranker provisioning'
- at: '2026-10-15T21:54:57'
  note: 'Milestone ''M4 Surfaces'': /rerank status+model, --rerank-model, TUI badges, menus, honest /download and /device; release.yml with ORT in the package'
- at: '2026-10-15T21:54:59'
  note: 'Milestone ''M5 Tests and NFR'': cargo test --workspace 349 passed/0 failed; real reranker and catalog_urls OK; imports verified'
- at: '2026-10-15T21:55:00'
  note: 'Milestone ''M6 Dual-Doc'': ADR-130 + rectification notes 118/123/125/127/129 + README v0.45.0'
- at: '2026-10-15T21:55:03'
  note: 'Milestone ''M7 Manual tests'': Scenario 45 (14 steps) at the top of MANUAL_TESTS.md'
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
- at: '2026-10-24T14:17:00'
  by: Levi Silveira
  status: superseded
  note: Superseded by the architecture v2 redesign and roadmap v2 (docs/ROADMAP.md, docs/plans/anycontext_roadmap_v2.md, decided 2026-10-23); the remaining work was rebuilt as backlog items.
---

# 📐 Engineering Blueprint: AnyContext v0.45.0
## Front C0 — Real ONNX Runtime (`ort` + `tokenizers`), a Truly Multilingual Cross-Encoder and Technical Honesty Rectification

---

### 📌 Metadata & Governance
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.45.0` (Minor — new runtime capability; successor of `v0.44.0`)
- **Phase**: Front C0 of the Next-Generation RAG Roadmap (inserted before Front C)
  - Revised roadmap: A (v0.43.0 ✅) → B (v0.44.0 ✅*) → **C0 (v0.45.0)** → C (v0.46.0: local embeddings + Model Kit) → D (v0.47.0: Hierarchical Chunking / Parent-Document / AST Expansion)
  - *Front B delivered the Two-Stage pipeline, flags and surfaces, but Stage 2 is a lexical heuristic — C0 truly completes Front B.
- **Alignment with the Master Plan** (`detect_drift`: 🟢 ALIGNED, 68%): completes the "Cross-Encoder in Rust" item of **Priority 3** and is a technical prerequisite of the **Model Kit** milestone (Phase 3), since local embeddings (Front C) require real ONNX inference.
- **Superseded Previous Plan**: `anycontext_blueprint_v0440_two_stage_neural_retrieval_com_re_ranking_local_cross_encoder_onnx_aceleracao_por_hardware_e_fallback_gracioso_adr_110_frente_b`
- **Global Policy**: Calgary, Alberta, Canada — a global, agnostic product; multilingual default model.
- **Architectural Pattern**: Hexagonal (Ports & Adapters). Inference isolated in `crates/actx-onnx`; business rules in the Core; CLI/TUI as adapters.

---

### 🔍 Context: Discovery Findings (Stage 1)

1. **There is no ONNX runtime in the workspace.** No crate depends on `ort`, `tract`, `candle` or `tokenizers` (`Cargo.toml`/`Cargo.lock`). `ModelSession::open` (`crates/actx-onnx/src/session.rs`) only checks the file's existence and size (> 1 KB).
2. **The reranker (v0.44.0) is heuristic.** `OnnxReranker::score_pair` (`crates/actx-onnx/src/reranker.rs:79-140`) splits on spaces, counts terms/bigrams and applies a sigmoid; the downloaded weights are never executed.
3. **Gemma 4 returns fixed text.** `OnnxGemmaEngine::generate` (`crates/actx-onnx/src/gemma.rs:137-153`) returns `"[Local … Inference (… - 38.4 tok/s)]: Processed prompt…"`; the provider injects `Throughput: 38.4 tok/s` (`crates/actx-lm/src/providers/gemma.rs:82`).
4. **Classifier, Scan Sentinel, Layout and Vision** are heuristics that declare themselves neural when the file exists (`is_neural_active()` = file present), with labels such as `🧠 [ONNX Neural Router: BGE-Small]` (`query_classifier.rs:175,181`).
5. **GPU acceleration** (ADR-127) is hardware detection + labels; no Execution Provider is instantiated. The release only publishes **Linux x86_64** and **Windows x86_64** (`.github/workflows/release.yml:25-30`) — there is no macOS/Metal binary.
6. **Catalog with non-existent links** (the Hugging Face API returns an error): `BAAI/bge-reranker-small`, `onnx-community/gemma-4-*-ONNX` (Core) and `anycontext/models`, `google/gemma-4-*-it-onnx` (Installer, `crates/actx-installer/src/hardware.rs:109-170`). This violates the **ADR-118** rule (live verification HTTP 200 + exact size).
7. **ADR-121 already foresaw** the runtime behind a feature flag (`onnx`) — never implemented (`crates/any-context-core-rs/Cargo.toml`: `[features] default = []`, `actx-onnx` mandatory).
8. **Documentation diverges from the code**: ADR-123 ("local KV-cache"), ADR-125 ("100% functional activation of local neural inference"), ADR-127 (`38.4 tok/s` telemetry, Metal/macOS), ADR-129 and `README.md:48-58` (cross-encoder with cross-attention, "latency under 18ms per batch").
9. **Strength to keep**: the reranker interface is already clean — `score_batch(&str, &[String]) -> Vec<f32>` with an empty vector = RRF fallback, consumed at `retrieval/pipeline.rs:611` and `:755`, with the `/rerank auto|on|off` modes, the `--rerank` flag, a TUI badge and SQLite persistence. C0 swaps the engine without touching the contract.

---

## 1. 🎯 Goal & Architecture Decisions (ADR-130)

**Goal**: deliver real ONNX inference in AnyContext, starting with Stage 2 of Two-Stage Retrieval (cross-encoder), and align code and documentation with the reality of what is neural and what is heuristic.

**ADR-130 — Native ONNX Runtime (`ort`), Real Multilingual Cross-Encoder and Rectification of ADRs 123/125/127/129**

| # | Decision | Rationale |
|---|---|---|
| D1 | **Runtime (revised during implementation)**: `ort = "=2.0.0-rc.14"` with the **`load-dynamic`** feature — no static linking and no binary downloads at build time. The ONNX Runtime library is loaded at runtime, lazily, from `ORT_DYLIB_PATH` (tests/CI) or from the **executable's folder** (`onnxruntime.dll` / `libonnxruntime.so`). **Tokenization**: `tokenizers = "0.23"` (HF, `onig` feature). | Found in M0: static linking (the only prebuilt `ort` build for Windows includes DirectML) made `actx.exe` import `DirectML.dll`, `d3d12.dll`, `dxgi.dll`, `MSVCP140.dll`, `MSVCP140_1.dll` and `VCRUNTIME140_1.dll` at load time — missing any of them would prevent the program from opening (an ADR-110 violation). With `load-dynamic`, the binary keeps exactly the v0.44.0 dependencies and any load failure becomes the RRF fallback. |
| D2 | **`runtime` feature flag in `actx-onnx`**, propagated as `onnx = ["actx-onnx/runtime"]` in the Core and **on by default in `actx-cli`/release**. Without the flag, `actx-onnx` builds without `ort` and every neural component reports inactive (ADR-110 fallback). | Fulfills the intent of ADR-121 without `cfg` scattered in the Core (the Core keeps referencing the `actx-onnx` types). Allows fast builds/tests without ONNX Runtime. |
| D3 | **Default reranker**: `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1`, variant `onnx/model_quint8_avx2.onnx` (118,620,016 bytes, SHA-256 `6c2513767fb63d008a4377bef7a7a3555433d9436342bb53e35a3a72ffc52d4b`) + `tokenizer.json` (17,082,660 bytes, SHA-256 `62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626`), pinned revision `1427fd652930e4ba29e8149678df786c240d8825`. XLM-RoBERTa architecture, 1-logit output, max 512 positions. Apache-2.0 license. | Multilingual (includes PT) for a global product; 119 MB fits every local profile; commercial license allowed. URLs with a pinned revision guarantee reproducibility. |
| D4 | **Optional premium reranker**: `onnx-community/bge-reranker-v2-m3-ONNX` (`onnx/model_int8.onnx`, ~571 MB, Apache-2.0), the same code path (XLM-R, 1 logit). Selectable through `/rerank model <id>` / `--rerank-model <id>`; size and SHA-256 collected and verified during implementation (M1). | Higher quality for profiles ≥ 32 GB without new inference code. |
| D5 | **CPU only in C0** (`CPUExecutionProvider`, `intra_threads = min(physical cores, 4)`). The device reported in badges/telemetry becomes the one **actually used**. GPU detection (ADR-127) stays as a diagnostic; `--device`/`/device` with a value ≠ `cpu` emit the warning "GPU Execution Providers planned" and continue on CPU. | Controlled scope; removes fictitious telemetry. GPU EPs (DirectML/CUDA) are left for a future version. |
| D6 | **Lazy loading** of the ONNX session (`OnceLock`) on the first `score_batch`, never at startup. | Keeps the sub-10ms cold start and the fast-path test (ADR-122). |
| D7 | **Execution outside the async runtime**: the calls at `pipeline.rs:611` and `:755` use `tokio::task::spawn_blocking`. Batches of up to 16 pairs; `longest_first` truncation at 512 tokens; a configurable ceiling of reranked candidates (default 30). | CPU inference is blocking; avoids stalling the Tokio executor and the TUI. |
| D8 | **Honest semantics of `is_neural_active()`**: `true` only when a real ONNX session was created successfully (valid model + tokenizer and compatible inputs/outputs). A present but invalid file ⇒ `false` + the reason logged. | Today "the file exists" ≠ "inference happens". |
| D9 | **Verified multi-file catalog (evolution of ADR-118)**: `OnnxModelSpec` gets `files: &[ModelFile { file_name, url, size_bytes, sha256 }]` and `availability: Available | Planned`. The download aggregates the progress of all files, validates **SHA-256** before the atomic rename and writes into a per-model subfolder (`models/<id>/`). | Real models need `tokenizer.json`; SHA-256 protects against corruption and silent upstream swaps. |
| D10 | **Honesty rectification in the code**: (a) Gemma 4 becomes `availability: Planned`; `OnnxGemmaEngine::is_neural_active()` returns `false` and `generate*` returns a clear error → `GemmaOnnxProvider` uses the existing fallback (ADR-110); `actx download compact|12b|27b|32b` reports "local generative inference planned"; (b) removal of every fictitious throughput string (`gemma.rs:148`, `providers/gemma.rs:82`, `tui/menu.rs:1171`); (c) classifier, scan sentinel, layout and vision report `heuristic` in `is_neural_active()`, in the `OnnxEngine` summary and in labels (`[Heuristic Router]`); (d) **"zero broken links" policy**: every non-existent link is **fixed** when there is a verified substitute, or **removed** (model `Planned`) when there is none — see D13. | A documentation/UX pillar faithful to the real behavior. |
| D13 | **Full fix of the catalog links**: (a) **Installer — 5 Document AI models** (`crates/actx-installer/src/hardware.rs:109-137`, today under `huggingface.co/anycontext/models/...`, a non-existent repository) switch to exactly the Core's verified URLs: `tozp/laya-onnx/resolve/main/model_int8.onnx`, `onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx`, `BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx`, `onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx`, `Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` (all confirmed to exist in the Hugging Face API during Discovery), with the same `size_bytes`/SHA-256 as the Core; (b) **Reranker** (Core and Installer): `bge-reranker-small` → `mmarco-reranker`; (c) **Gemma 4** (4 URLs in the Core `model_manager.rs:63-93` + 4 in the Installer `hardware.rs:146-170`): URLs removed, entries `Planned`, no download. | No broken link remains in the product and no link points to weights AnyContext does not execute yet. |
| D15 | **Runtime and reranker transparent to the user**: (a) the release workflow downloads Microsoft's **official ONNX Runtime v1.31.0 (CPU, no DirectML)** — `onnxruntime-win-x64-1.31.0.zip` (SHA-256 `3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195`) and `onnxruntime-linux-x64-1.31.0.tgz` (SHA-256 `cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57`) —, verifies the hash and includes `onnxruntime.dll` / `libonnxruntime.so` **inside** `actx-windows-x86_64.zip` / `actx-linux-x86_64.tar.gz`; the installer and `actx --update` already extract and atomically swap every file of the package (`atomic_swap.rs:157`), so executable and runtime arrive and are updated together, with no extra step or configuration; (b) the **installer provisions `mmarco-reranker` automatically** as part of the chosen profile (local profiles already list it in `target_models()`); the **Lite** profile downloads no models and runs on RRF; `actx download reranker` remains only as a repair tool. | A configuration-free experience: the user installs/updates and the neural reranker simply works; executable and runtime versions never diverge; it works offline after the package download. |
| D14 | **Verified facts about the Gemma 4 family recorded in ADR-130** (input for the future generative inference front): (1) the real family published by Google: **E2B, E4B, 12B, 26B-A4B (MoE) and 31B** — the current catalog tiers (`compact`, `27b`, `32b`) do not exist; (2) official ONNX (`onnx-community`, Apache-2.0, `transformers.js`) **only for E2B and E4B**; 12B only in unverified third-party ONNX repositories; 26B-A4B and 31B without ONNX (GGUF/QAT); (3) **multi-graph, multi-file** format: `decoder_model_merged`, `embed_tokens`, audio/vision encoders, with fragmented external weights (`.onnx_data`, `.onnx_data_1…_9`) — E2B q4 > 3.6 GB in total; (4) consequence: requires a multi-shard downloader, a generative loop (tokenization, KV-cache, sampling) and recalibration of the hardware tiers — the scope of its own front, not C0. | Avoids losing the diagnosis and prevents the next front from repeating a fictitious catalog. |
| D11 | **Documentation rectification**: ADR-130 in `TECDOC.md` records the facts; ADRs 123, 125, 127 and 129 get **a one-line rectification note** pointing to ADR-130 (history kept, no rewrite). `README.md` corrects the claims of v0.38–v0.44. | Faithful Dual-Doc; historical traceability. |
| D12 | **Continuous catalog verification**: an `#[ignore]` test that does HEAD/GET on each catalog file (status 200 + `content-length` = `size_bytes`), run explicitly in the release workflow. | Prevents regression of the ADR-118 rule without burdening every push. |

**Out of scope (explicit)**: local embeddings, recalibration of profile RAM/disk, `actx profile install`, unification of the Core/Installer catalogs (Front C); real inference for Gemma 4, the classifier, layout and vision; GPU Execution Providers; macOS; fixing legacy README passages about ChromaDB/Python; the bug of the query embedding fixed to `text-embedding-3-small` (`pipeline.rs:452`) — a candidate for a `v0.44.1` patch or Front C.

---

## 2. 🏛️ Surface Parity Mapping

| Surface | Change |
|---|---|
| **Core (`any-context-core-rs`)** | Multi-file catalog + SHA-256; `reranker_model` in settings; `spawn_blocking` in the pipeline; `/rerank` status with model, real device and measured latency; `/rerank model <id>`; Gemma `Planned`; `onnx` feature. |
| **`actx-onnx`** | Real `OnnxSession` (ort), HF tokenization, real `OnnxReranker`, honest `is_neural_active`, labeled heuristics, Gemma unavailable with a clear error. |
| **`actx-lm`** | `GemmaOnnxProvider`: removes fictitious telemetry; fallback whenever the engine is unavailable. |
| **Headless CLI (`actx-cli`)** | `--rerank-model <id>`; `actx download reranker` / `reranker-m3`; `actx download <gemma>` with a "planned" message; `--device` ≠ cpu with a warning. |
| **TUI** | Badges `[Rerank: ⚡ ONNX: mmarco-L12 (CPU)]`, `[Rerank: ⚡ RRF (Fallback)]`, `[Rerank: 🚫 Off]`; the acceleration badge shows the real **inference** device; the `[F1]` menu shows Gemma as "Planned" and heuristics as "Heuristic". |
| **Installer (`actx-installer`)** | **Fixes** the 5 Document AI URLs to the Core's verified URLs (D13a); the reranker becomes `mmarco-reranker` with the same files/hashes as the Core; Gemma 4 `Planned` without a URL. Unifying the catalog into a single source is left for Front C. |
| **REST / MCP** | No new endpoints; they inherit the Core's behavior automatically (same pipeline). |
| **Canonical database** | Only a new additive `reranker_model` key in the SQLite settings table. No change in LanceDB. |

---

## 3. 📁 Affected Files and Lines

**`crates/actx-onnx/`**
- `Cargo.toml` — optional dependencies `ort` (=2.0.0-rc.14) and `tokenizers` (0.23); `[features] runtime = ["dep:ort", "dep:tokenizers"]`.
- `src/session.rs` — `ModelSession` wraps `ort::session::Session` (under `runtime`), validation of inputs/outputs, lazy loading; without `runtime` → always invalid.
- `src/tokenizer.rs` (**new**) — a wrapper over `tokenizers::Tokenizer` for (query, doc) pairs, truncation/padding, generation of `input_ids`/`attention_mask` (and `token_type_ids` only if the model needs them).
- `src/reranker.rs` — replaces `tokenize_pair`/`compute_cross_attention_logit` (lines ~94-140) with real batched inference; keeps `score_batch`, `score_pair`, `is_neural_active`, `device`; sigmoid over the real logit.
- `src/gemma.rs` — `is_neural_active()` = `false`; `generate`/`generate_stream` (lines 137-200) return a descriptive `Err`; removes `38.4 tok/s`.
- `src/query_classifier.rs`, `src/scan_sentinel.rs`, `src/layout_extractor.rs`, `src/vision_encoder.rs` — `is_neural_active()` = `false`; "Heuristic" labels.
- `src/engine.rs` — the summary (`summary`, lines 60-80) reports `neural` / `heuristic` / `planned` per component; reranker path by active model.
- `src/device.rs` — exposes `ExecutionDevice::effective()` (CPU in this version) for honest telemetry.
- `tests/fixtures/` (**new**) — `tiny_reranker.onnx` + `tiny_tokenizer.json` (KBs) and `gen_tiny_reranker.py` (a reproducible generation script, run offline by the maintainer).

**`crates/any-context-core-rs/`**
- `Cargo.toml` — `[features] default = ["onnx"]`, `onnx = ["actx-onnx/runtime"]`.
- `src/ingestion/model_manager.rs` — `OnnxModelSpec` with `files[]` and `availability`; removes `bge-reranker-small` (lines 149-158); adds `mmarco-reranker` and `bge-reranker-v2-m3`; Gemma `Planned`; `download_model_with_progress` (lines 471-545) multi-file, per-model subfolder, SHA-256, user agent with `CARGO_PKG_VERSION`; `resolve_spec` (aliases `reranker`, `reranker-m3`); `active_reranker_state` (line 457); the `test_catalog_integrity` test.
- `src/retrieval/pipeline.rs` — `spawn_blocking` at `:611` and `:755`; candidate ceiling; test with a fixture.
- `src/commands/engine.rs` — `/rerank` status (lines ~1734-1831) with model, device and latency; the `/rerank model <id>` subcommand; `/download` for `Planned` models; `/device` with a warning.
- `src/storage/sqlite.rs` — `reranker_model` getter/setter (additive key).
- `src/hardware/mod.rs` — `target_models()` replaces `bge-reranker-small` with `mmarco-reranker` (lines 68-95).

**`crates/actx-lm/`**
- `src/providers/gemma.rs` — removes the telemetry block (line ~82); fallback when the engine is unavailable.

**`crates/actx-cli/`**
- `Cargo.toml` — `any-context-core-rs` with the `onnx` feature.
- `src/engine.rs` — `create_default_reranker()` (lines 114-119) resolves the active model through settings/catalog; Gemma (lines 90-100) uses the fallback with a warning.
- `src/cli/args.rs` / `src/cli/oneshot.rs` — `--rerank-model`; download messages for `Planned`; `--device` warning.
- `src/tui/app.rs` (line 248), `src/tui/ui.rs` (line 156) — badge with the real model and device.
- `src/tui/menu.rs` — removes "25-50 tok/s" (line 1171); Planned/Heuristic status.
- `tests/cli_tests.rs` — new flag and message tests.

**`crates/actx-installer/`**
- `src/hardware.rs` (lines 100-175) — lines 109-137: replace the 5 `anycontext/models/...` URLs with the Core's verified URLs (D13a) with identical `size_bytes`/SHA-256; reranker → `mmarco-reranker` (2 files); lines 146-170: Gemma 4 without a URL, marked `Planned`; `target_models()` tests (lines 548-552) updated.
- `src/downloader.rs` (line ~66) — review the URL construction (today through `format!`, probably tied to the non-existent repository), support multiple files and validate SHA-256; skip `Planned` models with a clear message.

**Root / Docs / CI**
- `Cargo.toml` (workspace) / `Cargo.lock` / `version.txt` / every crate — bump `0.45.0`.
- `.github/workflows/release.yml` — an explicit `cargo test -p any-context-core-rs -- --ignored catalog_urls` step before the build; a step that downloads the official ONNX Runtime v1.31.0, verifies SHA-256 and includes the library in the Windows/Linux packages (D15); `ORT_DYLIB_PATH` for the tests; a check that the binary does not import `DirectML.dll`/`d3d12.dll`/`MSVCP140.dll`.
- `.github/workflows/e2e-tests.yml` — the same runtime provisioning + `ORT_DYLIB_PATH` for the fixture tests.
- `crates/actx-installer/src/lib.rs` — profile installation includes `mmarco-reranker` (2 files, SHA-256).
- `README.md`, `TECDOC.md`, `tests/MANUAL_TESTS.md`, `docs/plans/<this blueprint>.md`.

---

## 4. 📚 Dual-Doc Update Plan

**UserDoc (`README.md`)**
- A new entry at the top of "Key Features" for `v0.45.0`: real ONNX runtime, multilingual mmarco reranker (119 MB), the `bge-reranker-v2-m3` option, `--rerank-model`, `/rerank model`, CPU execution.
- Fix `README.md:48-58` (v0.44.0): remove "typical latency under 18ms" and the DirectML/Metal/CUDA matrix of the reranker; state that the real neural Stage 2 arrives in v0.45.0 with measured latency (publish the real numbers from the NFR test).
- The "Assisted Download and Management of Local Weights (Gemma 4 ONNX)" section (lines 700-751): mark local Gemma 4 as **planned**; document `actx download reranker` / `reranker-m3` with sizes and SHA-256 verification.
- Profile sections (v0.37/v0.38, lines ~105-125): a note that the classifier/layout/vision run in heuristic mode in this version.
- `/help` and `--help` updated for `--rerank-model` and `/rerank model`.

**TecDoc (`TECDOC.md`)**
- Complete **ADR-130**: context (Discovery findings), decisions D1–D14, a **"Verified Facts about the Gemma 4 Family"** subsection (D14: real tiers, availability of official ONNX, multi-graph/multi-shard format, prerequisites for the generative front), a "before → after" table of every catalog URL (Core and Installer), consequences, a component matrix (`neural` / `heuristic` / `planned`), a Mermaid sequence diagram of the real Stage 2 (pipeline → `spawn_blocking` → tokenizer → `ort::Session` → logits → sigmoid → reordering → diversification → U-shaped), a diagram of the multi-file download with SHA-256 verification, a table of measured latency (p50/p95 for 30 candidates).
- A one-line rectification note at the top of ADRs 123, 125, 127 and 129 pointing to ADR-130.
- Update ADR-118 with an evolution note (multi-file catalog + SHA-256 + `catalog_urls` test).

---

## 5. 🧪 Automated Test Strategy & Regression Protection

**`actx-onnx` (`runtime` feature)**
- `test_session_missing_file_inactive` — missing file ⇒ inactive, no panic.
- `test_session_corrupt_file_inactive` — invalid bytes ⇒ inactive, error logged, no panic.
- `test_tokenizer_pair_encoding` — fixture tokenizer: pair structure, truncation at `max_len`, correct `attention_mask`.
- `test_reranker_fixture_scores_in_unit_interval` — fixture ONNX: scores ∈ [0,1], length = number of candidates.
- `test_reranker_fixture_ordering_deterministic` — same input ⇒ same scores; stable ordering.
- `test_reranker_batching_matches_single` — batch of 16 vs individual pairs ⇒ same scores (tolerance 1e-5).
- `test_gemma_engine_reports_unavailable` — `is_neural_active() == false` and `generate` returns a descriptive `Err` (replaces the current tests that assume "dummy file ⇒ active": `gemma.rs:241-242`, `reranker.rs:196`, `actx-lm/providers/gemma.rs:190-191`).
- `test_heuristic_components_report_heuristic` — classifier/sentinel/layout/vision never report `neural`.
- Build without the feature (`cargo test -p actx-onnx --no-default-features`) — everything inactive, zero panics.

**`any-context-core-rs`**
- `test_two_stage_pipeline_reranking_fallback` (existing, `pipeline.rs:951`) — stays green.
- `test_two_stage_pipeline_with_fixture_reranker` — the fixture reranker reorders candidates and does not block the runtime (`spawn_blocking`).
- `test_catalog_integrity` — every `Available` model has ≥ 1 file, `size_bytes > 0`, a 64-hex `sha256`; Gemma `Planned`; no `bge-reranker-small`.
- `test_download_sha256_mismatch_rejected` — a local test server delivers wrong bytes ⇒ error, temporaries removed, destination intact.
- `test_rerank_model_setting_roundtrip` — `/rerank model <id>` persists and is read by the factory.
- `catalog_urls` (`#[ignore]`) — HEAD/GET of every `Available` file **of the Core and the Installer** ⇒ 200 + exact size. Run in `release.yml`.

**`actx-installer`**
- `test_installer_catalog_matches_core` — for each model present in both catalogs, `url`, `size_bytes` and `sha256` are identical (a guard against divergence until Front C's unification).
- `test_installer_no_placeholder_urls` — no URL contains `anycontext/models` or `google/gemma-4-*-it-onnx`; no `Planned` model has a URL.
- Update of the `target_models()` count tests (`hardware.rs:548-552`).
- `real_reranker_multilingual` (`#[ignore]`, `ACTX_TEST_REAL_RERANKER=1`) — downloads (with a cache) the mmarco model; a question in PT with a relevant document and distractors (including negation) ⇒ relevant at the top; measures p50/p95 for 30 candidates and prints it for the TecDoc.

**`actx-cli`**
- `test_cli_flag_rerank_model_unknown` — invalid id ⇒ friendly error, exit code ≠ 0.
- `test_cli_download_planned_model_message` — `actx download compact` ⇒ "planned" message, no network.
- `test_cli_flag_rerank_off` (existing) — stays green.
- Startup fast-path test (ADR-122) — stays within the ceiling (lazy ONNX session).

**NFRs**
- No measurable regression in the `actx` cold start (library and session never loaded at boot).
- **Binary imports unchanged** relative to v0.44.0: an automatic check in the release that `actx.exe` does not import `DirectML.dll`, `d3d12.dll`, `dxgi.dll`, `MSVCP140*.dll`.
- Missing or incompatible runtime ⇒ RRF fallback without a crash (test with `ORT_DYLIB_PATH` pointing to a missing/invalid file).
- Package size increase (the ONNX Runtime library) documented in the TecDoc.

---

## 6. 📝 Manual Test Scenario Plan (`tests/MANUAL_TESTS.md`)

**Scenario 45 (v0.45.0 — Real ONNX Runtime, Multilingual mmarco Cross-Encoder and Honesty Rectification)** — inserted at the top, keeping Scenarios 1–44 fully intact:
1. `actx -v` ⇒ `actx 0.45.0`.
2. `actx download --check` ⇒ `mmarco-reranker` (~119 MB, `Available`), `bge-reranker-v2-m3` (~571 MB, `Available`), Gemma 4 as `Planned`; no `bge-reranker-small`.
3. `actx download reranker` ⇒ downloads 2 files with an aggregated bar, validates SHA-256, writes into `%LOCALAPPDATA%/actx/models/mmarco-reranker/`.
4. TUI: badge `[Rerank: ⚡ ONNX: mmarco-L12 (CPU)]`; `/rerank` shows the model, the `CPU` device and the latency of the last query.
5. A query in Portuguese in a workspace with similar documents (e.g. "v2 service timeout" vs "legacy v1") with `--rerank on` and `--rerank off` ⇒ with rerank, the correct passage rises to the top of the citations.
6. Rename/corrupt the downloaded `.onnx` ⇒ badge `[Rerank: ⚡ RRF (Fallback)]`, no crash; `actx download reranker` restores it.
7. `/rerank model bge-reranker-v2-m3` without weights ⇒ a warning with download instructions; after downloading, the badge changes to `bge-m3`.
8. `actx download compact` ⇒ the message "local generative inference planned", no network traffic.
9. `/model gemma-4-compact` ⇒ a clear warning and use of the fallback provider; no answer with "38.4 tok/s".
10. `actx --device dml "question"` ⇒ the warning "GPU Execution Providers planned; running on CPU".
11. `[F1]` → Document AI: classifier/layout/vision labeled "Heuristic".
12. Installer: `actx-installer --preflight-only` and installation of the `smart_balanced` profile ⇒ all Document AI models and `mmarco-reranker` download successfully (HTTP 200, valid SHA-256) without any extra user action; Gemma 4 appears as "Planned" and is not downloaded.
13. After installing/updating, the `%LOCALAPPDATA%\actx\bin\` folder contains `onnxruntime.dll` next to `actx.exe`; on the first query the badge shows the neural reranker active without any configuration command.
14. Rename `onnxruntime.dll` ⇒ `actx` opens normally and the reranker falls back to `[Rerank: ⚡ RRF (Fallback)]` with the reason shown in `/rerank`.

---

## 7. 🔒 Secret Hygiene, Database Migration and Rollback Plan

- **Secrets**: none. Hugging Face downloads are anonymous; no key/token added. `verify_gate` before the commit.
- **Migration**: only the additive `reranker_model` key in the SQLite settings (default `mmarco-reranker`). **Zero schema changes in LanceDB** (Front C will do the embedding dimension migration).
- **Compatibility**: old `bge_reranker_small_int8.onnx` files (impossible to obtain, since the URL does not exist) are ignored. The `rerank_mode` setting (auto/on/off) is kept.
- **Licenses**: mmarco and bge-reranker-v2-m3 Apache-2.0; ONNX Runtime MIT; `ort` MIT/Apache-2.0; `tokenizers` Apache-2.0 — record them in ADR-130.
- **Rollback**: a post-merge failure ⇒ revert the PR in `main` and tag `v0.45.1`. At runtime, `--rerank off` or missing weights bring back the v0.44.0 RRF behavior without a rebuild.

---

## 8. ⏱️ Step-by-Step Execution

**M0 — Base runtime**: optional `ort`/`tokenizers` dependencies, the `runtime`/`onnx` features, a real lazy `ModelSession`, `tokenizer.rs`; validate the Windows MSVC build + Linux locally/CI.
**M1 — Verified catalog**: multi-file `OnnxModelSpec` + `availability`; mmarco and bge-m3 entries (collect the m3 size/SHA-256); Gemma `Planned`; multi-file downloader with SHA-256 and a subfolder; the `catalog_urls` test.
**M2 — Real reranker**: `OnnxReranker` with batched inference; `spawn_blocking` in the pipeline; candidate ceiling; fixtures and tests.
**M3 — Rectification in the code**: Gemma unavailable with fallback; removal of fictitious telemetry; labeled heuristics; effective device; fix of the 5 Document AI URLs and the reranker in the installer; removal of the 8 fictitious Gemma 4 URLs (Core + Installer).
**M4 — Surfaces**: `/rerank model`, `--rerank-model`, `/rerank` status with real latency, TUI badges, `[F1]` menus, `Planned` download messages, `--device` warning.
**M5 — Tests & NFR**: `cargo test --workspace` 100% green; `--no-default-features` in `actx-onnx`; run `real_reranker_multilingual` locally and record p50/p95.
**M6 — Dual-Doc**: README + ADR-130 + rectification notes (ADR-118/123/125/127/129).
**M7 — Manual tests**: Scenario 45 at the top of `tests/MANUAL_TESTS.md`.
**M8 — Release**: bump `0.45.0`; `feat(onnx): ...` commit to `dev`; PR `dev → main`; green CI; merge; tag `v0.45.0`; follow `release.yml` until publication is confirmed in `Levix-Digital/any-context-releases`; `check_milestone`; record in `session-memory`; final checklist.
