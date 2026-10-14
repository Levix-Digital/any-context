use async_trait::async_trait;
use crate::models::ChunkPayload;

/// Strategy contract for file chunkers.
pub trait Chunker: Send + Sync {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String>;
}

/// Strategy contract for specialized Document AI and Layout Models (LayoutLMv3, Laya, etc.)
#[async_trait]
pub trait DocumentLayoutExtractor: Send + Sync {
    /// Extracts structured representation from a document page or complex form.
    async fn extract_page(
        &self,
        file_path: &str,
        page_num: usize,
        content: &str,
    ) -> Result<String, String>;
}

/// Assessment emitted by a ScanSentinel when inspecting a document or page.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanAssessment {
    pub is_scan: bool,
    pub confidence: f32,
    pub detected_typology: Option<String>,
}

/// Contract for scanned page and rasterized document detection (e.g. MobileNetV4 RVL-CDIP).
pub trait ScanSentinel: Send + Sync {
    /// Evaluates raw document bytes/metadata for scan and typology detection.
    fn assess_document(&self, raw_bytes: &[u8], filename: &str) -> Option<ScanAssessment>;

    /// Fast-path evaluation of extracted page text and embedded images.
    fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32);

    /// Checks if neural weights are loaded and active.
    fn is_neural_active(&self) -> bool;
}

/// Visual feature extraction and technical diagram description (e.g. CLIP ViT).
pub trait DocumentVisionExtractor: Send + Sync {
    /// Processes visual image bytes into a descriptive contextual representation.
    fn describe_visual_element(&self, image_bytes: &[u8]) -> Option<String>;

    /// Checks if visual feature encoder is active.
    fn is_neural_active(&self) -> bool;
}
