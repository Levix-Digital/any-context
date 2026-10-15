//! Central ONNX Neural Engine for AnyContext.
//!
//! Orchestrates the lifecycle of all local ONNX specialized models:
//! - BGE-Small Query Classifier
//! - MobileNetV4 Scan Sentinel
//! - LayoutLMv3 / Laya mmBERT Layout Extractor
//! - CLIP ViT Vision Encoder

use std::path::Path;
use crate::device::ExecutionDevice;
use crate::query_classifier::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier};
use crate::scan_sentinel::OnnxScanSentinel;
use crate::layout_extractor::OnnxLayoutExtractor;
use crate::vision_encoder::OnnxVisionEncoder;
use crate::reranker::OnnxReranker;

/// Unified facade for local ONNX neural inference.
#[derive(Debug, Clone)]
pub struct OnnxEngine {
    pub query_classifier: OnnxQueryClassifier,
    pub scan_sentinel: OnnxScanSentinel,
    pub layout_extractor: OnnxLayoutExtractor,
    pub vision_encoder: OnnxVisionEncoder,
    pub reranker: OnnxReranker,
    pub device: ExecutionDevice,
}

impl OnnxEngine {
    /// Initializes the ONNX engine by scanning the canonical models directory.
    pub fn init(models_dir: &Path) -> Self {
        Self::with_device(models_dir, ExecutionDevice::Auto)
    }

    /// Initializes the ONNX engine with an explicit hardware execution device.
    pub fn with_device(models_dir: &Path, device: ExecutionDevice) -> Self {
        let bge_path = models_dir.join("bge_small_query_int8.onnx");
        let mobilenet_path = models_dir.join("mobilenetv4_rvl_cdip.onnx");
        let layout_path = models_dir.join("layoutlmv3_base_int8.onnx");
        let clip_path = models_dir.join("clip_vit_vision_int8.onnx");
        let reranker_path = models_dir.join("bge_reranker_small_int8.onnx");

        Self {
            query_classifier: OnnxQueryClassifier::new(&bge_path),
            scan_sentinel: OnnxScanSentinel::new(&mobilenet_path),
            layout_extractor: OnnxLayoutExtractor::new(&layout_path),
            vision_encoder: OnnxVisionEncoder::new(&clip_path),
            reranker: OnnxReranker::with_device(&reranker_path, device),
            device,
        }
    }

    pub fn set_device(&mut self, device: ExecutionDevice) {
        self.device = device;
        self.reranker.set_device(device);
    }

    pub fn device(&self) -> ExecutionDevice {
        self.device
    }

    /// Returns the number of currently installed, verified neural models.
    pub fn active_models_count(&self) -> usize {
        let mut count = 0;
        if self.query_classifier.is_neural_active() { count += 1; }
        if self.scan_sentinel.is_neural_active() { count += 1; }
        if self.layout_extractor.is_neural_active() { count += 1; }
        if self.vision_encoder.is_neural_active() { count += 1; }
        if self.reranker.is_neural_active() { count += 1; }
        count
    }

    /// Returns a status breakdown for telemetry and UI badges.
    pub fn summary(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("BGE-Small Query Classifier", self.query_classifier.is_neural_active()),
            ("MobileNetV4 Scan Sentinel", self.scan_sentinel.is_neural_active()),
            ("LayoutLMv3 Layout Extractor", self.layout_extractor.is_neural_active()),
            ("CLIP ViT Vision Encoder", self.vision_encoder.is_neural_active()),
            ("BGE-Reranker-Small (Cross-Encoder)", self.reranker.is_neural_active()),
        ]
    }

    /// Evaluates query complexity and intent via neural model or deterministic fallback.
    pub fn classify_query(&self, query: &str) -> (OnnxComplexity, OnnxIntent, f32, String) {
        self.query_classifier.classify_query(query)
    }

    /// Evaluates scanned page status via MobileNetV4 or 2D spatial fallback.
    pub fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32) {
        self.scan_sentinel.is_scanned_page(raw_text, image_count)
    }

    /// Re-ranks a batch of candidate chunks against the query using local cross-attention.
    pub fn rerank(&self, query: &str, candidates: &[String]) -> Vec<f32> {
        self.reranker.score_batch(query, candidates)
    }
}
