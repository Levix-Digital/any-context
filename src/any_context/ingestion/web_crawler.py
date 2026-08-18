import re
import time
import hashlib
import urllib.request
import urllib.parse
import xml.etree.ElementTree as ET
from collections import deque
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Dict, Any, List, Set, Optional, Tuple
from html.parser import HTMLParser

from any_context.ingestion.web_ingestor import CleanHTMLTextExtractor, scrape_url


class HTMLLinkExtractor(HTMLParser):
    """
    Fast HTML parser to extract all href anchor links from a web page.
    """
    def __init__(self, base_url: str):
        super().__init__()
        self.reset()
        self.strict = False
        self.convert_charrefs = True
        self.base_url = base_url
        self.links: Set[str] = set()

    def handle_starttag(self, tag, attrs):
        if tag.lower() == "a":
            for attr, val in attrs:
                if attr.lower() == "href" and val:
                    clean_val = val.strip()
                    if clean_val.startswith("javascript:") or clean_val.startswith("mailto:") or clean_val.startswith("tel:"):
                        continue
                    # Remove fragments (#section)
                    clean_val = clean_val.split("#")[0]
                    if not clean_val:
                        continue
                    try:
                        resolved = urllib.parse.urljoin(self.base_url, clean_val)
                        parsed = urllib.parse.urlparse(resolved)
                        if parsed.scheme in ["http", "https"]:
                            # Normalize: lowercase scheme & netloc, strip trailing slash unless root
                            norm_path = parsed.path.rstrip("/") if parsed.path != "/" else "/"
                            norm_url = urllib.parse.urlunparse((parsed.scheme, parsed.netloc.lower(), norm_path, "", parsed.query, ""))
                            self.links.add(norm_url)
                    except Exception:
                        pass


def fetch_sitemap_urls(base_url: str, max_urls: int = 5000, timeout: int = 5) -> List[str]:
    """
    Attempts to locate and parse sitemaps (e.g. /sitemap.xml, /sitemap_index.xml, robots.txt) for a domain.
    """
    parsed_base = urllib.parse.urlparse(base_url)
    domain_root = f"{parsed_base.scheme}://{parsed_base.netloc}"
    candidate_sitemaps = [
        f"{domain_root}/sitemap.xml",
        f"{domain_root}/sitemap_index.xml",
        f"{domain_root}/sitemap/sitemap.xml"
    ]

    discovered = set()
    headers = {"User-Agent": "AnyContext-WebCrawler/1.0 (+https://levix-digital.github.io/any-context-releases/)"}

    for sitemap_url in candidate_sitemaps:
        try:
            req = urllib.request.Request(sitemap_url, headers=headers)
            with urllib.request.urlopen(req, timeout=timeout) as response:
                if response.status == 200:
                    xml_data = response.read()
                    root = ET.fromstring(xml_data)
                    for elem in root.iter():
                        if elem.tag.endswith("loc") and elem.text:
                            loc = elem.text.strip()
                            if loc.startswith("http") and parsed_base.netloc in loc:
                                discovered.add(loc)
                            if len(discovered) >= max_urls:
                                break
                    if discovered:
                        break
        except Exception:
            continue

    return list(discovered)


