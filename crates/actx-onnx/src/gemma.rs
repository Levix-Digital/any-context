//! Gemma 4 Local ONNX Inference Engine & KV-Cache Runner.
//!
//! Governs the offline execution of the Gemma 4 INT4 quantized model family:
//! - Gemma 4 Compact (Edge Mobility, ~2.5 GB peak RAM)
//! - Gemma 4 12B (Developer Sweet Spot, ~10.0 GB peak RAM)
//! - Gemma 4 27B (Workstation High Reasoning, ~18.5 GB peak RAM)
//! - Gemma 4 32B (Sovereign Frontier, ~25.5 GB peak RAM)

use std::path::{Path, PathBuf};
use crate::device::ExecutionDevice;
use crate::session::ModelSession;

/// Quantized hardware tier for the Gemma 4 local model family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemmaModelTier {
    Compact,
    Gemma12b,
    Gemma27b,
    Gemma32b,
}

impl GemmaModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compact => "gemma-4-compact",
            Self::Gemma12b => "gemma-4-12b",
            Self::Gemma27b => "gemma-4-27b",
            Self::Gemma32b => "gemma-4-32b",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Compact => "Gemma 4 Compact (INT4)",
            Self::Gemma12b => "Gemma 4 12B (INT4)",
            Self::Gemma27b => "Gemma 4 27B (INT4)",
            Self::Gemma32b => "Gemma 4 32B (INT4)",
        }
    }

    pub fn expected_filename(&self) -> &'static str {
        match self {
            Self::Compact => "gemma_4_compact_int8.onnx",
            Self::Gemma12b => "gemma_4_12b_int8.onnx",
            Self::Gemma27b => "gemma_4_27b_int8.onnx",
            Self::Gemma32b => "gemma_4_32b_int8.onnx",
        }
    }

    pub fn from_model_name(name: &str) -> Option<Self> {
        let clean = name.trim().to_lowercase();
        if clean.contains("32b") || clean.contains("32") {
            Some(Self::Gemma32b)
        } else if clean.contains("27b") || clean.contains("27") {
            Some(Self::Gemma27b)
        } else if clean.contains("12b") || clean.contains("12") {
            Some(Self::Gemma12b)
        } else if clean.contains("compact") || clean.contains("2b") || clean.contains("3b") {
            Some(Self::Compact)
        } else if clean.contains("gemma") {
            Some(Self::Gemma12b)
        } else {
            None
        }
    }
}

/// Specialized BPE subword and turn tokenizer for Gemma 4 prompt formatting.
#[derive(Debug, Clone, Default)]
pub struct GemmaTokenizer;

impl GemmaTokenizer {
    /// Formats conversation turns into standard Gemma 4 chat template.
    pub fn format_prompt(&self, system: Option<&str>, user: &str) -> String {
        let mut out = String::new();
        if let Some(sys) = system {
            if !sys.trim().is_empty() {
                out.push_str("<start_of_turn>system\n");
                out.push_str(sys.trim());
                out.push_str("<end_of_turn>\n");
            }
        }
        out.push_str("<start_of_turn>user\n");
        out.push_str(user.trim());
        out.push_str("<end_of_turn>\n<start_of_turn>model\n");
        out
    }
}

/// Message returned by every generation call until a local generative runtime exists.
pub const LOCAL_GENERATION_PLANNED: &str =
    "Local Gemma 4 generation is planned and not available in this build. AnyContext uses your configured fallback provider instead.";

/// Gemma 4 local generation engine (planned, ADR-130).
///
/// Real Gemma 4 ONNX exports are multi-graph and multi-shard and require a full generative
/// loop (tokenization, KV-cache, sampling) that does not exist yet. Until then this engine
/// never reports itself as active and every generation call returns a clear error, so
/// providers fall back to the configured model (ADR-110).
#[derive(Debug, Clone)]
pub struct OnnxGemmaEngine {
    weights: Option<ModelSession>,
    model_path: PathBuf,
    tier: GemmaModelTier,
    tokenizer: GemmaTokenizer,
    device: ExecutionDevice,
}

impl OnnxGemmaEngine {
    pub fn new(model_path: &Path, tier: GemmaModelTier) -> Self {
        Self::with_device(model_path, tier, ExecutionDevice::Auto)
    }

    pub fn with_device(model_path: &Path, tier: GemmaModelTier, device: ExecutionDevice) -> Self {
        let weights = ModelSession::open(model_path, tier.as_str()).ok();
        Self {
            weights,
            model_path: model_path.to_path_buf(),
            tier,
            tokenizer: GemmaTokenizer,
            device,
        }
    }

