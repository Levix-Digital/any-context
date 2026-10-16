# 🗺️ Roadmap pós-v0.45.0: Próximas Frentes de Desenvolvimento

- **Data**: 2026-10-16
- **Estado de partida**: `v0.45.0` publicada (tag `v0.45.0` → `1daebe4`, merge do PR #326 em `main`; correções da revisão no PR #327).
- **Objetivo deste documento**: handoff entre sessões. Resume o que está entregue, o que vem em cada frente, onde estão os pontos de código relevantes e quais convenções seguir. **Não é um blueprint**: cada frente ganha o seu próprio blueprint em `docs/plans/` antes da implementação.
- **Fontes**: ADR-130 (`TECDOC.md`, incluindo 130.10), `docs/plans/anycontext_blueprint_v0450_frente_c0_runtime_onnx_real_e_reranker_cross_encoder.md` (roadmap revisado, D14 e "Fora de escopo"), `docs/plans/anycontext_master_roadmap_v0_30_38_rfc042.md` (Fase 3 "Kit de Models").

> Itens marcados **(a verificar)** não foram confirmados no código ou em fonte externa ao escrever este documento. Confirme antes de entrar num blueprint (regra da ADR-118: nada de catálogo ou link sem verificação ao vivo).

---

## 1. Onde estamos

| Frente | Versão | Estado | Entrega |
|---|---|---|---|
| A | v0.43.0 | ✅ | RAG determinístico 1-hop, U-shaped reordering, isolamento XML, Zero Hallucination Gate |
| B | v0.44.0 | ✅ (concluída de fato pela C0) | Pipeline Two-Stage, `/rerank`, `--rerank`, badges |
| C0 | v0.45.0 | ✅ publicada | Runtime ONNX real (`ort` load-dynamic + `tokenizers`), cross-encoder `mmarco-reranker` real em CPU, catálogo verificado (SHA-256), retificação de honestidade (Gemma 4, GPU e heurísticas marcadas como planejadas) |
| **C** | **v0.46.0** | 🔜 próxima | Embeddings locais ONNX + Kit de Models |
| **D** | **v0.47.0** | planejada | Chunking hierárquico / Parent-Document / Expansão AST |
| Sem versão | — | backlog | Gemma 4 generativo local, GPU EPs, heurísticas → neural, macOS, limpezas |

Base técnica que a C0 deixou pronta para reuso: `actx_onnx::runtime::OnnxSession` (sessão preguiçosa, carga dinâmica, sem panic), `actx_onnx::tokenizer` (HF `tokenizers`), downloader multi-arquivo com SHA-256 (`OnnxModelManager::download_model_with_progress`), provisionamento no instalador (`crates/actx-installer/src/provision.rs`) e o padrão de fallback gracioso da ADR-110.

---

## 2. Frente C — v0.46.0: Embeddings Locais ONNX + Kit de Models

### 2.1 Problema
1. **A busca densa depende de um provedor externo de embeddings.** A ingestão resolve o modelo por workspace (`crates/any-context-core-rs/src/ingestion/orchestrator.rs:198`, `resolve_embedding_provider`), mas a consulta usa um modelo fixo:
   - `crates/any-context-core-rs/src/retrieval/pipeline.rs:536` → `lm.embed("text-embedding-3-small", …)`.
   - **Bug**: se o workspace foi indexado com outro modelo de embedding, o vetor da consulta fica em outro espaço (ou com outra dimensão) e a busca densa degrada ou falha em silêncio. Corrigir na Frente C, ou antes como patch `v0.45.1`.
2. **Dimensão fixa no LanceDB**: `crates/any-context-core-rs/src/storage/lancedb.rs:19` → `DEFAULT_VECTOR_DIM = 1536`. Embeddings locais compactos têm outra dimensão (tipicamente 384 ou 768).
3. **Dois catálogos de modelos** (Core `model_manager.rs` e Instalador `hardware.rs`) duplicados, protegidos só por teste de paridade.
4. **Perfis de hardware** calibrados para o Gemma 4 (RAM), que não roda localmente. O disco já foi corrigido no PR #327 (soma dos pesos realmente instalados), mas a RAM e a semântica dos tiers não.

### 2.2 Objetivos
1. **`OnnxEmbedder`** em `crates/actx-onnx` (novo `embedder.rs`), reutilizando `OnnxSession` + `tokenizer`: mean pooling + normalização L2, lotes, `spawn_blocking`, sessão preguiçosa e `is_neural_active()` honesto (mesma semântica do reranker, incluindo `recorded_load_error`).
2. **Provider de embedding local** no `actx-lm` (ou porta no Core) para a ingestão e a consulta usarem o mesmo modelo por workspace.
3. **Consulta e ingestão no mesmo modelo**: gravar o modelo e a dimensão usados por workspace/tabela (settings SQLite) e usar esse modelo em `pipeline.rs:536` em vez do valor fixo.
4. **Migração de dimensão no LanceDB**: dimensão por tabela; detectar divergência (modelo ou dimensão) e oferecer reindexação assistida (`/reindex` ou fluxo de sync), sem perda dos dados de origem. Esta é a primeira alteração de schema desde a C0, portanto precisa de plano de migração e rollback.
5. **Modelo padrão multilíngue**, coerente com a política global de Calgary. Candidatos **(a verificar)**: `intfloat/multilingual-e5-small` (384 dims, exige prefixos `query:`/`passage:`), `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` (384 dims), `BAAI/bge-m3` (1024 dims, ~2 GB, só para perfis grandes). Antes de escolher: confirmar a existência do ONNX oficial, a licença comercial, o tamanho exato, o SHA-256 e a revisão fixa, e medir p50/p95 em CPU.
6. **Kit de Models** (Fase 3 do master roadmap), com presets e auto-detecção:
   - **Offline / Edge**: LLM local (Ollama / LM Studio), embedding ONNX local, reranker ONNX local.
   - **Híbrido**: RAG e pré-processamento locais; síntese via API.
   - **Frontier**: modelos de API de ponta para síntese.
   - **Privacidade corporativa**: só endpoints privados (vLLM / Azure OpenAI).
   - Os nomes de modelos de API no master roadmap são de 2025 e precisam ser atualizados **(a verificar)**.
7. **Itens que a C0 deixou explicitamente para a Frente C**:
   - Recalibrar a RAM dos perfis para o que realmente roda (embedding + reranker + Document AI heurístico).
   - Criar `actx profile install` (instalar ou trocar o perfil depois da instalação, reaproveitando `provision.rs`).
   - Unificar os catálogos Core/Instalador numa fonte única (por exemplo, um crate ou módulo compartilhado consumido pelos dois), eliminando a duplicação.

### 2.3 Superfícies
- `/embed` (status: modelo, dimensão, runtime, latência) e `/embed model <id>`, espelhando `/rerank`.
- Badge na TUI e flag `--embed-model`.
- Mensagem clara quando o workspace precisa de reindexação.
- `actx download embedder` como ferramenta de reparo, no mesmo padrão do reranker.

### 2.4 Não-regressão
- ADR-110: sem pesos ou sem runtime ⇒ cai no provedor de embedding atual (API) ou só BM25, sem quebrar.
- Cold start preservado (ADR-122): nada carrega na inicialização.
- Workspaces existentes continuam consultáveis até a reindexação.
- Teste ao vivo do catálogo (`catalog_urls`) cobrindo os novos arquivos.

### 2.5 Documentação e release
- Próxima ADR: **ADR-131**.
- Cenário de teste manual: **Cenário 46** no topo de `tests/MANUAL_TESTS.md`.

---

## 3. Frente D — v0.47.0: Chunking Hierárquico / Parent-Document / Expansão AST

Definida no roadmap revisado da C0 apenas pelo título. O conteúdo abaixo é a proposta inicial para o blueprint.

### 3.1 Ideia
- **Recuperar fino, entregar contexto**: indexar chunks pequenos (melhor precisão no embedding e no cross-encoder) e, na entrega ao LLM, expandir para a unidade-pai (seção do documento, função ou classe inteira, tabela inteira).
- **Expansão AST**: o `ASTCodeChunker` (tree-sitter: Python, TS/JS, Java, Kotlin, Swift, Ruby, PHP, Lua, Dart…) já gera breadcrumbs `Context: file > class > def [lines X-Y]`. Usar essa hierarquia para subir de um trecho para a função ou classe que o contém, ou trazer as funções irmãs referenciadas.
- **Documentos e dados estruturados**: o `StructuredDataChunker` (Marco 3, v0.30.7) já preserva a estrutura 2D. Pai = tabela ou seção; filho = linha ou bloco.

### 3.2 Pontos de atenção
- **Schema**: o chunk precisa de `parent_id` e de nível. Combinar com a migração de schema da Frente C para não migrar o LanceDB duas vezes.
- **Orçamento de contexto**: a expansão deve respeitar o `apply_density_budget` existente e a reordenação em U (Frente A).
- **Reranking**: decidir se o cross-encoder pontua o filho (mais preciso) ou o pai, e depois deduplicar pais repetidos.
- **Proveniência**: as citações (`[arquivo:linhas]`) devem apontar para o trecho do pai efetivamente entregue.

---

## 4. Frentes sem versão definida (backlog técnico)

1. **Gemma 4 generativo local** (fatos verificados em D14 do blueprint da C0, e ADR-130 §130.4):
   - A família real publicada é E2B, E4B, 12B, 26B-A4B (MoE) e 31B. Os tiers atuais do catálogo (`compact`, `27b`, `32b`) não existem.
   - Há ONNX oficial (onnx-community) só para E2B e E4B, em formato multi-grafo e multi-shard (`.onnx_data_*`); o E2B q4 passa de 3,6 GB.
   - Exige downloader multi-shard, loop generativo (tokenização, KV-cache, amostragem) e recalibração dos tiers.
   - Hoje está `Planned` e sempre usa o provedor de fallback.
2. **Execution Providers de GPU** (DirectML no Windows, CUDA no Linux): hoje só CPU. Exige o pacote ONNX Runtime correspondente e cuidado com as dependências de DLL (ver D1 da C0: a linkagem estática com DirectML impedia o `actx.exe` de abrir).
3. **Heurísticas → neural**: o classificador de consultas (`query_classifier.rs`), o scan sentinel, o layout extractor e o vision encoder rodam heurísticas determinísticas, rotuladas como tal. Os pesos de Document AI já são baixados pelos perfis; falta a inferência real.
4. **macOS**: não há binário (o release só gera Linux x86_64 e Windows x86_64).
5. **Limpezas**:
   - Trechos legados do `README.md` sobre ChromaDB/Python.
   - O plano `implementation_plan_subscription_tiers.md` (tiers de assinatura / `BillingManager`).
   - O `BLUEPRINT_PRIORITY_4_CLI_TUI.md` **(a verificar o que ainda está pendente)**.
   - O estado da RFC-042 (Deep Search reflexivo em 5 fases) **(a verificar: partes como `retrieve_hybrid_batch`, `/fast` e `/deep` já existem)**.

---

## 5. Convenções do projeto (seguir em toda frente)

- **Blueprint primeiro**: `docs/plans/anycontext_blueprint_v0XY0_<tema>.md` com metadados, discovery com referências `arquivo:linha`, decisões numeradas (D1…), fora de escopo explícito, testes, migração, rollback e passo a passo por marcos (M0…M8).
- **Arquitetura hexagonal**: regra de negócio no Core (`crates/any-context-core-rs`), inferência em `crates/actx-onnx`, CLI/TUI só como adaptadores.
- **ADR-110**: nenhuma falha de modelo ou runtime pode quebrar o produto; sempre há fallback silencioso e honesto.
- **Honestidade técnica (ADR-130)**: nada é rotulado como neural sem inferência real; nenhuma telemetria fictícia; nenhum link de catálogo sem verificação ao vivo (ADR-118) e SHA-256.
- **Política global (Calgary)**: sem regras fiscais ou regionais; modelos padrão multilíngues.
- **Dual-Doc**: `README.md` (usuário) + ADR no `TECDOC.md` (técnico); prepend do cenário em `tests/MANUAL_TESTS.md` sem truncar os anteriores.
- **Release**:
  1. Bump de versão em todos os crates e no `version.txt`.
  2. Commit em `dev` e PR `dev → main`; o CI (`e2e-tests.yml`) roda no PR para `main` e no push em `dev`.
  3. Merge e depois a **tag `vX.Y.Z` sobre o commit de merge em `main`**, que dispara `release.yml` e publica em `Levix-Digital/any-context-releases`.
- **Testes locais**:
  - Precisam do `protoc` e do ONNX Runtime: `bash scripts/fetch_onnxruntime.sh .ort`, depois `ORT_DYLIB_PATH=.ort/libonnxruntime.so` (ou `onnxruntime.dll`) e `ACTX_REQUIRE_ORT=1 cargo test --workspace`.
  - O teste `test_router_sub_microsecond_performance` mede tempo e pode falhar se rodar em paralelo com uma compilação pesada.

### Limitações observadas em sessões na nuvem (Claude Code)
- A sessão só consegue fazer push na própria branch `claude/*`. Push direto em `dev` e criação de tags são bloqueados (modo Auto) ou recusados pelo proxy git (tags, HTTP 403). Fluxo que funciona: push na branch da sessão → PR para `dev` → merge pelo mantenedor → tag criada pelo mantenedor.
- O workflow de testes não roda em PRs para `dev`; para validar uma branch antes do merge, use `workflow_dispatch` do `e2e-tests.yml` nela.

---

## 6. Prompt sugerido para iniciar a próxima sessão

> Leia `docs/plans/anycontext_roadmap_pos_v0450_proximas_frentes.md` e a ADR-130 no `TECDOC.md`. Vamos iniciar a **Frente C (v0.46.0): embeddings locais ONNX + Kit de Models**. Primeiro faça o discovery: confirme os pontos de código citados (`pipeline.rs:536`, `lancedb.rs:19`, `orchestrator.rs:198`) e verifique ao vivo os candidatos a modelo de embedding (existência do ONNX, licença, tamanho, SHA-256, revisão fixa). Depois escreva o blueprint `docs/plans/anycontext_blueprint_v0460_...md` seguindo o formato do blueprint da v0.45.0, antes de qualquer código.
