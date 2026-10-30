pub mod traits;
pub mod router;
pub mod chunkers;
pub mod scanner;
pub mod orchestrator;
pub mod embedding;
pub mod embedding_catalog;
pub mod local_embedding;
pub mod enricher;
pub mod crawler;
pub mod quality_gate;
pub mod model_router;
pub mod model_manager;
pub mod onnx_adapter;
pub mod jina;
pub mod secrets;

pub use router::{IngestionRouter, RedactedChunks};
pub use traits::{Chunker, ScanSentinel, ScanAssessment, DocumentVisionExtractor};
pub use scanner::{WorkspaceScanner, DiffResult};
pub use orchestrator::{embed_with_retry, embedding_failure_summary, offline_summary, sync_error_headline, NativeSyncOrchestrator, SyncOptions, SyncResult};
pub use embedding::{
    EmbeddingModelSource, EmbeddingTarget, EmbeddingUnavailable, QueryEmbedding, ReindexDecision, ResolvedEmbedding,
    EMBEDDING_MODEL_SETTING, GLOBAL_KNOWLEDGE_TARGET_SETTING, active_embedding_model, active_profile,
    build_embedding_provider, decide_reindex, global_dense_enabled, is_model_allowed, pending_reindex_notice,
    prepare_embedding, profile_default_model, resolve_query_embedding, workspaces_reindexed_by,
};
pub use embedding_catalog::{
    apply_prefix, find_embedding_model, selectable_models, spec_for_target, EmbedRole, EmbeddingModelSpec,
    EmbeddingRuntime, EMBEDDING_CATALOG,
};
pub use local_embedding::LocalEmbeddingProvider;
pub use enricher::NativeContextualEnricher;
pub use crawler::{NativeWebCrawler, CrawlerConfig, CrawledPage, SitemapEntry, RobotsPolicy};
pub use jina::{NativeJinaExtractor, JinaExtractorConfig};
pub use secrets::{redact_secrets, Redaction, SECRET_PLACEHOLDER};
pub use quality_gate::{QualityGate, QualityGateConfig, QualityDecision, QualityRejectionReason, DocumentAiTarget};
pub use model_router::{IngestionModelRouter, DocumentLayoutExtractor, VisionExecutionMode};
pub use model_manager::{OnnxModelManager, OnnxModelSpec, OnnxModelCategory, ModelAvailability, ModelFile, ModelStoreSummary, ModelStatusItem, ONNX_CATALOG, DEFAULT_RERANKER_ID, MODELS_MIRROR_ENV, MODELS_MIRROR_SETTING, reranker_short_label};
pub use onnx_adapter::{
    OnnxComplexityClassifierAdapter,
    OnnxScanSentinelAdapter,
    OnnxLayoutExtractorAdapter,
    OnnxVisionEncoderAdapter,
};
pub use chunkers::spatial_form::SpatialFormChunker;
