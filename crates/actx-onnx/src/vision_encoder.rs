//! Visual encoder for diagrams & charts (not yet neural).
//!
//! CLIP ViT weights can be downloaded, but neural visual encoding is not implemented yet
//! (ADR-130). No synthetic description is ever produced; callers keep the native chunk or
//! use the configured multimodal LLM provider.

use std::path::Path;
use crate::session::ModelSession;

/// Visual encoder for technical diagrams and non-text visual assets.
#[derive(Debug, Clone)]
pub struct OnnxVisionEncoder {
    weights: Option<ModelSession>,
}

impl OnnxVisionEncoder {
    pub fn new(model_path: &Path) -> Self {
        let weights = ModelSession::open(model_path, "clip-vit-int8").ok();
        Self { weights }
    }

    /// Always `false`: neural visual encoding is not implemented yet (ADR-130).
    pub fn is_neural_active(&self) -> bool {
        false
    }

    /// True when CLIP ViT weights are present on disk (not executed yet).
    pub fn weights_present(&self) -> bool {
        self.weights.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Always `None` until a real visual encoder exists.
    pub fn describe_visual_element(&self, _image_bytes: &[u8]) -> Option<String> {
        None
    }
}
