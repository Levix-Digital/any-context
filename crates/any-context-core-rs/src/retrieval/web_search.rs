//! Live web search for AnyContext through allowed search APIs only (ADR-0152).
//!
//! The only provider is the Tavily API, with the user's key (`TAVILY_API_KEY` or `/keys tavily`).
//! Without a key web search is off and says so: there is no scraping fallback. Searches are
//! scoped to the workspace's web source domains first, then the open web.

use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::storage::sqlite::NativeConfigDb;

/// A single web search result item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Native web search orchestrator.
#[derive(Clone)]
pub struct NativeWebSearchEngine {
    client: reqwest::Client,
    db: Option<Arc<NativeConfigDb>>,
    key_resolver: Option<KeyResolver>,
}

/// Looks up a provider's API key; replaces the environment and settings lookup in tests.
pub type KeyResolver = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Returned when no allowed search API key is configured: web search is off, never a scraper.
pub const NO_SEARCH_PROVIDER: &str = "Web search is off: no search API key. Save a Tavily key with /keys tavily <key>.";

impl Default for NativeWebSearchEngine {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("AnyContext/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let db = NativeConfigDb::open_default().ok().map(Arc::new);
        Self { client, db, key_resolver: None }
    }
}

impl NativeWebSearchEngine {
    pub fn new(db: Option<Arc<NativeConfigDb>>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("AnyContext/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        Self { client, db, key_resolver: None }
    }

    pub fn with_key_resolver(mut self, resolver: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Self {
        self.key_resolver = Some(Arc::new(resolver));
        self
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Resolves an API key checking environment variables first, then SQLite settings.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        if let Some(ref resolver) = self.key_resolver {
            return resolver(provider);
        }
        let env_key = match provider {
            "tavily" => std::env::var("TAVILY_API_KEY").ok(),
            _ => None,
        };

        if let Some(k) = env_key {
            if !k.trim().is_empty() {
                return Some(k.trim().to_string());
            }
        }

        if let Some(ref db) = self.db {
            if let Ok(Some(k)) = db.get_api_key(provider) {
                if !k.trim().is_empty() {
                    return Some(k.trim().to_string());
                }
            }
        }

        None
    }

    /// Searches the workspace's domains first, then the open web. Fails with
    /// `NO_SEARCH_PROVIDER`, before any request, when no allowed key is configured.
    pub async fn search(
        &self,
        query: &str,
        domains: &[String],
        max_results: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        let clean_query = query.trim();
        if clean_query.is_empty() {
            return Ok(Vec::new());
        }
        let Some(key) = self.resolve_api_key("tavily") else {
            return Err(NO_SEARCH_PROVIDER.to_string());
        };

        let limit = max_results.clamp(1, 10);
        if !domains.is_empty() {
            let scoped = self.search_tavily(&key, clean_query, domains, limit).await?;
            if !scoped.is_empty() {
                return Ok(scoped);
            }
        }
        self.search_tavily(&key, clean_query, &[], limit).await
    }

    /// Searches using Tavily API.
    async fn search_tavily(
        &self,
        api_key: &str,
        query: &str,
        domains: &[String],
        max_results: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        let mut body = serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": max_results,
        });

        if !domains.is_empty() {
            body["include_domains"] = serde_json::json!(domains);
        }

        let resp = self
            .client
            .post("https://api.tavily.com/search")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Tavily HTTP error: {}", resp.status()));
        }

        let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        if let Some(items) = val.get("results").and_then(|r| r.as_array()) {
            for item in items {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("Web Result").to_string();
                let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
                let snippet = item
                    .get("content")
                    .or_else(|| item.get("snippet"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                if !url.is_empty() {
                    results.push(WebSearchResult { title, url, snippet });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression BUG-029: without an allowed search API key there is no request at all (no
    /// scraping fallback), and the error says how to turn web search on. A Serper key does
    /// not count: Serper resells Google result pages (ADR-0152).
    #[tokio::test]
    async fn search_without_a_key_makes_no_request_and_says_so() {
        for stored in [None, Some("serper")] {
            let engine = NativeWebSearchEngine::new(None)
                .with_key_resolver(move |provider| (Some(provider) == stored).then(|| "test-key".to_string()))
                .with_client(unroutable_client());
            let err = engine.search("rust release notes", &[], 5).await.expect_err("no provider");
            assert_eq!(err, NO_SEARCH_PROVIDER, "stored key: {stored:?}");
        }
    }

    /// A client whose every request fails at once, so a test sees an attempted request as an
    /// HTTP error instead of a network call.
    fn unroutable_client() -> reqwest::Client {
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:9").unwrap())
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
    }
}
