//! Universal error types for actx-lm

use thiserror::Error;

/// Central error enum encompassing all network, authentication, decoding, and provider errors.
#[derive(Error, Debug)]
pub enum LmError {
    #[error("HTTP / Network transport error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication error for provider '{provider}': {message}")]
    Auth {
        provider: String,
        message: String,
    },

    #[error("Rate limit exceeded for provider '{provider}': {message}")]
    RateLimit {
        provider: String,
        message: String,
        retry_after_secs: Option<u64>,
    },

    #[error("Provider API error from '{provider}' (HTTP {status}): {message}")]
    ProviderError {
        provider: String,
        status: u16,
        message: String,
    },

    #[error("Failed to parse JSON / payload: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Stream decoding / SSE error: {0}")]
    StreamError(String),

    #[error("Invalid request parameters: {0}")]
    InvalidRequest(String),

    #[error("Unsupported model or provider configuration: {0}")]
    Unsupported(String),
}

impl LmError {
    /// Maps a failed chat response from a provider API to an error. A 404 names the model,
    /// because it usually means the provider no longer serves (or never knew) that model id.
    pub fn from_http_status(provider: &str, status: u16, model: &str, body: String) -> Self {
        match status {
            401 | 403 => LmError::Auth {
                provider: provider.to_string(),
                message: body,
            },
            429 => LmError::RateLimit {
                provider: provider.to_string(),
                message: body,
                retry_after_secs: None,
            },
            404 => LmError::ProviderError {
                provider: provider.to_string(),
                status,
                message: format!(
                    "model `{model}` is not available from {provider}; choose another with /model ({body})"
                ),
            },
            _ => LmError::ProviderError {
                provider: provider.to_string(),
                status,
                message: body,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_names_the_model_and_provider() {
        let err = LmError::from_http_status("anthropic", 404, "claude-3-7-sonnet", "{\"type\":\"error\"}".to_string());
        let text = err.to_string();
        assert!(text.contains("model `claude-3-7-sonnet` is not available from anthropic"), "{text}");
        assert!(text.contains("/model"));
    }

    #[test]
    fn auth_and_rate_limit_keep_their_kinds() {
        assert!(matches!(LmError::from_http_status("openai", 401, "m", String::new()), LmError::Auth { .. }));
        assert!(matches!(LmError::from_http_status("openai", 403, "m", String::new()), LmError::Auth { .. }));
        assert!(matches!(LmError::from_http_status("groq", 429, "m", String::new()), LmError::RateLimit { .. }));
        assert!(matches!(
            LmError::from_http_status("gemini", 500, "m", "boom".to_string()),
            LmError::ProviderError { status: 500, ref message, .. } if message == "boom"
        ));
    }
}
