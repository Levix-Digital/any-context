---
plan_id: anycontext_blueprint_v0450_frente_c0_runtime_onnx_real_e_reranker_cross_encoder
project: any-context
title: 'Blueprint v0.45.0 (Frente C0): Runtime ONNX Real (ort load-dynamic + tokenizers), Cross-Encoder Multilingue mmarco e Retificacao de Honestidade Tecnica (ADR-130)'
version: v0.45.0
status: in_progress
author: Levi Silveira
created_at: '2026-10-15T21:03:23'
updated_at: '2026-10-15T21:55:03'
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
- name: M0 Runtime base
  done: true
- name: M1 Catalogo verificado
  done: true
- name: M2 Reranker real
  done: true
- name: M3 Retificacao no codigo
  done: true
- name: M4 Superficies
  done: true
- name: M5 Testes e NFR
  done: true
- name: M6 Dual-Doc
  done: true
- name: M7 Testes manuais
  done: true
- name: M8 Release
  done: false
status_log:
- at: '2026-10-15T21:54:51'
  note: 'Milestone ''M0 Runtime base'': ort 2.0.0-rc.14 load-dynamic + tokenizers 0.23 atras da feature runtime; carga explicita init_from; imports do exe identicos a v0.44 (commit ff042d4)'
- at: '2026-10-15T21:54:52'
  note: 'Milestone ''M1 Catalogo verificado'': Catalogo multi-arquivo com revisao fixada, tamanho e SHA-256; Gemma Planned; catalog_urls 13/13 OK'
- at: '2026-10-15T21:54:54'
  note: 'Milestone ''M2 Reranker real'': mmarco-mMiniLMv2-L12 real via ONNX Runtime CPU; spawn_blocking; top-30; p50 298ms/30 candidatos'
- at: '2026-10-15T21:54:56'
  note: 'Milestone ''M3 Retificacao no codigo'': Gemma planned/fallback, heuristicas rotuladas, GPU planned, instalador com URLs verificadas e provisionamento do reranker'
- at: '2026-10-15T21:54:57'
  note: 'Milestone ''M4 Superficies'': /rerank status+model, --rerank-model, badges TUI, menus, /download e /device honestos; release.yml com ORT no pacote'
- at: '2026-10-15T21:54:59'
  note: 'Milestone ''M5 Testes e NFR'': cargo test --workspace 349 passed/0 failed; real reranker e catalog_urls OK; imports verificados'
- at: '2026-10-15T21:55:00'
  note: 'Milestone ''M6 Dual-Doc'': ADR-130 + notas de retificacao 118/123/125/127/129 + README v0.45.0'
- at: '2026-10-15T21:55:03'
  note: 'Milestone ''M7 Testes manuais'': Cenario 45 (14 passos) no topo de MANUAL_TESTS.md'
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint de Engenharia: AnyContext v0.45.0
## Frente C0 — Runtime ONNX Real (`ort` + `tokenizers`), Cross-Encoder Multilíngue de Verdade e Retificação de Honestidade Técnica

---

### 📌 Metadados & Governança
- **Projeto**: AnyContext (`actx`)
- **Versão-Alvo**: `v0.45.0` (Minor — nova capacidade de runtime; sucessora da `v0.44.0`)
- **Fase**: Frente C0 do Roadmap de RAG de Nova Geração (inserida antes da Frente C)
  - Roadmap revisado: A (v0.43.0 ✅) → B (v0.44.0 ✅*) → **C0 (v0.45.0)** → C (v0.46.0: Embeddings locais + Kit de Models) → D (v0.47.0: Chunking Hierárquico / Parent-Document / Expansão AST)
  - *A Frente B entregou o pipeline Two-Stage, flags e superfícies, mas o Estágio 2 é uma heurística lexical — a C0 conclui a Frente B de verdade.
- **Alinhamento com o Master Plan** (`detect_drift`: 🟢 ALIGNED, 68%): conclui o item "Cross-Encoder em Rust" da **Prioridade 3** e é pré-requisito técnico do marco **Kit de Models** (Fase 3), pois embeddings locais (Frente C) exigem inferência ONNX real.
- **Plano Anterior Superado**: `anycontext_blueprint_v0440_two_stage_neural_retrieval_com_re_ranking_local_cross_encoder_onnx_aceleracao_por_hardware_e_fallback_gracioso_adr_110_frente_b`
- **Política Global**: Calgary, Alberta, Canadá — produto global e agnóstico; modelo padrão multilíngue.
- **Padrão Arquitetural**: Hexagonal (Ports & Adapters). Inferência isolada em `crates/actx-onnx`; regra de negócio no Core; CLI/TUI como adaptadores.

---

### 🔍 Contexto: Achados do Discovery (Etapa 1)

