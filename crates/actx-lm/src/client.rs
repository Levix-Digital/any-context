//! Unified Façade client and fluent builder (Façade Pattern)

use std::sync::Arc;
use crate::error::LmError;
use crate::providers::ProviderKind;
use crate::traits::{BoxedChunkStream, LmProvider};
use crate::types::{ChatMessage, ChatRequest, ChatResponse};

/// Primary entrypoint and Façade for dispatching language model generation requests
#[derive(Clone)]
pub struct LmClient {
    provider: Arc<dyn LmProvider>,
    default_model: Option<String>,
}

impl LmClient {
    /// Create a new client builder
    pub fn builder() -> LmClientBuilder {
        LmClientBuilder::new()
    }

    /// Construct directly from an existing provider strategy
    pub fn new(provider: Arc<dyn LmProvider>) -> Self {
        Self {
            provider,
            default_model: None,
        }
    }

    /// Access the underlying provider strategy implementation
    pub fn provider(&self) -> &Arc<dyn LmProvider> {
        &self.provider
    }

    /// Unique identifier of the configured provider (e.g. "openai", "anthropic", "gemini", "ollama")
    pub fn provider_id(&self) -> &'static str {
        self.provider.provider_id()
    }

    /// Complete a chat session non-streamingly
    pub async fn chat_complete(&self, mut request: ChatRequest) -> Result<ChatResponse, LmError> {
        if request.model.is_empty() {
            if let Some(ref def) = self.default_model {
                request.model = def.clone();
            } else {
                return Err(LmError::InvalidRequest("No model specified in ChatRequest and no default model set on LmClient".to_string()));
            }
        }
        self.provider.chat_complete(request).await
    }

    /// Stream a chat session token-by-token via Server-Sent Events (SSE)
    pub async fn chat_stream(&self, mut request: ChatRequest) -> Result<BoxedChunkStream, LmError> {
        if request.model.is_empty() {
            if let Some(ref def) = self.default_model {
                request.model = def.clone();
            } else {
                return Err(LmError::InvalidRequest("No model specified in ChatRequest and no default model set on LmClient".to_string()));
            }
        }
        self.provider.chat_stream(request).await
    }

    /// Generate vector embeddings for text chunks
    pub async fn embed(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
        self.provider.embed(model, texts).await
    }

    /// Fast one-shot helper for single prompt execution
    pub async fn quick_chat(&self, model: &str, prompt: &str) -> Result<String, LmError> {
        let req = ChatRequest::new(model, vec![ChatMessage::user(prompt)]);
        let resp = self.chat_complete(req).await?;
        Ok(resp.content)
    }
}

/// Fluent builder for constructing an [`LmClient`]
#[derive(Default)]
pub struct LmClientBuilder {
    provider_kind: Option<ProviderKind>,
    custom_provider: Option<Arc<dyn LmProvider>>,
    api_key: Option<String>,
    base_url: Option<String>,
    default_model: Option<String>,
}

impl LmClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select standard provider target
    pub fn provider_kind(mut self, kind: ProviderKind) -> Self {
        self.provider_kind = Some(kind);
        self
    }

    /// Supply a fully custom provider instance (implements [`LmProvider`])
    pub fn custom_provider(mut self, provider: Arc<dyn LmProvider>) -> Self {
        self.custom_provider = Some(provider);
        self
    }

    /// Set provider API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Read API key from an environment variable if present
    pub fn api_key_from_env(mut self, env_var: &str) -> Self {
        if let Ok(val) = std::env::var(env_var) {
            if !val.trim().is_empty() {
                self.api_key = Some(val.trim().to_string());
            }
        }
        self
    }

    /// Override API endpoint base URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Configure a fallback model identifier
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// Assemble the configured [`LmClient`]
    pub fn build(self) -> Result<LmClient, LmError> {
        let provider: Arc<dyn LmProvider> = if let Some(custom) = self.custom_provider {
            custom
        } else if let Some(kind) = self.provider_kind {
            kind.build(self.api_key, self.base_url)?
        } else {
            return Err(LmError::InvalidRequest(
                "Neither provider_kind nor custom_provider was set in LmClientBuilder".to_string(),
            ));
        };

        Ok(LmClient {
            provider,
            default_model: self.default_model,
        })
    }
}
