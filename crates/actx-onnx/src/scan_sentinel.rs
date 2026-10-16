//! Scanned-page sentinel (deterministic heuristic).
//!
//! Classifies whether a PDF page is a rasterized scan / photo without a text layer.
//! The MobileNetV4 RVL-CDIP weights can be downloaded, but neural inference for this
//! component is not implemented yet (ADR-130): this module always runs the 2D heuristic.

use std::path::Path;
use crate::session::ModelSession;

/// Sentinel for scanned / rasterized document pages.
#[derive(Debug, Clone)]
pub struct OnnxScanSentinel {
    weights: Option<ModelSession>,
}

impl OnnxScanSentinel {
    pub fn new(model_path: &Path) -> Self {
        let weights = ModelSession::open(model_path, "mobilenetv4-rvl-cdip").ok();
        Self { weights }
    }

    /// Always `false`: this component runs a deterministic heuristic (ADR-130).
    pub fn is_neural_active(&self) -> bool {
        false
    }

    /// True when the MobileNetV4 weights are present on disk (not executed yet).
    pub fn weights_present(&self) -> bool {
        self.weights.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Evaluates page text density and image count to classify if it is a pure scan.
    pub fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32) {
        let text_len = raw_text.trim().len();
        if text_len == 0 && image_count > 0 {
            (true, 0.90)
        } else if text_len < 50 && image_count > 0 {
            (true, 0.75)
        } else {
            (false, 0.90)
        }
    }
}