def discover_site_urls(start_url: str, max_discovery: int = 2500, timeout: int = 6) -> Dict[str, Any]:
    """
    Fast discovery phase: Scans the target page, internal links, and sitemaps.
    Categorizes discovered URLs into 'section_urls' (matching start path) and 'domain_urls' (same root domain).
    """
    if not start_url.startswith("http://") and not start_url.startswith("https://"):
        start_url = f"https://{start_url}"

    parsed_start = urllib.parse.urlparse(start_url)
    domain = parsed_start.netloc.lower()
    
    # Path prefix for section matching (e.g. '/en/immigration-refugees-citizenship')
    path_segments = [seg for seg in parsed_start.path.split("/") if seg]
    section_prefix = f"/{path_segments[0]}" if path_segments else "/"
    if len(path_segments) >= 2:
        section_prefix = f"/{path_segments[0]}/{path_segments[1]}"

    headers = {"User-Agent": "AnyContext-WebCrawler/1.0 (+https://levix-digital.github.io/any-context-releases/)"}
    page_title = start_url
    initial_links: Set[str] = set()

    # 1. Fetch start page
    try:
        req = urllib.request.Request(start_url, headers=headers)
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            html = resp.read().decode("utf-8", errors="ignore")
            title_match = re.search(r"<title>(.*?)</title>", html, re.IGNORECASE | re.DOTALL)
            if title_match:
                page_title = title_match.group(1).strip()
            link_extractor = HTMLLinkExtractor(base_url=start_url)
            link_extractor.feed(html)
            initial_links = link_extractor.links
    except Exception as e:
        return {
            "start_url": start_url,
            "domain": domain,
            "title": page_title,
            "section_urls": [start_url],
            "domain_urls": [start_url],
            "section_count": 1,
            "domain_count": 1,
            "has_sitemap": False,
            "error": str(e)
        }

    # 2. Check sitemap
    sitemap_urls = fetch_sitemap_urls(start_url, max_urls=max_discovery, timeout=timeout)

    all_domain_urls: Set[str] = {start_url}
    all_domain_urls.update(sitemap_urls)

    for link in initial_links:
        parsed_link = urllib.parse.urlparse(link)
        if parsed_link.netloc.lower() == domain:
            all_domain_urls.add(link)

    # 3. Fast BFS expansion on top 10 discovered pages if sitemap not present
    if not sitemap_urls and len(all_domain_urls) < 100:
        sample_urls = [u for u in list(all_domain_urls) if u != start_url][:12]
        for sub_u in sample_urls:
            try:
                sub_req = urllib.request.Request(sub_u, headers=headers)
                with urllib.request.urlopen(sub_req, timeout=3) as sub_resp:
                    sub_html = sub_resp.read().decode("utf-8", errors="ignore")
                    sub_extractor = HTMLLinkExtractor(base_url=sub_u)
                    sub_extractor.feed(sub_html)
                    for lk in sub_extractor.links:
                        if urllib.parse.urlparse(lk).netloc.lower() == domain:
                            all_domain_urls.add(lk)
                            if len(all_domain_urls) >= max_discovery:
                                break
            except Exception:
                continue

    # Filter into section vs domain
    section_urls = []
    domain_urls = sorted(list(all_domain_urls))

    for u in domain_urls:
        p = urllib.parse.urlparse(u).path
        if p.startswith(section_prefix) or u == start_url:
            section_urls.append(u)

    if not section_urls:
        section_urls = [start_url]

    return {
        "start_url": start_url,
        "domain": domain,
        "title": page_title,
        "section_prefix": section_prefix,
        "section_urls": section_urls,
        "section_count": len(section_urls),
        "domain_urls": domain_urls,
        "domain_count": len(domain_urls),
        "has_sitemap": bool(sitemap_urls)
    }


