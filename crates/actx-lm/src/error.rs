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