1. **Não existe runtime ONNX no workspace.** Nenhuma crate depende de `ort`, `tract`, `candle` ou `tokenizers` (`Cargo.toml`/`Cargo.lock`). `ModelSession::open` (`crates/actx-onnx/src/session.rs`) apenas verifica existência e tamanho (> 1 KB) do arquivo.
2. **Reranker (v0.44.0) é heurístico.** `OnnxReranker::score_pair` (`crates/actx-onnx/src/reranker.rs:79-140`) faz split por espaços, contagem de termos/bigramas e sigmoid; os pesos baixados nunca são executados.
3. **Gemma 4 retorna texto fixo.** `OnnxGemmaEngine::generate` (`crates/actx-onnx/src/gemma.rs:137-153`) devolve `"[Local … Inference (… - 38.4 tok/s)]: Processed prompt…"`; o provider injeta `Throughput: 38.4 tok/s` (`crates/actx-lm/src/providers/gemma.rs:82`).
4. **Classificador, Scan Sentinel, Layout e Visão** são heurísticas que se declaram neurais quando o arquivo existe (`is_neural_active()` = arquivo presente), com rótulos como `🧠 [ONNX Neural Router: BGE-Small]` (`query_classifier.rs:175,181`).
5. **Aceleração GPU** (ADR-127) é detecção de hardware + rótulos; nenhum Execution Provider é instanciado. O release publica apenas **Linux x86_64** e **Windows x86_64** (`.github/workflows/release.yml:25-30`) — não há binário macOS/Metal.
6. **Catálogo com links inexistentes** (API Hugging Face retorna erro): `BAAI/bge-reranker-small`, `onnx-community/gemma-4-*-ONNX` (Core) e `anycontext/models`, `google/gemma-4-*-it-onnx` (Instalador, `crates/actx-installer/src/hardware.rs:109-170`). Isso viola a regra da **ADR-118** (verificação ao vivo HTTP 200 + tamanho exato).
7. **ADR-121 já previa** o runtime atrás de uma feature flag (`onnx`) — nunca implementada (`crates/any-context-core-rs/Cargo.toml`: `[features] default = []`, `actx-onnx` obrigatório).
8. **Documentação diverge do código**: ADR-123 ("KV-cache local"), ADR-125 ("Ativação 100% funcional da inferência neural local"), ADR-127 (telemetria `38.4 tok/s`, Metal/macOS), ADR-129 e `README.md:48-58` (cross-encoder com atenção cruzada, "latência inferior a 18ms por lote").
9. **Ponto forte a preservar**: a interface do reranker já está limpa — `score_batch(&str, &[String]) -> Vec<f32>` com vetor vazio = fallback RRF, consumida em `retrieval/pipeline.rs:611` e `:755`, com modos `/rerank auto|on|off`, flag `--rerank`, badge na TUI e persistência SQLite. A C0 troca o motor sem tocar no contrato.

---

## 1. 🎯 Objetivo & Decisões Arquiteturais (ADR-130)

**Objetivo**: Entregar inferência ONNX real no AnyContext, começando pelo Estágio 2 do Two-Stage Retrieval (cross-encoder), e alinhar código e documentação à realidade do que é neural e do que é heurístico.

**ADR-130 — Runtime ONNX Nativo (`ort`), Cross-Encoder Multilíngue Real e Retificação das ADRs 123/125/127/129**

