import json
import re
import hashlib
import gzip
import zlib
import urllib.request
import xml.etree.ElementTree as ET
import email.utils
from datetime import datetime, timezone
from typing import Dict, Any, List, Optional
from html.parser import HTMLParser


def resilient_decompress(data: bytes, encoding: Optional[str] = None) -> bytes:
    """
    Safely decompresses raw bytes from HTTP responses, sitemaps, or archive streams.
    Handles gzip, deflate, and truncated streams without throwing zlib Error -5 (Z_BUF_ERROR).
    If the stream is truncated or malformed, it recovers and returns all decoded bytes up to the interruption.
    """
    if not data or not isinstance(data, (bytes, bytearray)):
        return b""

    enc = (encoding or "").lower().strip()
    is_gzip = "gzip" in enc or data.startswith(b"\x1f\x8b")
    is_deflate = "deflate" in enc

    if not is_gzip and not is_deflate:
        # Check magic bytes for gzip or zlib
        if data.startswith(b"\x1f\x8b"):
            is_gzip = True
        elif data.startswith(b"\x78\x9c") or data.startswith(b"\x78\x01") or data.startswith(b"\x78\xda"):
            is_deflate = True
        else:
            return bytes(data)

    # 1. GZIP with standard decompressor
    if is_gzip:
        try:
            return gzip.decompress(data)
        except Exception:
            pass

    # 2. Resilient zlib stream decompression (32 + MAX_WBITS auto-detects gzip/zlib headers)
    try:
        return zlib.decompress(data, 32 + zlib.MAX_WBITS)
    except Exception:
        pass

    # 3. Deflate raw stream (-MAX_WBITS)
    try:
        return zlib.decompress(data, -zlib.MAX_WBITS)
    except Exception:
        pass

    # 4. Resilient chunk recovery via decompressobj (immune to Error -5 truncated stream)
    for wbits in [32 + zlib.MAX_WBITS, -zlib.MAX_WBITS, zlib.MAX_WBITS]:
        try:
            d = zlib.decompressobj(wbits)
            decompressed = d.decompress(data)
            if decompressed:
                return decompressed
        except Exception:
            continue

    # Fallback: return raw bytes if decompression cannot process
    return bytes(data)


