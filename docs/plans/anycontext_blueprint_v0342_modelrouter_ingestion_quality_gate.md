---
plan_id: anycontext_blueprint_v0342_modelrouter_ingestion_quality_gate
project: any-context
title: 'Milestone 2 Blueprint (Phase 2): ModelRouter, Quality Gate, Laya AI & Hybrid Document AI (v0.34.2)'
version: v0.34.2
status: superseded
author: Levi Silveira
created_at: '2026-10-10T06:51:50'
updated_at: '2026-10-24T14:16:58'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- ingestion
- model_router
- quality_gate
- laya_ai
- document_ai
- moondream2
- ikeashipments
- rust
- vpc
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: quality_gate.rs Module'
  done: false
- name: 'M2: Agnostic spatial_form_chunker.rs Module'
  done: false
- name: 'M3: model_router.rs Module with Laya AI and MobileNet'
  done: false
- name: 'M4: Local vs VPC Hybrid Vision Hooks'
  done: false
- name: 'M5: NativeSyncOrchestrator Integration'
  done: false
- name: 'M6: Test Suite and IKEAShipments Benchmark'
  done: false
- name: 'M7: Dual-Doc TECDOC and MANUAL_TESTS'
  done: false
- name: 'M8: Gate 8 and Release v0.34.2'
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
- at: '2026-10-24T14:16:58'
  by: Levi Silveira
  status: superseded
  note: Superseded by the architecture v2 redesign and roadmap v2 (docs/ROADMAP.md, docs/plans/anycontext_roadmap_v2.md, decided 2026-10-23); the remaining work was rebuilt as backlog items.
---

# 📐 Engineering Blueprint: Milestone 2 (Phase 2) — ModelRouter in Ingestion, Quality Gate, Laya AI & Hybrid Document AI (v0.34.2)

## 📌 Plan Metadata
- **Project**: AnyContext (`actx`)
- **Target Version**: `v0.34.2`
- **Components**: `crates/any-context-core-rs`, `crates/actx-lm`, `crates/actx-agent`, `crates/actx-cli`
- **Phase**: Master Plan Phase 2 (RFC-042, ModelRouter, Laya AI & Document AI)
- **Status**: Consolidated Planning / Ready for Execution Approval
- **Reference**: Master Roadmap v0.30.38, ADR-110, IKEAShipments Benchmark, Laya AI (convaiinnovations/laya)

---

## 🏛️ 1. Context, Motivation and Lessons Learned

### 1.1 The Real Challenge
In version `v0.34.1`, AnyContext achieved record build speed and 100% Rust native Deep Search. However, in real corporate scenarios (hospitals, logistics, military units, law and accounting firms):
1. **Vector Space and BM25 Pollution**: degenerate chunks, forgotten minified code (`.min.js`), repetitive log dumps and duplicated licenses consume tokens and pollute searches.
2. **Failure on Complex Forms and Tables (IKEAShipments Case)**:
   - In reports such as the **`CMR for Single Pickup Report.pdf`**, the naive 2D spatial reconstruction sliced the document into dozens of disconnected Markdown tables (`| --- | --- |`), separating labels from the filled-in values.
   - Repetitive regulatory headers diluted the cosine vector representation and BM25.
   - Scanned pages produced empty *scanned notice* chunks.
3. **Corporate Hardware Constraints (a B2B Sales Bottleneck)**:
   - Most corporate customers run standard laptops (**Intel Core i5/i7, 8GB to 16GB of RAM, integrated graphics, no dedicated GPU**).
   - AnyContext **cannot** require dedicated GPUs or heavy 7B to 14B models that freeze the customer's machine for 1 minute per page.

### 1.2 Inviolable Architectural Principles
1. **Zero Hardcoded Vocabulary (100% Domain-Agnostic)**:
   - Hardcoding niche terms (`Consignee`, `Carrier`, `Patient`, `Plaintiff`, `Tax Rate`) is forbidden.
   - 2D box structuring is governed exclusively by **punctuation syntax** (`:`, `-`, `___`), **typographic contrast** (bold, capitals, font size hierarchy) and **spatial topology** (Voronoi/Delaunay neighborhood and vector rectangles).
2. **High-Precision Classifier without an 89% Ceiling**:
   - Replacement of the naive embedding classifier by **Laya AI (`convaiinnovations/laya`)** quantized to INT8 (~210MB, ~400MB RAM, ~60ms on CPU), reaching **~95% decision accuracy** with 100% dynamic classes at runtime.
   - **MobileNetV4 RVL-CDIP** visual sentinel (~14MB, ~45MB RAM, ~8ms) for 100% scanned/rasterized documents without a text layer.
