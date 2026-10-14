//! 100% Native Rust Jina Reader (`r.jina.ai`) Extractor.
//!
//! Provides high-fidelity Markdown extraction for documentation portals and web resources,
//! rendering modern SPAs (React, Vue, Next.js, Docusaurus) in clean, structured Markdown
//! without needing heavyweight headless browsers (Chromium/Puppeteer) on the local host.
//!
//! Adheres strictly to ADR-110 (Zero-Breakage & Pure Rust Fallback Guarantee).

use std::time::Duration;

/// Configuration parameters for the Jina Reader extractor.
#[derive(Debug, Clone)]
pub struct JinaExtractorConfig {
    /// Request timeout in seconds (default: 15).
    pub timeout_secs: u64,
    /// Optional JINA_API_KEY for higher rate limits.
    pub api_key: Option<String>,
    /// Whether to request generated alt text for images.
    pub with_generated_alt: bool,
    /// Maximum character limit for extracted content (default: 50,000).
    pub max_chars: usize,
}

impl Default for JinaExtractorConfig {
    fn default() -> Self {
        let api_key = std::env::var("JINA_API_KEY").ok().filter(|s| !s.trim().is_empty());
        Self {
            timeout_secs: 15,
            api_key,
            with_generated_alt: true,
            max_chars: 50_000,
        }
    }
}

/// Native Jina Reader extractor for documentation and web sources.
#[derive(Clone)]
pub struct NativeJinaExtractor {
    config: JinaExtractorConfig,
    client: reqwest::Client,
}

impl NativeJinaExtractor {
    /// Creates a new extractor with the provided configuration.
    pub fn new(config: JinaExtractorConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| format!("Failed to initialize Jina HTTP client: {e}"))?;

        Ok(Self { config, client })
    }

    /// Creates a new extractor with default settings and optional environment key.
    pub fn new_default() -> Result<Self, String> {
        Self::new(JinaExtractorConfig::default())
    }

    /// Access the configuration.
    pub fn config(&self) -> &JinaExtractorConfig {
        &self.config
    }

    /// Extracts clean, structured Markdown from `target_url` using `https://r.jina.ai/{url}`.
    pub async fn extract_markdown(&self, target_url: &str) -> Result<String, String> {
        let clean_url = target_url.trim();
        if clean_url.is_empty() {
            return Err("Empty target URL provided to Jina extractor.".to_string());
        }

        let jina_endpoint = format!("https://r.jina.ai/{}", clean_url);
        let mut req = self.client.get(&jina_endpoint)
            .header(reqwest::header::ACCEPT, "text/markdown");

        if self.config.with_generated_alt {
            req = req.header("X-With-Generated-Alt", "true");
        }

        if let Some(ref key) = self.config.api_key {
            req = req.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", key.trim()));
        }

        let resp = req.send().await
            .map_err(|e| format!("Network error calling Jina Reader for '{}': {}", clean_url, e))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("Jina Reader returned HTTP error {} for '{}'", status, clean_url));
        }

        let body = resp.text().await
            .map_err(|e| format!("Failed to read response body from Jina Reader for '{}': {}", clean_url, e))?;

        if body.trim().is_empty() {
            return Err(format!("Jina Reader returned empty body for '{}'", clean_url));
        }

        // Apply defensive character limit
        let truncated = if body.len() > self.config.max_chars {
            let mut end = self.config.max_chars;
            while end > 0 && !body.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}\n\n---\n*(Content truncated at {} characters)*", &body[..end], self.config.max_chars)
        } else {
            body
        };

        Ok(truncated)
    }

    /// Performs extraction with automatic fallback to native HTML tag-stripping (ADR-110).
    pub async fn extract_with_fallback(
        &self,
        target_url: &str,
        fallback_html: Option<&str>,
    ) -> (String, &'static str) {
        match self.extract_markdown(target_url).await {
            Ok(md) => (md, "jina"),
            Err(_) => {
                if let Some(html) = fallback_html {
                    let stripped = crate::ingestion::orchestrator::strip_html_tags(html);
                    (stripped, "native_fallback")
                } else {
                    (String::new(), "error")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jina_config_defaults() {
        let cfg = JinaExtractorConfig::default();
        assert_eq!(cfg.timeout_secs, 15);
        assert_eq!(cfg.max_chars, 50_000);
        assert!(cfg.with_generated_alt);
    }

    #[tokio::test]
    async fn test_empty_url_handling() {
        let extractor = NativeJinaExtractor::new_default().unwrap();
        let res = extractor.extract_markdown("   ").await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Empty target URL"));
    }

    #[tokio::test]
    async fn test_fallback_on_invalid_url() {
        let extractor = NativeJinaExtractor::new_default().unwrap();
        let sample_html = "<html><body><h1>Title</h1><p>Sample fallback text</p></body></html>";
        let (content, provider) = extractor.extract_with_fallback("http://invalid.local.domain.xyz.999", Some(sample_html)).await;
        assert_eq!(provider, "native_fallback");
        assert!(content.contains("Sample fallback text"));
    }
}