class CleanHTMLTextExtractor(HTMLParser):
    """
    Universal HTML parser to extract clean human-readable text and Schema.org structured metadata
    from web pages, e-commerce stores, documentation portals, and articles.
    """
    def __init__(self):
        super().__init__()
        self.reset()
        self.strict = False
        self.convert_charrefs = True
        self.text = []
        # Keep nav and global footers skipped, but allow header, form, aside, etc.
        self.skip_tags = {"script", "style", "noscript", "svg", "head", "nav", "footer"}
        self.tag_stack = []
        self.block_tags = {
            "p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr", "div",
            "section", "article", "header", "form", "aside", "dl", "dt", "dd"
        }
        self.json_ld_data = []
        self.current_tag = None
        self.current_attrs = {}

    def handle_starttag(self, tag, attrs):
        t = tag.lower()
        self.tag_stack.append(t)
        self.current_tag = t
        self.current_attrs = dict(attrs)
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
        # Capture Schema.org JSON-LD structured data
        if self.tag_stack and self.tag_stack[-1] == "script":
            attr_type = self.current_attrs.get("type", "").lower()
            if attr_type == "application/ld+json":
                try:
                    raw_json = data.strip()
                    if raw_json:
                        parsed = json.loads(raw_json)
                        if isinstance(parsed, list):
                            self.json_ld_data.extend(parsed)
                        elif isinstance(parsed, dict):
                            if "@graph" in parsed and isinstance(parsed["@graph"], list):
                                self.json_ld_data.extend(parsed["@graph"])
                            else:
                                self.json_ld_data.append(parsed)
                except Exception:
                    pass
            return

        if any(t in self.skip_tags for t in self.tag_stack):
            return
        content = data.strip()
        if content:
            self.text.append(content)

    def _extract_structured_metadata(self) -> List[str]:
        """Formats discovered Schema.org entities into clear, RAG-optimized text blocks."""
        structured_blocks = []
        seen_products = set()

        for item in self.json_ld_data:
            if not isinstance(item, dict):
                continue
            item_type = str(item.get("@type", ""))
            
            # Handle Product / IndividualProduct
            if any(pt in item_type for pt in ["Product", "IndividualProduct"]):
                name = item.get("name", "").strip()
                if not name or name in seen_products:
                    continue
                seen_products.add(name)

                brand = item.get("brand", {})
                brand_name = brand.get("name", "") if isinstance(brand, dict) else str(brand)
                
                rating = item.get("aggregateRating", {})
                rating_val = rating.get("ratingValue", "") if isinstance(rating, dict) else ""
                review_count = (rating.get("reviewCount", "") or rating.get("ratingCount", "")) if isinstance(rating, dict) else ""
                best_rating = rating.get("bestRating", "5") if isinstance(rating, dict) else "5"
                
                offers = item.get("offers", {})
                if isinstance(offers, list) and offers:
                    offers = offers[0]
                price = offers.get("price", "") if isinstance(offers, dict) else ""
                currency = offers.get("priceCurrency", "") if isinstance(offers, dict) else ""
                availability = offers.get("availability", "") if isinstance(offers, dict) else ""
                if "InStock" in availability:
                    availability = "In Stock"
                elif "OutOfStock" in availability:
                    availability = "Out of Stock"

                parts = [f"Product: {name}"]
                if brand_name:
                    parts.append(f"Brand: {brand_name}")
                if rating_val:
                    rating_str = f"Rating: {rating_val} / {best_rating} stars"
                    if review_count:
                        rating_str += f" ({review_count} reviews)"
                    parts.append(rating_str)
                if price:
                    parts.append(f"Price: {currency} {price}".strip())
                if availability:
                    parts.append(f"Status: {availability}")

                structured_blocks.append(" | ".join(parts))

            # Handle FAQPage
            elif "FAQPage" in item_type:
                main_entity = item.get("mainEntity", [])
                if isinstance(main_entity, list):
                    for q in main_entity:
                        if isinstance(q, dict) and q.get("@type") == "Question":
                            q_name = q.get("name", "")
                            ans = q.get("acceptedAnswer", {}).get("text", "")
                            if q_name and ans:
                                structured_blocks.append(f"FAQ: {q_name}\nAnswer: {ans}")

        return structured_blocks

    def get_text(self) -> str:
        structured_headers = self._extract_structured_metadata()
        raw = " ".join(self.text)
        cleaned = re.sub(r"\n\s*\n+", "\n\n", raw)
        cleaned = re.sub(r"[ \t]+", " ", cleaned).strip()

        if structured_headers:
            return "\n\n".join(structured_headers) + "\n\n" + cleaned
        return cleaned

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
    bot_patterns = [
        r"Verify Your Identity",
        r"Bot Protection Page",
        r"Robot or human\?",
        r"px-captcha",
        r"cf-browser-verification",
        r"Click the button below to continue shopping",
        r"Access Denied.*Cloudflare",
        r"Attention Required! \| Cloudflare"
    ]
    if any(re.search(pat, html_text, re.IGNORECASE) for pat in bot_patterns):
        content_type = "Blocked by Anti-Bot Firewall (Captcha)"
    else:
        url_lower = url.lower()
        if any(k in url_lower for k in ["/news/", "/blog/", "/press/", "/announcement", "/backgrounder"]):
            content_type = "Historical News / Press Release"
        elif any(k in url_lower for k in ["/services/", "/guide/", "/doc/", "/docs/", "/manual/", "/policy/"]):
            content_type = "Canonical Service / Documentation"
        elif any(k in url_lower for k in ["/ip/", "/dp/", "/product/", "/item/", "/p/", "/produtos/"]):
            content_type = "E-Commerce Product Page"
        elif '"@type": "Product"' in html_text or '"@type":"Product"' in html_text or '"@type": "IndividualProduct"' in html_text:
            content_type = "E-Commerce Product Page"
        else:
            content_type = "Web Documentation"

    return {
        "last_modified": last_modified,
        "date_confidence": date_confidence,
        "content_type": content_type
    }

