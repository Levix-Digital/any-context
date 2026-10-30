---
title: Model Catalog and Hardware Profiling
summary: Catalog and downloader of local ONNX models, hardware probing and installation profiles.
code: [crates/any-context-core-rs/src/ingestion/model_manager.rs, crates/any-context-core-rs/src/hardware, crates/any-context-core-rs/src/ingestion/onnx_adapter.rs]
---

# Model Catalog and Hardware Profiling

Catalog and downloader of local ONNX models, hardware probing and installation profiles.

## Responsibilities

- Keep the single, verified catalog of local models (`ONNX_CATALOG`): what exists, what can be downloaded, where each file goes and how to verify it.
- Download model files safely: pre-flight disk and RAM checks, streaming with progress and cancellation, SHA-256 and size verification, atomic rename.
- Report the state of the local model store and of the active re-ranker.
- Probe the machine (RAM, free disk, GPU adapters) and map it to hardware profiles and model safety verdicts.
- Adapt `actx-onnx` components to the core and agent traits (`onnx_adapter.rs`).

Running the models is the job of [ONNX Inference](onnx.md). The installer has its own copy of the profile logic for first-time provisioning (see [Installer and Updater](installer.md)).

## Structure

| Item | File | Role |
|---|---|---|
| `ONNX_CATALOG`, `OnnxModelSpec`, `ModelFile`, `OnnxModelCategory`, `ModelAvailability` | `ingestion/model_manager.rs` | Catalog entries. |
| `OnnxModelManager` | `ingestion/model_manager.rs` | Lookup (`find_spec`, `resolve_spec`), paths, installation state, safety checks, downloads, active re-ranker. |
| `DownloadProgressInfo` | `ingestion/model_manager.rs` | Downloaded and total MB, speed and ETA. |
| `HardwareProfiler`, `SystemMetrics`, `GpuDevice` | `hardware/mod.rs` | Probing and evaluation. |
| `ProfileTier`, `SemaphoreStatus`, `ProfileEvaluation`, `HardwareSafetyVerdict` | `hardware/mod.rs` | Profiles and verdicts. |
| `OnnxComplexityClassifierAdapter`, `OnnxScanSentinelAdapter`, `OnnxLayoutExtractorAdapter`, `OnnxVisionEncoderAdapter` | `ingestion/onnx_adapter.rs` | Bridges from `actx-onnx` to `actx-agent` and ingestion traits. |

## Behavior

### Catalog

| Id | Category | Availability | Files |
|---|---|---|---|
| `mmarco-reranker` (default re-ranker) | `ReRanker` | Available | `mmarco-reranker/model.onnx` (118,620,016 bytes) + `tokenizer.json` (17,082,660 bytes) |
| `bge-reranker-v2-m3` | `ReRanker` | Available | `bge-reranker-v2-m3/model.onnx` (570,727,094 bytes) + `tokenizer.json` |
| `multilingual-e5-small` | `Embedding` | Available | `multilingual-e5-small/model.onnx` (118,054,593 bytes) + `tokenizer.json` (17,082,730 bytes) |
| `multilingual-e5-base` | `Embedding` | Available | `multilingual-e5-base/model.onnx` + `tokenizer.json` (295,266,822 bytes in all) |
| `bge-m3` | `Embedding` | Available | `bge-m3/model.onnx` + `tokenizer.json` (585,539,515 bytes in all) |
| `laya-int8` | `IngestionClassifier` | Available | `laya_mmbert_int8.onnx` (424,348,081 bytes) |
| `layoutlmv3-int8` | `IngestionClassifier` | Available | `layoutlmv3_base_int8.onnx` (127,001,085 bytes) |
| `mobilenetv4-rvl-cdip` | `ScannedClassifier` | Available | `mobilenetv4_rvl_cdip.onnx` (3,931,745 bytes) |
| `bge-small-onnx` | `QueryClassifier` | Available | `bge_small_query_int8.onnx` (133,093,490 bytes) |
| `clip-vit-int8` | `DocumentVision` | Available | `clip_vit_vision_int8.onnx` (88,648,877 bytes) |
| `gemma-4-compact`, `gemma-4-12b`, `gemma-4-27b`, `gemma-4-32b` | `LanguageModel` | Planned | none |

Every file URL is pinned to an upstream revision and carries its exact size and SHA-256. Re-rankers and embedding models live in a subfolder per model. Only the re-rankers and the embedding models are executed by a neural runtime today; the Document AI and query classifier weights can be downloaded but their components run heuristics (see [ONNX Inference](onnx.md)). Planned entries cannot be downloaded and `planned_message` explains why.

### Lookup and the active re-ranker

`resolve_spec` accepts ids and aliases (an exact id wins, so `bge-m3` is the embedding model and `m3` or `reranker-m3` the re-ranker): `reranker`, `rerank` and anything containing `mmarco` resolve to `mmarco-reranker` (checked first, so digits in model names do not match Gemma aliases), and `compact`, `12b`, `27b`, `32b` resolve to the Gemma tiers. `active_reranker_spec` returns the model from the `reranker_model` setting (read once and cached), defaulting to `mmarco-reranker`; `set_active_reranker` persists it.

### Downloads

1. `check_download_safety(spec, force)` requires free disk space in the models directory for the model plus a 500 MB buffer, and checks RAM with `HardwareProfiler::validate_model_safety`. Without `force`, an unsafe verdict blocks the download.
2. `download_model_with_progress` streams each file with `reqwest` (redirects limited to 10) from its pinned URL, or from `<mirror>/<file path>` when `models_mirror()` returns a mirror (`ACTX_MODELS_MIRROR`, else the `models_mirror` setting; the public source is then never contacted), into `<file>.download.tmp`, reports progress through a channel and observes a `tokio::sync::watch` cancellation signal.
3. The size and SHA-256 are checked; only then is the temporary file renamed into place. Files that are already present and intact are kept.
4. On any error (stream, write, verification or cancellation) the `.download.tmp` file is removed.

