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
from any_context.ingestion.robots_policy import is_url_allowed_by_robots


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


def fetch_sitemap_urls(base_url: str, max_urls: int = 5000, timeout: int = 6) -> List[str]:
    """
    Locates and parses sitemaps, properly handling sitemap indexes by following sub-sitemaps
    to extract actual web page URLs (excluding raw XML files).
    """
    parsed_base = urllib.parse.urlparse(base_url)
    domain_root = f"{parsed_base.scheme}://{parsed_base.netloc}"
    candidate_sitemaps = [
        f"{domain_root}/sitemap.xml",
        f"{domain_root}/sitemap_index.xml",
        f"{domain_root}/sitemap/sitemap.xml"
    ]

    discovered_pages = set()
    headers = {"User-Agent": "AnyContext-WebCrawler/1.0 (+https://levix-digital.github.io/any-context-releases/)"}

    def _parse_xml_locs(xml_content: bytes) -> List[str]:
        try:
            root = ET.fromstring(xml_content)
            locs = []
            for elem in root.iter():
                if elem.tag.endswith("loc") and elem.text:
                    l = elem.text.strip()
                    if l.startswith("http") and parsed_base.netloc in l:
                        locs.append(l)
            return locs
        except Exception:
            return []

    for sitemap_url in candidate_sitemaps:
        try:
            req = urllib.request.Request(sitemap_url, headers=headers)
            with urllib.request.urlopen(req, timeout=timeout) as response:
                if response.status == 200:
                    raw_xml = response.read()
                    locs = _parse_xml_locs(raw_xml)
                    
                    sub_sitemaps = [l for l in locs if l.endswith(".xml") or l.endswith(".xml.gz") or "sitemap" in l]
                    page_locs = [l for l in locs if l not in sub_sitemaps]

                    for p in page_locs:
                        discovered_pages.add(p)
                        if len(discovered_pages) >= max_urls:
                            break

                    # If this was a sitemap index, follow relevant sub-sitemaps
                    if sub_sitemaps and len(discovered_pages) < max_urls:
                        clean_base = parsed_base.path
                        if clean_base.endswith((".html", ".htm", ".php", ".asp", ".aspx")):
                            clean_base = clean_base.rsplit(".", 1)[0]
                        raw_parts = [p.lower() for p in clean_base.split("/") if p and len(p) > 2]
                        path_parts = list(raw_parts)
                        for rp in raw_parts:
                            path_parts.extend([sub for sub in rp.split("-") if len(sub) > 2])
                            path_parts.extend([sub for sub in rp.split("_") if len(sub) > 2])

                        matched_subs = [s for s in sub_sitemaps if any(part in s.lower() for part in path_parts)]
                        prioritized_subs = matched_subs + [s for s in sub_sitemaps if s not in matched_subs]

                        for sub in prioritized_subs[:12]:
                            if len(discovered_pages) >= max_urls:
                                break
                            try:
                                sub_req = urllib.request.Request(sub, headers=headers)
                                with urllib.request.urlopen(sub_req, timeout=timeout) as sub_resp:
                                    if sub_resp.status == 200:
                                        sub_locs = _parse_xml_locs(sub_resp.read())
                                        for sp in sub_locs:
                                            if not sp.endswith(".xml") and not sp.endswith(".xml.gz"):
                                                discovered_pages.add(sp)
                                                if len(discovered_pages) >= max_urls:
                                                    break
                            except Exception:
                                continue

                    if discovered_pages:
                        break
        except Exception:
            continue

    return list(discovered_pages)


