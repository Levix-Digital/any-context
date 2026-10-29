//! Local embedding provider: the ONNX embedder behind the `LmProvider::embed` port, so ingestion
//! and retrieval use a local model exactly like a cloud one (FEAT-067, ADR-0154).

use async_trait::async_trait;

use actx_lm::error::LmError;
use actx_lm::traits::{BoxedChunkStream, LmProvider};
use actx_lm::types::{ChatRequest, ChatResponse};
use actx_onnx::embedder::{model_files, EmbedderConfig, EmbeddingPooling, OnnxEmbedder};

use crate::ingestion::embedding_catalog::{EmbeddingModelSpec, Pooling};
use crate::ingestion::OnnxModelManager;

/// Embeds text with a local ONNX model. Chat is not supported.
#[derive(Debug, Clone)]
pub struct LocalEmbeddingProvider {
    embedder: OnnxEmbedder,
}

impl LocalEmbeddingProvider {
    /// Provider for a local catalog model whose files live in the models directory.
    pub fn for_spec(spec: &EmbeddingModelSpec) -> Self {
        let dir = OnnxModelManager::find_spec(spec.id)
            .map(OnnxModelManager::model_path)
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| crate::storage::sqlite::get_default_models_dir().join(spec.id));
        let (model_path, tokenizer_path) = model_files(&dir);
        Self::new(OnnxEmbedder::new(EmbedderConfig {
            model_id: spec.id.to_string(),
            model_path,
            tokenizer_path,
            dimension: spec.dimension,
            max_tokens: spec.max_tokens,
            pooling: match spec.pooling {
                Pooling::Mean => EmbeddingPooling::Mean,
                Pooling::Cls => EmbeddingPooling::Cls,
            },
        }))
    }

    pub fn new(embedder: OnnxEmbedder) -> Self {
        Self { embedder }
    }

    pub fn embedder(&self) -> &OnnxEmbedder {
        &self.embedder
    }
}

#[async_trait]
impl LmProvider for LocalEmbeddingProvider {
    fn provider_id(&self) -> &'static str {
        "local"
    }

    async fn chat_complete(&self, _request: ChatRequest) -> Result<ChatResponse, LmError> {
        Err(LmError::Unsupported("the local embedding provider does not chat".to_string()))
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        Err(LmError::Unsupported("the local embedding provider does not chat".to_string()))
    }

    async fn embed(&self, _model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        let embedder = self.embedder.clone();
        let texts = texts.to_vec();
        tokio::task::spawn_blocking(move || embedder.embed(&texts))
            .await
            .map_err(|e| LmError::StreamError(format!("local embedding task failed: {e}")))?
            .map_err(LmError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::embedding_catalog::find_embedding_model;

    #[tokio::test]
    async fn missing_weights_are_an_error() {
        let embedder = OnnxEmbedder::new(EmbedderConfig {
            model_id: "missing".into(),
            model_path: "missing/model.onnx".into(),
            tokenizer_path: "missing/tokenizer.json".into(),
            dimension: 384,
            max_tokens: 512,
            pooling: EmbeddingPooling::Mean,
        });
        let provider = LocalEmbeddingProvider::new(embedder);
        assert!(provider.embed("x", &["text".to_string()]).await.is_err());
    }

    #[test]
    fn paths_come_from_the_model_catalog() {
        let spec = find_embedding_model("multilingual-e5-small").unwrap();
        let provider = LocalEmbeddingProvider::for_spec(spec);
        let config = provider.embedder().config();
        assert!(config.model_path.ends_with("multilingual-e5-small/model.onnx"));
        assert!(config.tokenizer_path.ends_with("multilingual-e5-small/tokenizer.json"));
        assert_eq!(config.dimension, 384);
    }
}
