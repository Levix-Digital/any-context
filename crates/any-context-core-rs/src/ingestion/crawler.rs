//! 100% Native Rust Asynchronous Web Crawler & Documentation Ingestor.
//!
//! Provides recursive BFS crawling, XML sitemap auto-discovery and parsing,
//! RFC-compliant robots.txt policy evaluation, fast HTML link extraction,
//! and clean text content extraction. Zero external Python dependencies.

use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;
use url::Url;

/// Represents an entry discovered inside an XML sitemap or sitemap index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapEntry {
    pub loc: String,
    pub lastmod: Option<String>,
}

/// A parsed web page with extracted text and metadata.
#[derive(Debug, Clone)]
pub struct CrawledPage {
    pub url: String,
    pub title: String,
    pub text: String,
    pub lastmod: Option<String>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

/// Metadata extracted from HTML meta tags, headers, JSON-LD schema, and URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebMetadata {
    pub last_modified: Option<String>,
    pub date_confidence: String,
    pub content_type: String,
}

/// Result of checking whether a URL was modified via HTTP HEAD / Conditional GET (RFC 7232).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlCheckResult {
    NotModified,
    Modified { last_modified: Option<String>, etag: Option<String> },
    Error(String),
}

/// Extracts temporal publication/modification date and content classification from HTML, headers, and URL.
pub fn extract_web_metadata(
    html: &str,
    url: &str,
    headers: Option<&reqwest::header::HeaderMap>,
) -> WebMetadata {
    let mut last_modified: Option<String> = None;
    let mut date_confidence = "none".to_string();

    // 1. Meta tags (Highest confidence)
    let meta_regexes = [
        r#"(?i)<meta[^>]+(?:property|name)=["'](?:article:modified_time|dcterms\.modified|dc\.date\.modified)["'][^>]+content=["']([^"']+)["']"#,
        r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+(?:property|name)=["'](?:article:modified_time|dcterms\.modified|dc\.date\.modified)["']"#,
        r#"(?i)<meta[^>]+(?:property|name)=["'](?:article:published_time|dcterms\.issued|dc\.date\.issued|dc\.date)["'][^>]+content=["']([^"']+)["']"#,
        r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+(?:property|name)=["'](?:article:published_time|dcterms\.issued|dc\.date\.issued|dc\.date)["']"#,
        r#"(?i)"dateModified"\s*:\s*"([^"]+)""#,
        r#"(?i)"datePublished"\s*:\s*"([^"]+)""#,
        r#"(?i)<time[^>]+datetime=["']([^"']+)["']"#,
    ];

    let date_clean_re = Regex::new(r"(\d{4}-\d{2}-\d{2})").unwrap();

    for pat in &meta_regexes {
        if let Ok(re) = Regex::new(pat) {
            if let Some(cap) = re.captures(html) {
                if let Some(val) = cap.get(1) {
                    if let Some(d_match) = date_clean_re.captures(val.as_str()) {
                        last_modified = Some(d_match.get(1).unwrap().as_str().to_string());
                        date_confidence = "meta_tag".to_string();
                        break;
                    }
                }
            }
        }
    }

    // 2. In-page text / footer patterns
    if last_modified.is_none() {
        let text_patterns = [
            r#"(?i)Page details\s*(\d{4}-\d{2}-\d{2})"#,
            r#"(?i)Date modified:\s*(\d{4}-\d{2}-\d{2})"#,
            r#"(?i)Last modified:\s*(\d{4}-\d{2}-\d{2})"#,
            r#"(?i)Last updated:\s*(\d{4}-\d{2}-\d{2})"#,
            r#"(?i)Updated:\s*(\d{4}-\d{2}-\d{2})"#,
        ];
        for pat in &text_patterns {
            if let Ok(re) = Regex::new(pat) {
                if let Some(cap) = re.captures(html) {
                    if let Some(val) = cap.get(1) {
                        last_modified = Some(val.as_str().to_string());
                        date_confidence = "footer_pattern".to_string();
                        break;
                    }
                }
            }
        }
    }

    // 3. URL Date Pattern (e.g. /2024/06/15/ or /2024/06/)
    if last_modified.is_none() {
        if let Ok(url_re) = Regex::new(r"/(20\d{2})/(0[1-9]|1[0-2])(?:/([0-3]\d))?/") {
            if let Some(cap) = url_re.captures(url) {
                let y = cap.get(1).map_or("2026", |m| m.as_str());
                let m = cap.get(2).map_or("01", |m| m.as_str());
                let d = cap.get(3).map_or("01", |m| m.as_str());
                last_modified = Some(format!("{}-{}-{}", y, m, d));
                date_confidence = "url_pattern".to_string();
            }
        }
    }

    // 4. HTTP Headers (Last-Modified)
    if last_modified.is_none() {
        if let Some(h) = headers {
            if let Some(lm_val) = h.get(reqwest::header::LAST_MODIFIED).and_then(|v| v.to_str().ok()) {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(lm_val) {
                    last_modified = Some(dt.format("%Y-%m-%d").to_string());
                    date_confidence = "http_header".to_string();
                }
            }
        }
    }

    // 5. Fallback: current crawl timestamp
    if last_modified.is_none() {
        last_modified = Some(chrono::Utc::now().format("%Y-%m-%d").to_string());
        date_confidence = "crawl_timestamp".to_string();
    }

    // Content Type Classification
    let url_lower = url.to_ascii_lowercase();
    let content_type = if url_lower.contains("/news/") || url_lower.contains("/blog/") || url_lower.contains("/press/") || url_lower.contains("/announcement") {
        "News & Announcements".to_string()
    } else if url_lower.contains("/docs/") || url_lower.contains("/doc/") || url_lower.contains("/guide/") || url_lower.contains("/manual/") || url_lower.contains("/api/") {
        "Technical Documentation".to_string()
    } else {
        "Web Documentation".to_string()
    };

    WebMetadata {
        last_modified,
        date_confidence,
        content_type,
    }
}

/// RFC 9309 compliant robots.txt policy manager.
#[derive(Debug, Clone, Default)]
pub struct RobotsPolicy {
    disallowed_prefixes: Vec<String>,
    allowed_prefixes: Vec<String>,
}

impl RobotsPolicy {
    /// Parses a raw robots.txt string targeting AnyContext or universal user agents (*).
    pub fn parse(content: &str) -> Self {
        let mut policy = Self::default();
        let mut active_agent_relevant = false;

        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ':').map(|s| s.trim()).collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].to_ascii_lowercase();
            let value = parts[1];

            if key == "user-agent" {
                let agent = value.to_ascii_lowercase();
                active_agent_relevant = agent == "*" || agent.contains("anycontext");
            } else if active_agent_relevant {
                if key == "disallow" {
                    if !value.is_empty() {
                        policy.disallowed_prefixes.push(value.to_string());
                    }
                } else if key == "allow" {
                    if !value.is_empty() {
                        policy.allowed_prefixes.push(value.to_string());
                    }
                }
            }
        }

        policy
    }

    /// Checks if a given path or URL is permitted by this robots.txt policy.
    pub fn is_allowed(&self, path_or_url: &str) -> bool {
        let path = if let Ok(u) = Url::parse(path_or_url) {
            u.path().to_string()
        } else {
            path_or_url.to_string()
        };

        // Longest match wins between Allow and Disallow
        let mut matched_disallow: Option<&str> = None;
        let mut matched_allow: Option<&str> = None;

        for dis in &self.disallowed_prefixes {
            if path.starts_with(dis) {
                if matched_disallow.map_or(true, |m| dis.len() > m.len()) {
                    matched_disallow = Some(dis);
                }
            }
        }

        for al in &self.allowed_prefixes {
            if path.starts_with(al) {
                if matched_allow.map_or(true, |m| al.len() > m.len()) {
                    matched_allow = Some(al);
                }
            }
        }

        match (matched_allow, matched_disallow) {
            (Some(a), Some(d)) => a.len() >= d.len(),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    }
}

