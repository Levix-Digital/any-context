//! Layout & form extractor (deterministic passthrough).
//!
//! LayoutLMv3 / Laya mmBERT weights can be downloaded, but neural layout inference is not
//! implemented yet (ADR-130). Page content is returned unchanged so the Core's pure-Rust
//! 2D spatial chunkers keep full control of table and form reconstruction.

use std::path::Path;
use crate::session::ModelSession;

/// Layout extractor for complex forms and tables.
#[derive(Debug, Clone)]
pub struct OnnxLayoutExtractor {
    weights: Option<ModelSession>,
}

impl OnnxLayoutExtractor {
    pub fn new(model_path: &Path) -> Self {
        let weights = ModelSession::open(model_path, "layoutlmv3-int8")
            .or_else(|_| ModelSession::open(model_path, "laya-int8"))
            .ok();
        Self { weights }
    }

    /// Always `false`: neural layout inference is not implemented yet (ADR-130).
    pub fn is_neural_active(&self) -> bool {
        false
    }

    /// True when layout weights are present on disk (not executed yet).
    pub fn weights_present(&self) -> bool {
        self.weights.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Returns the page content unchanged (geometric layout is preserved by the Core chunkers).
    pub fn extract_page_layout(&self, content: &str) -> String {
        content.to_string()
    }
}
