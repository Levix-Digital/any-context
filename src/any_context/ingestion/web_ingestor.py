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

def scrape_url(url: str, timeout: int = 15) -> Dict[str, Any]:
    """
    Fetches a web page URL, cleans HTML tags, computes SHA-256 content hash, and extracts page title and body text.
    """
    if not url.startswith("http://") and not url.startswith("https://"):
        url = f"https://{url}"

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 AnyContext-WebScraper/1.0"
    }
    req = urllib.request.Request(url, headers=headers)
    
    with urllib.request.urlopen(req, timeout=timeout) as response:
        html_bytes = response.read()
        html_text = html_bytes.decode("utf-8", errors="ignore")

    # Extract title
    title_match = re.search(r"<title>(.*?)</title>", html_text, re.IGNORECASE | re.DOTALL)
    page_title = title_match.group(1).strip() if title_match else url

    # Parse clean text content
    parser = CleanHTMLTextExtractor()
    parser.feed(html_text)
    clean_text = parser.get_text()

    content_hash = hashlib.sha256(clean_text.encode("utf-8")).hexdigest()

    return {
        "url": url,
        "title": page_title,
        "content": clean_text,
        "hash": content_hash,
        "char_count": len(clean_text)
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
