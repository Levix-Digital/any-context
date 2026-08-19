import re
import hashlib
import urllib.request
import xml.etree.ElementTree as ET
from typing import Dict, Any, List, Optional
from html.parser import HTMLParser

class CleanHTMLTextExtractor(HTMLParser):
    """
    Lightweight HTML parser to extract clean human-readable text from web pages,
    stripping scripts, styles, navigation bars, headers, footers, and sidebars.
    """
    def __init__(self):
        super().__init__()
        self.reset()
        self.strict = False
        self.convert_charrefs = True
        self.text = []
        self.skip_tags = {
            "script", "style", "noscript", "svg", "head",
            "nav", "header", "footer", "aside", "form"
        }
        self.tag_stack = []
        self.block_tags = {"p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr", "div", "section", "article"}

    def handle_starttag(self, tag, attrs):
        t = tag.lower()
        self.tag_stack.append(t)
        if t in self.block_tags:
            self.text.append("\n")

    def handle_endtag(self, tag):
        t = tag.lower()
        if self.tag_stack and self.tag_stack[-1] == t:
            self.tag_stack.pop()
        elif t in self.tag_stack:
            while self.tag_stack:
                if self.tag_stack.pop() == t:
                    break
        if t in self.block_tags:
            self.text.append("\n")

    def handle_data(self, data):
        if any(t in self.skip_tags for t in self.tag_stack):
            return
        content = data.strip()
        if content:
            self.text.append(content)

    def get_text(self) -> str:
        raw = " ".join(self.text)
        # Normalize excessive newlines and spaces
        cleaned = re.sub(r"\n\s*\n+", "\n\n", raw)
        cleaned = re.sub(r"[ \t]+", " ", cleaned)
        return cleaned.strip()

import email.utils
from datetime import datetime, timezone

def extract_web_metadata(url: str, html_text: str, headers: dict = None) -> Dict[str, str]:
    """
    Extracts publication / last modified date and classifies content type from HTML, headers, and URL.
    Uses a multi-tier confidence fallback cascade.
    """
    last_modified = None
    date_confidence = "none"
    
    # 1. Meta tags (Highest confidence)
    meta_patterns = [
        (r'<meta[^>]+(?:property|name)=["\'](?:article:modified_time|dcterms\.modified|dc\.date\.modified)["\'][^>]+content=["\']([^"\']+)["\']', "high"),
        (r'<meta[^>]+content=["\']([^"\']+)["\'][^>]+(?:property|name)=["\'](?:article:modified_time|dcterms\.modified|dc\.date\.modified)["\']', "high"),
        (r'<meta[^>]+(?:property|name)=["\'](?:article:published_time|dcterms\.issued|dc\.date\.issued|dc\.date)["\'][^>]+content=["\']([^"\']+)["\']', "high"),
        (r'<meta[^>]+content=["\']([^"\']+)["\'][^>]+(?:property|name)=["\'](?:article:published_time|dcterms\.issued|dc\.date\.issued|dc\.date)["\']', "high"),
        (r'"dateModified":\s*"([^"]+)"', "high"),
        (r'"datePublished":\s*"([^"]+)"', "high"),
        (r'<time[^>]+datetime=["\']([^"\']+)["\']', "medium"),
    ]
    
    for pattern, conf in meta_patterns:
        match = re.search(pattern, html_text, re.IGNORECASE)
        if match:
            raw_val = match.group(1).strip()
            d_match = re.search(r"(\d{4}-\d{2}-\d{2})", raw_val)
            if d_match:
                last_modified = d_match.group(1)
                date_confidence = conf
                break

    # 2. In-page text / footer patterns
    if not last_modified:
        text_patterns = [
            (r'Page details\s*(\d{4}-\d{2}-\d{2})', "high"),
            (r'Date modified:\s*(\d{4}-\d{2}-\d{2})', "high"),
            (r'Last modified:\s*(\d{4}-\d{2}-\d{2})', "high"),
            (r'Last updated:\s*(\d{4}-\d{2}-\d{2})', "high"),
            (r'Updated:\s*(\d{4}-\d{2}-\d{2})', "high"),
        ]
        for pattern, conf in text_patterns:
            match = re.search(pattern, html_text, re.IGNORECASE)
            if match:
                last_modified = match.group(1)
                date_confidence = conf
                break

    # 3. URL Date Pattern (e.g. /2023/06/...)
    if not last_modified:
        url_match = re.search(r"/(20\d{2})/(0[1-9]|1[0-2])(?:/([0-3]\d))?/", url)
        if url_match:
            year = url_match.group(1)
            month = url_match.group(2)
            day = url_match.group(3) or "01"
            last_modified = f"{year}-{month}-{day}"
            date_confidence = "url_pattern"

    # 4. HTTP Headers
    if not last_modified and headers:
        http_lm = headers.get("Last-Modified") or headers.get("last-modified")
        if http_lm:
            try:
                parsed_tuple = email.utils.parsedate_tz(http_lm)
                if parsed_tuple:
                    dt = datetime(*parsed_tuple[:6], tzinfo=timezone.utc)
                    last_modified = dt.strftime("%Y-%m-%d")
                    date_confidence = "http_header"
            except Exception:
                pass

    # 5. Fallback: Crawl timestamp
    if not last_modified:
        last_modified = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        date_confidence = "crawl_timestamp"

    # Content Type Classification
    url_lower = url.lower()
    if any(k in url_lower for k in ["/news/", "/blog/", "/press/", "/announcement", "/backgrounder"]):
        content_type = "Historical News / Press Release"
    elif any(k in url_lower for k in ["/services/", "/guide/", "/doc/", "/docs/", "/manual/", "/policy/"]):
        content_type = "Canonical Service / Documentation"
    else:
        content_type = "Web Documentation"

    return {
        "last_modified": last_modified,
        "date_confidence": date_confidence,
        "content_type": content_type
    }

