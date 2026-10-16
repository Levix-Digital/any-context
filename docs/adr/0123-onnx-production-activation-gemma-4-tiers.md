---
id: 123
title: Ativação dos Modelos ONNX nos Fluxos de Produção e Suporte à Família Gemma 4 com 5 Tiers de Hardware
status: accepted
date: '2026-10-14'
version: v0.38.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0123: Ativação dos Modelos ONNX nos Fluxos de Produção e Suporte à Família Gemma 4 com 5 Tiers de Hardware

- **Status**: Implemented & Verified
- **Date**: 2026-10-14
- **Context**:
  A Fase 3 do Master Plan do AnyContext estabelece a ativação dos modelos neurais ONNX em produção local sem violar o Inviolable ADR-110 (Zero-Breakage & Pure Rust Fallback Guarantee).
  Com a seleção da família Gemma 4 (Compact, 12B, 27B e 32B INT4) como cérebro generativo local, identificamos os seguintes imperativos arquiteturais:
  1. Desacoplamento entre Camada de Percepção (Document AI: MobileNetV4, LayoutLMv3, BGE-Small, CLIP-ViT, Laya mmBERT ~740MB) e Camada Generativa (Gemma 4 INT4).
  2. Isolamento estrito de hardware: o modelo Gemma 4 32B INT4 consome ~25.5GB RAM com KV-cache. Em máquinas de 32GB de RAM, somado ao SO (4-6GB), ocorre swap thrashing catastrófico. Portanto, o 32B exige o tier Sovereign Ultra (64GB+), enquanto o 27B INT4 (~18.5GB RAM) opera no Workstation Pro (32GB).
  3. Expansão da matriz de hardware de 3 para 5 tiers canônicos (`SovereignUltra`, `WorkstationPro`, `AirGappedPower`, `SmartBalanced`, `Lite`).
  4. Adapters de percepção em `any-context-core-rs` implementando traits universais (`ScanSentinel`, `DocumentLayoutExtractor`, `DocumentVisionExtractor`) integrados ao `QualityGate` e `IngestionModelRouter`.
  5. Provedor `GemmaOnnxProvider` em `actx-lm` com suporte a chat não-streaming e streaming SSE com fallback gracioso para nuvem.
- **Decisão**:
  1. **Matriz de Hardware em 5 Tiers**:
     - `SovereignUltra` (64GB+ RAM, 19.2GB Disco): Gemma 4 32B INT4 (~25.5GB Peak RAM) + Suíte Completa Document AI (740MB).
     - `WorkstationPro` (32GB RAM, 15.7GB Disco): Gemma 4 27B INT4 (~18.5GB Peak RAM) + Suíte Completa Document AI (740MB).
     - `AirGappedPower` (16GB RAM, 7.9GB Disco): Gemma 4 12B INT4 (~10.0GB Peak RAM) + Suíte Completa Document AI (740MB).
     - `SmartBalanced` (8GB RAM, 1.7GB Disco): Gemma 4 Compact INT4 (~2.5GB Peak RAM) + Document AI Essencial (MobileNetV4, LayoutLMv3, BGE-Small ~252MB).
     - `Lite` (Qualquer Hardware, 0MB Disco, <45MB RAM): Heurísticas determinísticas Rust puro (<10ms) + Provedores Cloud/API.
  2. **Perception Adapters em `any-context-core-rs`**:
     - `OnnxScanSentinelAdapter` encapsula `actx_onnx::OnnxScanSentinel` e implementa `ScanSentinel` para classificar páginas rasterizadas em sub-8ms no `QualityGate`.
     - `OnnxLayoutExtractorAdapter` encapsula `actx_onnx::OnnxLayoutExtractor` e implementa `DocumentLayoutExtractor` para reconstrução tabular 2D no `IngestionModelRouter`.
     - `OnnxVisionEncoderAdapter` encapsula `actx_onnx::OnnxVisionEncoder` e implementa `DocumentVisionExtractor` para embeddings CLIP ViT de diagramas e elementos visuais.
  3. **Generative Runner em `actx-onnx` e `actx-lm`**:
     - `OnnxGemmaEngine` em `actx-onnx::gemma`: gerencia sessões ONNX do Gemma 4, formatação BPE com tokens `<start_of_turn>` e `<end_of_turn>`, KV-cache local e streaming de saída por callback.
     - `GemmaOnnxProvider` em `actx-lm::providers::gemma`: implementa a trait `LmProvider` (`chat_complete` e `chat_stream`). Na ausência de pesos, delega automaticamente ao provedor de fallback configurado (Claude, Gemini, OpenAI, Groq, Ollama, Mock).
  4. **Multi-Surface Parity**:
     - `/profile` (alias `/preset`) permite alternar dinamicamente entre os 5 tiers com atualização em tempo real no SQLite e na TUI.
     - `/model` e menu TUI listam os 4 modelos Gemma 4 com badges dinâmicos de prontidão local (`[Local Ready]`, `[Active / ONNX Ready]`, `[Not Installed]`, `[Fallback]`).
     - `actx-installer` exibe pre-flight check com métricas em MB e GB amigáveis e auto-seleção transparente do perfil ótimo.
- **Consequências**:
  - Zero risco de panics ou falhas caso os pesos ONNX não estejam instalados ou o arquivo esteja corrompido (ADR-110 rigorosamente preservado).
  - Isolamento seguro de memória para máquinas corporativas e de desenvolvimento sem swap death.
