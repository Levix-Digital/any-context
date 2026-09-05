import os
import sqlite3
import threading
import time
from datetime import datetime
from typing import List, Dict, Any, Optional

import chromadb
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_ingestor import scrape_url
from any_context.billing import BillingManager
from any_context.tools.search_tools import configure_embedding_model

from llama_index.core import Settings, Document
from llama_index.core.ingestion import IngestionPipeline, DocstoreStrategy
from llama_index.core.storage.docstore import SimpleDocumentStore
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore

class WebSchedulerStore:
    """
    SQLite persistence for workspace web URLs, content hashes, and polling schedules.
    """
    def __init__(self, db_path: Optional[str] = None):
        if db_path:
            self.db_path = os.path.abspath(db_path)
        else:
            self.db_path = ConfigDBStore.find_db_file("settings.db")
        parent_dir = os.path.dirname(self.db_path)
        if parent_dir and not os.path.exists(parent_dir):
            os.makedirs(parent_dir, exist_ok=True)
        self._init_db()

    def _get_connection(self):
        from any_context.config.database import DatabaseManager
        return DatabaseManager(self.db_path).get_connection()

    def _init_db(self):
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_web_urls (
                    id TEXT PRIMARY KEY,
                    workspace_name TEXT NOT NULL,
                    url TEXT NOT NULL,
                    title TEXT,
                    last_hash TEXT,
                    polling_interval_hours INTEGER DEFAULT 24,
                    last_scraped_at TEXT,
                    created_at TEXT,
                    page_count INTEGER DEFAULT 1,
                    root_url TEXT,
                    scope TEXT
                );
            """)
            # Ensure optional columns exist for existing tables
            cursor.execute("PRAGMA table_info(workspace_web_urls)")
            cols = [r[1] for r in cursor.fetchall()]
            if "page_count" not in cols:
                cursor.execute("ALTER TABLE workspace_web_urls ADD COLUMN page_count INTEGER DEFAULT 1")
            if "root_url" not in cols:
                cursor.execute("ALTER TABLE workspace_web_urls ADD COLUMN root_url TEXT")
            if "scope" not in cols:
                cursor.execute("ALTER TABLE workspace_web_urls ADD COLUMN scope TEXT")
            if "etag" not in cols:
                cursor.execute("ALTER TABLE workspace_web_urls ADD COLUMN etag TEXT")
            if "http_last_modified" not in cols:
                cursor.execute("ALTER TABLE workspace_web_urls ADD COLUMN http_last_modified TEXT")

            # Drop legacy workspace_indexed_web_pages table (LanceDB is Single Source of Truth)
            cursor.execute("DROP TABLE IF EXISTS workspace_indexed_web_pages;")
            conn.commit()

    def add_web_url(self, workspace_name: str, url: str, polling_interval_hours: int = 24, title: Optional[str] = None, scope: str = "domain") -> Dict[str, Any]:
        import uuid
        # Check if already exists in this workspace
        existing = self.get_workspace_web_urls(workspace_name)
        for e in existing:
            if e["url"] == url:
                return e

        url_id = f"web_{uuid.uuid4().hex[:8]}"
        now_str = datetime.utcnow().isoformat()
        clean_scope = scope or "domain"
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_web_urls (id, workspace_name, url, title, polling_interval_hours, page_count, root_url, created_at, scope)
                VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?)
            """, (url_id, workspace_name, url, title, polling_interval_hours, url, now_str, clean_scope))
            conn.commit()
        return {"id": url_id, "workspace_name": workspace_name, "url": url, "title": title, "polling_interval_hours": polling_interval_hours, "page_count": 1, "created_at": now_str, "scope": clean_scope}


    def add_or_update_root_web_source(
        self,
        workspace_name: str,
        root_url: str,
        title: str,
        page_count: int = 1,
        scope: str = "custom",
        polling_interval_hours: int = 24
    ) -> Dict[str, Any]:
        import uuid
        now_str = datetime.utcnow().isoformat()
        alt_root = root_url.rstrip("/") if root_url.endswith("/") else f"{root_url}/"
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT id, url FROM workspace_web_urls WHERE workspace_name = ? AND (url = ? OR url = ? OR root_url = ? OR root_url = ?)",
                (workspace_name, root_url, alt_root, root_url, alt_root)
            )
            row = cursor.fetchone()
            if row:
                url_id = row[0]
                matched_url = row[1]
                cursor.execute("""
                    UPDATE workspace_web_urls
                    SET title = ?, page_count = ?, scope = ?, last_scraped_at = ?
                    WHERE id = ?
                """, (title, page_count, scope, now_str, url_id))
            else:
                url_id = f"web_{uuid.uuid4().hex[:8]}"
                matched_url = root_url
                cursor.execute("""
                    INSERT INTO workspace_web_urls (id, workspace_name, url, title, page_count, root_url, scope, polling_interval_hours, last_scraped_at, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """, (url_id, workspace_name, root_url, title, page_count, root_url, scope, polling_interval_hours, now_str, now_str))

            # Clean up any legacy sub-urls that might have been recorded as individual rows under this domain/prefix
            cursor.execute("""
                DELETE FROM workspace_web_urls
                WHERE workspace_name = ? AND url != ? AND url != ? AND (root_url = ? OR root_url = ? OR url LIKE ?)
            """, (workspace_name, root_url, alt_root, root_url, alt_root, f"{root_url.rstrip('/')}/%"))

            conn.commit()
        return {
            "id": url_id,
            "workspace_name": workspace_name,
            "url": matched_url,
            "title": title,
            "page_count": page_count,
            "scope": scope,
            "last_scraped_at": now_str
        }

    def get_workspace_web_urls(self, workspace_name: str) -> List[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM workspace_web_urls WHERE workspace_name = ?", (workspace_name,))
            return [dict(r) for r in cursor.fetchall()]

    def get_all_web_urls(self) -> List[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM workspace_web_urls")
            return [dict(r) for r in cursor.fetchall()]

    def get_web_url_by_id(self, url_id: str) -> Optional[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM workspace_web_urls WHERE id = ?", (url_id,))
            row = cursor.fetchone()
            return dict(row) if row else None

    def delete_web_url(self, url_id: str, workspace_name: Optional[str] = None) -> bool:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            if workspace_name:
                cursor.execute("DELETE FROM workspace_web_urls WHERE id = ? AND workspace_name = ?", (url_id, workspace_name))
            else:
                cursor.execute("DELETE FROM workspace_web_urls WHERE id = ?", (url_id,))
            conn.commit()
            return cursor.rowcount > 0

    def delete_web_url_by_url(self, workspace_name: str, url: str) -> bool:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM workspace_web_urls WHERE workspace_name = ? AND (url = ? OR root_url = ?)", (workspace_name, url, url))
            conn.commit()
            rowcount = cursor.rowcount
        self.delete_indexed_pages_for_root(workspace_name, url)
        return rowcount > 0

    def update_url_hash(self, url_id: str, title: str, content_hash: str, etag: Optional[str] = None, http_last_modified: Optional[str] = None):
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                UPDATE workspace_web_urls
                SET title = ?, last_hash = ?, last_scraped_at = ?, etag = ?, http_last_modified = ?
                WHERE id = ?
            """, (title, content_hash, now_str, etag, http_last_modified, url_id))
            conn.commit()

    def _get_lance_store(self):
        from any_context.vector_engine.store import LanceDBStore
        if self.db_path:
            parent = os.path.dirname(self.db_path)
            candidate1 = os.path.join(parent, "lancedb")
            if os.path.exists(candidate1):
                return LanceDBStore.get_instance(db_path=candidate1)
            candidate2 = os.path.join(parent, "context_db", "lancedb")
            if os.path.exists(candidate2):
                return LanceDBStore.get_instance(db_path=candidate2)

        settings = AppSettings.load()
        db_save_path = settings.context.db_path if settings else "./context_db"
        lance_dir = os.path.join(db_save_path, "lancedb")
        return LanceDBStore.get_instance(db_path=lance_dir)

    def get_indexed_pages_map(
        self,
        workspace_name: str,
        root_url: Optional[str] = None,
        domain_or_prefix: Optional[str] = None
    ) -> Dict[str, Dict[str, Any]]:
        """
        Returns a dictionary mapping url -> {url, title, content_hash, last_modified, ...}
        for all pages previously indexed in this workspace matching root_url or domain_or_prefix.
        Queries LanceDB directly as the Single Source of Truth.
        """
        try:
            l_store = self._get_lance_store()
            target_prefix = domain_or_prefix or root_url
            return l_store.get_indexed_pages_map(workspace_name, domain_or_prefix=target_prefix)
        except Exception:
            return {}

    def get_indexed_pages_count(
        self,
        workspace_name: str,
        root_url: Optional[str] = None,
        domain_or_prefix: Optional[str] = None
    ) -> int:
        """
        Returns total count of distinct web pages indexed for this workspace / root_url from LanceDB.
        """
        return len(self.get_indexed_pages_map(workspace_name, root_url=root_url, domain_or_prefix=domain_or_prefix))

    def delete_indexed_pages_for_root(self, workspace_name: str, root_url: str):
        """Purges indexed web chunks for a given root URL from LanceDB."""
        try:
            l_store = self._get_lance_store()
            l_store.delete_by_file(root_url, workspace_name=workspace_name)
        except Exception:
            pass

    def transfer_web_source(
        self,
        source_ws: str,
        target_ws: str,
        url_or_root: str
    ) -> Dict[str, Any]:
        """
        Transfers a web source, all its indexed sub-pages and LanceDB vector chunks from source_ws to target_ws in < 50ms.
        1. Updates workspace_web_urls in SQLite.
        2. Updates chunk metadata ('workspace': target_ws) in LanceDB.
        """
        source_ws = source_ws.strip()
        target_ws = target_ws.strip()
        target_url = url_or_root.strip()

        if source_ws == target_ws:
            return {"success": False, "error": "Source and target workspaces cannot be the same."}

        exact_root = target_url
        with self._get_connection() as conn:
            cursor = conn.cursor()
            # Find matching web source
            cursor.execute(
                "SELECT * FROM workspace_web_urls WHERE workspace_name = ? AND (url = ? OR root_url = ?)",
                (source_ws, target_url, target_url)
            )
            rows = cursor.fetchall()
            if not rows:
                cursor.execute(
                    "SELECT * FROM workspace_web_urls WHERE workspace_name = ? AND url LIKE ?",
                    (source_ws, f"%{target_url}%")
                )
                rows = cursor.fetchall()

            if rows:
                exact_root = rows[0]["root_url"] or rows[0]["url"] or target_url

            # Update workspace_web_urls
            cursor.execute(
                "UPDATE workspace_web_urls SET workspace_name = ? WHERE workspace_name = ? AND (url = ? OR root_url = ? OR url LIKE ?)",
                (target_ws, source_ws, exact_root, exact_root, f"%{exact_root}%")
            )
            conn.commit()

        # Update LanceDB vector metadata
        transferred_chunks = 0
        try:
            l_store = self._get_lance_store()
            transferred_chunks = l_store.transfer_file(source_ws, target_ws, exact_root)
        except Exception:
            pass

        return {
            "success": True,
            "source_workspace": source_ws,
            "target_workspace": target_ws,
            "url": exact_root,
            "transferred_pages": transferred_chunks,
            "transferred_chunks": transferred_chunks
        }



def index_web_url_to_chromadb(workspace_name: str, url: str, url_id: Optional[str] = None, force: bool = False) -> Dict[str, Any]:
    """
    Scrapes a web URL, computes SHA-256 hash, and indexes content into ChromaDB if updated.
    Enforces feature gates via BillingManager and stores into the active workspace context_docs collection.
    """
    b_mgr = BillingManager()
    if not b_mgr.can_ingest_source("web"):
        msg = "⚠️ Feature Gate: Web Scraping requires 'Pro', 'Team', or 'Enterprise' plan tier."
        print(msg)
        return {"status": "error", "message": msg, "gate_blocked": True}

    store = WebSchedulerStore()
    if not url_id:
        entry = store.add_web_url(workspace_name=workspace_name, url=url)
        url_id = entry["id"]

    try:
        urls = store.get_workspace_web_urls(workspace_name)
        curr_entry = next((u for u in urls if u["id"] == url_id or u["url"] == url), None)
        c_etag = curr_entry.get("etag") if curr_entry else None
        c_lastmod = curr_entry.get("http_last_modified") if curr_entry else None

        data = scrape_url(url, cached_etag=c_etag, cached_last_modified=c_lastmod)

        if not force and data.get("is_not_modified"):
            return {
                "status": "unchanged",
                "message": f"Content for '{url}' has not changed (HTTP 304 Not Modified).",
                "title": curr_entry.get("title", url) if curr_entry else url
            }

        if not force and curr_entry and curr_entry.get("last_hash") == data["hash"]:
            return {"status": "unchanged", "message": f"Content for '{url}' has not changed.", "title": data["title"]}

        # Index to main context_db via ParallelIndexer & LanceDBStore
        configure_embedding_model()
        settings = AppSettings.load()
        db_save_path = settings.context.db_path if settings else "./context_db"
        lance_dir = os.path.join(db_save_path, "lancedb")
        os.makedirs(lance_dir, exist_ok=True)
        
        from any_context.vector_engine.store import LanceDBStore
        from any_context.vector_engine.indexer import ParallelIndexer
        from any_context.vector_engine.models import IngestionConfig

        lance_store = LanceDBStore.get_instance(db_path=lance_dir)

        doc = Document(
            text=f"Web Document Title: {data['title']}\nSource URL: {data['url']}\n\n{data['content']}",
            metadata={
                "workspace": workspace_name,
                "file_name": data["title"],
                "file_path": data["url"],
                "source": data["url"],
                "type": "web",
                "content_type": "Web Documentation",
                "last_modified": time.strftime("%Y-%m-%d"),
                "char_count": data["char_count"]
            },
            id_=f"web_{data['url']}"
        )

        chunk_size = settings.context.chunk_size if (settings and settings.context) else 1024
        chunk_overlap = settings.context.chunk_overlap if (settings and settings.context) else 200

        indexer = ParallelIndexer(store=lance_store)
        cfg = IngestionConfig(chunk_size=chunk_size, chunk_overlap=chunk_overlap, max_workers=2)
        indexer.index_documents(documents=[doc], workspace_name=workspace_name, config=cfg)

        store.update_url_hash(
            url_id,
            data["title"],
            data["hash"],
            etag=data.get("etag"),
            http_last_modified=data.get("http_last_modified")
        )
        return {
            "status": "success",
            "message": f"Successfully scraped and indexed '{data['title']}' ({data['char_count']} chars).",
            "title": data["title"],
            "url": url,
            "char_count": data["char_count"]
        }
    except Exception as e:
        err_msg = f"Error scraping and indexing web URL '{url}': {str(e)}"
        print(f"❌ {err_msg}")
        return {"status": "error", "message": err_msg}


def remove_web_url_from_chromadb(workspace_name: str, url: str) -> bool:
    """
    Deletes vectors associated with a specific web URL or root source URL in a workspace from LanceDB.
    """
    try:
        settings = AppSettings.load()
        db_save_path = settings.context.db_path if settings else "./context_db"
        from any_context.vector_engine.store import LanceDBStore
        l_store = LanceDBStore.get_instance(db_path=os.path.join(db_save_path, "lancedb"))
        l_store.delete_by_file(url, workspace_name=workspace_name)
        clean_url = url.rstrip("/")
        l_store.delete_by_file(f"{clean_url}/", workspace_name=workspace_name)
        return True
    except Exception as e:
        print(f"⚠️ Warning removing web vectors for '{url}': {e}")
        return False


def sync_workspace_web_urls(
    workspace_name: str,
    force: bool = False,
    progress_callback: Optional[Any] = None
) -> Dict[str, Any]:
    """
    Synchronizes / re-indexes all web URLs and crawled portals configured for a workspace.
    Properly handles both single URLs and multi-page crawled portals.
    """
    store = WebSchedulerStore()
    urls = store.get_workspace_web_urls(workspace_name)
    # Prioritize unscraped URLs so newly added web sources are processed immediately
    urls.sort(key=lambda x: 0 if not x.get("last_scraped_at") else 1)
    results = []
    total_u = len(urls)
    for idx, u in enumerate(urls):
        if progress_callback:
            progress_callback(idx + 1, total_u, "web", u["url"])
        page_count = u.get("page_count", 1) or 1
        scope = u.get("scope") or "domain"
        if page_count > 1 or scope in ["domain", "section", "custom"]:
            from any_context.ingestion.web_crawler import crawl_website

            def _sub_crawl_prog(curr, tot, idxed, skp, url, title):
                if progress_callback:
                    progress_callback(curr, tot, "pages", url)

            try:
                crawl_res = crawl_website(
                    workspace_name=workspace_name,
                    start_url=u["url"],
                    scope=scope,
                    force_rescrape=force,
                    progress_callback=_sub_crawl_prog
                )
                results.append({"url": u["url"], "result": crawl_res, "type": "portal"})
            except Exception as e:
                results.append({"url": u["url"], "result": {"status": "error", "error": str(e)}, "type": "portal"})
        else:
            res = index_web_url_to_chromadb(workspace_name=workspace_name, url=u["url"], url_id=u["id"], force=force)
            results.append({"url": u["url"], "result": res, "type": "single_page"})
    return {"workspace_name": workspace_name, "total_urls": len(urls), "synced": results}