def crawl_and_index_urls(
    workspace_name: str,
    urls: List[str],
    root_url: Optional[str] = None,
    root_title: Optional[str] = None,
    scope: str = "custom",
    max_workers: int = 12,
    progress_callback = None,
    embed_progress_callback = None
) -> Dict[str, Any]:
    """
    Concurrent multi-threaded scraper and ChromaDB batch vector indexer.
    Registers a single clean Root Web Source in SQLite with aggregated metadata.
    """
    import os
    import logging
    import chromadb

    # Suppress verbose HTTP/OpenAI retry logs in terminal
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("openai").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    logging.getLogger("llama_index").setLevel(logging.WARNING)

    from any_context.config.app_settings import AppSettings
    from any_context.tools.search_tools import configure_embedding_model
    from any_context.ingestion.web_scheduler import WebSchedulerStore
    from llama_index.core import Document, VectorStoreIndex
    from llama_index.vector_stores.chroma import ChromaVectorStore

    settings = AppSettings.load()
    db_path = settings.context.db_path if settings else "./context_db"
    collection_name = settings.context.collection_name if settings else "context_docs"

    os.makedirs(db_path, exist_ok=True)
    db = chromadb.PersistentClient(path=db_path)
    chroma_collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=chroma_collection)
    configure_embedding_model()

    store = WebSchedulerStore()
    total_urls = len(urls)
    indexed_count = 0
    total_chars = 0
    errors = 0

    documents_batch: List[Document] = []
    now_str = time.strftime("%Y-%m-%d %H:%M:%S")
    effective_root = root_url or (urls[0] if urls else "https://unknown")

    # Worker function to scrape single url
    def _fetch_single(url: str) -> Optional[Dict[str, Any]]:
        try:
            return scrape_url(url, timeout=10)
        except Exception:
            return None

    # Multi-threaded concurrent download (Stage 1)
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_url = {executor.submit(_fetch_single, u): u for u in urls}
        
        for i, future in enumerate(as_completed(future_to_url)):
            url = future_to_url[future]
            data = None
            try:
                data = future.result()
                if data and data.get("content") and len(data["content"].strip()) > 30:
                    text_content = data["content"]
                    doc = Document(
                        text=f"=== Web Page: {data['title']} ({url}) ===\n\n{text_content}",
                        metadata={
                            "file_name": f"[Web] {data['title'][:60]}",
                            "file_path": url,
                            "url": url,
                            "root_url": effective_root,
                            "workspace": workspace_name,
                            "title": data["title"],
                            "content_hash": data["hash"],
                            "source_type": "web",
                            "scraped_at": now_str
                        }
                    )
                    documents_batch.append(doc)
                    total_chars += len(text_content)
                    indexed_count += 1
                else:
                    errors += 1
            except Exception:
                errors += 1

            if progress_callback:
                progress_callback(i + 1, total_urls, indexed_count, url, (data.get("title") if data else ""))

    # Batch index vectors into ChromaDB with micro-batching and Stage 2 Progress Ticker
    if documents_batch:
        try:
            chunk_size = 15
            total_docs_to_embed = len(documents_batch)
            if embed_progress_callback:
                embed_progress_callback(0, total_docs_to_embed)

            for k in range(0, total_docs_to_embed, chunk_size):
                batch = documents_batch[k:k+chunk_size]
                VectorStoreIndex.from_documents(batch, vector_store=vector_store)
                processed = min(k + len(batch), total_docs_to_embed)
                if embed_progress_callback:
                    embed_progress_callback(processed, total_docs_to_embed)
                time.sleep(0.04)

            # Register ONE unified root source in SQLite
            store.add_or_update_root_web_source(
                workspace_name=workspace_name,
                root_url=effective_root,
                title=root_title or f"Web Portal ({indexed_count} pages)",
                page_count=indexed_count,
                scope=scope
            )
        except Exception as e:
            return {
                "status": "partial_error",
                "indexed_count": indexed_count,
                "total_chars": total_chars,
                "error": f"Vector indexing failed: {str(e)}"
            }

    return {
        "status": "success",
        "total_requested": total_urls,
        "indexed_count": indexed_count,
        "total_chars": total_chars,
        "errors": errors
    }


