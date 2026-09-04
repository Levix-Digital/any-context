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

from any_context.ingestion.web_ingestor import CleanHTMLTextExtractor, scrape_url, resilient_decompress
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


def fetch_sitemap_urls(base_url: str, max_urls: int = 5000, timeout: int = 6) -> Tuple[List[str], Dict[str, str]]:
    """
    Locates and parses sitemaps, extracting web page URLs and their <lastmod> timestamps.
    Properly handles sitemap indexes by following sub-sitemaps (excluding raw XML files).
    Returns (discovered_urls, sitemap_lastmods).
    """
    parsed_base = urllib.parse.urlparse(base_url)
    domain_root = f"{parsed_base.scheme}://{parsed_base.netloc}"
    candidate_sitemaps = [
        f"{domain_root}/sitemap.xml",
        f"{domain_root}/sitemap_index.xml",
        f"{domain_root}/sitemap/sitemap.xml"
    ]

    discovered_pages = set()
    sitemap_lastmods: Dict[str, str] = {}
    headers = {"User-Agent": "AnyContext-WebCrawler/1.0 (+https://levix-digital.github.io/any-context-releases/)"}

    def _parse_xml_entries(xml_content: bytes) -> List[Tuple[str, Optional[str]]]:
        try:
            root = ET.fromstring(xml_content)
            entries = []
            for elem in root.iter():
                tag_lower = elem.tag.lower()
                if tag_lower.endswith("url") or tag_lower.endswith("sitemap"):
                    loc_val = None
                    lastmod_val = None
                    for child in elem:
                        c_tag = child.tag.lower()
                        if c_tag.endswith("loc") and child.text:
                            loc_val = child.text.strip()
                        elif c_tag.endswith("lastmod") and child.text:
                            lastmod_val = child.text.strip()
                    if loc_val and loc_val.startswith("http") and parsed_base.netloc in loc_val:
                        entries.append((loc_val, lastmod_val))
            if not entries:
                for elem in root.iter():
                    if elem.tag.lower().endswith("loc") and elem.text:
                        l = elem.text.strip()
                        if l.startswith("http") and parsed_base.netloc in l:
                            entries.append((l, None))
            return entries
        except Exception:
            return []

    for sitemap_url in candidate_sitemaps:
        try:
            req = urllib.request.Request(sitemap_url, headers=headers)
            with urllib.request.urlopen(req, timeout=timeout) as response:
                if response.status == 200:
                    raw_xml = response.read()
                    enc = response.headers.get("Content-Encoding") or response.headers.get("content-encoding")
                    raw_xml = resilient_decompress(raw_xml, encoding=enc)
                    entries = _parse_xml_entries(raw_xml)
                    
                    sub_sitemaps = [l for l, _ in entries if l.endswith(".xml") or l.endswith(".xml.gz") or "sitemap" in l]
                    page_entries = [(l, lm) for l, lm in entries if l not in sub_sitemaps]

                    for p, lm in page_entries:
                        discovered_pages.add(p)
                        if lm:
                            sitemap_lastmods[p] = lm
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
                                        sub_data = sub_resp.read()
                                        sub_enc = sub_resp.headers.get("Content-Encoding") or sub_resp.headers.get("content-encoding")
                                        sub_data = resilient_decompress(sub_data, encoding=sub_enc)
                                        sub_entries = _parse_xml_entries(sub_data)
                                        for sp, slm in sub_entries:
                                            if not sp.endswith(".xml") and not sp.endswith(".xml.gz"):
                                                discovered_pages.add(sp)
                                                if slm:
                                                    sitemap_lastmods[sp] = slm
                                                if len(discovered_pages) >= max_urls:
                                                    break
                            except Exception:
                                continue

                    if discovered_pages:
                        break
        except Exception:
            continue

    return list(discovered_pages), sitemap_lastmods


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
    effective_url = start_url

    # 1. Fetch start page
    try:
        req = urllib.request.Request(start_url, headers=headers)
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            effective_url = resp.geturl() or start_url
            html = resp.read().decode("utf-8", errors="ignore")
            title_match = re.search(r"<title>(.*?)</title>", html, re.IGNORECASE | re.DOTALL)
            if title_match:
                page_title = title_match.group(1).strip()
            link_extractor = HTMLLinkExtractor(base_url=effective_url)
            link_extractor.feed(html)
            initial_links = link_extractor.links
    except Exception as e:
        return {
            "start_url": start_url,
            "effective_url": effective_url,
            "domain": domain,
            "title": page_title,
            "section_urls": [start_url],
            "domain_urls": [start_url],
            "section_count": 1,
            "domain_count": 1,
            "has_sitemap": False,
            "error": str(e)
        }

    # Recompute semantic section prefix and domain if redirected (e.g. canonical trailing slash 301)
    if effective_url != start_url:
        parsed_effective = urllib.parse.urlparse(effective_url)
        domain = parsed_effective.netloc.lower()
        clean_path = parsed_effective.path
        if clean_path.endswith((".html", ".htm", ".php", ".asp", ".aspx")):
            clean_path = clean_path.rsplit(".", 1)[0]
        path_segments = [seg for seg in clean_path.split("/") if seg]
        section_prefix = "/" + "/".join(path_segments) if path_segments else "/"
        key_terms = [seg.lower() for seg in path_segments if len(seg) > 2]

    # 2. Check sitemap
    sitemap_urls, sitemap_lastmods = fetch_sitemap_urls(effective_url, max_urls=max_discovery, timeout=timeout)
    if not sitemap_urls and effective_url != start_url:
        fb_urls, fb_lastmods = fetch_sitemap_urls(start_url, max_urls=max_discovery, timeout=timeout)
        sitemap_urls.extend(fb_urls)
        sitemap_lastmods.update(fb_lastmods)

    all_domain_urls: Set[str] = {start_url, effective_url}
    all_domain_urls.update(sitemap_urls)

    for link in initial_links:
        parsed_link = urllib.parse.urlparse(link)
        if parsed_link.netloc.lower() == domain:
            all_domain_urls.add(link)

    # 3. Fast BFS expansion on top discovered pages if sitemap is small or absent
    if len(all_domain_urls) < 300:
        sample_urls = [u for u in list(all_domain_urls) if u not in (start_url, effective_url)][:20]
        for sub_u in sample_urls:
            try:
                sub_req = urllib.request.Request(sub_u, headers=headers)
                with urllib.request.urlopen(sub_req, timeout=3) as sub_resp:
                    sub_effective = sub_resp.geturl() or sub_u
                    sub_html = sub_resp.read().decode("utf-8", errors="ignore")
                    sub_extractor = HTMLLinkExtractor(base_url=sub_effective)
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

    # Rank all discovered URLs by semantic proximity and relevance to start_url / effective_url
    def _rank_url(u: str) -> tuple:
        if u in (start_url, effective_url):
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
        if p.startswith(clean_section_prefix + "/") or p == clean_section_prefix or p == (clean_section_prefix + ".html") or u in (start_url, effective_url):
            section_urls.append(u)
        elif any(term in p for term in key_terms if len(term) >= 5):
            section_urls.append(u)

    if not section_urls:
        section_urls = [effective_url] if effective_url else [start_url]

    return {
        "start_url": start_url,
        "effective_url": effective_url,
        "domain": domain,
        "title": page_title,
        "section_prefix": section_prefix,
        "section_urls": section_urls,
        "section_count": len(section_urls),
        "domain_urls": ranked_domain_urls,
        "domain_count": len(ranked_domain_urls),
        "has_sitemap": bool(sitemap_urls),
        "sitemap_lastmods": sitemap_lastmods
    }