| # | Decisão | Rationale |
|---|---|---|
| D1 | **Runtime (revisado na implementação)**: `ort = "=2.0.0-rc.14"` com a feature **`load-dynamic`** — sem linkagem estática e sem download de binários na compilação. A biblioteca do ONNX Runtime é carregada em tempo de execução, preguiçosamente, a partir de `ORT_DYLIB_PATH` (testes/CI) ou da **pasta do executável** (`onnxruntime.dll` / `libonnxruntime.so`). **Tokenização**: `tokenizers = "0.23"` (HF, feature `onig`). | Constatado no M0: a linkagem estática (único build pré-compilado do `ort` para Windows inclui DirectML) fazia o `actx.exe` importar `DirectML.dll`, `d3d12.dll`, `dxgi.dll`, `MSVCP140.dll`, `MSVCP140_1.dll` e `VCRUNTIME140_1.dll` na carga — falta de qualquer uma impediria o programa de abrir (violação da ADR-110). Com `load-dynamic`, o binário mantém exatamente as dependências da v0.44.0 e qualquer falha de carga vira fallback RRF. |
| D2 | **Feature flag `runtime` em `actx-onnx`**, propagada como `onnx = ["actx-onnx/runtime"]` no Core e **ligada por padrão no `actx-cli`/release**. Sem a flag, `actx-onnx` compila sem `ort` e todo componente neural reporta inativo (fallback ADR-110). | Realiza a intenção da ADR-121 sem `cfg` espalhado no Core (o Core continua referenciando os tipos de `actx-onnx`). Permite builds/testes rápidos sem ONNX Runtime. |
| D3 | **Reranker padrão**: `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1`, variante `onnx/model_quint8_avx2.onnx` (118.620.016 bytes, SHA-256 `6c2513767fb63d008a4377bef7a7a3555433d9436342bb53e35a3a72ffc52d4b`) + `tokenizer.json` (17.082.660 bytes, SHA-256 `62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626`), revisão fixada `1427fd652930e4ba29e8149678df786c240d8825`. Arquitetura XLM-RoBERTa, saída de 1 logit, máx. 512 posições. Licença Apache-2.0. | Multilíngue (inclui PT) para um produto global; 119 MB cabe em todos os perfis locais; licença comercial permitida. URLs com revisão fixa garantem reprodutibilidade. |
| D4 | **Reranker opcional premium**: `onnx-community/bge-reranker-v2-m3-ONNX` (`onnx/model_int8.onnx`, ~571 MB, Apache-2.0), mesmo caminho de código (XLM-R, 1 logit). Selecionável via `/rerank model <id>` / `--rerank-model <id>`; tamanho e SHA-256 coletados e verificados na implementação (M1). | Maior qualidade para perfis ≥ 32 GB sem novo código de inferência. |
| D5 | **Somente CPU na C0** (`CPUExecutionProvider`, `intra_threads = min(núcleos físicos, 4)`). O dispositivo reportado em badges/telemetria passa a ser o **realmente usado**. A detecção de GPU (ADR-127) permanece como diagnóstico; `--device`/`/device` com valor ≠ `cpu` emitem aviso "Execution Providers de GPU planejados" e seguem em CPU. | Escopo controlado; elimina telemetria fictícia. GPU EPs (DirectML/CUDA) ficam para versão futura. |
| D6 | **Carregamento preguiçoso** da sessão ONNX (`OnceLock`) no primeiro `score_batch`, nunca na inicialização. | Preserva o cold-start sub-10ms e o teste de fast-path (ADR-122). |
| D7 | **Execução fora do runtime async**: as chamadas em `pipeline.rs:611` e `:755` passam a usar `tokio::task::spawn_blocking`. Lotes de até 16 pares; truncamento `longest_first` em 512 tokens; teto de candidatos reranqueados configurável (padrão 30). | Inferência CPU é bloqueante; evita travar o executor Tokio e a TUI. |
| D8 | **Semântica honesta de `is_neural_active()`**: `true` somente quando uma sessão ONNX real foi criada com sucesso (modelo + tokenizer válidos e entradas/saídas compatíveis). Arquivo presente mas inválido ⇒ `false` + motivo registrado em log. | Hoje "arquivo existe" ≠ "inferência acontece". |
| D9 | **Catálogo multi-arquivo verificado (evolução da ADR-118)**: `OnnxModelSpec` passa a ter `files: &[ModelFile { file_name, url, size_bytes, sha256 }]` e `availability: Available | Planned`. Download agrega progresso de todos os arquivos, valida **SHA-256** antes do rename atômico e grava em subpasta por modelo (`models/<id>/`). | Modelos reais precisam de `tokenizer.json`; SHA-256 protege contra corrupção e troca silenciosa upstream. |
| D10 | **Retificação de honestidade no código**: (a) Gemma 4 fica `availability: Planned`; `OnnxGemmaEngine::is_neural_active()` retorna `false` e `generate*` retorna erro claro → `GemmaOnnxProvider` usa o fallback já existente (ADR-110); `actx download compact|12b|27b|32b` informa "inferência generativa local planejada"; (b) remoção de todas as strings de throughput fictício (`gemma.rs:148`, `providers/gemma.rs:82`, `tui/menu.rs:1171`); (c) classificador, scan sentinel, layout e visão reportam `heuristic` em `is_neural_active()`, resumo do `OnnxEngine` e rótulos (`[Heuristic Router]`); (d) **política "zero links quebrados"**: todo link inexistente é **corrigido** quando há substituto verificado, ou **removido** (modelo `Planned`) quando não há — ver D13. | Pilar de documentação/UX fiel ao comportamento real. |
| D13 | **Correção integral dos links do catálogo**: (a) **Instalador — 5 modelos de Document AI** (`crates/actx-installer/src/hardware.rs:109-137`, hoje em `huggingface.co/anycontext/models/...`, repositório inexistente) passam a usar exatamente as URLs verificadas do Core: `tozp/laya-onnx/resolve/main/model_int8.onnx`, `onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx`, `BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx`, `onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx`, `Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx` (todas confirmadas existentes na API do Hugging Face no Discovery), com os mesmos `size_bytes`/SHA-256 do Core; (b) **Reranker** (Core e Instalador): `bge-reranker-small` → `mmarco-reranker`; (c) **Gemma 4** (4 URLs no Core `model_manager.rs:63-93` + 4 no Instalador `hardware.rs:146-170`): URLs removidas, entradas `Planned`, sem download. | Nenhum link quebrado permanece no produto e nenhum link aponta para pesos que o AnyContext ainda não executa. |
| D15 | **Runtime e reranker transparentes para o usuário**: (a) o workflow de release baixa o ONNX Runtime **oficial da Microsoft v1.31.0 (CPU, sem DirectML)** — `onnxruntime-win-x64-1.31.0.zip` (SHA-256 `3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195`) e `onnxruntime-linux-x64-1.31.0.tgz` (SHA-256 `cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57`) —, verifica o hash e inclui `onnxruntime.dll` / `libonnxruntime.so` **dentro** de `actx-windows-x86_64.zip` / `actx-linux-x86_64.tar.gz`; o instalador e o `actx --update` já extraem e trocam atomicamente todos os arquivos do pacote (`atomic_swap.rs:157`), então executável e runtime chegam e são atualizados juntos, sem passo extra nem configuração; (b) o **instalador provisiona o `mmarco-reranker` automaticamente** como parte do perfil escolhido (perfis locais já o listam em `target_models()`); perfil **Lite** não baixa modelos e opera em RRF; `actx download reranker` permanece apenas como ferramenta de reparo. | Experiência sem configuração: o usuário instala/atualiza e o reranker neural simplesmente funciona; versões de executável e runtime nunca divergem; funciona offline após o download do pacote. |
| D14 | **Fatos verificados da família Gemma 4 registrados na ADR-130** (insumo para a futura frente de inferência generativa): (1) família real publicada pelo Google: **E2B, E4B, 12B, 26B-A4B (MoE) e 31B** — os tiers do catálogo atual (`compact`, `27b`, `32b`) não existem; (2) ONNX oficial (`onnx-community`, Apache-2.0, `transformers.js`) **apenas para E2B e E4B**; 12B somente em repositórios ONNX de terceiros não verificados; 26B-A4B e 31B sem ONNX (GGUF/QAT); (3) formato **multi-grafo e multi-arquivo**: `decoder_model_merged`, `embed_tokens`, encoders de áudio/visão, com pesos externos fragmentados (`.onnx_data`, `.onnx_data_1…_9`) — E2B q4 > 3,6 GB somados; (4) consequência: exige downloader multi-shard, loop generativo (tokenização, KV-cache, amostragem) e recalibração dos tiers de hardware — escopo de uma frente própria, não da C0. | Evita perder o diagnóstico e impede que a próxima frente repita catálogo fictício. |
| D11 | **Retificação documental**: ADR-130 no `TECDOC.md` registra os fatos; ADRs 123, 125, 127 e 129 recebem **uma linha de nota de retificação** apontando para a ADR-130 (histórico preservado, sem reescrita). `README.md` corrige as afirmações de v0.38–v0.44. | Dual-Doc fiel; rastreabilidade histórica. |
| D12 | **Verificação contínua do catálogo**: teste `#[ignore]` que faz HEAD/GET em cada arquivo do catálogo (status 200 + `content-length` = `size_bytes`), executado explicitamente no workflow de release. | Impede regressão da regra ADR-118 sem onerar cada push. |

