---
id: 118
title: Catálogo Canônico de Modelos ONNX Standalone Verificados e Resiliência HTTP
status: accepted
date: '2026-10-10'
version: v0.34.10
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0118: Catálogo Canônico de Modelos ONNX Standalone Verificados e Resiliência HTTP

- **Contexto**:
  - Na versão v0.34.9, o download do modelo `Laya AI (mmBERT INT8 ONNX)` resultava em erro HTTP 404 Not Found porque o repositório upstream na Hugging Face distribuía apenas arquivos PyTorch `model.safetensors`, sem o export ONNX esperado. Além disso, muitos modelos multimodais ONNX utilizam arquivos externos de pesos (`model.onnx_data`), o que corrompe downloads de arquivo único.
- **Decisão**:
  1. **Seleção Estrita de Modelos ONNX Single-File Auto-Contidos**:
     - Todos os 5 modelos do catálogo foram testados e verificados ao vivo via requisições HTTP HEAD e stream de primeiros bytes contra a API da Hugging Face, retornando HTTP 200 com tamanho exato:
       * `laya-int8`: `https://huggingface.co/tozp/laya-onnx/resolve/main/model_int8.onnx` (424.348.081 bytes / ~404 MB) - Motor de decisão INT8 para classificação de documentos e faturas.
       * `mobilenetv4-rvl-cdip`: `https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx` (3.931.745 bytes / ~3.9 MB) - Sentinela visual ultraleve para páginas digitalizadas.
       * `bge-small-onnx`: `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx` (133.093.490 bytes / ~127 MB) - Roteador neural RFC-042 para complexidade de perguntas.
       * `layoutlmv3-int8`: `https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx` (127.001.085 bytes / ~121 MB) - Extração multimodal de layout e formulários.
       * `clip-vit-int8`: `https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` (88.648.877 bytes / ~84 MB) - Encoder visual denso air-gapped para gráficos e esquemas.
  2. **Configuração Blindada do Cliente HTTP `reqwest`**:
     - Configurado `User-Agent: AnyContext/0.34.10 (Windows; x86_64)` e política de redirecionamento `reqwest::redirect::Policy::limited(10)` para seguir redirects CDN/Cloudflare/S3 da Hugging Face sem rejeição.
  3. **Alinhamento dos Submenus TUI e Status em Runtime**:
     - `build_doc_ai_ingestion_menu` atualizado para expor `laya-int8` e `layoutlmv3-int8`.
     - `build_doc_ai_vision_menu` atualizado para expor `clip-vit-int8`.
     - `active_vision_state` sincronizado para validar presença de `clip-vit-int8`.
- **Consequências**:
  - Eliminação de links quebrados ou 404 em todos os downloads do Document AI.
  - Modelos 100% autocontidos em um único arquivo `.onnx`, dispensando downloads secundários de pesos externos.
  - Preservação integral do fallback determinístico instantâneo caso o usuário não faça download de nenhum modelo.