3. **Flexible Computer Vision (Light Local vs. Corporate VPC)**:
   - **Ultralight Local Option**: compact vision SLMs (e.g. **Moondream2 1.8B INT4** or **SmolVLM-500M**), using ~1.5GB of RAM and ~2s on the CPU.
   - **Cloud / Customer VPC Option**: native support through `actx-lm` for private vision LLM instances in the customer's corporate cloud (Azure OpenAI, AWS Bedrock, on-premise vLLM), using 0 MB of local RAM.
   - **Default Graceful Fallback**: without vision, it runs with the 2D Spatial Heuristic in pure Rust (< 5MB RAM, zero download), without breaking indexing.

---

## 🏗️ 2. Topology of the 3-Level Ingestion Pipeline

```mermaid
flowchart TD
    DOC["Ingested Document (PDF / Image / Office / Code)"] --> QG_FAST{"Initial Quality Gate:<br/>Entropy, Minified Lines,<br/>Degenerate Syntactic Fragments"}
    
    QG_FAST -->|Garbage / Minified / Boilerplate| DROP["Preventive Discard<br/>(chunks_dropped_quality += 1)"]
    QG_FAST -->|Pure Code / Markdown| N1["Level 1: Direct AST / Markdown Chunker"]
    QG_FAST -->|PDF / Image / Ambiguous Document| CLASSIFIER{"IngestionModelRouter:<br/>Typology Classifier"}

    subgraph "High-Precision Classification (~95%)"
        CLASSIFIER -->|Has Digital Text (90% of cases)| LAYA["Laya AI (mmBERT INT8 ONNX)<br/>• 210 MB / 400 MB RAM / 60 ms CPU<br/>• Dynamic Decision at Runtime"]
        CLASSIFIER -->|Scanned Page / No Text| MOBILENET["MobileNetV4 RVL-CDIP ONNX<br/>• 14 MB / 45 MB RAM / 8 ms CPU<br/>• 224x224 Visual Thumbnail"]
    end

    LAYA & MOBILENET --> ROTA{"Document Type Decision"}

    ROTA -->|Article / Book / Running Prose| N1
    ROTA -->|Form / Invoice / Dense Report| HEUR_2D["Agnostic 2D Spatial Heuristic (Pure Rust)<br/>• Syntactic Punctuation (':', '-')<br/>• Typographic Contrast (Bold, Font Size)<br/>• Vector Boxes and Rectangles<br/>• 0 MB download / < 5 MB RAM / 3 ms"]
    ROTA -->|Technical Diagram / Graphic Schematic| VISION_HOOK{"Vision Mode Configured?"}

    subgraph "Level 3: Computer Vision (Diagrams and Complex Scans)"
        VISION_HOOK -->|1. Ultralight Local| MOONDREAM["Local SLM: Moondream2 (1.8B INT4)<br/>• 1 GB download / 1.5 GB RAM / ~2 s CPU"]
        VISION_HOOK -->|2. Corporate VPC / Cloud| VPC_VISION["actx-lm Vision Endpoint (Customer VPC)<br/>• Azure OpenAI / AWS Bedrock / Private vLLM<br/>• 0 MB local RAM / High Fidelity"]
        VISION_HOOK -->|3. Disabled / Offline Base| HEUR_FALLBACK["Native Structured Fallback (2D Metadata)"]
    end

    HEUR_2D --> ENVELOPE["NativeContextualEnricher"]
    MOONDREAM --> ENVELOPE
    VPC_VISION --> ENVELOPE
    HEUR_FALLBACK --> ENVELOPE
    N1 --> ENVELOPE

    ENVELOPE --> EMBED["Batch Embeddings (LanceDB + BM25)"]
```

---

## 🧩 3. Rust Modules and Components

### 3.1 `quality_gate.rs` (`crates/any-context-core-rs/src/ingestion/quality_gate.rs`)
- Sub-0.1ms validations:
  - Shannon entropy: $1.5 \le H \le 5.95$.
  - Lexical density: proportion of unique vs. repetitive vocabulary.
  - Detection of minified lines ($> 2000$ continuous characters).
  - Blocking of degenerate syntactic fragments (`}`, `};`, isolated imports).