def discover_site_urls(start_url: str, max_discovery: int = 2500, timeout: int = 6) -> Dict[str, Any]:
    """
    Fast discovery phase: Scans the target page, internal links, and sitemaps.
    Categorizes discovered URLs into 'section_urls' (matching start path) and 'domain_urls' (same root domain).
    """
    if not start_url.startswith("http://") and not start_url.startswith("https://"):
        start_url = f"https://{start_url}"

    parsed_start = urllib.parse.urlparse(start_url)
    domain = parsed_start.netloc.lower()
    
    # Path prefix and semantic keywords for section matching
    clean_path = parsed_start.path
    if clean_path.endswith((".html", ".htm", ".php", ".asp", ".aspx")):
        clean_path = clean_path.rsplit(".", 1)[0]

    path_segments = [seg for seg in clean_path.split("/") if seg]
    section_prefix = "/" + "/".join(path_segments) if path_segments else "/"
    key_terms = [seg.lower() for seg in path_segments if len(seg) > 2]

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

    # 3. Fast BFS expansion on top discovered pages if sitemap is small or absent
    if len(all_domain_urls) < 300:
        sample_urls = [u for u in list(all_domain_urls) if u != start_url][:20]
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

    # RFC 9309 Compliance: Filter out any URLs disallowed by robots.txt
    all_domain_urls = {u for u in all_domain_urls if is_url_allowed_by_robots(u)}

    # Rank all discovered URLs by semantic proximity and relevance to start_url
    def _rank_url(u: str) -> tuple:
        if u == start_url:
            return (10000, 0)
        p = urllib.parse.urlparse(u)
        path = p.path.lower()
        
        score = 0
        clean_section = section_prefix.lower()
        if path.startswith(clean_section + "/") or path == clean_section or path == (clean_section + ".html"):
            score += 2000
        elif path.startswith(clean_section):
            score += 1500

        # Term matching from path components
        matched_terms = sum(1 for term in key_terms if term in path or term in u.lower())
        score += matched_terms * 300

        if u in initial_links:
            score += 500

        return (score, -len(path), u)

    ranked_domain_urls = sorted(list(all_domain_urls), key=_rank_url, reverse=True)

    # Filter into section vs domain
    section_urls = []
    clean_section_prefix = section_prefix.lower()

    for u in ranked_domain_urls:
        p = urllib.parse.urlparse(u).path.lower()
        if p.startswith(clean_section_prefix + "/") or p == clean_section_prefix or p == (clean_section_prefix + ".html") or u == start_url:
            section_urls.append(u)
        elif any(term in p for term in key_terms if len(term) >= 5):
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
        "domain_urls": ranked_domain_urls,
        "domain_count": len(ranked_domain_urls),
        "has_sitemap": bool(sitemap_urls)
    }


