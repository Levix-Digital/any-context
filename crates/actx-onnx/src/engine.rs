//! Central facade over the local inference components.
//!
//! Component status (ADR-130):
//! - Cross-Encoder Re-Ranker: neural (ONNX Runtime, CPU).
//! - Query Classifier, Scan Sentinel, Layout Extractor, Vision Encoder: heuristic.

use std::path::Path;
use crate::device::ExecutionDevice;
use crate::query_classifier::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier};
use crate::scan_sentinel::OnnxScanSentinel;
use crate::layout_extractor::OnnxLayoutExtractor;
use crate::vision_encoder::OnnxVisionEncoder;
use crate::reranker::OnnxReranker;

/// Default re-ranker model directory inside the models store.
pub const DEFAULT_RERANKER_DIR: &str = "mmarco-reranker";

/// Execution status of a local inference component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStatus {
    /// Real neural inference through ONNX Runtime.
    Neural,
    /// Deterministic Rust heuristic.
    Heuristic,
    /// Neural path expected but currently unavailable (weights or runtime missing).
    Fallback,
}

impl ComponentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Neural => "neural",
            Self::Heuristic => "heuristic",
            Self::Fallback => "fallback",
        }
    }
}

/// Unified facade for local inference components.
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
    /// Initializes the engine by scanning the canonical models directory.
    pub fn init(models_dir: &Path) -> Self {
        Self::with_device(models_dir, ExecutionDevice::Auto)
    }

    /// Initializes the engine with an explicit requested execution device.
    pub fn with_device(models_dir: &Path, device: ExecutionDevice) -> Self {
        Self {
            query_classifier: OnnxQueryClassifier::new(&models_dir.join("bge_small_query_int8.onnx")),
            scan_sentinel: OnnxScanSentinel::new(&models_dir.join("mobilenetv4_rvl_cdip.onnx")),
            layout_extractor: OnnxLayoutExtractor::new(&models_dir.join("layoutlmv3_base_int8.onnx")),
            vision_encoder: OnnxVisionEncoder::new(&models_dir.join("clip_vit_vision_int8.onnx")),
            reranker: OnnxReranker::with_device(&models_dir.join(DEFAULT_RERANKER_DIR).join("model.onnx"), device),
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

    /// Number of components currently running real neural inference.
    pub fn active_models_count(&self) -> usize {
        self.summary().iter().filter(|(_, s)| *s == ComponentStatus::Neural).count()
    }

    /// Status breakdown for telemetry and UI badges.
    pub fn summary(&self) -> Vec<(&'static str, ComponentStatus)> {
        let reranker = if self.reranker.is_neural_active() {
            ComponentStatus::Neural
        } else {
            ComponentStatus::Fallback
        };
        vec![
            ("Query Classifier", ComponentStatus::Heuristic),
            ("Scan Sentinel", ComponentStatus::Heuristic),
            ("Layout Extractor", ComponentStatus::Heuristic),
            ("Vision Encoder", ComponentStatus::Heuristic),
            ("Cross-Encoder Re-Ranker", reranker),
        ]
    }

    /// Evaluates query complexity and intent (deterministic heuristic).
    pub fn classify_query(&self, query: &str) -> (OnnxComplexity, OnnxIntent, f32, String) {
        self.query_classifier.classify_query(query)
    }

    /// Evaluates scanned page status (deterministic heuristic).
    pub fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32) {
        self.scan_sentinel.is_scanned_page(raw_text, image_count)
    }

    /// Re-ranks candidate chunks with the cross-encoder; empty result means RRF fallback.
    pub fn rerank(&self, query: &str, candidates: &[String]) -> Vec<f32> {
        self.reranker.score_batch(query, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_without_weights_is_honest() {
        let engine = OnnxEngine::init(Path::new("definitely_missing_models_dir"));
        assert_eq!(engine.active_models_count(), 0);
        let summary = engine.summary();
        assert_eq!(summary.len(), 5);
        assert!(summary[..4].iter().all(|(_, s)| *s == ComponentStatus::Heuristic));
        assert_eq!(summary[4].1, ComponentStatus::Fallback);
    }
}