**Fora de escopo (explícito)**: embeddings locais, recalibração de RAM/disco dos perfis, `actx profile install`, unificação dos catálogos Core/Instalador (Frente C); inferência real do Gemma 4, classificador, layout e visão; Execution Providers de GPU; macOS; correção de trechos legados do README sobre ChromaDB/Python; bug do embedding de consulta fixo em `text-embedding-3-small` (`pipeline.rs:452`) — candidato a patch `v0.44.1` ou Frente C.

---

## 2. 🏛️ Mapeamento de Paridade de Superfícies

| Superfície | Mudança |
|---|---|
| **Core (`any-context-core-rs`)** | Catálogo multi-arquivo + SHA-256; `reranker_model` em settings; `spawn_blocking` no pipeline; status `/rerank` com modelo, dispositivo real e latência medida; `/rerank model <id>`; Gemma `Planned`; feature `onnx`. |
| **`actx-onnx`** | `OnnxSession` real (ort), tokenização HF, `OnnxReranker` real, `is_neural_active` honesto, heurísticas rotuladas, Gemma indisponível com erro claro. |
| **`actx-lm`** | `GemmaOnnxProvider`: remove telemetria fictícia; fallback sempre que o engine estiver indisponível. |
| **CLI headless (`actx-cli`)** | `--rerank-model <id>`; `actx download reranker` / `reranker-m3`; `actx download <gemma>` com mensagem "planejado"; `--device` ≠ cpu com aviso. |
| **TUI** | Badge `[Rerank: ⚡ ONNX: mmarco-L12 (CPU)]`, `[Rerank: ⚡ RRF (Fallback)]`, `[Rerank: 🚫 Off]`; badge de aceleração indica dispositivo **de inferência** real; menu `[F1]` mostra Gemma como "Planejado" e heurísticas como "Heurística". |
| **Instalador (`actx-installer`)** | **Corrige** as 5 URLs de Document AI para as URLs verificadas do Core (D13a); reranker passa a ser `mmarco-reranker` com os mesmos arquivos/hashes do Core; Gemma 4 `Planned` sem URL. A unificação do catálogo em fonte única fica para a Frente C. |
| **REST / MCP** | Sem endpoints novos; herdam o comportamento do Core automaticamente (mesmo pipeline). |
| **Banco canônico** | Apenas nova chave aditiva `reranker_model` na tabela de settings do SQLite. Nenhuma alteração no LanceDB. |

