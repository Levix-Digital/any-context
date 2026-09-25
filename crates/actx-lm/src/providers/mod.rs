//! Provider implementations, factory, and supported provider taxonomy

pub mod anthropic;
pub mod gemini;
pub mod mock;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use mock::MockLmProvider;
pub use openai::OpenAiCompatibleProvider;

use std::sync::Arc;
use crate::error::LmError;
use crate::traits::LmProvider;

/// Enumeration of all supported model provider targets
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    /// Official OpenAI Cloud
    OpenAi,
    /// Anthropic Claude Messages API
    Anthropic,
    /// Google Gemini Developer API
    Gemini,
    /// Local SLMs via Ollama (default http://localhost:11434/v1)
    Ollama { base_url: Option<String> },
    /// Groq Cloud LPU acceleration
    Groq,
    /// DeepSeek Cloud API
    DeepSeek,
    /// OpenRouter Unified Gateway
    OpenRouter,
    /// Arbitrary custom OpenAI-compatible server (LM Studio, vLLM, LocalAI)
    CustomCompatible { name: String, base_url: String },
    /// Deterministic offline mock provider
    Mock,
}

impl ProviderKind {
    /// Instantiate the concrete provider strategy based on kind and credentials
    pub fn build(
        &self,
        api_key: Option<String>,
        base_url_override: Option<String>,
    ) -> Result<Arc<dyn LmProvider>, LmError> {
        match self {
            ProviderKind::Mock => Ok(Arc::new(MockLmProvider::new())),
            ProviderKind::OpenAi => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "openai".to_string(),
                    message: "OpenAI requires an API key".to_string(),
                })?;
                let base = base_url_override.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let provider = OpenAiCompatibleProvider::new("openai", base, Some(key));
                Ok(Arc::new(provider))
            }
            ProviderKind::Anthropic => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "anthropic".to_string(),
                    message: "Anthropic requires an API key".to_string(),
                })?;
                let mut provider = AnthropicProvider::new(key);
                if let Some(base) = base_url_override {
                    provider = provider.with_base_url(base);
                }
                Ok(Arc::new(provider))
            }
            ProviderKind::Gemini => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "gemini".to_string(),
                    message: "Gemini requires an API key".to_string(),
                })?;
                let mut provider = GeminiProvider::new(key);
                if let Some(base) = base_url_override {
                    provider = provider.with_base_url(base);
                }
                Ok(Arc::new(provider))
            }
            ProviderKind::Ollama { base_url } => {
                let url = base_url_override
                    .or_else(|| base_url.clone())
                    .unwrap_or_else(|| "http://localhost:11434/v1".to_string());
                Ok(Arc::new(OpenAiCompatibleProvider::ollama(Some(url))))
            }
            ProviderKind::Groq => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "groq".to_string(),
                    message: "Groq requires an API key".to_string(),
                })?;
                Ok(Arc::new(OpenAiCompatibleProvider::groq(key)))
            }
            ProviderKind::DeepSeek => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "deepseek".to_string(),
                    message: "DeepSeek requires an API key".to_string(),
                })?;
                Ok(Arc::new(OpenAiCompatibleProvider::deepseek(key)))
            }
            ProviderKind::OpenRouter => {
                let key = api_key.ok_or_else(|| LmError::Auth {
                    provider: "openrouter".to_string(),
                    message: "OpenRouter requires an API key".to_string(),
                })?;
                Ok(Arc::new(OpenAiCompatibleProvider::openrouter(key)))
            }
            ProviderKind::CustomCompatible { name, base_url } => {
                let url = base_url_override.unwrap_or_else(|| base_url.clone());
                Ok(Arc::new(OpenAiCompatibleProvider::new(name, url, api_key)))
            }
        }
    }
}
