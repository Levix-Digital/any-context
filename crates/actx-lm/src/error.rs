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

    /// Sets the wait a rate-limited provider asked for; other errors are returned unchanged.
    pub fn with_retry_after(self, secs: Option<u64>) -> Self {
        match self {
            LmError::RateLimit { provider, message, .. } => LmError::RateLimit { provider, message, retry_after_secs: secs },
            other => other,
        }
    }
}

/// Seconds from a `Retry-After` header given as a number (an HTTP date is ignored).
pub(crate) fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
}

/// One-request HTTP server for provider tests: answers `response` and returns the raw request.
#[cfg(test)]
pub(crate) fn serve_once(response: &'static str) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];
        // Read the headers and the body announced by Content-Length.
        loop {
            let n = stream.read(&mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&request).to_string();
            if let Some(end) = text.find("\r\n\r\n") {
                let length = text[..end]
                    .lines()
                    .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&request).to_string()
    });
    (base, handle)
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
        assert!(matches!(
            LmError::from_http_status("openai", 429, "m", String::new()).with_retry_after(Some(7)),
            LmError::RateLimit { retry_after_secs: Some(7), .. }
        ));
        assert!(matches!(
            LmError::from_http_status("openai", 401, "m", String::new()).with_retry_after(Some(7)),
            LmError::Auth { .. }
        ));
        assert!(matches!(LmError::from_http_status("groq", 429, "m", String::new()), LmError::RateLimit { .. }));
        assert!(matches!(
            LmError::from_http_status("gemini", 500, "m", "boom".to_string()),
            LmError::ProviderError { status: 500, ref message, .. } if message == "boom"
        ));
    }
}