---

## 3. 📁 Arquivos e Linhas Impactadas

**`crates/actx-onnx/`**
- `Cargo.toml` — dependências opcionais `ort` (=2.0.0-rc.14) e `tokenizers` (0.23); `[features] runtime = ["dep:ort", "dep:tokenizers"]`.
- `src/session.rs` — `ModelSession` passa a encapsular `ort::session::Session` (sob `runtime`), validação de entradas/saídas, carregamento preguiçoso; sem `runtime` → sempre inválida.
- `src/tokenizer.rs` (**novo**) — wrapper sobre `tokenizers::Tokenizer` para pares (query, doc), truncamento/padding, geração de `input_ids`/`attention_mask` (e `token_type_ids` somente se o modelo exigir).
- `src/reranker.rs` — substitui `tokenize_pair`/`compute_cross_attention_logit` (linhas ~94-140) por inferência real em lote; mantém `score_batch`, `score_pair`, `is_neural_active`, `device`; sigmoid sobre o logit real.
- `src/gemma.rs` — `is_neural_active()` = `false`; `generate`/`generate_stream` (linhas 137-200) retornam `Err` descritivo; remove `38.4 tok/s`.
- `src/query_classifier.rs`, `src/scan_sentinel.rs`, `src/layout_extractor.rs`, `src/vision_encoder.rs` — `is_neural_active()` = `false`; rótulos "Heuristic".
- `src/engine.rs` — resumo (`summary`, linhas 60-80) reporta `neural` / `heuristic` / `planned` por componente; caminho do reranker por modelo ativo.
- `src/device.rs` — expõe `ExecutionDevice::effective()` (CPU nesta versão) para telemetria honesta.
- `tests/fixtures/` (**novo**) — `tiny_reranker.onnx` + `tiny_tokenizer.json` (KBs) e `gen_tiny_reranker.py` (script de geração reprodutível, executado offline pelo mantenedor).

**`crates/any-context-core-rs/`**
- `Cargo.toml` — `[features] default = ["onnx"]`, `onnx = ["actx-onnx/runtime"]`.
- `src/ingestion/model_manager.rs` — `OnnxModelSpec` com `files[]` e `availability`; remove `bge-reranker-small` (linhas 149-158); adiciona `mmarco-reranker` e `bge-reranker-v2-m3`; Gemma `Planned`; `download_model_with_progress` (linhas 471-545) multi-arquivo, subpasta por modelo, SHA-256, user-agent com `CARGO_PKG_VERSION`; `resolve_spec` (aliases `reranker`, `reranker-m3`); `active_reranker_state` (linha 457); teste `test_catalog_integrity`.
- `src/retrieval/pipeline.rs` — `spawn_blocking` em `:611` e `:755`; teto de candidatos; teste com fixture.
- `src/commands/engine.rs` — `/rerank` status (linhas ~1734-1831) com modelo, dispositivo e latência; subcomando `/rerank model <id>`; `/download` para modelos `Planned`; `/device` com aviso.
- `src/storage/sqlite.rs` — getter/setter `reranker_model` (chave aditiva).
- `src/hardware/mod.rs` — `target_models()` substitui `bge-reranker-small` por `mmarco-reranker` (linhas 68-95).

**`crates/actx-lm/`**
- `src/providers/gemma.rs` — remove bloco de telemetria (linha ~82); fallback quando engine indisponível.