def scrape_url(url: str, timeout: int = 15) -> Dict[str, Any]:
    """
    Fetches a web page URL, cleans HTML tags, computes SHA-256 content hash,
    and extracts page title, body text, last modified date, and content classification.
    """
    if not url.startswith("http://") and not url.startswith("https://"):
        url = f"https://{url}"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 AnyContext-WebScraper/1.0"
    }
    req = urllib.request.Request(url, headers=headers)
    
    resp_headers = {}
    with urllib.request.urlopen(req, timeout=timeout) as response:
        html_bytes = response.read()
        html_text = html_bytes.decode("utf-8", errors="ignore")
        resp_headers = dict(response.headers)

    # Extract title
    title_match = re.search(r"<title>(.*?)</title>", html_text, re.IGNORECASE | re.DOTALL)
    page_title = title_match.group(1).strip() if title_match else url

    # Parse clean text content
    parser = CleanHTMLTextExtractor()
    parser.feed(html_text)
    clean_text = parser.get_text()

    content_hash = hashlib.sha256(clean_text.encode("utf-8")).hexdigest()
    
    # Extract temporal and content metadata
    meta = extract_web_metadata(url, html_text, resp_headers)

    return {
        "url": url,
        "title": page_title,
        "content": clean_text,
        "hash": content_hash,
        "char_count": len(clean_text),
        "last_modified": meta["last_modified"],
        "date_confidence": meta["date_confidence"],
        "content_type": meta["content_type"]
    }

def scrape_sitemap(sitemap_url: str, max_urls: int = 50) -> List[str]:
    """
    Parses an XML sitemap URL to discover child page URLs.
    """
    urls = []
    try:
        headers = {"User-Agent": "AnyContext-WebScraper/1.0"}
        req = urllib.request.Request(sitemap_url, headers=headers)
        with urllib.request.urlopen(req, timeout=10) as response:
            xml_data = response.read()
        
        root = ET.fromstring(xml_data)
        for elem in root.iter():
            if elem.tag.endswith("loc") and elem.text:
                loc = elem.text.strip()
                if loc and loc not in urls:
                    urls.append(loc)
                if len(urls) >= max_urls:
                    break
    except Exception:
        pass

    return urls