/// Extracts all valid anchor hrefs from HTML and resolves them relative to base URL.
pub fn extract_html_links(base_url: &Url, html: &str) -> Vec<Url> {
    let re = match Regex::new(r#"(?i)<a\s+[^>]*?href\s*=\s*["']([^"'\s>]+)["']"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let host = base_url.host_str().unwrap_or("").to_ascii_lowercase();
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();

    let binary_extensions = [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".ico", ".bmp",
        ".pdf", ".zip", ".tar", ".gz", ".7z", ".rar", ".exe", ".bin",
        ".mp3", ".mp4", ".wav", ".avi", ".mov", ".webm", ".woff", ".woff2", ".ttf",
    ];

    for cap in re.captures_iter(html) {
        if let Some(href_match) = cap.get(1) {
            let mut href = href_match.as_str().trim();
            if href.starts_with("javascript:") || href.starts_with("mailto:") || href.starts_with("tel:") {
                continue;
            }

            // Strip fragments
            if let Some(pos) = href.find('#') {
                href = &href[..pos];
            }
            if href.is_empty() {
                continue;
            }

            if let Ok(mut u) = base_url.join(href) {
                if u.scheme() != "http" && u.scheme() != "https" {
                    continue;
                }
                // Only crawl within the same host
                if let Some(target_host) = u.host_str() {
                    if target_host.to_ascii_lowercase() != host {
                        continue;
                    }
                } else {
                    continue;
                }

                u.set_fragment(None);
                let path_lower = u.path().to_ascii_lowercase();
                if binary_extensions.iter().any(|ext| path_lower.ends_with(ext)) {
                    continue;
                }

                let norm_url = u.to_string();
                if seen.insert(norm_url) {
                    resolved.push(u);
                }
            }
        }
    }

    resolved
}

/// Parses an XML sitemap or sitemap index, returning discovered page URLs and sub-sitemaps.
pub fn parse_sitemap_xml(xml: &str) -> (Vec<SitemapEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut sub_sitemaps = Vec::new();

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_tag = String::new();
    let mut inside_url = false;
    let mut inside_sitemap = false;
    let mut current_loc: Option<String> = None;
    let mut current_lastmod: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_ascii_lowercase();
                current_tag = name.clone();
                if name == "url" {
                    inside_url = true;
                    current_loc = None;
                    current_lastmod = None;
                } else if name == "sitemap" {
                    inside_sitemap = true;
                    current_loc = None;
                    current_lastmod = None;
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_ascii_lowercase();
                if name == "url" {
                    if let Some(loc) = current_loc.take() {
                        entries.push(SitemapEntry {
                            loc,
                            lastmod: current_lastmod.take(),
                        });
                    }
                    inside_url = false;
                } else if name == "sitemap" {
                    if let Some(loc) = current_loc.take() {
                        sub_sitemaps.push(loc);
                    }
                    inside_sitemap = false;
                }
                current_tag.clear();
            }
            Ok(Event::Text(e)) => {
                let text = match e.unescape() {
                    Ok(t) => t.trim().to_string(),
                    Err(_) => String::new(),
                };
                if !text.is_empty() {
                    if (inside_url || inside_sitemap) && current_tag == "loc" {
                        current_loc = Some(text);
                    } else if inside_url && current_tag == "lastmod" {
                        current_lastmod = Some(text);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    (entries, sub_sitemaps)
}

/// Configuration parameters for the recursive web crawler.
#[derive(Debug, Clone)]
pub struct CrawlerConfig {
    pub max_pages: usize,
    pub max_depth: usize,
    pub timeout_secs: u64,
    pub user_agent: String,
    pub respect_robots: bool,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 3,
            timeout_secs: 10,
            user_agent: "AnyContext-WebCrawler/0.33.0 (+https://levix-digital.github.io/any-context-releases/)".to_string(),
            respect_robots: true,
        }
    }
}

/// Native asynchronous web crawler for documentation portals and web resources.
pub struct NativeWebCrawler {
    config: CrawlerConfig,
    client: reqwest::Client,
}

impl NativeWebCrawler {
    pub fn new(config: CrawlerConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| format!("Failed to initialize crawler HTTP client: {e}"))?;

        Ok(Self { config, client })
    }

    pub fn new_default() -> Result<Self, String> {
        Self::new(CrawlerConfig::default())
    }

    /// Crawls the site starting from `start_url` using BFS, checking sitemaps and robots.txt.
    pub async fn crawl(&self, start_url: &str) -> Result<Vec<CrawledPage>, String> {
        let parsed_start = Url::parse(start_url)
            .map_err(|e| format!("Invalid start URL '{start_url}': {e}"))?;

        let scheme = parsed_start.scheme();
        let host = parsed_start.host_str().ok_or_else(|| "Start URL missing host".to_string())?;
        let domain_root = format!("{}://{}", scheme, host);

        // 1. Robots.txt evaluation
        let robots_policy = if self.config.respect_robots {
            self.fetch_robots_txt(&domain_root).await
        } else {
            RobotsPolicy::default()
        };

        let mut crawled_pages = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // 2. Discover Sitemaps
        let sitemap_pages = self.discover_sitemaps(&domain_root).await;
        for entry in sitemap_pages {
            if robots_policy.is_allowed(&entry.loc) {
                queue.push_back((entry.loc, 1, entry.lastmod));
            }
        }

        // Always ensure start_url is prioritized
        if !visited.contains(start_url) && robots_policy.is_allowed(start_url) {
            queue.push_front((start_url.to_string(), 0, None));
        }

        // 3. BFS Traversal
        while let Some((curr_url_str, depth, lastmod)) = queue.pop_front() {
            if crawled_pages.len() >= self.config.max_pages {
                break;
            }

            if !visited.insert(curr_url_str.clone()) {
                continue;
            }

            if !robots_policy.is_allowed(&curr_url_str) {
                continue;
            }

            let curr_url = match Url::parse(&curr_url_str) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let resp = match self.client.get(&curr_url_str).send().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let status = resp.status();
            if !status.is_success() {
                continue;
            }

            let headers = resp.headers().clone();
            let etag = headers.get(reqwest::header::ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
            let http_lastmod = headers.get(reqwest::header::LAST_MODIFIED).and_then(|v| v.to_str().ok()).map(|s| s.to_string());

            let content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_ascii_lowercase();

            // Only process text/html or text/plain
            if !content_type.is_empty() && !content_type.contains("text/html") && !content_type.contains("text/plain") {
                continue;
            }

            let html = match resp.text().await {
                Ok(t) => t,
                Err(_) => continue,
            };

            let meta = extract_web_metadata(&html, &curr_url_str, Some(&headers));
            let effective_lastmod = meta.last_modified
                .or(lastmod)
                .or(http_lastmod)
                .or_else(|| Some(chrono::Utc::now().to_rfc3339()));

            let title = extract_title(&html).unwrap_or_else(|| curr_url_str.clone());
            let clean_text = crate::ingestion::orchestrator::strip_html_tags(&html);

            if !clean_text.trim().is_empty() {
                crawled_pages.push(CrawledPage {
                    url: curr_url_str.clone(),
                    title,
                    text: clean_text,
                    lastmod: effective_lastmod,
                    etag,
                    content_type: Some(meta.content_type),
                });
            }

            // Extract outbound links if within max_depth
            if depth < self.config.max_depth && crawled_pages.len() < self.config.max_pages {
                let links = extract_html_links(&curr_url, &html);
                for l in links {
                    let l_str = l.to_string();
                    if !visited.contains(&l_str) && robots_policy.is_allowed(&l_str) {
                        queue.push_back((l_str, depth + 1, None));
                    }
                }
            }
        }

        Ok(crawled_pages)
    }

    /// Checks if a web page URL has changed without downloading the body (RFC 7232).
    /// Sends an HTTP HEAD request with If-None-Match and If-Modified-Since.
    pub async fn check_url_modified(
        &self,
        url: &str,
        cached_etag: Option<&str>,
        cached_last_modified: Option<&str>,
    ) -> Result<UrlCheckResult, String> {
        let mut req = self.client.head(url);
        if let Some(etag) = cached_etag {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(lm) = cached_last_modified {
            req = req.header(reqwest::header::IF_MODIFIED_SINCE, lm);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::NOT_MODIFIED {
                    return Ok(UrlCheckResult::NotModified);
                }
                let etag = resp.headers().get(reqwest::header::ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
                let last_mod = resp.headers().get(reqwest::header::LAST_MODIFIED).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
                Ok(UrlCheckResult::Modified { last_modified: last_mod, etag })
            }
            Err(e) => Err(format!("Network check error for '{url}': {e}")),
        }
    }

    /// Fetches and parses robots.txt for the given domain root.
    async fn fetch_robots_txt(&self, domain_root: &str) -> RobotsPolicy {
        let robots_url = format!("{}/robots.txt", domain_root.trim_end_matches('/'));
        if let Ok(resp) = self.client.get(&robots_url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    return RobotsPolicy::parse(&text);
                }
            }
        }
        RobotsPolicy::default()
    }

    /// Discovers and parses sitemaps at standard locations.
    async fn discover_sitemaps(&self, domain_root: &str) -> Vec<SitemapEntry> {
        let candidates = [
            format!("{}/sitemap.xml", domain_root.trim_end_matches('/')),
            format!("{}/sitemap_index.xml", domain_root.trim_end_matches('/')),
            format!("{}/sitemap/sitemap.xml", domain_root.trim_end_matches('/')),
        ];

        let mut all_entries = Vec::new();

        for candidate in candidates {
            if let Ok(resp) = self.client.get(&candidate).send().await {
                if resp.status().is_success() {
                    if let Ok(xml) = resp.text().await {
                        let (entries, subs) = parse_sitemap_xml(&xml);
                        all_entries.extend(entries);

                        // Follow sub-sitemaps (up to 5)
                        for sub in subs.into_iter().take(5) {
                            if let Ok(sub_resp) = self.client.get(&sub).send().await {
                                if sub_resp.status().is_success() {
                                    if let Ok(sub_xml) = sub_resp.text().await {
                                        let (sub_entries, _) = parse_sitemap_xml(&sub_xml);
                                        all_entries.extend(sub_entries);
                                    }
                                }
                            }
                            if all_entries.len() >= self.config.max_pages {
                                break;
                            }
                        }

                        if !all_entries.is_empty() {
                            break;
                        }
                    }
                }
            }
        }

        all_entries
    }
}

/// Extracts page title from <title> tag.
pub fn extract_title(html: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)<title\b[^>]*>(.*?)</title>"#).ok()?;
    let cap = re.captures(html)?;
    let title = cap.get(1)?.as_str().trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robots_policy_disallow_and_allow() {
        let robots_content = "
User-agent: *
Disallow: /admin
Disallow: /private/
Allow: /private/public-doc
Disallow: /secret.html
";
        let policy = RobotsPolicy::parse(robots_content);
        assert!(!policy.is_allowed("https://example.com/admin"));
        assert!(!policy.is_allowed("https://example.com/admin/dashboard"));
        assert!(!policy.is_allowed("https://example.com/private/data"));
        assert!(policy.is_allowed("https://example.com/private/public-doc"));
        assert!(!policy.is_allowed("https://example.com/secret.html"));
        assert!(policy.is_allowed("https://example.com/docs/intro"));
    }

    #[test]
    fn test_parse_sitemap_xml() {
        let sitemap_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/docs/getting-started</loc>
    <lastmod>2026-10-08T10:00:00Z</lastmod>
  </url>
  <url>
    <loc>https://example.com/docs/architecture</loc>
  </url>
</urlset>"#;

        let (entries, subs) = parse_sitemap_xml(sitemap_xml);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].loc, "https://example.com/docs/getting-started");
        assert_eq!(entries[0].lastmod, Some("2026-10-08T10:00:00Z".to_string()));
        assert_eq!(entries[1].loc, "https://example.com/docs/architecture");
        assert_eq!(entries[1].lastmod, None);
        assert!(subs.is_empty());
    }

    #[test]
    fn test_parse_sitemap_index_xml() {
        let index_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap>
    <loc>https://example.com/sitemaps/core.xml</loc>
  </sitemap>
  <sitemap>
    <loc>https://example.com/sitemaps/plugins.xml</loc>
  </sitemap>
</sitemapindex>"#;

        let (entries, subs) = parse_sitemap_xml(index_xml);
        assert!(entries.is_empty());
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0], "https://example.com/sitemaps/core.xml");
        assert_eq!(subs[1], "https://example.com/sitemaps/plugins.xml");
    }

    #[test]
    fn test_extract_html_links() {
        let base = Url::parse("https://example.com/docs/index.html").unwrap();
        let html = r#"
        <html>
            <body>
                <a href="/guide.html">Guide</a>
                <a href="deep/module.html#heading">Deep Module</a>
                <a href="https://external.com/api">External</a>
                <a href="image.png">Image</a>
                <a href="mailto:test@example.com">Email</a>
            </body>
        </html>
        "#;

        let links = extract_html_links(&base, html);
        let link_strs: Vec<String> = links.into_iter().map(|u| u.to_string()).collect();

        assert!(link_strs.contains(&"https://example.com/guide.html".to_string()));
        assert!(link_strs.contains(&"https://example.com/docs/deep/module.html".to_string()));
        assert!(!link_strs.iter().any(|s| s.contains("external.com")));
        assert!(!link_strs.iter().any(|s| s.ends_with(".png")));
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>  AnyContext Documentation  </title></head></html>";
        assert_eq!(extract_title(html), Some("AnyContext Documentation".to_string()));
    }

    #[test]
    fn test_extract_web_metadata_meta_tag() {
        let html = r#"<html><head><meta property="article:modified_time" content="2026-10-09T14:30:00Z"></head><body>Text</body></html>"#;
        let meta = extract_web_metadata(html, "https://example.com/docs/guide", None);
        assert_eq!(meta.last_modified, Some("2026-10-09".to_string()));
        assert_eq!(meta.date_confidence, "meta_tag");
        assert_eq!(meta.content_type, "Technical Documentation");
    }

    #[test]
    fn test_extract_web_metadata_json_ld() {
        let html = r#"<html><head><script type="application/ld+json">{"@context":"https://schema.org","dateModified":"2026-10-05T18:00:00Z"}</script></head><body>Text</body></html>"#;
        let meta = extract_web_metadata(html, "https://example.com/page", None);
        assert_eq!(meta.last_modified, Some("2026-10-05".to_string()));
        assert_eq!(meta.date_confidence, "meta_tag");
    }

    #[test]
    fn test_extract_web_metadata_footer_pattern() {
        let html = r#"<html><body><main>Content</main><footer>Last modified: 2026-09-25</footer></body></html>"#;
        let meta = extract_web_metadata(html, "https://example.com/page", None);
        assert_eq!(meta.last_modified, Some("2026-09-25".to_string()));
        assert_eq!(meta.date_confidence, "footer_pattern");
    }

    #[test]
    fn test_extract_web_metadata_url_pattern() {
        let html = r#"<html><body>Simple text without date tags</body></html>"#;
        let meta = extract_web_metadata(html, "https://example.com/2026/07/12/release-notes", None);
        assert_eq!(meta.last_modified, Some("2026-07-12".to_string()));
        assert_eq!(meta.date_confidence, "url_pattern");
    }
}