def crawl_and_index_urls(
    workspace_name: str,
    urls: List[str],
    root_url: Optional[str] = None,
    root_title: Optional[str] = None,
    scope: str = "custom",
    force_refresh: bool = False,
    max_workers: int = 20,
    progress_callback = None,
    embed_progress_callback = None,
    sitemap_lastmods: Optional[Dict[str, str]] = None
) -> Dict[str, Any]:
    """
    High-speed concurrent multi-threaded scraper and LanceDB batch vector indexer.
    Implements:
      1. Sitemap <lastmod> in-memory diff (0 network calls for unchanged sitemap entries).
      2. HTTP Conditional GET (If-None-Match / If-Modified-Since with 304 Not Modified support).
      3. Incremental SHA-256 content deduplication ($0.00 vector cost for unchanged content).
      4. Parallel vector embeddings & columnar LanceDB persistence via ParallelIndexer.
    """
    import os
    import time
    import random
    import logging
    import hashlib
    import urllib.parse
    import urllib.error
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
    from any_context.vector_engine.indexer import ParallelIndexer
    from any_context.vector_engine.store import LanceDBStore
    from llama_index.core import Document

    store = WebSchedulerStore()
    lance_store = store._get_lance_store() if hasattr(store, "_get_lance_store") else LanceDBStore.get_instance()
    configure_embedding_model()
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

    # Load existing indexed pages map for incremental comparison
    indexed_map = store.get_indexed_pages_map(workspace_name, domain_or_prefix=domain)

    # 1. Sitemap <lastmod> In-Memory Pre-Filtering
    urls_to_scrape: List[str] = []
    if sitemap_lastmods and not force_refresh:
        for u in urls:
            cached_item = indexed_map.get(u)
            s_lm = sitemap_lastmods.get(u)
            if cached_item and s_lm and cached_item.get("sitemap_lastmod") == s_lm:
                # Sitemap timestamp unchanged - skip network call entirely
                skipped_count += 1
                processed_records.append({
                    "url": u,
                    "title": cached_item.get("title", u),
                    "content_hash": cached_item.get("content_hash", ""),
                    "char_count": cached_item.get("char_count", 0),
                    "scraped_at": now_str,
                    "etag": cached_item.get("etag"),
                    "http_last_modified": cached_item.get("http_last_modified"),
                    "sitemap_lastmod": s_lm
                })
                if progress_callback:
                    progress_callback(len(processed_records), total_urls, indexed_count, skipped_count, u, cached_item.get("title", u))
            else:
                urls_to_scrape.append(u)
    else:
        urls_to_scrape = list(urls)

    # 2. Multi-threaded concurrent download with HTTP Conditional GET & Anti-429 Backoff
    def _fetch_single(url: str) -> Optional[Dict[str, Any]]:
        if not is_url_allowed_by_robots(url):
            return None
        cached = indexed_map.get(url, {})
        c_etag = cached.get("etag")
        c_lastmod = cached.get("http_last_modified")

        for attempt in range(3):
            try:
                return scrape_url(url, cached_etag=c_etag, cached_last_modified=c_lastmod, timeout=10)
            except urllib.error.HTTPError as he:
                if he.code == 429 and attempt < 2:
                    retry_after = he.headers.get("Retry-After") if he.headers else None
                    try:
                        sleep_time = float(retry_after) if retry_after else (1.5 * (2 ** attempt) + random.uniform(0.1, 0.4))
                    except Exception:
                        sleep_time = (1.5 * (2 ** attempt) + random.uniform(0.1, 0.4))
                    time.sleep(sleep_time)
                else:
                    return None
            except Exception:
                if attempt < 2:
                    time.sleep(1.0 + random.uniform(0.1, 0.4))
                else:
                    return None
        return None

    # Process in streaming mini-batches (25 URLs at a time) for incremental resilience
    batch_size = 25
    url_batches = [urls_to_scrape[i:i + batch_size] for i in range(0, len(urls_to_scrape), batch_size)] if urls_to_scrape else []
    completed_urls_count = len(processed_records)

    for b_idx, batch_urls in enumerate(url_batches):
        batch_documents: List[Document] = []
        batch_records: List[Dict[str, Any]] = []

        with ThreadPoolExecutor(max_workers=min(max_workers, max(1, len(batch_urls)))) as executor:
            future_to_url = {executor.submit(_fetch_single, u): u for u in batch_urls}
            
            for future in as_completed(future_to_url):
                url = future_to_url[future]
                completed_urls_count += 1
                data = None
                try:
                    data = future.result()
                    if data and data.get("is_dynamic_spa"):
                        spa_detected_count += 1

                    # Scenario A: HTTP 304 Not Modified
                    if data and data.get("is_not_modified"):
                        skipped_count += 1
                        cached_meta = indexed_map.get(url, {})
                        rec = {
                            "url": url,
                            "title": cached_meta.get("title", url),
                            "content_hash": cached_meta.get("content_hash", ""),
                            "char_count": cached_meta.get("char_count", 0),
                            "scraped_at": now_str,
                            "etag": data.get("etag") or cached_meta.get("etag"),
                            "http_last_modified": data.get("http_last_modified") or cached_meta.get("http_last_modified"),
                            "sitemap_lastmod": (sitemap_lastmods.get(url) if sitemap_lastmods else cached_meta.get("sitemap_lastmod"))
                        }
                        batch_records.append(rec)
                        processed_records.append(rec)

                    # Scenario B: HTTP 200 OK with extracted text
                    elif data and data.get("content") and len(data["content"].strip()) > 30:
                        text_content = data["content"]
                        url_hash = data["hash"]
                        etag_val = data.get("etag")
                        http_lm_val = data.get("http_last_modified")
                        s_lm_val = sitemap_lastmods.get(url) if sitemap_lastmods else None

                        # Check if page is already indexed and hash has not changed (Skip / Zero Cost)
                        is_cached = (not force_refresh) and (url in indexed_map) and (indexed_map[url].get("content_hash") == url_hash)

                        rec = {
                            "url": url,
                            "title": data["title"],
                            "content_hash": url_hash,
                            "char_count": len(text_content),
                            "scraped_at": now_str,
                            "etag": etag_val,
                            "http_last_modified": http_lm_val,
                            "sitemap_lastmod": s_lm_val
                        }
                        batch_records.append(rec)
                        processed_records.append(rec)

                        if is_cached:
                            skipped_count += 1
                        else:
                            # If page was previously indexed but content changed, remove old vectors
                            if url in indexed_map:
                                try:
                                    lance_store.delete_by_file(url, workspace_name=workspace_name)
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
                                    "scraped_at": now_str,
                                    "etag": etag_val,
                                    "http_last_modified": http_lm_val,
                                    "sitemap_lastmod": s_lm_val
                                }
                            )
                            batch_documents.append(doc)
                            total_chars += len(text_content)
                            indexed_count += 1
                    else:
                        errors += 1
                except Exception:
                    errors += 1

                if progress_callback:
                    progress_callback(completed_urls_count, total_urls, indexed_count, skipped_count, url, (data.get("title") if data else ""))

        # Incremental Vector Indexing for this batch
        if batch_documents:
            try:
                parallel_indexer = ParallelIndexer(store=lance_store)
                parallel_indexer.index_documents(
                    documents=batch_documents,
                    workspace_name=workspace_name
                )
            except Exception as e:
                logging.getLogger("any_context").warning(f"Batch vector indexing error: {e}")

        # Incremental Root Web Source Update in SQLite
        if batch_records:
            curr_distinct = store.get_indexed_pages_count(workspace_name, domain_or_prefix=domain)
            store.add_or_update_root_web_source(
                workspace_name=workspace_name,
                root_url=effective_root,
                title=root_title or f"Web Portal ({curr_distinct} pages)",
                page_count=curr_distinct,
                scope=scope
            )

        if embed_progress_callback:
            embed_progress_callback(completed_urls_count, total_urls, indexed_count, skipped_count, "Vector Knowledge Base")

    # 4. Final distinct count consolidation in SQLite root source
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


