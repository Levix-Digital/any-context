//! Intelligent Ingestion ModelRouter and Document AI Dispatcher.
//!
//! Orchestrates the three ingestion layers:
//! - Nível 1: Specialized native high-speed extractors (AST, Markdown, Office, Tabular, Lopdf).
//! - Nível 2: Deterministic Quality Gate & Dynamic Typology Classification.
//! - Nível 3: Document AI & Hybrid Vision Dispatcher (Local SLM vs Corporate VPC vs Heuristic 2D).

use std::sync::Arc;
use async_trait::async_trait;
use actx_lm::traits::LmProvider;
use actx_lm::types::{ChatMessage, ChatRequest};
use crate::models::ChunkPayload;
use crate::ingestion::quality_gate::{QualityGate, QualityDecision, DocumentAiTarget};
use crate::ingestion::chunkers::spatial_form::SpatialFormChunker;

/// Strategy contract for specialized Document AI and Layout Models (LayoutLMv3, Laya, etc.)
#[async_trait]
pub trait DocumentLayoutExtractor: Send + Sync {
    /// Extracts structured representation from a document page or complex form.
    async fn extract_page(
        &self,
        file_path: &str,
        page_num: usize,
        content: &str,
    ) -> Result<String, String>;
}

/// Vision Execution Mode for Document AI Level 3 processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisionExecutionMode {
    /// Zero external AI: Pure Rust 2D spatial & typographic heuristic. Zero RAM/download.
    Disabled,
    /// Local Lightweight SLM (e.g. Moondream2 1.8B INT4 or SmolVLM) via local inference provider.
    LocalSlm { model_name: String },
    /// Corporate VPC Private Gateway (e.g. Azure OpenAI, AWS Bedrock, client internal vLLM).
    CorporateVpc { endpoint: String, model_name: String },
}

impl Default for VisionExecutionMode {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Central Ingestion ModelRouter orchestrating quality assurance and Document AI.
pub struct IngestionModelRouter {
    pub quality_gate: QualityGate,
    pub spatial_chunker: SpatialFormChunker,
    pub layout_extractor: Option<Box<dyn DocumentLayoutExtractor>>,
    pub vision_mode: VisionExecutionMode,
}

impl Default for IngestionModelRouter {
    fn default() -> Self {
        Self::new(QualityGate::default())
    }
}

impl IngestionModelRouter {
    pub fn new(quality_gate: QualityGate) -> Self {
        Self {
            quality_gate,
            spatial_chunker: SpatialFormChunker::default(),
            layout_extractor: None,
            vision_mode: VisionExecutionMode::Disabled,
        }
    }

    pub fn with_vision_mode(mut self, mode: VisionExecutionMode) -> Self {
        self.vision_mode = mode;
        self
    }

    pub fn with_layout_extractor(mut self, extractor: Box<dyn DocumentLayoutExtractor>) -> Self {
        self.layout_extractor = Some(extractor);
        self
    }

    /// Evaluates extracted raw chunks, filtering out noise/degenerate fragments and separating
    /// Document AI candidates (complex forms, scans, diagrams).
    ///
    /// Returns:
    /// - `approved`: Chunks that passed Quality Gate and are ready for index.
    /// - `candidates`: Chunks that require Document AI / Vision / Spatial reconstruction.
    /// - `dropped_count`: Total noise chunks rejected.
    pub fn triage_chunks(
        &self,
        chunks: Vec<ChunkPayload>,
    ) -> (Vec<ChunkPayload>, Vec<(ChunkPayload, DocumentAiTarget)>, usize) {
        let mut approved = Vec::new();
        let mut candidates = Vec::new();
        let mut dropped_count = 0usize;

        for chunk in chunks {
            match self.quality_gate.evaluate_chunk(&chunk) {
                QualityDecision::Pass { .. } => {
                    approved.push(chunk);
                }
                QualityDecision::Reject { reason: _, score: _ } => {
                    dropped_count += 1;
                }
                QualityDecision::NeedsDocumentAi { target } => {
                    candidates.push((chunk, target));
                }
            }
        }

        (approved, candidates, dropped_count)
    }