### 3.2 `spatial_form_chunker.rs` (`crates/any-context-core-rs/src/ingestion/chunkers/spatial_form.rs`)
- **100% domain-agnostic** 2D extraction algorithm:
  - Identification of labels by terminal punctuation (`:`, `-`, `___`) or a cell break.
  - Grouping of key-value pairs by horizontal ($\Delta y \approx 0, \Delta x > 0$) and vertical ($\Delta x \approx 0, \Delta y > 0$) proximity.
  - Respect for the vector rectangle boxes drawn in the PDF.
  - Emits chunks with a dense key-value structure:
    ```markdown
    // Context: document.pdf > Page 2 [Structured Block]
    - Label A: Value A
    - Label B: Value B
    - Item Table: [Row 1: Column 1 | Column 2 ...]
    ```

### 3.3 `model_router.rs` (`crates/any-context-core-rs/src/ingestion/model_router.rs`)
- **Laya AI** integration (through the `ort` ONNX Runtime crate):
  - Structured dynamic query at runtime:
    `options: ["formulario_ou_tabela", "artigo_prosa", "codigo_fonte", "diagrama_visual"]`
  - Rigorous probabilistic calibration (ECE 0.081).
- **MobileNetV4 RVL-CDIP** integration as the fallback for scanned pages.
- Level 3 dispatcher for Computer Vision:
  - `VisionExecutionMode::LocalSlm` (Moondream2 / SmolVLM through ONNX / llama.cpp).
  - `VisionExecutionMode::CorporateVpc` (an OpenAI-compatible endpoint pointed at the customer's VPC through `actx-lm`).
  - `VisionExecutionMode::Disabled` (falling back to the native 2D heuristic).

### 3.4 `orchestrator.rs` (`crates/any-context-core-rs/src/ingestion/orchestrator.rs`)
- `SyncResult` extended with telemetry:
  - `chunks_created`: total useful chunks.
  - `chunks_dropped_quality`: discarded noise chunks.
  - `documents_classified_laya`: documents processed by Laya AI.
  - `documents_enriched_vision`: documents enriched by vision (local or VPC).

---

## 🧪 4. Acceptance Criteria and Tests

1. **Quality Gate Unit Tests (`quality_gate_tests.rs`)**:
   - Rejection of low entropy (`--------`), minified code and loose syntactic fragments.
2. **Agnostic Spatial Heuristic Test (`spatial_form_tests.rs`)**:
   - Validation with 3 layouts from different niches (a medical report, a CMR shipping document and a court petition), proving correct extraction of key-value pairs with no specific word hardcoded.
3. **Real `IKEAShipments` Benchmark Test (`ikea_shipments_benchmark_tests.rs`)**:
   - Run on page 2 of `CMR for Single Pickup Report.pdf`.
   - Retrieval of the *Sender*, *Consignee*, *Carrier*, *Packages* and *Gross Weight* information with vector similarity above $0.80$ in RAG.
4. **Fault Tolerance / Offline Mode Test**:
   - Simulated network disconnection: indexing must finish 100% green without hanging, falling back to the native structured mode.

---

## 📝 5. Dual-Doc Updates

1. **Technical Documentation ([`TECDOC.md`](file:///C:/Users/guilh/source/repos/any-context/TECDOC.md))**:
   - **ADR-110**: Ingestion ModelRouter, Laya AI Decision Engine, Agnostic 2D Spatial Heuristic and Hybrid Vision Architecture (Local SLM vs. Corporate VPC).
2. **Manual Suite ([`tests/MANUAL_TESTS.md`](file:///C:/Users/guilh/source/repos/any-context/tests/MANUAL_TESTS.md))**:
   - **Scenario 25**: Validation of the Quality Gate, Laya AI Triage and Ingestion of Complex Forms in AnyContext `v0.34.2`.

---

## 📋 6. Action Milestones (Checklist)

- [ ] **M1**: Implement `quality_gate.rs` (sub-0.1ms deterministic filters).
- [ ] **M2**: Implement `spatial_form_chunker.rs` (domain-agnostic 2D spatial heuristic).
- [ ] **M3**: Implement `model_router.rs` with support for Laya AI (ONNX) and MobileNetV4.
- [ ] **M4**: Integrate Vision hooks (local Moondream2 vs. corporate VPC through `actx-lm`).
- [ ] **M5**: Integrate into `NativeSyncOrchestrator` and extend `SyncResult`.
- [ ] **M6**: Validate the full test suite (100% PASS), including the `IKEAShipments` benchmark.
- [ ] **M7**: Update Dual-Doc (ADR-110 in `TECDOC.md` and Scenario 25 in `MANUAL_TESTS.md`).
- [ ] **M8**: Bump to `v0.34.2`, Gate 8 verification and commit/release.