**`crates/actx-cli/`**
- `Cargo.toml` — `any-context-core-rs` com feature `onnx`.
- `src/engine.rs` — `create_default_reranker()` (linhas 114-119) resolve o modelo ativo via settings/catálogo; Gemma (linhas 90-100) usa fallback com aviso.
- `src/cli/args.rs` / `src/cli/oneshot.rs` — `--rerank-model`; mensagens de download para `Planned`; aviso de `--device`.
- `src/tui/app.rs` (linha 248), `src/tui/ui.rs` (linha 156) — badge com modelo real e dispositivo.
- `src/tui/menu.rs` — remove "25-50 tok/s" (linha 1171); status Planejado/Heurística.
- `tests/cli_tests.rs` — novos testes de flag e mensagens.

**`crates/actx-installer/`**
- `src/hardware.rs` (linhas 100-175) — linhas 109-137: substituir as 5 URLs `anycontext/models/...` pelas URLs verificadas do Core (D13a) com `size_bytes`/SHA-256 idênticos; reranker → `mmarco-reranker` (2 arquivos); linhas 146-170: Gemma 4 sem URL, marcado `Planned`; testes `target_models()` (linhas 548-552) atualizados.
- `src/downloader.rs` (linha ~66) — revisar a montagem de URL (hoje via `format!`, provavelmente presa ao repositório inexistente), suportar múltiplos arquivos e validar SHA-256; pular modelos `Planned` com mensagem clara.

**Raiz / Docs / CI**
- `Cargo.toml` (workspace) / `Cargo.lock` / `version.txt` / todos os crates — bump `0.45.0`.
- `.github/workflows/release.yml` — passo explícito `cargo test -p any-context-core-rs -- --ignored catalog_urls` antes do build; passo que baixa o ONNX Runtime oficial v1.31.0, verifica SHA-256 e inclui a biblioteca nos pacotes Windows/Linux (D15); `ORT_DYLIB_PATH` para os testes; verificação de que o binário não importa `DirectML.dll`/`d3d12.dll`/`MSVCP140.dll`.
- `.github/workflows/e2e-tests.yml` — mesmo provisionamento do runtime + `ORT_DYLIB_PATH` para os testes com fixture.
- `crates/actx-installer/src/lib.rs` — instalação de perfil inclui o `mmarco-reranker` (2 arquivos, SHA-256).
- `README.md`, `TECDOC.md`, `tests/MANUAL_TESTS.md`, `docs/plans/<este blueprint>.md`.

---

## 4. 📚 Plano de Atualização Dual-Doc

**UserDoc (`README.md`)**
- Nova entrada no topo de "Key Features" para `v0.45.0`: runtime ONNX real, reranker multilíngue mmarco (119 MB), opção `bge-reranker-v2-m3`, `--rerank-model`, `/rerank model`, execução em CPU.
- Corrigir `README.md:48-58` (v0.44.0): remover "latência típica inferior a 18ms" e a matriz DirectML/Metal/CUDA do reranker; indicar que o Estágio 2 neural real chega na v0.45.0 com latência medida (publicar números reais do teste NFR).
- Seção "Download e Gerenciamento Assistido de Pesos Locais (Gemma 4 ONNX)" (linhas 700-751): marcar Gemma 4 local como **planejado**; documentar `actx download reranker` / `reranker-m3` com tamanhos e verificação SHA-256.
- Seções de perfis (v0.37/v0.38, linhas ~105-125): nota de que classificador/layout/visão operam em modo heurístico nesta versão.
- `/help` e `--help` atualizados para `--rerank-model` e `/rerank model`.

**TecDoc (`TECDOC.md`)**
- **ADR-130** completa: contexto (achados do Discovery), decisões D1–D14, subseção **"Fatos Verificados da Família Gemma 4"** (D14: tiers reais, disponibilidade de ONNX oficial, formato multi-grafo/multi-shard, pré-requisitos para a frente generativa), tabela "antes → depois" de todas as URLs do catálogo (Core e Instalador), consequências, matriz de componentes (`neural` / `heuristic` / `planned`), diagrama de sequência Mermaid do Estágio 2 real (pipeline → `spawn_blocking` → tokenizer → `ort::Session` → logits → sigmoid → reordenação → diversificação → U-shaped), diagrama do download multi-arquivo com verificação SHA-256, tabela de latência medida (p50/p95 para 30 candidatos).
- Nota de retificação de uma linha no topo das ADRs 123, 125, 127 e 129 apontando para a ADR-130.
- Atualizar a ADR-118 com nota de evolução (catálogo multi-arquivo + SHA-256 + teste `catalog_urls`).

---

## 5. 🧪 Estratégia de Testes Automatizados & Proteção contra Regressão

