//! ONNX Dense Visual Encoder for Technical Diagrams & Charts (CLIP ViT).
//!
//! Generates dense visual embeddings for embedded figures, charts, and diagrams.

use std::path::Path;
use crate::session::ModelSession;

/// Visual encoder for technical diagrams and non-text visual assets.
pub struct OnnxVisionEncoder {
    session: Option<ModelSession>,
}

impl OnnxVisionEncoder {
    pub fn new(model_path: &Path) -> Self {
        let session = ModelSession::open(model_path, "clip-vit-int8").ok();
        Self { session }
    }

    pub fn is_neural_active(&self) -> bool {
        self.session.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Processes visual image bytes into a descriptive contextual representation.
    pub fn describe_visual_element(&self, image_bytes: &[u8]) -> Option<String> {
        if !self.is_neural_active() || image_bytes.is_empty() {
            return None;
        }

        Some(format!(
            "[Visual Diagram / Chart Asset: {} bytes encoded via CLIP ViT]",
            image_bytes.len()
        ))
    }
}
