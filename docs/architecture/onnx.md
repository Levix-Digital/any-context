---
title: ONNX Inference
summary: Optional local neural inference with the cross-encoder re-ranker on ONNX Runtime, plus heuristic perception components.
code: [crates/actx-onnx]
---

# ONNX Inference

Optional local neural inference with the cross-encoder re-ranker on ONNX Runtime, plus heuristic perception components.

## Responsibilities

- Load the ONNX Runtime shared library lazily and run inference sessions on the CPU.
- Score `(query, document)` pairs with the cross-encoder re-ranker used by Stage 2 of [retrieval](retrieval.md).
- Provide the query classifier, scan sentinel, layout extractor and vision encoder components, which today are deterministic heuristics.
- Model execution devices and Gemma 4 tiers for the surfaces that display them.
- Never break the caller: a missing library, missing weights or a failed load always degrade to the non-neural path.

The catalog and downloads of the model files belong to [Model Catalog and Hardware Profiling](model-catalog.md).

## Structure

| Module | Role | Status |
|---|---|---|
| `runtime.rs` | `OnnxSession`, `TensorOutput`, library resolution and lazy loading (`runtime` feature). | Neural runtime |
| `tokenizer.rs` | `PairTokenizer` over `tokenizer.json` (`tokenizers` crate). | Used by the re-ranker |
| `reranker.rs` | `OnnxReranker`: cross-encoder scoring. | **Neural** |
| `query_classifier.rs` | `OnnxQueryClassifier`, `OnnxComplexity`, `OnnxIntent`. | Heuristic |
| `scan_sentinel.rs` | `OnnxScanSentinel`. | Heuristic |
| `layout_extractor.rs` | `OnnxLayoutExtractor`. | Heuristic (passthrough) |
| `vision_encoder.rs` | `OnnxVisionEncoder`. | Heuristic |
| `gemma.rs` | `GemmaModelTier`, `GemmaTokenizer`, `OnnxGemmaEngine`. | **Planned** |
| `device.rs` | `ExecutionDevice`, `GpuDevice`, `AcceleratorVendor`, `AcceleratorBackend`. | CPU only in practice |
| `engine.rs` | `OnnxEngine` facade and `ComponentStatus`. | |
| `session.rs` | `ModelSession`: inspects a weight file (existence, size, header) without executing it. | |

Cargo features: `runtime = ["dep:ort", "dep:tokenizers"]` (off by default in this crate). `any-context-core-rs` enables it through its default `onnx` feature, so release builds include the runtime. Dependencies: `ort = "=2.0.0-rc.14"` with `load-dynamic` and `tokenizers = "0.23"` with `onig`.

## Behavior

### Runtime loading

- The library file is `onnxruntime.dll` on Windows and `libonnxruntime.so` on Linux. It is looked up at `ORT_DYLIB_PATH` when that variable is set, otherwise next to the running executable, where the release package ships it.
- The binary has no link-time dependency on ONNX Runtime. The library is loaded explicitly with `ort::init_from(path)` (inside `catch_unwind`) before any other `ort` API is used, because an implicit load would panic.
- Sessions use the CPU execution provider with `intra_threads = min(cores, 4)`.
- Without the `runtime` feature, or when the library is missing or incompatible, every load returns a descriptive error.

### Cross-encoder re-ranker

- Each model is a directory with `model.onnx` and `tokenizer.json` (default `mmarco-reranker`, the multilingual `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1`; alternative `bge-reranker-v2-m3`).
- The session is created on the first `score_batch` call, never at startup (`ensure_loaded`).
- Pairs are tokenized with `longest_first` truncation at 512 tokens (`DEFAULT_MAX_LENGTH`) and scored in batches of 16 (`BATCH_SIZE`), passing `input_ids`, `attention_mask` and, when the model has it, `token_type_ids`. Logits go through a sigmoid to give scores in [0, 1].
- `is_neural_active()` is true only when the library is present, the weights are present and no load failure has been recorded for that model path in this process. `recorded_load_error` lets status surfaces report a failed load.
- `score_batch` returns an empty vector on any failure, which tells the pipeline to keep the RRF order; `try_score_batch` returns the error.
- The device can be set (`with_device`, `set_device`), but `effective_device()` is always the CPU.
- In tests, `ACTX_REQUIRE_ORT=1` (set in CI) turns a missing runtime into a failure instead of a skip.

### Heuristic components

- **Query classifier**: a keyword and structure heuristic (sub-microsecond) that returns a complexity (fast or deep) and an intent; it is used by the agent's `ChatModelRouter` through `OnnxComplexityClassifierAdapter`. The BGE-Small weights are tracked but not executed.
- **Scan sentinel**: decides whether a page is a scan from 2D text density.
- **Layout extractor**: passes content through; layout reconstruction is done by the core's spatial chunkers.
- **Vision encoder**: produces no synthetic description.

These components report their status as `heuristic` and never claim to be neural, even when their weight files exist.

### Gemma 4 and devices

`OnnxGemmaEngine::is_neural_active()` is always false and generation returns a clear error; `GemmaModelTier` (`compact`, `12b`, `27b`, `32b`) and `expected_filename` exist so that providers, menus and presets can name the tiers. `ExecutionDevice` parses `auto`, `dml`/`directml`/`gpu`, `metal`/`coreml`, `cuda`, `vulkan` and `cpu`; GPU execution providers are not instantiated in this build.

## Interfaces

- **Called by**: [retrieval](retrieval.md) (`OnnxReranker` in `NativeHybridPipeline`), the [CLI](cli.md) (`create_default_reranker`, `--device`, `--rerank`), the [TUI](tui.md) (re-ranker and device badges), the [agent](agent.md) (query classifier through the core adapter), [language model providers](lm-providers.md) (`GemmaOnnxProvider` uses `OnnxGemmaEngine` and always falls back).
- **Depends on**: the ONNX Runtime shared library shipped by the release (see [CI and Release](ci-release.md)) and the model files downloaded by the [model catalog](model-catalog.md) or provisioned by the [installer](installer.md).

## Decisions

- [ADR-0121](../adr/0121-modular-onnx-crate-hardware-profiles.md): separate `actx-onnx` crate coupled to the core through traits.
- [ADR-0123](../adr/0123-onnx-production-activation-gemma-4-tiers.md): perception adapters and the Gemma 4 engine.
- [ADR-0127](../adr/0127-cross-platform-hardware-acceleration-matrix.md): execution device abstraction.
- [ADR-0129](../adr/0129-two-stage-retrieval-cross-encoder-reranking.md): `OnnxReranker` with `score_batch` and `score_pair`.
- [ADR-0130](../adr/0130-real-onnx-runtime-multilingual-cross-encoder.md): real ONNX Runtime with `load-dynamic`, CPU only, multilingual cross-encoder, heuristic components reported honestly, Gemma 4 planned.