**`actx-onnx` (feature `runtime`)**
- `test_session_missing_file_inactive` — arquivo ausente ⇒ inativo, sem panic.
- `test_session_corrupt_file_inactive` — bytes inválidos ⇒ inativo, erro registrado, sem panic.
- `test_tokenizer_pair_encoding` — fixture tokenizer: estrutura do par, truncamento em `max_len`, `attention_mask` correta.
- `test_reranker_fixture_scores_in_unit_interval` — fixture ONNX: scores ∈ [0,1], tamanho = nº de candidatos.
- `test_reranker_fixture_ordering_deterministic` — mesma entrada ⇒ mesmos scores; ordenação estável.
- `test_reranker_batching_matches_single` — lote de 16 vs. pares individuais ⇒ mesmos scores (tolerância 1e-5).
- `test_gemma_engine_reports_unavailable` — `is_neural_active() == false` e `generate` retorna `Err` descritivo (substitui os testes atuais que assumem "arquivo dummy ⇒ ativo": `gemma.rs:241-242`, `reranker.rs:196`, `actx-lm/providers/gemma.rs:190-191`).
- `test_heuristic_components_report_heuristic` — classificador/sentinel/layout/visão nunca reportam `neural`.
- Build sem feature (`cargo test -p actx-onnx --no-default-features`) — tudo inativo, zero panics.

**`any-context-core-rs`**
- `test_two_stage_pipeline_reranking_fallback` (existente, `pipeline.rs:951`) — permanece verde.
- `test_two_stage_pipeline_with_fixture_reranker` — reranker fixture reordena candidatos e não bloqueia o runtime (`spawn_blocking`).
- `test_catalog_integrity` — todo modelo `Available` tem ≥ 1 arquivo, `size_bytes > 0`, `sha256` com 64 hex; Gemma `Planned`; sem `bge-reranker-small`.
- `test_download_sha256_mismatch_rejected` — servidor local de teste entrega bytes errados ⇒ erro, temporários removidos, destino intacto.
- `test_rerank_model_setting_roundtrip` — `/rerank model <id>` persiste e é lido pelo factory.
- `catalog_urls` (`#[ignore]`) — HEAD/GET de todos os arquivos `Available` **do Core e do Instalador** ⇒ 200 + tamanho exato. Executado no `release.yml`.

**`actx-installer`**
- `test_installer_catalog_matches_core` — para cada modelo presente nos dois catálogos, `url`, `size_bytes` e `sha256` são idênticos (guarda contra divergência até a unificação da Frente C).
- `test_installer_no_placeholder_urls` — nenhuma URL contém `anycontext/models` ou `google/gemma-4-*-it-onnx`; nenhum modelo `Planned` possui URL.
- Atualização dos testes de contagem de `target_models()` (`hardware.rs:548-552`).
- `real_reranker_multilingual` (`#[ignore]`, `ACTX_TEST_REAL_RERANKER=1`) — baixa (com cache) o mmarco; pergunta em PT com documento relevante e distratores (incluindo negação) ⇒ relevante no topo; mede p50/p95 de 30 candidatos e imprime para o TecDoc.

**`actx-cli`**
- `test_cli_flag_rerank_model_unknown` — id inválido ⇒ erro amigável, exit code ≠ 0.
- `test_cli_download_planned_model_message` — `actx download compact` ⇒ mensagem "planejado", sem rede.
- `test_cli_flag_rerank_off` (existente) — permanece verde.
- Teste de fast-path de inicialização (ADR-122) — permanece dentro do teto (sessão ONNX preguiçosa).

**NFRs**
- Cold start do `actx` sem regressão mensurável (biblioteca e sessão nunca carregadas no boot).
- **Imports do binário inalterados** em relação à v0.44.0: verificação automática no release de que `actx.exe` não importa `DirectML.dll`, `d3d12.dll`, `dxgi.dll`, `MSVCP140*.dll`.
- Runtime ausente ou incompatível ⇒ fallback RRF sem crash (teste com `ORT_DYLIB_PATH` apontando para arquivo inexistente/inválido).
- Aumento do pacote (biblioteca do ONNX Runtime) documentado no TecDoc.

---

## 6. 📝 Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)

