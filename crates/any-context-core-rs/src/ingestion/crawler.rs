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

            let content_type = resp
                .headers()
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

            let title = extract_title(&html).unwrap_or_else(|| curr_url_str.clone());
            let clean_text = crate::ingestion::orchestrator::strip_html_tags(&html);

            if !clean_text.trim().is_empty() {
                crawled_pages.push(CrawledPage {
                    url: curr_url_str.clone(),
                    title,
                    text: clean_text,
                    lastmod,
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
}