`verify_installed(spec)` checks every file of an installed model against its catalog SHA-256 (`check_sha256`), caching the result per path, size and modification time. The embedding resolution calls it before loading a local embedding model, so files copied into the models folder by hand or by IT are used only when they are the verified ones.

### Hardware probing

`HardwareProfiler::probe_system` reads total and available RAM (Windows `GlobalMemoryStatusEx`, Linux `/proc/meminfo`), free disk space for the data path (Windows `GetDiskFreeSpaceExW`) and GPU adapters:

- Windows: DXGI loaded dynamically (`dxgi.dll`, `CreateDXGIFactory1`, `EnumAdapters1`), skipping software adapters.
- Linux: `/proc/driver/nvidia`, `/dev/nvidia0` and DRM render nodes (`/dev/dri/renderD128`).
- macOS: Apple Silicon detection through `sysctl`.

`SystemMetrics::recommended_execution_device` suggests a device, but inference currently runs on the CPU only (see [ONNX Inference](onnx.md)).

### Profiles

| Profile | Models it installs | Green when |
|---|---|---|
| `SovereignUltra` | full Document AI suite, `mmarco-reranker`, `bge-m3`, Gemma 4 32B (planned) | ≥ 30 GB RAM available or ≥ 56 GB total |
| `WorkstationPro` | full suite, `mmarco-reranker`, `bge-m3`, Gemma 4 27B (planned) | ≥ 16 GB available or ≥ 28 GB total |
| `AirGappedPower` | full suite, `mmarco-reranker`, `bge-m3`, Gemma 4 12B (planned) | ≥ 6.5 GB available or ≥ 14 GB total |
| `SmartBalanced` | MobileNetV4, LayoutLMv3, BGE-Small, `mmarco-reranker`, `multilingual-e5-base`, Gemma 4 Compact (planned) | ≥ 2.5 GB available or ≥ 7 GB total |
| `Lite` | `multilingual-e5-small` | always |

Each profile also needs its disk requirement plus 1 GB of headroom for green (0.25 GB for yellow); the disk requirement is the sum of the *available* models it installs, so planned Gemma models do not count. `evaluate_profiles` returns a green, yellow or red evaluation with a reason for every profile, and `auto_select_recommended` picks the most capable green profile. `recommended_local_gemma` maps RAM to the Gemma tier that would fit the machine and is used in safety recommendations; the presets no longer select Gemma (see [ADR-0133](../adr/0133-offline-and-privacy-presets-require-a-working-local-model.md)). Every id a profile lists must exist in the catalog, which a unit test checks. The profile's embedding model is also its default embedding model (`ProfileTier::default_embedding_model`), chosen with the retrieval benchmark ([ADR-0154](../adr/0154-local-embedding-model-with-its-own-setting-and-no-fallback.md)).

### Model safety verdicts

`validate_model_safety(model, metrics)` returns `Safe`, `Tight` or `Unsafe` with the required, total and available RAM. Cloud models are always safe; local models are compared with their estimated peak RAM. The [command engine](commands.md) uses it to block unsafe `/model` selections and downloads unless `--force` is given.

### Adapters

`OnnxComplexityClassifierAdapter` wraps the `actx-onnx` query classifier as the agent's `ComplexityClassifier`, and the other adapters wrap the scan sentinel, layout extractor and vision encoder as ingestion traits. The agent builder in the CLI attaches the classifier adapter (with `bge_small_query_int8.onnx` as its path); the ingestion pipeline does not attach the perception adapters today.

## Interfaces

- **Called by**: the [command engine](commands.md) (`/download`, `/model`, `/profile`, `/preset`, `/rerank`, `/status`), the [TUI](tui.md) (download task, model menus, header badges), the [CLI](cli.md) (`actx download`, re-ranker creation, `--diag`).
- **Calls**: [storage](storage.md) (models directory, settings), [ONNX Inference](onnx.md) (`ExecutionDevice`, component types).

## Decisions

- [ADR-0115](../adr/0115-onnx-weights-repository-fallbacks.md): native ONNX weights repository with fallbacks.
- [ADR-0117](../adr/0117-aurora-palette-borderless-canvas-onboarding.md): download progress and graceful cancellation.
- [ADR-0118](../adr/0118-verified-single-file-onnx-model-catalog.md): verified catalog and HTTP redirect handling.
- [ADR-0121](../adr/0121-modular-onnx-crate-hardware-profiles.md): hardware pre-flight check and semaphore profiles.
- [ADR-0123](../adr/0123-onnx-production-activation-gemma-4-tiers.md): five hardware tiers.
- [ADR-0125](../adr/0125-local-gemma-activation-hardware-guardrails.md): `Safe`/`Tight`/`Unsafe` model guardrails.
- [ADR-0126](../adr/0126-assisted-gemma-weights-downloader.md): `resolve_spec`, `check_download_safety`, `DownloadProgressInfo`.
- [ADR-0127](../adr/0127-cross-platform-hardware-acceleration-matrix.md): GPU adapter probing.
- [ADR-0130](../adr/0130-real-onnx-runtime-multilingual-cross-encoder.md): multi-file catalog pinned by revision with SHA-256, Gemma 4 marked planned, disk requirements from installed weights only.