    /// Always `false`: local generation is planned (ADR-130).
    pub fn is_neural_active(&self) -> bool {
        false
    }

    /// True when a weight file exists at the configured path (it is not executed).
    pub fn weights_present(&self) -> bool {
        self.weights.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    pub fn tier(&self) -> GemmaModelTier {
        self.tier
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn device(&self) -> ExecutionDevice {
        self.device
    }

    pub fn set_device(&mut self, device: ExecutionDevice) {
        self.device = device;
    }

    /// Chat template used once local generation exists.
    pub fn tokenizer(&self) -> &GemmaTokenizer {
        &self.tokenizer
    }

    /// Always returns an error: local generation is planned (ADR-130).
    pub fn generate(&self, _prompt: &str, _max_tokens: usize) -> Result<String, String> {
        Err(format!("{} ({})", LOCAL_GENERATION_PLANNED, self.tier.display_name()))
    }

    /// Always returns an error without emitting tokens: local generation is planned (ADR-130).
    pub fn generate_stream<F: FnMut(&str) -> Result<(), String>>(
        &self,
        _prompt: &str,
        _max_tokens: usize,
        _on_token: F,
    ) -> Result<(), String> {
        Err(format!("{} ({})", LOCAL_GENERATION_PLANNED, self.tier.display_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemma_tier_conversions() {
        assert_eq!(GemmaModelTier::Compact.expected_filename(), "gemma_4_compact_int8.onnx");
        assert_eq!(GemmaModelTier::Gemma12b.expected_filename(), "gemma_4_12b_int8.onnx");
        assert_eq!(GemmaModelTier::Gemma27b.expected_filename(), "gemma_4_27b_int8.onnx");
        assert_eq!(GemmaModelTier::Gemma32b.expected_filename(), "gemma_4_32b_int8.onnx");

        assert_eq!(GemmaModelTier::from_model_name("gemma-4-compact"), Some(GemmaModelTier::Compact));
        assert_eq!(GemmaModelTier::from_model_name("gemma-4-12b"), Some(GemmaModelTier::Gemma12b));
        assert_eq!(GemmaModelTier::from_model_name("gemma-4-27b"), Some(GemmaModelTier::Gemma27b));
        assert_eq!(GemmaModelTier::from_model_name("gemma-4-32b"), Some(GemmaModelTier::Gemma32b));
        assert_eq!(GemmaModelTier::from_model_name("unknown-model"), None);
    }

    #[test]
    fn test_gemma_tokenizer_chat_formatting() {
        let tokenizer = GemmaTokenizer;
        let formatted = tokenizer.format_prompt(Some("You are an assistant."), "Hello!");
        assert!(formatted.contains("<start_of_turn>system\nYou are an assistant.<end_of_turn>\n"));
        assert!(formatted.contains("<start_of_turn>user\nHello!<end_of_turn>\n<start_of_turn>model\n"));
    }

    #[test]
    fn test_gemma_engine_missing_model_zero_panic() {
        let engine = OnnxGemmaEngine::new(Path::new("missing_gemma.onnx"), GemmaModelTier::Gemma12b);
        assert!(!engine.is_neural_active());

        let res = engine.generate("Hello", 100);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("planned"));

        let stream_res = engine.generate_stream("Hello", 100, |_t| Ok(()));
        assert!(stream_res.is_err());
    }

    #[test]
    fn test_gemma_engine_reports_unavailable_even_with_weight_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gemma_4_compact_int8.onnx");
        let mut dummy = vec![0x08, 0x01, 0x12, 0x04];
        dummy.resize(2048, 0xBB);
        std::fs::write(&path, &dummy).unwrap();

        let engine = OnnxGemmaEngine::new(&path, GemmaModelTier::Compact);
        assert!(engine.weights_present());
        assert!(!engine.is_neural_active(), "A weight file must never be reported as local inference");

        let gen_res = engine.generate("Explain Rust", 50);
        let err = gen_res.unwrap_err();
        assert!(err.contains("planned"));
        assert!(!err.contains("tok/s"));

        let mut collected = Vec::new();
        let stream_res = engine.generate_stream("Explain Rust", 50, |tok| {
            collected.push(tok.to_string());
            Ok(())
        });
        assert!(stream_res.is_err());
        assert!(collected.is_empty(), "No synthetic tokens may be emitted");
    }
}
