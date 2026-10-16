//! Gemma 4 Local Provider for `actx-lm` (planned, ADR-130).
//!
//! Local Gemma 4 generation is not available yet: this provider always delegates to the
//! configured fallback provider (cloud, Ollama, LM Studio or Mock), and returns a clear
//! `Unsupported` error when no fallback is configured. It never fabricates output.

use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use actx_onnx::{gemma::LOCAL_GENERATION_PLANNED, ExecutionDevice, GemmaModelTier, OnnxGemmaEngine};
use crate::error::LmError;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{ChatRequest, ChatResponse};

/// Provider strategy for local Gemma 4 inference (fallback-only until local generation exists).
pub struct GemmaOnnxProvider {
    engine: OnnxGemmaEngine,
    fallback: Option<Arc<dyn LmProvider>>,
}

impl GemmaOnnxProvider {
    pub fn new(model_path: &Path, tier: GemmaModelTier) -> Self {
        Self::with_device(model_path, tier, ExecutionDevice::Auto)
    }

    pub fn with_device(model_path: &Path, tier: GemmaModelTier, device: ExecutionDevice) -> Self {
        Self {
            engine: OnnxGemmaEngine::with_device(model_path, tier, device),
            fallback: None,
        }
    }

    pub fn with_fallback(mut self, fallback: Arc<dyn LmProvider>) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Always `false` until local Gemma 4 generation exists.
    pub fn is_neural_active(&self) -> bool {
        self.engine.is_neural_active()
    }

    pub fn tier(&self) -> GemmaModelTier {
        self.engine.tier()
    }

    pub fn device(&self) -> ExecutionDevice {
        self.engine.device()
    }

    pub fn set_device(&mut self, device: ExecutionDevice) {
        self.engine.set_device(device);
    }

    fn unavailable(&self) -> LmError {
        LmError::Unsupported(format!(
            "{} ({}). Configure a cloud or local server model (e.g. Ollama / LM Studio).",
            LOCAL_GENERATION_PLANNED,
            self.engine.tier().display_name()
        ))
    }
}

#[async_trait]
impl LmProvider for GemmaOnnxProvider {
    fn provider_id(&self) -> &'static str {
        "gemma-onnx"
    }

    async fn chat_complete(&self, request: ChatRequest) -> Result<ChatResponse, LmError> {
        match &self.fallback {
            Some(fallback) => fallback.chat_complete(request).await,
            None => Err(self.unavailable()),
        }
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        match &self.fallback {
            Some(fallback) => fallback.chat_stream(request).await,
            None => Err(self.unavailable()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use crate::types::{ChatMessage, StreamChunk};
    use crate::providers::mock::MockLmProvider;

    #[tokio::test]
    async fn test_gemma_provider_missing_model_fallback() {
        let mock = Arc::new(MockLmProvider::new().with_canned_response("Fallback response!"));
        let provider = GemmaOnnxProvider::new(
            Path::new("non_existent_gemma.onnx"),
            GemmaModelTier::Gemma12b,
        ).with_fallback(mock);

        assert!(!provider.is_neural_active());
        assert_eq!(provider.provider_id(), "gemma-onnx");

        let req = ChatRequest::new("gemma-4-12b", vec![ChatMessage::user("Hello")]);
        let resp = provider.chat_complete(req).await.expect("Should fallback cleanly");
        assert_eq!(resp.content, "Fallback response!");
    }

    #[tokio::test]
    async fn test_gemma_provider_without_fallback_reports_planned() {
        let provider = GemmaOnnxProvider::new(
            Path::new("non_existent_gemma.onnx"),
            GemmaModelTier::Compact,
        );

        assert!(!provider.is_neural_active());
        let req = ChatRequest::new("gemma-4-compact", vec![ChatMessage::user("Hello")]);
        let err = provider.chat_complete(req).await.unwrap_err();
        assert!(err.to_string().contains("planned"));
    }

    #[tokio::test]
    async fn test_gemma_provider_weight_file_never_fabricates_output() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gemma_4_27b_int8.onnx");
        let mut dummy = vec![0x08, 0x01, 0x12, 0x04];
        dummy.resize(2048, 0xCC);
        std::fs::write(&path, &dummy).unwrap();

        let mock = Arc::new(MockLmProvider::new().with_canned_response("From fallback"));
        let provider = GemmaOnnxProvider::with_device(&path, GemmaModelTier::Gemma27b, ExecutionDevice::DirectML { adapter_index: 0 })
            .with_fallback(mock);
        assert!(!provider.is_neural_active());

        let req = ChatRequest::new("gemma-4-27b", vec![ChatMessage::user("What is Rust?")]);
        let resp = provider.chat_complete(req).await.expect("Should use fallback");
        assert_eq!(resp.content, "From fallback");
        assert!(!resp.thinking.unwrap_or_default().contains("tok/s"));

        let req_stream = ChatRequest::new("gemma-4-27b", vec![ChatMessage::user("Stream test")]);
        let mut stream = provider.chat_stream(req_stream).await.expect("Should stream from fallback");
        let mut tokens = String::new();
        while let Some(chunk_res) = stream.next().await {
            if let StreamChunk::Token(t) = chunk_res.unwrap() {
                tokens.push_str(&t);
            }
        }
        assert!(!tokens.contains("ONNX engine"));
    }
}