def crawl_website(
    workspace_name: str,
    start_url: str,
    scope: str = "domain",
    max_pages: Optional[int] = None,
    force_rescrape: bool = False,
    max_workers: int = 20,
    progress_callback: Optional[Any] = None,
    embed_progress_callback: Optional[Any] = None
) -> Dict[str, Any]:
    """
    Programmatic, non-interactive website crawler and indexer with High-Speed Dual-Stage Parallel Pipeline.
    Discovers internal links/sitemaps and crawls them automatically into LanceDB.
    """
    disc = discover_site_urls(start_url)
    effective_url = disc.get("effective_url") or start_url
    if scope == "section":
        target_urls = disc.get("section_urls") or [effective_url]
    else:
        target_urls = disc.get("domain_urls") or disc.get("section_urls") or [effective_url]

    if max_pages and len(target_urls) > max_pages:
        target_urls = target_urls[:max_pages]

    return crawl_and_index_urls(
        workspace_name=workspace_name,
        urls=target_urls,
        root_url=effective_url,
        root_title=disc.get("title") or effective_url,
        scope=scope,
        force_refresh=force_rescrape,
        max_workers=max_workers,
        sitemap_lastmods=disc.get("sitemap_lastmods"),
        progress_callback=progress_callback,
        embed_progress_callback=embed_progress_callback
    )


def run_interactive_web_crawler(workspace_name: str, start_url: Optional[str] = None) -> bool:
    """
    Proxy for backward compatibility.
    Interactive terminal crawling wizards are part of the CLI presentation layer (any_context.cli.formatters).
    """
    from any_context.cli.formatters import run_interactive_web_crawler as _cli_crawler
    return _cli_crawler(workspace_name=workspace_name, start_url=start_url)