def scrape_url(
    url: str,
    cached_etag: Optional[str] = None,
    cached_last_modified: Optional[str] = None,
    timeout: int = 15
) -> Dict[str, Any]:
    """
    Fetches a web page URL with HTTP Conditional GET support (If-None-Match, If-Modified-Since),
    cleans HTML tags, computes SHA-256 content hash, and extracts page title, body text,
    last modified date, ETag, and content classification.
    Strictly obeys RFC 9309 robots.txt policies.
    """
    import urllib.error
    if not url.startswith("http://") and not url.startswith("https://"):
        url = f"https://{url}"

    from any_context.ingestion.robots_policy import is_url_allowed_by_robots
    if not is_url_allowed_by_robots(url):
        now_date = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        blocked_msg = f"[COMPLIANCE NOTICE] Crawling for '{url}' was skipped because this path is disallowed by the website's robots.txt policy."
        return {
            "url": url,
            "title": f"[Blocked by robots.txt] {url}",
            "content": blocked_msg,
            "hash": hashlib.sha256(blocked_msg.encode("utf-8")).hexdigest(),
            "char_count": len(blocked_msg),
            "last_modified": now_date,
            "date_confidence": "robots_compliance",
            "content_type": "Disallowed by Robots.txt",
            "is_not_modified": False
        }

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 AnyContext-WebScraper/1.0",
        "Accept-Encoding": "gzip, deflate"
    }
    if cached_etag:
        headers["If-None-Match"] = cached_etag
    if cached_last_modified:
        headers["If-Modified-Since"] = cached_last_modified

    req = urllib.request.Request(url, headers=headers)
    
    resp_headers = {}
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            resp_headers = dict(response.headers)
            raw_bytes = response.read()
            encoding = resp_headers.get("Content-Encoding") or resp_headers.get("content-encoding")
            html_bytes = resilient_decompress(raw_bytes, encoding=encoding)
            html_text = html_bytes.decode("utf-8", errors="ignore")
    except urllib.error.HTTPError as e:
        if e.code == 304:
            # 304 Not Modified - Server authoritatively confirms page has not changed since last crawl
            return {
                "url": url,
                "title": "",
                "content": "",
                "hash": "",
                "char_count": 0,
                "is_not_modified": True,
                "etag": cached_etag,
                "http_last_modified": cached_last_modified,
                "last_modified": cached_last_modified,
                "date_confidence": "http_conditional_304",
                "content_type": "Unchanged (HTTP 304)",
                "is_dynamic_spa": False
            }
        raise

    # Extract HTTP caching headers
    response_etag = resp_headers.get("ETag") or resp_headers.get("etag")
    response_last_mod = resp_headers.get("Last-Modified") or resp_headers.get("last-modified")

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

    # Check for SPA / Client-Side Dynamic Rendering / Bot Challenge indicators
    DYNAMIC_SPA_MARKERS = [
        "__NEXT_DATA__",
        "data-reactroot",
        "id=\"__next\"",
        "id=\"root\"",
        "window.__INITIAL_STATE__",
        "_next/static",
        "px-captcha",
        "Verify Your Identity",
        "Bot Protection Page",
        "cf-browser-verification",
        "challenge-platform",
        "Robot or human"
    ]
    has_spa_markers = any(k.lower() in html_text.lower() for k in DYNAMIC_SPA_MARKERS)
    text_density = len(clean_text) / max(len(html_text), 1)
    is_spa = bool(has_spa_markers and (text_density < 0.05 or len(clean_text) < 2500))

    return {
        "url": url,
        "title": page_title,
        "content": clean_text,
        "hash": content_hash,
        "char_count": len(clean_text),
        "last_modified": meta["last_modified"] or response_last_mod,
        "date_confidence": meta["date_confidence"],
        "content_type": meta["content_type"],
        "is_dynamic_spa": is_spa,
        "is_not_modified": False,
        "etag": response_etag,
        "http_last_modified": response_last_mod
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