    /// Processes Document AI candidates through the hybrid resolution pipeline:
    /// 1. Pluggable layout extractor (if registered)
    /// 2. Vision provider (Local SLM or Corporate VPC via actx-lm)
    /// 3. Domain-agnostic 2D spatial & typographic heuristic (pure Rust fallback)
    pub async fn process_document_ai_candidates(
        &self,
        candidates: Vec<(ChunkPayload, DocumentAiTarget)>,
        lm_provider: Option<&Arc<dyn LmProvider>>,
    ) -> Vec<ChunkPayload> {
        let mut processed = Vec::with_capacity(candidates.len());

        for (chunk, target) in candidates {
            match target {
                DocumentAiTarget::DenseComplexForm { .. } | DocumentAiTarget::ScannedPdfPage { .. } => {
                    // Try specialized layout extractor first if configured
                    if let Some(extractor) = &self.layout_extractor {
                        if let Ok(extracted) = extractor.extract_page(&chunk.file_path, chunk.start_line, &chunk.text).await {
                            let mut enriched_chunk = chunk.clone();
                            enriched_chunk.text = format!("// Context: {} > [Document AI Extracted Form]\n\n{}", chunk.file_name, extracted);
                            enriched_chunk.content_type = "document_form".to_string();
                            processed.push(enriched_chunk);
                            continue;
                        }
                    }

                    // If Vision mode is enabled and provider is supplied, attempt multimodal synthesis
                    if self.is_vision_enabled() {
                        if let Some(provider) = lm_provider {
                            if let Ok(synth) = self.synthesize_form_multimodal(provider, &chunk).await {
                                let mut enriched_chunk = chunk.clone();
                                enriched_chunk.text = format!("// Context: {} > [Form Key-Value Synthesis]\n\n{}", chunk.file_name, synth);
                                enriched_chunk.content_type = "document_form".to_string();
                                processed.push(enriched_chunk);
                                continue;
                            }
                        }
                    }

                    // Fallback to pure Rust domain-agnostic spatial & typographic form chunker
                    let spatial_chunks = self.spatial_chunker.chunk_dense_text(
                        &chunk.file_name,
                        &chunk.file_path,
                        chunk.start_line.max(1),
                        &chunk.text,
                    );

                    if !spatial_chunks.is_empty() {
                        processed.extend(spatial_chunks);
                    } else {
                        // Preserves original chunk safely
                        processed.push(chunk);
                    }
                }
                DocumentAiTarget::VisualDiagram { width, height } => {
                    // Check if vision is enabled
                    if self.is_vision_enabled() {
                        if let Some(provider) = lm_provider {
                            if let Ok(diag_synth) = self.synthesize_diagram_multimodal(provider, &chunk, width, height).await {
                                let mut enriched_chunk = chunk.clone();
                                enriched_chunk.text = format!("// Context: {} > [Visual Diagram Synthesis]\n\n{}", chunk.file_name, diag_synth);
                                enriched_chunk.content_type = "visual_diagram_enriched".to_string();
                                processed.push(enriched_chunk);
                                continue;
                            }
                        }
                    }

                    // Preserves native structured visual specification
                    processed.push(chunk);
                }
            }
        }

        processed
    }

    /// Determines if vision execution is active.
    pub fn is_vision_enabled(&self) -> bool {
        !matches!(self.vision_mode, VisionExecutionMode::Disabled)
    }

    /// Synthesizes complex forms using the configured vision provider via actx-lm.
    async fn synthesize_form_multimodal(
        &self,
        provider: &Arc<dyn LmProvider>,
        chunk: &ChunkPayload,
    ) -> Result<String, String> {
        let model = match &self.vision_mode {
            VisionExecutionMode::LocalSlm { model_name } => model_name.as_str(),
            VisionExecutionMode::CorporateVpc { model_name, .. } => model_name.as_str(),
            VisionExecutionMode::Disabled => return Err("Vision disabled".to_string()),
        };

        let system_prompt = "Você é um extrator de alta precisão de documentos e formulários corporativos. \
Analise a página do documento e extraia com precisão todos os pares chave-valor, campos de formulário, \
tabelas de itens e totais em Markdown limpo e estruturado. Não invente dados e preserve números, códigos e datas.";

        let user_prompt = format!(
            "Documento: {}\n\nTexto bruto/fragmentado extraído da página:\n{}\n\nExtraia os campos em formato limpo:",
            chunk.file_name,
            chunk.text.chars().take(2500).collect::<String>()
        );

        let request = ChatRequest::new(
            model,
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
        );

        match provider.chat_complete(request).await {
            Ok(resp) => Ok(resp.content.trim().to_string()),
            Err(e) => Err(format!("Multimodal form synthesis failed: {}", e)),
        }
    }

    /// Synthesizes architectural and technical diagrams using the configured vision provider via actx-lm.
    async fn synthesize_diagram_multimodal(
        &self,
        provider: &Arc<dyn LmProvider>,
        chunk: &ChunkPayload,
        width: u32,
        height: u32,
    ) -> Result<String, String> {
        let model = match &self.vision_mode {
            VisionExecutionMode::LocalSlm { model_name } => model_name.as_str(),
            VisionExecutionMode::CorporateVpc { model_name, .. } => model_name.as_str(),
            VisionExecutionMode::Disabled => return Err("Vision disabled".to_string()),
        };

        let system_prompt = "Você é um arquiteto de sistemas e especialista em diagramas técnicos. \
Descreva tecnicamente este diagrama: liste os componentes, conexões de rede/mensageria entre eles, \
contratos, portas e o fluxo principal de dados.";

        let user_prompt = format!(
            "Arquivo: {} (Resolução: {}x{} px)\nEspecificação visual:\n{}\n\nDescreva detalhadamente a arquitetura/diagrama ilustrado:",
            chunk.file_name, width, height, chunk.text
        );

        let request = ChatRequest::new(
            model,
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
        );

        match provider.chat_complete(request).await {
            Ok(resp) => Ok(resp.content.trim().to_string()),
            Err(e) => Err(format!("Diagram synthesis failed: {}", e)),
        }
    }
}
