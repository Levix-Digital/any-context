//! 100% Native Rust Web Search Engine for AnyContext.
//!
//! Provides internet search orchestration with automatic cascade across:
//! 1. Tavily API (if configured via TAVILY_API_KEY or SQLite credentials)
//! 2. Google Serper API (if configured via SERPER_API_KEY or SQLite credentials)
//! 3. DuckDuckGo Instant Answer & HTML Search (zero API-key required fallback)
//!
//! Supports domain filtering (e.g. scoping searches to official documentation domains)
//! with resilient cascade to the open web.

use std::sync::Arc;
use std::time::Duration;
use regex::Regex;
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
}

impl Default for NativeWebSearchEngine {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        let db = NativeConfigDb::open_default().ok().map(Arc::new);
        Self { client, db }
    }
}

impl NativeWebSearchEngine {
    pub fn new(db: Option<Arc<NativeConfigDb>>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self { client, db }
    }

    /// Resolves an API key checking environment variables first, then SQLite settings.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = match provider {
            "tavily" => std::env::var("TAVILY_API_KEY").ok(),
            "serper" => std::env::var("SERPER_API_KEY").ok(),
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

    /// Performs web search cascading across Tavily -> Serper -> DuckDuckGo.
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

        let limit = max_results.clamp(1, 10);

