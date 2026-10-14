pub mod traits;
pub mod router;
pub mod chunkers;
pub mod scanner;
pub mod orchestrator;
pub mod enricher;
pub mod crawler;
pub mod quality_gate;
pub mod model_router;
pub mod model_manager;
pub mod onnx_adapter;

pub use router::IngestionRouter;
pub use traits::{Chunker, ScanSentinel, ScanAssessment, DocumentVisionExtractor};
pub use scanner::{WorkspaceScanner, DiffResult};
pub use orchestrator::{NativeSyncOrchestrator, SyncOptions, SyncResult};
pub use enricher::NativeContextualEnricher;
pub use crawler::{NativeWebCrawler, CrawlerConfig, CrawledPage, SitemapEntry, RobotsPolicy};
pub use quality_gate::{QualityGate, QualityGateConfig, QualityDecision, QualityRejectionReason, DocumentAiTarget};
pub use model_router::{IngestionModelRouter, DocumentLayoutExtractor, VisionExecutionMode};
pub use model_manager::{OnnxModelManager, OnnxModelSpec, OnnxModelCategory, ModelStoreSummary, ModelStatusItem, ONNX_CATALOG};
pub use onnx_adapter::{
    OnnxComplexityClassifierAdapter,
    OnnxScanSentinelAdapter,
    OnnxLayoutExtractorAdapter,
    OnnxVisionEncoderAdapter,
};
pub use chunkers::spatial_form::SpatialFormChunker;
