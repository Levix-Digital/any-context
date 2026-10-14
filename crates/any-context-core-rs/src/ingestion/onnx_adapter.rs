//! Adapter connecting `actx-onnx` neural models to `actx-agent` and Core traits.

use std::path::Path;
use async_trait::async_trait;
use actx_agent::router::{ComplexityClassifier, QueryComplexity, QueryIntent};
use actx_onnx::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier, OnnxScanSentinel, OnnxLayoutExtractor, OnnxVisionEncoder};
use crate::ingestion::traits::{ScanSentinel, ScanAssessment, DocumentLayoutExtractor, DocumentVisionExtractor};

/// Adapter wrapping `OnnxQueryClassifier` to implement `actx_agent::router::ComplexityClassifier`.
pub struct OnnxComplexityClassifierAdapter {
    inner: OnnxQueryClassifier,
}

impl OnnxComplexityClassifierAdapter {
    pub fn new(model_path: &Path) -> Self {
        Self {
            inner: OnnxQueryClassifier::new(model_path),
        }
    }

    pub fn is_neural_active(&self) -> bool {
        self.inner.is_neural_active()
    }
}

impl ComplexityClassifier for OnnxComplexityClassifierAdapter {
    fn classify(&self, query: &str) -> (QueryComplexity, QueryIntent, f32, String) {
        let (complexity, intent, confidence, reason) = self.inner.classify_query(query);
        let c = match complexity {
            OnnxComplexity::Fast => QueryComplexity::Fast,
            OnnxComplexity::Deep => QueryComplexity::Deep,
        };
        let i = match intent {
            OnnxIntent::Code => QueryIntent::Code,
            OnnxIntent::Architecture => QueryIntent::Architecture,
            OnnxIntent::Document => QueryIntent::Document,
            OnnxIntent::General => QueryIntent::General,
        };
        (c, i, confidence, reason)
    }
}

/// Adapter wrapping `OnnxScanSentinel` to implement `crate::ingestion::traits::ScanSentinel`.
#[derive(Debug)]
pub struct OnnxScanSentinelAdapter {
    inner: OnnxScanSentinel,
}

impl OnnxScanSentinelAdapter {
    pub fn new(model_path: &Path) -> Self {
        Self {
            inner: OnnxScanSentinel::new(model_path),
        }
    }

    pub fn from_sentinel(inner: OnnxScanSentinel) -> Self {
        Self { inner }
    }
}

impl ScanSentinel for OnnxScanSentinelAdapter {
    fn assess_document(&self, raw_bytes: &[u8], filename: &str) -> Option<ScanAssessment> {
        let is_pdf = filename.ends_with(".pdf") || raw_bytes.starts_with(b"%PDF");
        if !is_pdf {
            return None;
        }

        // Sub-0.1ms byte inspection: detect absence of text streams with presence of raster images
        let has_text_stream = raw_bytes.windows(5).any(|w| w == b"/Font" || w == b"/Text");
        let has_image_stream = raw_bytes.windows(6).any(|w| w == b"/Image" || w == b"/XObject");

        if !has_text_stream && has_image_stream {
            Some(ScanAssessment {
                is_scan: true,
                confidence: if self.inner.is_neural_active() { 0.98 } else { 0.88 },
                detected_typology: Some("scanned_document".to_string()),
            })
        } else {
            Some(ScanAssessment {
                is_scan: false,
                confidence: 0.92,
                detected_typology: Some("digital_native_pdf".to_string()),
            })
        }
    }

    fn is_scanned_page(&self, raw_text: &str, image_count: usize) -> (bool, f32) {
        self.inner.is_scanned_page(raw_text, image_count)
    }

    fn is_neural_active(&self) -> bool {
        self.inner.is_neural_active()
    }
}

/// Adapter wrapping `OnnxLayoutExtractor` to implement `crate::ingestion::traits::DocumentLayoutExtractor`.
#[derive(Debug)]
pub struct OnnxLayoutExtractorAdapter {
    inner: OnnxLayoutExtractor,
}

