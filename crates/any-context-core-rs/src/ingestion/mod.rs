pub mod traits;
pub mod router;
pub mod chunkers;
pub mod scanner;
pub mod orchestrator;
pub mod embedding;
pub mod embedding_migration;
pub mod enricher;
pub mod crawler;
pub mod quality_gate;
pub mod model_router;
pub mod model_manager;
pub mod onnx_adapter;
pub mod jina;

pub use router::IngestionRouter;
pub use traits::{Chunker, ScanSentinel, ScanAssessment, DocumentVisionExtractor};
pub use scanner::{WorkspaceScanner, DiffResult};
pub use orchestrator::{embed_with_retry, embedding_failure_summary, sync_error_headline, NativeSyncOrchestrator, SyncOptions, SyncResult};
pub use embedding::{
    active_embedding_config, global_dense_enabled, known_embedding_configs, preset_config, probe_dimension,
    provider_for, resolve_active_embedding, resolve_embedding, set_active_embedding_config, stale_files_notice,
    ApiFormat, EmbeddingConfig, EmbeddingRuntime, EmbeddingUnavailable, EMBEDDING_PRESETS,
};
pub use embedding_migration::backfill_chunk_models;
pub use enricher::NativeContextualEnricher;
pub use crawler::{NativeWebCrawler, CrawlerConfig, CrawledPage, SitemapEntry, RobotsPolicy};
pub use jina::{NativeJinaExtractor, JinaExtractorConfig};
pub use quality_gate::{QualityGate, QualityGateConfig, QualityDecision, QualityRejectionReason, DocumentAiTarget};
pub use model_router::{IngestionModelRouter, DocumentLayoutExtractor, VisionExecutionMode};
pub use model_manager::{OnnxModelManager, OnnxModelSpec, OnnxModelCategory, ModelAvailability, ModelFile, ModelStoreSummary, ModelStatusItem, ONNX_CATALOG, DEFAULT_RERANKER_ID, reranker_short_label};
pub use onnx_adapter::{
    OnnxComplexityClassifierAdapter,
    OnnxScanSentinelAdapter,
    OnnxLayoutExtractorAdapter,
    OnnxVisionEncoderAdapter,
};
pub use chunkers::spatial_form::SpatialFormChunker;