def crawl_and_index_urls(
    workspace_name: str,
    urls: List[str],
    root_url: Optional[str] = None,
    root_title: Optional[str] = None,
    scope: str = "custom",
    force_refresh: bool = False,
    max_workers: int = 12,
    progress_callback = None,
    embed_progress_callback = None
) -> Dict[str, Any]:
    """
    Concurrent multi-threaded scraper and ChromaDB batch vector indexer.
    Implements incremental SHA-256 hash checking, automatic deduplication, and atomic vector updates.
    """
    import os
    import logging
    import hashlib
    import urllib.parse
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
    from llama_index.core import Document
    from llama_index.core.node_parser import SentenceSplitter
    from llama_index.core.ingestion import IngestionPipeline
    from llama_index.core.settings import Settings
    from llama_index.vector_stores.chroma import ChromaVectorStore

    settings = AppSettings.load()
    db_path = settings.context.db_path if settings else "./context_db"
    collection_name = settings.context.collection_name if settings else "context_docs"

    os.makedirs(db_path, exist_ok=True)
    db = chromadb.PersistentClient(path=db_path)
    chroma_collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=chroma_collection)
    configure_embedding_model()

    chunk_size = settings.context.chunk_size if (settings and settings.context) else 1024
    chunk_overlap = settings.context.chunk_overlap if (settings and settings.context) else 200

    pipeline = IngestionPipeline(
        transformations=[
            SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
            Settings.embed_model
        ],
        vector_store=vector_store
    )

    store = WebSchedulerStore()
    total_urls = len(urls)
    indexed_count = 0
    skipped_count = 0
    spa_detected_count = 0
    total_chars = 0
    errors = 0

    documents_batch: List[Document] = []
    processed_records: List[Dict[str, Any]] = []
    now_str = time.strftime("%Y-%m-%d %H:%M:%S")
    effective_root = root_url or (urls[0] if urls else "https://unknown")
    domain = urllib.parse.urlparse(effective_root).netloc.lower()

    # Load existing indexed pages map for incremental SHA-256 comparison
    indexed_map = store.get_indexed_pages_map(workspace_name, domain_or_prefix=domain)

    # Worker function to scrape single url with RFC 9309 robots.txt compliance
    def _fetch_single(url: str) -> Optional[Dict[str, Any]]:
        if not is_url_allowed_by_robots(url):
            return None
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
                if data and data.get("is_dynamic_spa"):
                    spa_detected_count += 1

                if data and data.get("content") and len(data["content"].strip()) > 30:
                    text_content = data["content"]
                    url_hash = data["hash"]

                    # Check if page is already indexed and hash has not changed (Skip / Zero Cost)
                    is_cached = (not force_refresh) and (url in indexed_map) and (indexed_map[url].get("content_hash") == url_hash)

                    if is_cached:
                        skipped_count += 1
                        processed_records.append({
                            "url": url,
                            "title": data["title"],
                            "content_hash": url_hash,
                            "char_count": len(text_content),
                            "scraped_at": now_str
                        })
                    else:
                        # If page was previously indexed but hash changed, delete old vectors from ChromaDB
                        if url in indexed_map:
                            try:
                                chroma_collection.delete(where={"$and": [{"workspace": workspace_name}, {"url": url}]})
                            except Exception:
                                pass

                        doc_id = f"web_{workspace_name}_{hashlib.sha256(url.encode()).hexdigest()[:20]}"
                        doc = Document(
                            text=f"=== Web Page: {data['title']} ({url}) ===\n\n{text_content}",
                            doc_id=doc_id,
                            metadata={
                                "file_name": f"[Web] {data['title'][:60]}",
                                "file_path": url,
                                "url": url,
                                "root_url": effective_root,
                                "workspace": workspace_name,
                                "title": data["title"],
                                "content_hash": url_hash,
                                "source_type": "web",
                                "last_modified_date": data.get("last_modified", now_str[:10]),
                                "date_confidence": data.get("date_confidence", "crawl_timestamp"),
                                "content_type": data.get("content_type", "Web Documentation"),
                                "scraped_at": now_str
                            }
                        )
                        documents_batch.append(doc)
                        total_chars += len(text_content)
                        indexed_count += 1
                        processed_records.append({
                            "url": url,
                            "title": data["title"],
                            "content_hash": url_hash,
                            "char_count": len(text_content),
                            "scraped_at": now_str
                        })
                else:
                    errors += 1
            except Exception:
                errors += 1

            if progress_callback:
                progress_callback(i + 1, total_urls, indexed_count, skipped_count, url, (data.get("title") if data else ""))

    # Batch index vectors into ChromaDB using IngestionPipeline with micro-batching and Stage 2 Progress Ticker
    if documents_batch:
        try:
            chunk_size = 20
            total_docs_to_embed = len(documents_batch)
            if embed_progress_callback:
                embed_progress_callback(0, total_docs_to_embed)

            for k in range(0, total_docs_to_embed, chunk_size):
                batch = documents_batch[k:k+chunk_size]
                pipeline.run(documents=batch, show_progress=False)
                processed = min(k + len(batch), total_docs_to_embed)
                if embed_progress_callback:
                    embed_progress_callback(processed, total_docs_to_embed)
                time.sleep(0.04)
        except Exception as e:
            return {
                "status": "partial_error",
                "indexed_count": indexed_count,
                "skipped_count": skipped_count,
                "total_chars": total_chars,
                "error": f"Vector indexing failed: {str(e)}"
            }

    # Record all indexed pages and update SQLite root source with distinct page count
    if processed_records:
        store.record_indexed_web_pages(
            workspace_name=workspace_name,
            root_url=effective_root,
            pages=processed_records
        )

    total_distinct_pages = store.get_indexed_pages_count(workspace_name, domain_or_prefix=domain)
    store.add_or_update_root_web_source(
        workspace_name=workspace_name,
        root_url=effective_root,
        title=root_title or f"Web Portal ({total_distinct_pages} pages)",
        page_count=total_distinct_pages,
        scope=scope
    )

    is_dynamic_site = bool(spa_detected_count > 0 and (spa_detected_count / max(indexed_count + skipped_count, 1)) >= 0.05)

    return {
        "status": "success",
        "total_requested": total_urls,
        "indexed_count": indexed_count,
        "skipped_count": skipped_count,
        "total_distinct_indexed": total_distinct_pages,
        "total_chars": total_chars,
        "is_dynamic_spa": is_dynamic_site,
        "errors": errors
    }


