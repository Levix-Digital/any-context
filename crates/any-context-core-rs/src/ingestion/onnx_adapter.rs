//! Adapter connecting `actx-onnx` neural models to `actx-agent` and Core traits.

use std::path::Path;
use actx_agent::router::{ComplexityClassifier, QueryComplexity, QueryIntent};
use actx_onnx::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier};

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
}
