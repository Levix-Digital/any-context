//! Embedding model resolution shared by ingestion and retrieval.
//!
//! A workspace's vectors are only comparable with query vectors produced by the same
//! provider and model, so both sides resolve the embedding target through this module.

use std::sync::Arc;

use actx_lm::providers::gemini::GeminiProvider;
use actx_lm::providers::mock::MockLmProvider;
use actx_lm::providers::openai::OpenAiCompatibleProvider;
use actx_lm::traits::LmProvider;

use crate::storage::sqlite::NativeConfigDb;

/// Concrete provider and model used to embed the texts of a workspace.
#[derive(Clone)]
pub struct EmbeddingTarget {
    pub provider: Arc<dyn LmProvider>,
    pub model: String,
}

impl EmbeddingTarget {
    /// Identity of the embedding space: texts embedded under the same key are comparable.
    pub fn space_key(&self) -> (String, String) {
        (self.provider.provider_id().to_string(), self.model.clone())
    }
}

/// Resolves the embedding target of `workspace`.
///
/// Model precedence: `model_override` > `ACTX_EMBEDDING_MODEL` > `ACTX_MODEL` > workspace model >
/// default model > `text-embedding-3-small`.
pub fn resolve_embedding_target(
    db: &NativeConfigDb,
    model_override: Option<&str>,
    workspace: &str,
) -> EmbeddingTarget {
    let model = model_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ACTX_EMBEDDING_MODEL").ok())
        .or_else(|| std::env::var("ACTX_MODEL").ok())
        .or_else(|| db.get_workspace_model(workspace).ok())
        .or_else(|| db.get_default_model().ok())
        .unwrap_or_else(|| "text-embedding-3-small".to_string());

    embedding_target_for_model(db, &model)
}

/// Maps a configured (chat or embedding) model name to the embedding provider strategy.
pub fn embedding_target_for_model(db: &NativeConfigDb, model: &str) -> EmbeddingTarget {
    let raw = model.trim().to_lowercase();

    if raw.starts_with("mock") {
        return target(Arc::new(MockLmProvider::new()), "mock-embed");
    }

    if raw.starts_with("gemini") {
        if let Ok(Some(key)) = db.get_api_key("gemini") {
            return target(Arc::new(GeminiProvider::new(key)), "text-embedding-004");
        }
    }

    if raw.starts_with("ollama") || raw.starts_with("local") {
        let provider = OpenAiCompatibleProvider::new("ollama", "http://localhost:11434/v1".to_string(), None);
        return target(Arc::new(provider), "nomic-embed-text");
    }

    // Default OpenAI embedding provider
    if let Ok(Some(key)) = db.get_api_key("openai") {
        let provider = OpenAiCompatibleProvider::new("openai", "https://api.openai.com/v1".to_string(), Some(key));
        return target(Arc::new(provider), "text-embedding-3-small");
    }

    // Deterministic fallback mock provider
    target(Arc::new(MockLmProvider::new()), "text-embedding-3-small")
}

fn target(provider: Arc<dyn LmProvider>, model: &str) -> EmbeddingTarget {
    EmbeddingTarget { provider, model: model.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(db: &NativeConfigDb, model: &str) -> (String, String) {
        embedding_target_for_model(db, model).space_key()
    }

    #[test]
    fn test_mock_model_uses_mock_embeddings() {
        let db = NativeConfigDb::open_in_memory().unwrap();
        assert_eq!(space(&db, "mock-model").1, "mock-embed");
    }

    #[test]
    fn test_ollama_and_local_models_use_nomic_embeddings() {
        let db = NativeConfigDb::open_in_memory().unwrap();
        for model in ["ollama/llama3", "local/qwen"] {
            assert_eq!(space(&db, model), ("ollama".to_string(), "nomic-embed-text".to_string()));
        }
    }

    #[test]
    fn test_gemini_model_uses_gemini_embeddings_when_key_exists() {
        let db = NativeConfigDb::open_in_memory().unwrap();
        db.set_api_key("gemini", "test-gemini-key").unwrap();
        assert_eq!(space(&db, "gemini-2.5-flash"), ("gemini".to_string(), "text-embedding-004".to_string()));
    }

    #[test]
    fn test_non_embedding_chat_models_use_openai_embeddings_when_key_exists() {
        let db = NativeConfigDb::open_in_memory().unwrap();
        db.set_api_key("openai", "test-openai-key").unwrap();
        for model in ["claude-sonnet-4", "gpt-4o-mini", "gemma-4-e2b"] {
            assert_eq!(space(&db, model), ("openai".to_string(), "text-embedding-3-small".to_string()), "{model}");
        }
    }

    #[test]
    fn test_without_keys_falls_back_to_mock_provider() {
        if std::env::var("OPENAI_API_KEY").is_ok_and(|k| !k.trim().is_empty()) {
            eprintln!("skipping: OPENAI_API_KEY is set in the environment");
            return;
        }
        let db = NativeConfigDb::open_in_memory().unwrap();
        let t = embedding_target_for_model(&db, "claude-sonnet-4");
        assert_eq!(t.model, "text-embedding-3-small");
        assert_eq!(t.provider.provider_id(), "mock");
    }
}
