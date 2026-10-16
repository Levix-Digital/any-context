---
id: 118
title: Canonical Catalog of Verified Standalone ONNX Models and HTTP Resilience
status: accepted
date: '2026-10-10'
version: v0.34.10
subsystems: [model-catalog]
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0118: Canonical Catalog of Verified Standalone ONNX Models and HTTP Resilience

## Context

In v0.34.9, downloading the `Laya AI (mmBERT INT8 ONNX)` model failed with HTTP 404 Not Found because the upstream Hugging Face repository only distributed PyTorch `model.safetensors` files, without the expected ONNX export. In addition, many multimodal ONNX models use external weight files (`model.onnx_data`), which breaks single-file downloads.

## Decision

1. **Strict selection of self-contained single-file ONNX models**. All five catalog models were tested live with HTTP HEAD requests and first-byte streams against the Hugging Face API, returning HTTP 200 with the exact size:
   - `laya-int8`: `https://huggingface.co/tozp/laya-onnx/resolve/main/model_int8.onnx` (424,348,081 bytes, about 404 MB): INT8 decision engine for document and invoice classification.
   - `mobilenetv4-rvl-cdip`: `https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx` (3,931,745 bytes, about 3.9 MB): ultra-light visual sentinel for scanned pages.
   - `bge-small-onnx`: `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx` (133,093,490 bytes, about 127 MB): RFC-042 neural router for question complexity.
   - `layoutlmv3-int8`: `https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx` (127,001,085 bytes, about 121 MB): multimodal layout and form extraction.
   - `clip-vit-int8`: `https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` (88,648,877 bytes, about 84 MB): air-gapped dense visual encoder for charts and diagrams.
2. **Hardened `reqwest` HTTP client**: `User-Agent: AnyContext/0.34.10 (Windows; x86_64)` and redirect policy `reqwest::redirect::Policy::limited(10)`, to follow the Hugging Face CDN/Cloudflare/S3 redirects without rejection.
3. **TUI submenus and runtime status aligned**:
   - `build_doc_ai_ingestion_menu` exposes `laya-int8` and `layoutlmv3-int8`.
   - `build_doc_ai_vision_menu` exposes `clip-vit-int8`.
   - `active_vision_state` checks for the presence of `clip-vit-int8`.

## Alternatives Considered

Not recorded.

## Consequences

- No broken links or 404s in any Document AI download.
- Every model is a single self-contained `.onnx` file, with no secondary external weight downloads.
- The instant deterministic fallback is fully preserved when the user downloads no model.
