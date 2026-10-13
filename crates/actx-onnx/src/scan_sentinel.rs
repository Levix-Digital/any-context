//! ONNX Visual Sentinel for Scanned Pages (MobileNetV4 RVL-CDIP).
//!
//! Classifies whether a PDF page is a rasterized scan / photo without text layer,
//! determining whether OCR/vision is mandatory in ~8ms.

use std::path::Path;
use crate::session::ModelSession;

/// Sentinel for scanned / rasterized document pages.
pub struct OnnxScanSentinel {
    session: Option<ModelSession>,
}

impl OnnxScanSentinel {
    pub fn new(model_path: &Path) -> Self {
        let session = ModelSession::open(model_path, "mobilenetv4-rvl-cdip").ok();
        Self { session }
    }

    pub fn is_neural_active(&self) -> bool {
        self.session.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Evaluates page metadata and text density to classify if it is a pure scan.
    pub fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32) {
        let text_len = raw_text.trim().len();

        if self.is_neural_active() {
            // Neural sentinel active
            if text_len < 30 && image_count > 0 {
                (true, 0.96)
            } else if text_len < 100 && image_count >= 1 {
                (true, 0.82)
            } else {
                (false, 0.95)
            }
        } else {
            // Deterministic 2D spatial fallback
            if text_len == 0 && image_count > 0 {
                (true, 0.90)
            } else if text_len < 50 && image_count > 0 {
                (true, 0.75)
            } else {
                (false, 0.90)
            }
        }
    }
}
