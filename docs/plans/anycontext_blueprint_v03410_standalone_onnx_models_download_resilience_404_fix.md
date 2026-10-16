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
- name: 'M1: Validacao Live URLs ONNX'
  done: true
- name: 'M2: Config Cliente HTTP Reqwest'
  done: true
- name: 'M3: Alinhamento Submenus TUI'
  done: true
- name: 'M4: Testes Automatizados'
  done: true
- name: 'M5: Dual-Doc ADR-118 e Cenario 33'
  done: true
- name: 'M6: Release e Deploy v0.34.10'
  done: true
status_log:
- at: '2026-10-10T14:43:04'
  note: 'Milestone ''M1: Validacao Live URLs ONNX'': 5 repositórios testados ao vivo via HTTP HEAD 200'
- at: '2026-10-10T14:43:05'
  note: 'Milestone ''M2: Config Cliente HTTP Reqwest'': User-Agent e policy de 10 redirects configurados'
- at: '2026-10-10T14:43:06'
  note: 'Milestone ''M3: Alinhamento Submenus TUI'': Ingestão com layoutlmv3 e visão com clip-vit'
- at: '2026-10-10T14:43:07'
  note: 'Milestone ''M4: Testes Automatizados'': 206 unit/integration tests passando 100%'
- at: '2026-10-10T14:43:09'
  note: 'Milestone ''M5: Dual-Doc ADR-118 e Cenario 33'': ADR-118 em TECDOC.md e Cenario 33 em MANUAL_TESTS.md'
- at: '2026-10-10T14:43:10'
  note: 'Milestone ''M6: Release e Deploy v0.34.10'': Binário instalado localmente, tag v0.34.10 criada e push para origin dev'
- at: '2026-10-10T14:59:47'
  note: Superseded by anycontext_blueprint_v0350_english_i18n_braille_spinner_point_by_point_release_notes_standard
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 🏛️ AnyContext Architecture Blueprint: v0.34.10
## Catálogo de Modelos ONNX Standalone, Resolução de 404 e Resiliência de Download

---

### 1. Contexto e Causa Raiz
Na versão `v0.34.9`, o usuário iniciou o download do modelo `Laya AI (mmBERT INT8 ONNX)` pelo menu da TUI e recebeu erro imediato de HTTP 404 Not Found:
```text
❌ [Falha no Download] Não foi possível baixar Laya AI (mmBERT INT8 ONNX): Model repository returned HTTP status 404 Not Found: https://huggingface.co/convaiinnovations/laya/resolve/main/model_int8.onnx.
```

**Diagnóstico Detalhado:**
1. O repositório canônico upstream `convaiinnovations/laya` distribui pesos apenas no formato PyTorch `model.safetensors`, sem export ONNX.
2. Modelos multimodais modernos no Hugging Face frequentemente dividem os pesos em arquivos compostos (`model.onnx` + `model.onnx_data`), de forma que o download de um único arquivo `.onnx` resultaria em falhas silenciosas de external data missing durante o carregamento de runtime.
3. Requisições contra a CDN da Hugging Face exigem `User-Agent` explícito e seguimento automático de redirecionamentos HTTP 302 para endpoints Cloudflare/S3.

---

### 2. Catálogo Canônico Verificado (HTTP 200 Standalone Single-File)

Todos os 5 modelos foram testados e validados ao vivo via requisições HTTP HEAD e stream de primeiros bytes:

| ID | Nome do Modelo | Categoria | Tamanho Exato | URL Canônica Hugging Face | Função |
| :--- | :--- | :--- | :---: | :--- | :--- |
| `laya-int8` | **Laya AI (mmBERT INT8 ONNX)** | Ingestão | 424.348.081 B (~404 MB) | `https://huggingface.co/tozp/laya-onnx/resolve/main/model_int8.onnx` | Classificação dinâmica de formulários e faturas (~95% precisão, 60ms CPU) |
| `mobilenetv4-rvl-cdip` | **MobileNetV4 RVL-CDIP (ONNX)** | Scans | 3.931.745 B (~3.9 MB) | `https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx` | Sentinela visual para documentos escaneados e fotos (224x224, 8ms CPU) |
| `bge-small-onnx` | **BGE-Small Query Classifier (ONNX)** | Perguntas | 133.093.490 B (~127 MB) | `https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx` | Classificação neural RFC-042 da complexidade da pergunta (Fast vs Deep) |
| `layoutlmv3-int8` | **LayoutLMv3 Base (INT8 ONNX)** | Ingestão | 127.001.085 B (~121 MB) | `https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx` | Extração multimodal de layouts de documentos, formulários e tabelas |
| `clip-vit-int8` | **CLIP ViT Vision Encoder (INT8 ONNX)** | Visão | 88.648.877 B (~84 MB) | `https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` | Extração visual densa e OCR air-gapped para esquemas e diagramas |

---

### 3. Melhorias Implementadas

1. **Blindagem do Cliente HTTP (`download_model_with_progress`)**:
   - `reqwest::Client::builder()` configurado com `user_agent("AnyContext/0.34.10 (Windows; x86_64)")`.
   - `redirect(reqwest::redirect::Policy::limited(10))` para seguir saltos de CDN e Cloudflare transparentemente.
   - Timeout estendido de 600 segundos para downloads de modelos grandes em conexões com variação de banda.

2. **Alinhamento dos Submenus da TUI (`menu.rs`)**:
   - `build_doc_ai_ingestion_menu`: Expõe `laya-int8` e `layoutlmv3-int8`.
   - `build_doc_ai_vision_menu`: Expõe `clip-vit-int8` e a opção Nuvem Multimodal.
   - `active_vision_state`: Atualizado para detectar `clip-vit-int8` e manter fallback determinístico de 0 MB de RAM.

3. **Garantia Arquitetural Invariante (Zero Crash Fallback)**:
   - Caso nenhum modelo seja baixado, o AnyContext permanece 100% operacional utilizando a heurística espacial 2D em Rust puro (<5MB RAM / <1µs).

---

### 4. Validação & Entregáveis
- **Testes Unitários e de Integração**: 100% dos 206 testes de `any-context-core-rs`, 13 testes de `actx-cli`, 7 de `actx-agent`, 8 de `actx-lm` e 9 de `actx-installer` passaram com 0 falhas.
- **Dual-Doc**:
  - `TECDOC.md`: ADR-118 registrado.
  - `tests/MANUAL_TESTS.md`: Cenário 33 registrado.
- **Deploy**: Binário de release `actx.exe` v0.34.10 gerado e instalado localmente em `%LOCALAPPDATA%\actx\bin\actx.exe`.
- **Git**: Commit realizado, tag `v0.34.10` criada e enviada com sucesso para `origin dev`.
