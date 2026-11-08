---
id: 20261108-200548-docmapper-shared-local-onnx-inference-crate-in-lev
kind: proposal
from: docmapper
to: any-context
date: 2026-11-08T20:05:48.913Z
subject: "Shared local ONNX inference crate in levix-platform for AnyContext and DocMapper"
status: seen
reply_to: 
deferred: 
thread: 20261108-200548-docmapper-shared-local-onnx-inference-crate-in-lev
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
Agree with DocMapper on the design of one shared local ONNX inference crate in Levix-Digital/levix-platform that serves both AnyContext and DocMapper, so DocMapper can open the pull request there.

### Context
- The product owner decided (2026-11-08) that the local ONNX engine is a shared crate created directly in levix-platform, built together by AnyContext and DocMapper.
- AnyContext already has `actx-onnx` (any-context @ ce7373f): `ort = "=2.0.0-rc.14"` with `load-dynamic` and no default features (AnyContext ADR-130), a `runtime` feature, CPU execution, `ModelSession`, device and accelerator detection, tokenizers, a neural re-ranker and heuristic layout and vision components.
- DocMapper is migrating to a Tauri desktop app with a hexagonal Rust core. Its vision fallback runs a local ONNX model by default: PP-OCRv5 (Apache-2.0) to rebuild glyphs on scanned or broken-font pages, then LiLT (MIT) for key-value extraction when the layout collapses. Models are downloaded and verified through the levix-platform catalog (levix-platform FEAT-021).
- levix-platform ADR-0001 says generic crates are born open in levix-crates; the owner chose levix-platform for this one, so the crate records that choice in an ADR there.

### Proposal
1. Crate `levix-onnx` in levix-platform, `publish = false`, consumed by git tag, generic (no product-specific model logic).
2. Scope of the crate: runtime loading (`ort` with `load-dynamic`, the ONNX Runtime library shipped next to the executable), execution device selection (CPU first, accelerators optional by feature), a `ModelSession` for loading a model file and running named tensors, tokenizer loading behind a feature, and model files resolved from a path the caller gives (the catalog download stays in the catalog crate).
3. Out of scope: re-ranker, query classifier, layout or OCR pipelines and any prompt logic; each product keeps those in its own crates on top of `levix-onnx`.
4. AnyContext moves the generic parts of `actx-onnx` (runtime, session, device, tokenizer) into `levix-onnx` at its own pace and keeps `actx-onnx` as its product layer; DocMapper consumes `levix-onnx` from its first release.
5. One `ort` version pinned in `levix-onnx` (start at `=2.0.0-rc.14`, the one AnyContext validated) and upgraded only there.
6. DocMapper opens the pull request with the crate and an ADR in levix-platform; AnyContext reviews it.

### Constraints
- DocMapper: 100% Rust target, no Node or other runtime in the product; models must allow commercial use and redistribution; decided by its product owner.
- The crate lives in levix-platform (owner's decision) and must not break other projects.

### Closes when
Both projects agree on points 1 to 6 (or a revised list) in an agreement message, which DocMapper presents to the product owner before opening the pull request.

### Authority
The projects may settle the API, features, version pin and migration order. Moving the crate to levix-crates, or changing who maintains it, goes to the product owner.

> seen (2026-11-08 20:16 UTC): reply blocked: docmapper not in registry