def run_interactive_web_crawler(workspace_name: str, start_url: Optional[str] = None) -> bool:
    """
    Guides the user through interactive website discovery, scope selection, and concurrent crawling.
    """
    import sys
    import questionary
    from any_context.cli.spinner import Spinner

    if not start_url:
        start_url = questionary.text(
            "Enter website URL or documentation portal (e.g. https://docs.python.org/3/):"
        ).ask()
        if not start_url or not start_url.strip():
            return False

    start_url = start_url.strip()
    if not start_url.startswith("http://") and not start_url.startswith("https://"):
        start_url = f"https://{start_url}"

    # 1. Discovery Phase with clean Spinner
    with Spinner(f"Mapping site structure, internal links & sitemaps for '{start_url}'..."):
        disc = discover_site_urls(start_url)

    title = disc.get("title") or start_url
    section_count = disc.get("section_count", 1)
    domain_count = disc.get("domain_count", 1)
    has_sitemap = disc.get("has_sitemap", False)

    print("\n================================================================================")
    print(f"🌐 \033[93mWebsite Discovery Report:\033[0m \033[1m{title}\033[0m")
    print(f"🔗 \033[96m{start_url}\033[0m")
    print("================================================================================")
    print(f"  • 📄 Section Pages (matching path prefix) : \033[92m{section_count}\033[0m pages")
    print(f"  • 🌐 Total Internal Domain URLs Found    : \033[92m{domain_count}\033[0m pages")
    print(f"  • 🗺️ XML Sitemap Detected                : \033[95m{'Yes (Structured XML)' if has_sitemap else 'No (Fast Recursive Link Scan)'}\033[0m")
    print("================================================================================\n")

    scope_name = "Single Page"
    if domain_count == 1:
        chosen_urls = [start_url]
    else:
        choices = []
        if section_count > 1 and section_count != domain_count:
            choices.append(f"1. 📄 Current Section Only ({section_count} pages) [Recommended]")
        
        if domain_count > 50:
            choices.append(f"2. ⚡ Fast Crawl Limit (Top 50 pages) ~ 5s")
        if domain_count > 250:
            choices.append(f"3. 🚀 Deep Crawl Limit (Top 250 pages) ~ 20s")
        if domain_count > 500:
            choices.append(f"4. 📦 Extensive Crawl Limit (Top 500 pages) ~ 45s")
            
        choices.append(f"5. 🌐 Entire Discovered Domain ({domain_count} pages)")
        choices.append(f"6. 📄 Single Start Page Only (1 page) ~ 1s")
        choices.append("❌ Cancel")

        choice = questionary.select(
            f"Select indexing scope for workspace '{workspace_name}':",
            choices=choices
        ).ask()

        if not choice or choice.startswith("❌"):
            print("Operation cancelled.\n")
            return False

        if "Current Section Only" in choice:
            chosen_urls = disc["section_urls"]
            scope_name = f"Section ({len(chosen_urls)} pages)"
        elif "Top 50 pages" in choice:
            chosen_urls = disc["domain_urls"][:50]
            scope_name = "Top 50 pages"
        elif "Top 250 pages" in choice:
            chosen_urls = disc["domain_urls"][:250]
            scope_name = "Top 250 pages"
        elif "Top 500 pages" in choice:
            chosen_urls = disc["domain_urls"][:500]
            scope_name = "Top 500 pages"
        elif "Entire Discovered Domain" in choice:
            chosen_urls = disc["domain_urls"]
            scope_name = f"Domain ({len(chosen_urls)} pages)"
        else:
            chosen_urls = [start_url]
            scope_name = "Single Page"

    total_target = len(chosen_urls)
    print(f"\n🚀 Ingesting and indexing \033[92m{total_target}\033[0m web pages into workspace '\033[93m{workspace_name}\033[0m'...")

    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    
    def _render_crawl_progress(current: int, total: int, indexed: int, latest_url: str = "", latest_title: str = ""):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        display_url = latest_url
        if len(display_url) > 38:
            display_url = display_url[:16] + "..." + display_url[-19:]

        sys.stdout.write(f"\r\033[K\033[96m{frame}\033[0m [1/2 Crawling] [{bar}] {current}/{total} ({pct}%) • \033[90m{display_url}\033[0m")
        sys.stdout.flush()

    def _render_embed_progress(current: int, total: int):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        sys.stdout.write(f"\r\033[K\033[95m{frame}\033[0m [2/2 Embedding] [{bar}] {current}/{total} pages ({pct}%) • \033[92mChromaDB\033[0m")
        sys.stdout.flush()

    res = crawl_and_index_urls(
        workspace_name=workspace_name,
        urls=chosen_urls,
        root_url=start_url,
        root_title=title,
        scope=scope_name,
        max_workers=12,
        progress_callback=_render_crawl_progress,
        embed_progress_callback=_render_embed_progress
    )

    # Completely clear the live ticker line and print a clean final summary
    sys.stdout.write("\r\033[K")
    sys.stdout.write(f"✔ Successfully ingested and indexed \033[92m{res.get('indexed_count', 0)}\033[0m web pages ({res.get('total_chars', 0):,} chars) from \033[96m{start_url}\033[0m into workspace '\033[93m{workspace_name}\033[0m'!\n\n")
    sys.stdout.flush()
    return True