        // 1. Try Tavily if configured
        if let Some(key) = self.resolve_api_key("tavily") {
            if let Ok(results) = self.search_tavily(&key, clean_query, domains, limit).await {
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }

        // 2. Try Serper if configured
        if let Some(key) = self.resolve_api_key("serper") {
            if let Ok(results) = self.search_serper(&key, clean_query, domains, limit).await {
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }

        // 3. Resilient fallback: DuckDuckGo (no key required)
        self.search_duckduckgo(clean_query, domains, limit).await
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

    /// Searches using Serper API.
    async fn search_serper(
        &self,
        api_key: &str,
        query: &str,
        domains: &[String],
        max_results: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        let effective_query = if let Some(dom) = domains.first() {
            format!("site:{} {}", dom, query)
        } else {
            query.to_string()
        };

        let body = serde_json::json!({
            "q": effective_query,
            "num": max_results,
        });

        let resp = self
            .client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Serper HTTP error: {}", resp.status()));
        }

        let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        if let Some(items) = val.get("organic").and_then(|r| r.as_array()) {
            for item in items {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("Web Result").to_string();
                let url = item.get("link").and_then(|u| u.as_str()).unwrap_or("").to_string();
                let snippet = item.get("snippet").and_then(|s| s.as_str()).unwrap_or("").to_string();

                if !url.is_empty() {
                    results.push(WebSearchResult { title, url, snippet });
                }
            }
        }

        Ok(results)
    }

    /// Searches DuckDuckGo (zero-config fallback).
    async fn search_duckduckgo(
        &self,
        query: &str,
        domains: &[String],
        max_results: usize,
    ) -> Result<Vec<WebSearchResult>, String> {
        let effective_query = if let Some(dom) = domains.first() {
            format!("site:{} {}", dom, query)
        } else {
            query.to_string()
        };

        // 1. First attempt: Instant Answer API (very fast, JSON structured)
        let instant_url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding_encode(&effective_query)
        );

        let mut results = Vec::new();

        if let Ok(resp) = self.client.get(&instant_url).send().await {
            if resp.status().is_success() {
                if let Ok(val) = resp.json::<serde_json::Value>().await {
                    let abstract_text = val.get("AbstractText").and_then(|v| v.as_str()).unwrap_or("");
                    let abstract_url = val.get("AbstractURL").and_then(|v| v.as_str()).unwrap_or("");
                    let heading = val.get("Heading").and_then(|v| v.as_str()).unwrap_or("DuckDuckGo Instant Answer");

                    if !abstract_text.is_empty() && !abstract_url.is_empty() {
                        results.push(WebSearchResult {
                            title: heading.to_string(),
                            url: abstract_url.to_string(),
                            snippet: abstract_text.to_string(),
                        });
                    }

                    if let Some(related) = val.get("RelatedTopics").and_then(|v| v.as_array()) {
                        for item in related {
                            if results.len() >= max_results {
                                break;
                            }
                            let text = item.get("Text").and_then(|t| t.as_str()).unwrap_or("");
                            let url = item.get("FirstURL").and_then(|u| u.as_str()).unwrap_or("");
                            if !text.is_empty() && !url.is_empty() {
                                results.push(WebSearchResult {
                                    title: "DuckDuckGo Result".to_string(),
                                    url: url.to_string(),
                                    snippet: text.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if !results.is_empty() {
            return Ok(results);
        }

        // 2. Second attempt: DuckDuckGo HTML scraping
        let html_url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding_encode(&effective_query));
        let html_resp = match self.client.get(&html_url).send().await {
            Ok(r) => r,
            Err(e) => return Err(format!("DuckDuckGo connection failed: {e}")),
        };

        if !html_resp.status().is_success() {
            return Ok(results);
        }

        let html = html_resp.text().await.unwrap_or_default();
        let parsed = parse_duckduckgo_html(&html, max_results);

        // If domain-scoped search yielded 0 results, cascade to global open query
        if parsed.is_empty() && !domains.is_empty() {
            return Box::pin(self.search_duckduckgo(query, &[], max_results)).await;
        }

        Ok(parsed)
    }
}

/// Percent-encodes a query string for HTTP URLs.
fn urlencoding_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(b as char),
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

/// Parses DuckDuckGo HTML results extracting snippet, URL, and title.
pub fn parse_duckduckgo_html(html: &str, limit: usize) -> Vec<WebSearchResult> {
    let mut results = Vec::new();

    // Regex to extract result elements
    // DDG HTML results typically have:
    // <a class="result__snippet" ...>...</a>
    // <a class="result__url" href="...">...</a>
    let title_link_re = match Regex::new(r#"(?is)<h2 class="result__title">\s*<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let snippet_re = match Regex::new(r#"(?is)<a class="result__snippet"[^>]*>(.*?)</a>"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|c| crate::ingestion::orchestrator::strip_html_tags(c.get(1).map_or("", |m| m.as_str())))
        .collect();

    for (idx, cap) in title_link_re.captures_iter(html).enumerate() {
        if results.len() >= limit {
            break;
        }

        let raw_url = cap.get(1).map_or("", |m| m.as_str());
        let raw_title = cap.get(2).map_or("", |m| m.as_str());

        // Extract actual target from DuckDuckGo redirect /l/?uddg=URL
        let resolved_url = if raw_url.contains("uddg=") {
            raw_url
                .split("uddg=")
                .nth(1)
                .and_then(|u| u.split('&').next())
                .and_then(|u| percent_decode(u).ok())
                .unwrap_or_else(|| raw_url.to_string())
        } else if raw_url.starts_with("//") {
            format!("https:{}", raw_url)
        } else {
            raw_url.to_string()
        };

        let clean_title = crate::ingestion::orchestrator::strip_html_tags(raw_title);
        let snippet = snippets.get(idx).cloned().unwrap_or_default();

        if !resolved_url.is_empty() && (resolved_url.starts_with("http://") || resolved_url.starts_with("https://")) {
            results.push(WebSearchResult {
                title: if clean_title.is_empty() { "Web Result".to_string() } else { clean_title },
                url: resolved_url,
                snippet,
            });
        }
    }

    results
}

fn percent_decode(input: &str) -> Result<String, ()> {
    let mut bytes = Vec::new();
    let mut chars = input.bytes();

    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().ok_or(())?;
            let h2 = chars.next().ok_or(())?;
            let hex_bytes = [h1, h2];
            let hex_str = std::str::from_utf8(&hex_bytes).map_err(|_| ())?;
            let val = u8::from_str_radix(hex_str, 16).map_err(|_| ())?;
            bytes.push(val);
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }

    String::from_utf8(bytes).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duckduckgo_html() {
        let sample_html = r##"
        <div class="result results_links results_links_deep web-result">
            <h2 class="result__title">
                <a class="result__a" href="/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&rut=1">Rust Programming Language</a>
            </h2>
            <a class="result__snippet" href="#">Empowering everyone to build reliable and efficient software.</a>
        </div>
        <div class="result results_links results_links_deep web-result">
            <h2 class="result__title">
                <a class="result__a" href="https://doc.rust-lang.org/book/">The Rust Programming Language Book</a>
            </h2>
            <a class="result__snippet" href="#">Official book covering rust ownership, borrowing, and concurrency.</a>
        </div>
        "##;

        let results = parse_duckduckgo_html(sample_html, 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Rust Programming Language");
        assert_eq!(results[0].url, "https://www.rust-lang.org/");
        assert!(results[0].snippet.contains("Empowering everyone"));

        assert_eq!(results[1].title, "The Rust Programming Language Book");
        assert_eq!(results[1].url, "https://doc.rust-lang.org/book/");
    }

    #[test]
    fn test_urlencoding_and_decoding() {
        let original = "https://example.com/search?q=rust + cargo & test=1";
        let encoded = urlencoding_encode(original);
        assert!(encoded.contains("%3A"));
        assert!(encoded.contains("%2F"));
    }
}
