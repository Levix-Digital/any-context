# 📐 Blueprint de Engenharia: Marco 2 (Fase 2) — ModelRouter na Ingestão, Quality Gate, Laya AI & Document AI Híbrido (v0.34.2)

## 📌 Metadados do Plano
- **Projeto**: AnyContext (`actx`)
- **Versão-Alvo**: `v0.34.2`
- **Componentes**: `crates/any-context-core-rs`, `crates/actx-lm`, `crates/actx-agent`, `crates/actx-cli`
- **Fase**: Fase 2 do Master Plan (RFC-042, ModelRouter, Laya AI & Document AI)
- **Status**: Planejamento Consolidado / Pronto para Aprovação de Execução
- **Referência**: Master Roadmap v0.30.38, ADR-110, Benchmark IKEAShipments, Laya AI (convaiinnovations/laya)

---

## 🏛️ 1. Contexto, Motivação e Lições Aprendidas

### 1.1 O Desafio Real
Na versão `v0.34.1`, o AnyContext conquistou velocidade recorde de compilação e Deep Search nativo 100% Rust. Contudo, em cenários corporativos reais (hospitais, logística, unidades militares, escritórios jurídicos e contabilidades):
1. **Poluição de Espaço Vetorial e BM25**: Chunks degenerados, código minificado esquecido (`.min.js`), dumps repetitivos de logs e licenças duplicadas consomem tokens e poluem buscas.
2. **Falha em Formulários e Tabelas Complexas (Caso IKEAShipments)**:
   - Em relatórios como o **`CMR for Single Pickup Report.pdf`**, a reconstrução espacial 2D ingênua fatiava o documento em dezenas de tabelas Markdown desconexas (`| --- | --- |`), separando rótulos dos valores preenchidos.
   - Cabeçalhos regulatórios repetitivos diluíam a representação vetorial cosseno e o BM25.
   - Páginas escaneadas geravam chunks vazios de *scanned notice*.
3. **Restrições de Hardware Corporativo (Gargalo de Vendas B2B)**:
   - A maioria dos clientes corporativos opera com notebooks padrão (**Intel Core i5/i7, 8GB a 16GB de RAM, vídeo integrado, sem GPU dedicada**).
   - O AnyContext **não pode** exigir GPUs dedicadas nem modelos pesados de 7B a 14B que congelem a máquina do cliente por 1 minuto por página.

### 1.2 Princípios Arquiteturais Invioláveis
1. **Zero Vocabulário Hardcoded (100% Agnóstico de Domínio)**:
   - Proibido fixar termos de nicho (`Consignee`, `Carrier`, `Paciente`, `Autor`, `Alíquota`).
   - A estruturação de caixas 2D é governada exclusivamente por **sintaxe de pontuação** (`:`, `-`, `___`), **contraste tipográfico** (negrito, caixa alta, hierarquia de tamanho de fonte) e **topologia espacial** (vizinhança Voronoi/Delaunay e retângulos vetoriais).
2. **Classificador de Alta Precisão sem Teto de 89%**:
   - Substituição do classificador ingênuo de embeddings pelo **Laya AI (`convaiinnovations/laya`)** quantizado em INT8 (~210MB, ~400MB RAM, ~60ms em CPU), atingindo **~95% de precisão de decisão** com classes 100% dinâmicas em tempo de execução.
   - Sentinela visual **MobileNetV4 RVL-CDIP** (~14MB, ~45MB RAM, ~8ms) para documentos 100% escaneados/rasterizados sem camada de texto.
3. **Visão Computacional Flexível (Local Leve vs. VPC Corporativa)**:
   - **Opção Local Ultraleve**: SLMs compactos de visão (ex: **Moondream2 1.8B INT4** ou **SmolVLM-500M**), consumindo ~1.5GB de RAM e ~2s na CPU.
   - **Opção Cloud / VPC do Cliente**: Suporte nativo via `actx-lm` para instâncias privadas de LLM de visão na nuvem corporativa do cliente (Azure OpenAI, AWS Bedrock, vLLM on-premise), consumindo 0 MB de RAM local.
   - **Fallback Gracioso Padrão**: Se sem visão, opera com Heurística Espacial 2D em Rust puro (< 5MB RAM, zero download), sem quebrar a indexação.

---

## 🏗️ 2. Topologia do Pipeline de Ingestão em 3 Níveis