def run_interactive_web_crawler(workspace_name: str, start_url: Optional[str] = None) -> bool:
    """
    Guides the user through interactive website discovery, scope selection, and concurrent crawling.
    Provides clear visibility of already indexed vs new unindexed pages with automatic incremental deduplication.
    """
    import sys
    import urllib.parse
    import questionary
    from any_context.cli.spinner import Spinner
    from any_context.ingestion.web_scheduler import WebSchedulerStore

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
    domain = disc.get("domain") or urllib.parse.urlparse(start_url).netloc.lower()

    # Query already indexed pages in this workspace for this domain
    store = WebSchedulerStore()
    indexed_map = store.get_indexed_pages_map(workspace_name, domain_or_prefix=domain)
    already_indexed_urls = set(indexed_map.keys())
    already_indexed_count = len(already_indexed_urls)

    # Classify unindexed pages
    new_section_urls = [u for u in disc.get("section_urls", []) if u not in already_indexed_urls]
    new_domain_urls = [u for u in disc.get("domain_urls", []) if u not in already_indexed_urls]
    new_section_count = len(new_section_urls)
    new_domain_count = len(new_domain_urls)

    print("\n================================================================================")
    print(f"🌐 \033[93mWebsite Discovery Report:\033[0m \033[1m{title}\033[0m")
    print(f"🔗 \033[96m{start_url}\033[0m")
    print("================================================================================")
    print(f"  • 📄 Section Pages (matching path prefix) : \033[92m{section_count}\033[0m pages")
    print(f"  • 🌐 Total Internal Domain URLs Found    : \033[92m{domain_count}\033[0m pages")
    if already_indexed_count > 0:
        print(f"  • 📦 Already Indexed in this Workspace   : \033[96m{already_indexed_count}\033[0m pages (Cached in Vector DB)")
    else:
        print(f"  • 📦 Already Indexed in this Workspace   : \033[90m0 pages (First time indexing)\033[0m")
    print(f"  • ✨ New Unindexed Pages Available       : \033[93m{new_section_count}\033[0m section / \033[93m{new_domain_count}\033[0m domain pages")
    print(f"  • 🗺️ XML Sitemap Detected                : \033[95m{'Yes (Structured XML)' if has_sitemap else 'No (Fast Recursive Link Scan)'}\033[0m")
    print("================================================================================\n")

    scope_name = "Single Page"
    force_refresh = False

    if domain_count == 1:
        chosen_urls = [start_url]
    else:
        choices = []

        if already_indexed_count > 0:
            # Incremental options when workspace already has pages from this site
            if new_section_count > 0:
                choices.append(f"1. ⚡ Incremental Section Ingestion ({new_section_count} NEW pages) [Recommended]")
            if new_domain_count > 0:
                choices.append(f"2. 🚀 Quick Incremental Crawl (Next {min(50, new_domain_count)} NEW pages) ~ 5s")
                if new_domain_count > 50:
                    choices.append(f"3. 🌐 Deep Incremental Crawl (Next {min(250, new_domain_count)} NEW pages) ~ 20s")
                if new_domain_count > 250:
                    choices.append(f"4. 📦 Extensive Incremental Crawl (Next {min(500, new_domain_count)} NEW pages) ~ 45s")
                choices.append(f"5. 🌌 Ingest All Remaining Domain Pages ({new_domain_count} NEW pages)")
            choices.append(f"6. 🔄 Full Re-Sync & Refresh (Re-check all {domain_count} pages with SHA-256)")
            choices.append(f"7. 📄 Ingest / Refresh Landing Page Only (1 page) ~ 1s")
            choices.append("❌ Cancel")
        else:
            # Fresh initial indexing choices
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

        if "Incremental Section Ingestion" in choice:
            chosen_urls = new_section_urls
            scope_name = f"Incremental Section (+{len(chosen_urls)} pages)"
        elif "Quick Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:50]
            scope_name = f"Incremental Top 50 (+{len(chosen_urls)} pages)"
        elif "Deep Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:250]
            scope_name = f"Incremental Top 250 (+{len(chosen_urls)} pages)"
        elif "Extensive Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:500]
            scope_name = f"Incremental Top 500 (+{len(chosen_urls)} pages)"
        elif "Ingest All Remaining" in choice:
            chosen_urls = new_domain_urls
            scope_name = f"Remaining Domain (+{len(chosen_urls)} pages)"
        elif "Full Re-Sync" in choice:
            chosen_urls = disc["domain_urls"]
            force_refresh = True
            scope_name = f"Full Re-Sync ({len(chosen_urls)} pages)"
        elif "Current Section Only" in choice:
            chosen_urls = disc["section_urls"]
            scope_name = f"Section ({len(chosen_urls)} pages)"
        elif "Fast Crawl Limit" in choice or "Top 50 pages" in choice:
            chosen_urls = disc["domain_urls"][:50]
            scope_name = "Top 50 pages"
        elif "Deep Crawl Limit" in choice or "Top 250 pages" in choice:
            chosen_urls = disc["domain_urls"][:250]
            scope_name = "Top 250 pages"
        elif "Extensive Crawl Limit" in choice or "Top 500 pages" in choice:
            chosen_urls = disc["domain_urls"][:500]
            scope_name = "Top 500 pages"
        elif "Entire Discovered Domain" in choice:
            chosen_urls = disc["domain_urls"]
            scope_name = f"Domain ({len(chosen_urls)} pages)"
        else:
            chosen_urls = [start_url]
            scope_name = "Single Page"

    total_target = len(chosen_urls)
    print(f"\n🚀 Processing and indexing \033[92m{total_target}\033[0m web pages into workspace '\033[93m{workspace_name}\033[0m'...")

    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    
    def safe_stdout_write(msg: str):
        try:
            sys.stdout.write(msg)
            sys.stdout.flush()
        except (UnicodeEncodeError, Exception):
            try:
                clean_msg = msg.encode("ascii", errors="ignore").decode("ascii")
                sys.stdout.write(clean_msg)
                sys.stdout.flush()
            except Exception:
                pass

    def _render_crawl_progress(current: int, total: int, indexed: int, skipped: int, latest_url: str = "", latest_title: str = ""):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        display_url = latest_url
        if len(display_url) > 30:
            display_url = display_url[:12] + "..." + display_url[-15:]

        status_text = f"{indexed} new"
        if skipped > 0:
            status_text += f", {skipped} cached"

        safe_stdout_write(f"\r\033[K\033[96m{frame}\033[0m [1/2 Crawling] [{bar}] {current}/{total} ({pct}%) • \033[93m{status_text}\033[0m • \033[90m{display_url}\033[0m")

    def _render_embed_progress(current: int, total: int):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        safe_stdout_write(f"\r\033[K\033[95m{frame}\033[0m [2/2 Embedding] [{bar}] {current}/{total} pages ({pct}%) • \033[92mVector Knowledge Base\033[0m")

    # Hide terminal cursor during active live progress ticks
    safe_stdout_write("\033[?25l")
    try:
        res = crawl_and_index_urls(
            workspace_name=workspace_name,
            urls=chosen_urls,
            root_url=start_url,
            root_title=title,
            scope=scope_name,
            force_refresh=force_refresh,
            max_workers=12,
            progress_callback=_render_crawl_progress,
            embed_progress_callback=_render_embed_progress
        )
    finally:
        # Restore terminal cursor visibility
        safe_stdout_write("\033[?25h")

    # Completely clear the live ticker line and print a clean final summary
    safe_stdout_write("\r\033[K")
    indexed_cnt = res.get("indexed_count", 0)
    skipped_cnt = res.get("skipped_count", 0)
    total_distinct = res.get("total_distinct_indexed", indexed_cnt + skipped_cnt)
    total_chars = res.get("total_chars", 0)

    if res.get("status") == "partial_error":
        safe_stdout_write(f"⚠️ Partial indexing completed: \033[92m{indexed_cnt}\033[0m pages indexed ({skipped_cnt} cached), but encountered error: \033[91m{res.get('error')}\033[0m\n\n")
    elif indexed_cnt == 0 and skipped_cnt > 0:
        safe_stdout_write(f"✔ All \033[96m{skipped_cnt}\033[0m web pages from \033[96m{start_url}\033[0m are already up-to-date in workspace '\033[93m{workspace_name}\033[0m' (SHA-256 verified, 0 embeddings consumed). Total in knowledge base: \033[92m{total_distinct}\033[0m pages.\n\n")
    elif indexed_cnt > 0 and skipped_cnt > 0:
        safe_stdout_write(f"✔ Successfully ingested \033[92m{indexed_cnt}\033[0m new/updated web pages ({total_chars:,} chars) from \033[96m{start_url}\033[0m into workspace '\033[93m{workspace_name}\033[0m' (\033[90m{skipped_cnt} unchanged pages cached\033[0m). Total in knowledge base: \033[92m{total_distinct}\033[0m pages!\n\n")
    else:
        safe_stdout_write(f"✔ Successfully ingested and indexed \033[92m{indexed_cnt}\033[0m web pages ({total_chars:,} chars) from \033[96m{start_url}\033[0m into workspace '\033[93m{workspace_name}\033[0m'! Total in knowledge base: \033[92m{total_distinct}\033[0m pages.\n\n")

    if res.get("is_dynamic_spa"):
        safe_stdout_write(
            f"⚠️ \033[1;93mImportante:\033[0m\n"
            f"Este site carrega seu conteúdo de forma dinâmica no navegador. Apenas a estrutura\n"
            f"estática foi capturada. Para consultar detalhes específicos, adicione o link\n"
            f"direto da página via '\033[96m/web add <url>\033[0m'.\n"
            f"\033[90m[Nota técnica: Client-Side Rendering (CSR / SPA) detectado no domínio {domain}]\033[0m\n\n"
        )

    return True