**Cenário 45 (v0.45.0 — Runtime ONNX Real, Cross-Encoder Multilíngue mmarco e Retificação de Honestidade)** — inserido no topo, preservando integralmente os Cenários 1–44:
1. `actx -v` ⇒ `actx 0.45.0`.
2. `actx download --check` ⇒ `mmarco-reranker` (~119 MB, `Available`), `bge-reranker-v2-m3` (~571 MB, `Available`), Gemma 4 como `Planned`; nenhum `bge-reranker-small`.
3. `actx download reranker` ⇒ baixa 2 arquivos com barra agregada, valida SHA-256, grava em `%LOCALAPPDATA%/actx/models/mmarco-reranker/`.
4. TUI: badge `[Rerank: ⚡ ONNX: mmarco-L12 (CPU)]`; `/rerank` exibe modelo, dispositivo `CPU` e latência da última consulta.
5. Consulta em português num workspace com documentos parecidos (ex.: "timeout do serviço v2" vs. "v1 legado") com `--rerank on` e `--rerank off` ⇒ com rerank, o trecho correto sobe ao topo nas citações.
6. Renomear/corromper o `.onnx` baixado ⇒ badge `[Rerank: ⚡ RRF (Fallback)]`, sem crash; `actx download reranker` restaura.
7. `/rerank model bge-reranker-v2-m3` sem pesos ⇒ aviso com instrução de download; após download, badge muda para `bge-m3`.
8. `actx download compact` ⇒ mensagem "inferência generativa local planejada", sem tráfego de rede.
9. `/model gemma-4-compact` ⇒ aviso claro e uso do provedor de fallback; nenhuma resposta com "38.4 tok/s".
10. `actx --device dml "pergunta"` ⇒ aviso "Execution Providers de GPU planejados; executando em CPU".
11. `[F1]` → Document AI: classificador/layout/visão rotulados "Heurística".
12. Instalador: `actx-installer --preflight-only` e instalação do perfil `smart_balanced` ⇒ todos os modelos de Document AI e o `mmarco-reranker` baixam com sucesso (HTTP 200, SHA-256 válido) sem nenhuma ação extra do usuário; Gemma 4 aparece como "Planejado" e não é baixado.
13. Após instalar/atualizar, a pasta `%LOCALAPPDATA%\actx\bin\` contém `onnxruntime.dll` ao lado de `actx.exe`; na primeira consulta o badge mostra o reranker neural ativo sem nenhum comando de configuração.
14. Renomear `onnxruntime.dll` ⇒ `actx` abre normalmente e o reranker cai em `[Rerank: ⚡ RRF (Fallback)]` com motivo exibido em `/rerank`.

---

## 7. 🔒 Higiene de Segredos, Migração de Banco e Plano de Rollback

- **Segredos**: nenhum. Downloads do Hugging Face são anônimos; nenhuma chave/token adicionada. `verify_gate` antes do commit.
- **Migração**: apenas a chave aditiva `reranker_model` nas settings do SQLite (default `mmarco-reranker`). **Zero alterações de schema no LanceDB** (Frente C fará a migração de dimensão de embeddings).
- **Compatibilidade**: arquivos antigos `bge_reranker_small_int8.onnx` (impossíveis de obter, pois a URL não existe) são ignorados. Setting `rerank_mode` (auto/on/off) preservado.
- **Licenças**: mmarco e bge-reranker-v2-m3 Apache-2.0; ONNX Runtime MIT; `ort` MIT/Apache-2.0; `tokenizers` Apache-2.0 — registrar na ADR-130.
- **Rollback**: falha pós-merge ⇒ revert do PR em `main` e tag `v0.45.1`. Em runtime, `--rerank off` ou ausência de pesos devolve o comportamento RRF da v0.44.0 sem rebuild.

---

## 8. ⏱️ Passo a Passo da Execução

**M0 — Runtime base**: dependências opcionais `ort`/`tokenizers`, feature `runtime`/`onnx`, `ModelSession` real e preguiçosa, `tokenizer.rs`; validar compilação Windows MSVC + Linux local/CI.
**M1 — Catálogo verificado**: `OnnxModelSpec` multi-arquivo + `availability`; entradas mmarco e bge-m3 (coletar tamanho/SHA-256 do m3); Gemma `Planned`; downloader multi-arquivo com SHA-256 e subpasta; teste `catalog_urls`.
**M2 — Reranker real**: `OnnxReranker` com inferência em lote; `spawn_blocking` no pipeline; teto de candidatos; fixtures e testes.
**M3 — Retificação no código**: Gemma indisponível com fallback; remoção de telemetria fictícia; heurísticas rotuladas; dispositivo efetivo; correção das 5 URLs de Document AI e do reranker no instalador; remoção das 8 URLs fictícias do Gemma 4 (Core + Instalador).
**M4 — Superfícies**: `/rerank model`, `--rerank-model`, status `/rerank` com latência real, badges TUI, menus `[F1]`, mensagens de download `Planned`, aviso `--device`.
**M5 — Testes & NFR**: `cargo test --workspace` 100% verde; `--no-default-features` em `actx-onnx`; rodar `real_reranker_multilingual` localmente e registrar p50/p95.
**M6 — Dual-Doc**: README + ADR-130 + notas de retificação (ADR-118/123/125/127/129).
**M7 — Testes manuais**: Cenário 45 no topo de `tests/MANUAL_TESTS.md`.
**M8 — Release**: bump `0.45.0`; commit `feat(onnx): ...` em `dev`; PR `dev → main`; CI verde; merge; tag `v0.45.0`; acompanhar `release.yml` até publicação comprovada em `Levix-Digital/any-context-releases`; `check_milestone`; registro na `session-memory`; checklist final.