```mermaid
flowchart TD
    DOC["Documento Ingerido (PDF / Imagem / Office / Código)"] --> QG_FAST{"Quality Gate Inicial:<br/>Entropia, Linhas Minificadas,<br/>Fragmentos Sintáticos Degenerados"}
    
    QG_FAST -->|Lixo / Minified / Boilerplate| DROP["Descarte Preventivo<br/>(chunks_dropped_quality += 1)"]
    QG_FAST -->|Código / Markdown Puro| N1["Nível 1: Chunker AST / Markdown Direto"]
    QG_FAST -->|PDF / Imagem / Documento Amíguo| CLASSIFIER{"IngestionModelRouter:<br/>Classificador de Tipologia"}

    subgraph "Classificação de Alta Precisão (~95%)"
        CLASSIFIER -->|Possui Texto Digital (90% dos casos)| LAYA["Laya AI (mmBERT INT8 ONNX)<br/>• 210 MB / 400 MB RAM / 60 ms CPU<br/>• Decisão Dinâmica em Runtime"]
        CLASSIFIER -->|Página Escaneada / Sem Texto| MOBILENET["MobileNetV4 RVL-CDIP ONNX<br/>• 14 MB / 45 MB RAM / 8 ms CPU<br/>• Thumbnail Visual 224x224"]
    end

    LAYA & MOBILENET --> ROTA{"Decisão do Tipo de Documento"}

    ROTA -->|Artigo / Livro / Prosa Corrida| N1
    ROTA -->|Formulário / Fatura / Relatório Denso| HEUR_2D["Heurística Espacial 2D Agnóstica (Rust Puro)<br/>• Pontuação Sintática (':', '-')<br/>• Contraste Tipográfico (Bold, Font Size)<br/>• Caixas e Retângulos Vetoriais<br/>• 0 MB download / < 5 MB RAM / 3 ms"]
    ROTA -->|Diagrama Técnico / Esquema Gráfico| VISION_HOOK{"Modo de Visão Configurado?"}

    subgraph "Nível 3: Visão Computacional (Diagramas e Scans Complexos)"
        VISION_HOOK -->|1. Local Ultraleve| MOONDREAM["SLM Local: Moondream2 (1.8B INT4)<br/>• 1 GB download / 1.5 GB RAM / ~2 s CPU"]
        VISION_HOOK -->|2. VPC Corporativa / Cloud| VPC_VISION["actx-lm Vision Endpoint (VPC do Cliente)<br/>• Azure OpenAI / AWS Bedrock / vLLM Privado<br/>• 0 MB RAM local / Alta Fidelidade"]
        VISION_HOOK -->|3. Desativado / Offline Base| HEUR_FALLBACK["Fallback Estruturado Nativo (Metadados 2D)"]
    end

    HEUR_2D --> ENVELOPE["NativeContextualEnricher"]
    MOONDREAM --> ENVELOPE
    VPC_VISION --> ENVELOPE
    HEUR_FALLBACK --> ENVELOPE
    N1 --> ENVELOPE

    ENVELOPE --> EMBED["Batch Embeddings (LanceDB + BM25)"]
```

---

## 🧩 3. Módulos e Componentes em Rust

### 3.1 `quality_gate.rs` (`crates/any-context-core-rs/src/ingestion/quality_gate.rs`)
- Validações sub-0.1ms:
  - Entropia de Shannon: $1.5 \le H \le 5.95$.
  - Densidade lexical: proporção de vocabulário único vs. repetitivo.
  - Detecção de linhas minificadas ($> 2000$ caracteres contínuos).
  - Bloqueio de fragmentos sintáticos degenerados (`}`, `};`, imports isolados).

### 3.2 `spatial_form_chunker.rs` (`crates/any-context-core-rs/src/ingestion/chunkers/spatial_form.rs`)
- Algoritmo de extração 2D **100% agnóstico de domínio**:
  - Identificação de rótulos por pontuação terminal (`:`, `-`, `___`) ou quebra de célula.
  - Agrupamento de pares chave-valor por proximidade horizontal ($\Delta y \approx 0, \Delta x > 0$) e vertical ($\Delta x \approx 0, \Delta y > 0$).
  - Respeito às caixas retangulares vetoriais desenhadas no PDF.
  - Emite chunks com estrutura densa chave-valor:
    ```markdown
    // Context: documento.pdf > Página 2 [Bloco Estruturado]
    - Rótulo A: Valor A
    - Rótulo B: Valor B
    - Tabela de Itens: [Linha 1: Coluna 1 | Coluna 2 ...]
    ```