impl OnnxLayoutExtractorAdapter {
    pub fn new(model_path: &Path) -> Self {
        Self {
            inner: OnnxLayoutExtractor::new(model_path),
        }
    }

    pub fn from_extractor(inner: OnnxLayoutExtractor) -> Self {
        Self { inner }
    }

    pub fn is_neural_active(&self) -> bool {
        self.inner.is_neural_active()
    }
}

#[async_trait]
impl DocumentLayoutExtractor for OnnxLayoutExtractorAdapter {
    async fn extract_page(
        &self,
        _file_path: &str,
        _page_num: usize,
        content: &str,
    ) -> Result<String, String> {
        Ok(self.inner.extract_page_layout(content))
    }
}

/// Adapter wrapping `OnnxVisionEncoder` to implement `crate::ingestion::traits::DocumentVisionExtractor`.
#[derive(Debug)]
pub struct OnnxVisionEncoderAdapter {
    inner: OnnxVisionEncoder,
}

impl OnnxVisionEncoderAdapter {
    pub fn new(model_path: &Path) -> Self {
        Self {
            inner: OnnxVisionEncoder::new(model_path),
        }
    }

    pub fn from_encoder(inner: OnnxVisionEncoder) -> Self {
        Self { inner }
    }
}

impl DocumentVisionExtractor for OnnxVisionEncoderAdapter {
    fn describe_visual_element(&self, image_bytes: &[u8]) -> Option<String> {
        self.inner.describe_visual_element(image_bytes)
    }

    fn is_neural_active(&self) -> bool {
        self.inner.is_neural_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onnx_complexity_adapter() {
        let adapter = OnnxComplexityClassifierAdapter::new(Path::new("dummy_missing.onnx"));
        assert!(!adapter.is_neural_active());

        let (comp, intent, score, reason) = adapter.classify("where is timeout?");
        assert_eq!(comp, QueryComplexity::Fast);
        assert_eq!(intent, QueryIntent::General);
        assert!(score > 0.0);
        assert!(reason.contains("Deterministic Fallback"));
    }

    #[test]
    fn test_onnx_scan_sentinel_adapter() {
        let adapter = OnnxScanSentinelAdapter::new(Path::new("dummy_missing_sentinel.onnx"));
        assert!(!adapter.is_neural_active());

        // Test empty text with image -> identified as scan
        let (is_scan, conf) = adapter.is_scanned_page("", 1);
        assert!(is_scan);
        assert!(conf >= 0.75);

        // Test normal text -> not a scan
        let (is_scan, conf) = adapter.is_scanned_page("This is a standard document page with plenty of text.", 0);
        assert!(!is_scan);
        assert!(conf >= 0.90);

        // Test raw bytes assessment for non-pdf
        assert_eq!(adapter.assess_document(b"hello world", "test.txt"), None);

        // Test raw bytes assessment for scan-like PDF
        let scan_bytes = b"%PDF-1.4 ... /Image /XObject ...";
        let assess = adapter.assess_document(scan_bytes, "scan.pdf");
        assert!(assess.is_some());
        let a = assess.unwrap();
        assert!(a.is_scan);
        assert_eq!(a.detected_typology, Some("scanned_document".to_string()));
    }

    #[tokio::test]
    async fn test_onnx_layout_extractor_adapter() {
        let adapter = OnnxLayoutExtractorAdapter::new(Path::new("dummy_missing_layout.onnx"));
        assert!(!adapter.is_neural_active());

        let raw = "Item: Widget\nPrice: $10.00";
        let res = adapter.extract_page("doc.pdf", 1, raw).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), raw);
    }

    #[test]
    fn test_onnx_vision_encoder_adapter() {
        let adapter = OnnxVisionEncoderAdapter::new(Path::new("dummy_missing_vision.onnx"));
        assert!(!adapter.is_neural_active());

        // Without neural weights, returns None gracefully
        let desc = adapter.describe_visual_element(b"dummy image bytes");
        assert_eq!(desc, None);
    }
}