### 3.3 `model_router.rs` (`crates/any-context-core-rs/src/ingestion/model_router.rs`)
- Integração do **Laya AI** (via crate `ort` ONNX Runtime):
  - Consulta dinâmica estruturada em tempo de execução:
    `options: ["formulario_ou_tabela", "artigo_prosa", "codigo_fonte", "diagrama_visual"]`
  - Calibração probabilística rigorosa (ECE 0.081).
- Integração do **MobileNetV4 RVL-CDIP** como fallback para páginas escaneadas.
- Dispatcher do Nível 3 para Visão Computacional:
  - `VisionExecutionMode::LocalSlm` (Moondream2 / SmolVLM via ONNX / llama.cpp).
  - `VisionExecutionMode::CorporateVpc` (endpoint OpenAI-compatível apontado para a VPC do cliente via `actx-lm`).
  - `VisionExecutionMode::Disabled` (recaída na heurística nativa 2D).

### 3.4 `orchestrator.rs` (`crates/any-context-core-rs/src/ingestion/orchestrator.rs`)
- `SyncResult` expandido com telemetria:
  - `chunks_created`: Total de chunks úteis.
  - `chunks_dropped_quality`: Chunks de ruído descartados.
  - `documents_classified_laya`: Documentos processados pelo Laya AI.
  - `documents_enriched_vision`: Documentos enriquecidos por visão (local ou VPC).

---

## 🧪 4. Critérios de Aceitação e Testes

1. **Testes Unitários de Quality Gate (`quality_gate_tests.rs`)**:
   - Rejeição de baixa entropia (`--------`), código minificado e fragmentos sintáticos soltos.
2. **Teste Agnóstico de Heurística Espacial (`spatial_form_tests.rs`)**:
   - Validação com 3 layouts de nichos diferentes (um laudo médico, um documento fiscal CMR e uma petição judicial) comprovando extração correta de pares chave-valor sem nenhuma palavra específica hardcoded.
3. **Teste do Benchmark Real `IKEAShipments` (`ikea_shipments_benchmark_tests.rs`)**:
   - Execução sobre a página 2 do `CMR for Single Pickup Report.pdf`.
   - Recuperação das informações de *Sender*, *Consignee*, *Carrier*, *Packages* e *Gross Weight* com similaridade vetorial superior a $0.80$ no RAG.
4. **Teste de Tolerância a Falhas / Modo Offline**:
   - Desconexão de rede simulada: a indexação deve concluir 100% verde sem travamentos, recaindo no modo estruturado nativo.

---

## 📝 5. Atualizações Dual-Doc

1. **Documentação Técnica ([`TECDOC.md`](file:///C:/Users/guilh/source/repos/any-context/TECDOC.md))**:
   - **ADR-110**: Ingestion ModelRouter, Laya AI Decision Engine, Heurística Espacial 2D Agnóstica e Arquitetura Híbrida de Visão (Local SLM vs. VPC Corporativa).
2. **Suíte Manual ([`tests/MANUAL_TESTS.md`](file:///C:/Users/guilh/source/repos/any-context/tests/MANUAL_TESTS.md))**:
   - **Cenário 25**: Validação do Quality Gate, Triagem pelo Laya AI e Ingestão de Formulários Complexos no AnyContext `v0.34.2`.

---

## 📋 6. Marcos de Ação (Checklist)

- [ ] **M1**: Implementar `quality_gate.rs` (filtros determinísticos de sub-0.1ms).
- [ ] **M2**: Implementar `spatial_form_chunker.rs` (heurística espacial 2D agnóstica de domínio).
- [ ] **M3**: Implementar `model_router.rs` com suporte ao Laya AI (ONNX) e MobileNetV4.
- [ ] **M4**: Integrar hooks de Visão (Moondream2 local vs. VPC corporativa via `actx-lm`).
- [ ] **M5**: Integrar no `NativeSyncOrchestrator` e expandir `SyncResult`.
- [ ] **M6**: Validar suíte completa de testes (100% PASS), incluindo benchmark `IKEAShipments`.
- [ ] **M7**: Atualizar Dual-Doc (ADR-110 no `TECDOC.md` e Cenário 25 no `MANUAL_TESTS.md`).
- [ ] **M8**: Bump para `v0.34.2`, verificação Gate 8 e commit/release.
