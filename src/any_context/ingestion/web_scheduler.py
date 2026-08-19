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
        self.db_path = db_path or ConfigDBStore.find_db_file("settings.db")
        self._init_db()

    def _get_connection(self):
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

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

            # Table for granular tracking of individual web pages indexed within websites
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspace_indexed_web_pages (
                    id TEXT PRIMARY KEY,
                    workspace_name TEXT NOT NULL,
                    url TEXT NOT NULL,
                    root_url TEXT NOT NULL,
                    title TEXT,
                    content_hash TEXT NOT NULL,
                    char_count INTEGER DEFAULT 0,
                    scraped_at TEXT,
                    created_at TEXT
                );
            """)
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_wiwp_ws_root ON workspace_indexed_web_pages (workspace_name, root_url);")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_wiwp_ws_url ON workspace_indexed_web_pages (workspace_name, url);")
            conn.commit()

    def add_web_url(self, workspace_name: str, url: str, polling_interval_hours: int = 24, title: Optional[str] = None) -> Dict[str, Any]:
        import uuid
        # Check if already exists in this workspace
        existing = self.get_workspace_web_urls(workspace_name)
        for e in existing:
            if e["url"] == url:
                return e

        url_id = f"web_{uuid.uuid4().hex[:8]}"
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_web_urls (id, workspace_name, url, title, polling_interval_hours, page_count, root_url, created_at)
                VALUES (?, ?, ?, ?, ?, 1, ?, ?)
            """, (url_id, workspace_name, url, title, polling_interval_hours, url, now_str))
            conn.commit()
        return {"id": url_id, "workspace_name": workspace_name, "url": url, "title": title, "polling_interval_hours": polling_interval_hours, "page_count": 1, "created_at": now_str}

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
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "SELECT id FROM workspace_web_urls WHERE workspace_name = ? AND url = ?",
                (workspace_name, root_url)
            )
            row = cursor.fetchone()
            if row:
                url_id = row[0]
                cursor.execute("""
                    UPDATE workspace_web_urls
                    SET title = ?, page_count = ?, scope = ?, last_scraped_at = ?
                    WHERE id = ?
                """, (title, page_count, scope, now_str, url_id))
            else:
                url_id = f"web_{uuid.uuid4().hex[:8]}"
                cursor.execute("""
                    INSERT INTO workspace_web_urls (id, workspace_name, url, title, page_count, root_url, scope, polling_interval_hours, last_scraped_at, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """, (url_id, workspace_name, root_url, title, page_count, root_url, scope, polling_interval_hours, now_str, now_str))

            # Clean up any legacy sub-urls that might have been recorded as individual rows under this domain/prefix
            cursor.execute("""
                DELETE FROM workspace_web_urls
                WHERE workspace_name = ? AND url != ? AND (root_url = ? OR url LIKE ?)
            """, (workspace_name, root_url, root_url, f"{root_url.rstrip('/')}/%"))

            conn.commit()
        return {
            "id": url_id,
            "workspace_name": workspace_name,
            "url": root_url,
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
            cursor.execute("DELETE FROM workspace_indexed_web_pages WHERE workspace_name = ? AND (url = ? OR root_url = ?)", (workspace_name, url, url))
            conn.commit()
            return cursor.rowcount > 0

    def update_url_hash(self, url_id: str, title: str, content_hash: str):
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                UPDATE workspace_web_urls
                SET title = ?, last_hash = ?, last_scraped_at = ?
                WHERE id = ?
            """, (title, content_hash, now_str, url_id))
            conn.commit()

    def get_indexed_pages_map(
        self,
        workspace_name: str,
        root_url: Optional[str] = None,
        domain_or_prefix: Optional[str] = None
    ) -> Dict[str, Dict[str, Any]]:
        """
        Returns a dictionary mapping url -> {title, content_hash, char_count, scraped_at, root_url}
        for all pages previously indexed in this workspace matching root_url or domain_or_prefix.
        """
        with self._get_connection() as conn:
            cursor = conn.cursor()
            if root_url:
                cursor.execute(
                    "SELECT url, title, content_hash, char_count, scraped_at, root_url FROM workspace_indexed_web_pages WHERE workspace_name = ? AND root_url = ?",
                    (workspace_name, root_url)
                )
            elif domain_or_prefix:
                cursor.execute(
                    "SELECT url, title, content_hash, char_count, scraped_at, root_url FROM workspace_indexed_web_pages WHERE workspace_name = ? AND (url LIKE ? OR root_url LIKE ?)",
                    (workspace_name, f"%{domain_or_prefix}%", f"%{domain_or_prefix}%")
                )
            else:
                cursor.execute(
                    "SELECT url, title, content_hash, char_count, scraped_at, root_url FROM workspace_indexed_web_pages WHERE workspace_name = ?",
                    (workspace_name,)
                )
            res_map = {
                r["url"]: {
                    "title": r["title"],
                    "content_hash": r["content_hash"],
                    "char_count": r["char_count"],
                    "scraped_at": r["scraped_at"],
                    "root_url": r["root_url"]
                }
                for r in cursor.fetchall()
            }

        # Auto-sync backfill from ChromaDB if table was empty for legacy sessions
        if not res_map:
            try:
                import chromadb
                from any_context.config.app_settings import AppSettings
                settings = AppSettings.load()
                db_save_path = settings.context.db_path if settings else "./context_db"
                coll_name = settings.context.collection_name if settings else "context_docs"
                if os.path.exists(db_save_path):
                    client = chromadb.PersistentClient(path=db_save_path)
                    try:
                        coll = client.get_collection(coll_name)
                        records = coll.get(where={"workspace": workspace_name})
                        if records and records.get("metadatas"):
                            backfill_pages = []
                            for m in records["metadatas"]:
                                if m and m.get("source_type") == "web" and m.get("url"):
                                    u = m.get("url")
                                    if domain_or_prefix and domain_or_prefix not in u:
                                        continue
                                    if root_url and m.get("root_url") != root_url and u != root_url:
                                        continue
                                    if u not in res_map:
                                        res_map[u] = {
                                            "title": m.get("title", u),
                                            "content_hash": m.get("content_hash", "legacy"),
                                            "char_count": 0,
                                            "scraped_at": m.get("scraped_at", ""),
                                            "root_url": m.get("root_url", u)
                                        }
                                        backfill_pages.append({
                                            "url": u,
                                            "title": m.get("title", u),
                                            "content_hash": m.get("content_hash", "legacy"),
                                            "char_count": 0,
                                            "scraped_at": m.get("scraped_at", "")
                                        })
                            if backfill_pages:
                                self.record_indexed_web_pages(workspace_name, root_url or list(res_map.keys())[0], backfill_pages)
                    except Exception:
                        pass
            except Exception:
                pass

        return res_map

    def get_indexed_pages_count(
        self,
        workspace_name: str,
        root_url: Optional[str] = None,
        domain_or_prefix: Optional[str] = None
    ) -> int:
        """
        Returns total count of distinct web pages indexed for this workspace / root_url.
        """
        return len(self.get_indexed_pages_map(workspace_name, root_url=root_url, domain_or_prefix=domain_or_prefix))

    def record_indexed_web_pages(
        self,
        workspace_name: str,
        root_url: str,
        pages: List[Dict[str, Any]]
    ):
        """
        Upserts multiple crawled web pages into workspace_indexed_web_pages with SHA-256 content hashes.
        """
        import hashlib
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            for p in pages:
                url = p["url"]
                page_id = hashlib.sha256(f"{workspace_name}:{url}".encode()).hexdigest()[:24]
                cursor.execute("""
                    INSERT INTO workspace_indexed_web_pages (
                        id, workspace_name, url, root_url, title, content_hash, char_count, scraped_at, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        title = excluded.title,
                        content_hash = excluded.content_hash,
                        char_count = excluded.char_count,
                        scraped_at = excluded.scraped_at
                """, (
                    page_id,
                    workspace_name,
                    url,
                    root_url,
                    p.get("title", ""),
                    p.get("content_hash", ""),
                    p.get("char_count", 0),
                    p.get("scraped_at", now_str),
                    now_str
                ))
            conn.commit()

    def delete_indexed_pages_for_root(self, workspace_name: str, root_url: str):
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute(
                "DELETE FROM workspace_indexed_web_pages WHERE workspace_name = ? AND (root_url = ? OR url = ?)",
                (workspace_name, root_url, root_url)
            )
            conn.commit()

    def transfer_web_source(
        self,
        source_ws: str,
        target_ws: str,
        url_or_root: str
    ) -> Dict[str, Any]:
        """
        Transfers a web source, all its indexed sub-pages and ChromaDB vector chunks from source_ws to target_ws in < 50ms.
        1. Updates workspace_web_urls and workspace_indexed_web_pages in SQLite.
        2. Updates chunk metadata ('workspace': target_ws) in ChromaDB.
        """
        source_ws = source_ws.strip()
        target_ws = target_ws.strip()
        target_url = url_or_root.strip()

        if source_ws == target_ws:
            return {"success": False, "error": "Source and target workspaces cannot be the same."}

        transferred_pages = 0
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

            # Update workspace_indexed_web_pages
            cursor.execute(
                "UPDATE workspace_indexed_web_pages SET workspace_name = ? WHERE workspace_name = ? AND (root_url = ? OR url = ? OR root_url LIKE ? OR url LIKE ?)",
                (target_ws, source_ws, exact_root, exact_root, f"%{exact_root}%", f"%{exact_root}%")
            )
            transferred_pages = cursor.rowcount
            conn.commit()

        # Update ChromaDB vector metadata
        transferred_chunks = 0
        try:
            settings = AppSettings.load()
            db_path = settings.context.db_path if (settings and settings.context) else "./context_db"
            coll_name = settings.context.collection_name if (settings and settings.context) else "context_docs"

            if os.path.exists(db_path):
                import chromadb
                client = chromadb.PersistentClient(path=db_path)
                try:
                    collection = client.get_collection(coll_name)
                    results = collection.get(
                        where={"workspace": source_ws},
                        include=["metadatas"]
                    )
                    ids_to_update = []
                    metas_to_update = []

                    if results and results.get("ids"):
                        for cid, meta in zip(results["ids"], results["metadatas"]):
                            c_root = meta.get("root_url", "")
                            c_url = meta.get("url", "")
                            c_source = meta.get("source", "")
                            if (exact_root and (exact_root in c_root or exact_root in c_url or exact_root in c_source)) or (target_url and target_url in c_url):
                                ids_to_update.append(cid)
                                new_meta = dict(meta)
                                new_meta["workspace"] = target_ws
                                metas_to_update.append(new_meta)

                    if ids_to_update:
                        collection.update(
                            ids=ids_to_update,
                            metadatas=metas_to_update
                        )
                        transferred_chunks = len(ids_to_update)
                except Exception:
                    pass
        except Exception:
            pass

        return {
            "success": True,
            "source_workspace": source_ws,
            "target_workspace": target_ws,
            "url": exact_root,
            "transferred_pages": transferred_pages,
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
        data = scrape_url(url)
        urls = store.get_workspace_web_urls(workspace_name)
        curr_entry = next((u for u in urls if u["id"] == url_id or u["url"] == url), None)

        if not force and curr_entry and curr_entry.get("last_hash") == data["hash"]:
            return {"status": "unchanged", "message": f"Content for '{url}' has not changed.", "title": data["title"]}

        # Index to main context_db ChromaDB collection
        configure_embedding_model()
        settings = AppSettings.load()
        db_save_path = settings.context.db_path if settings else "./context_db"
        collection_name = settings.context.collection_name if settings else "context_docs"
        os.makedirs(db_save_path, exist_ok=True)

        db = chromadb.PersistentClient(path=db_save_path)
        chroma_collection = db.get_or_create_collection(collection_name)
        vector_store = ChromaVectorStore(chroma_collection=chroma_collection)

        docstore_path = os.path.join(db_save_path, "docstore.json")
        if os.path.exists(docstore_path):
            docstore = SimpleDocumentStore.from_persist_path(persist_path=docstore_path)
        else:
            docstore = SimpleDocumentStore()

        chunk_size = settings.context.chunk_size if (settings and settings.context) else 1024
        chunk_overlap = settings.context.chunk_overlap if (settings and settings.context) else 200

        pipeline = IngestionPipeline(
            transformations=[
                SentenceSplitter(chunk_size=chunk_size, chunk_overlap=chunk_overlap),
                Settings.embed_model
            ],
            vector_store=vector_store,
            docstore=docstore,
            docstore_strategy=DocstoreStrategy.UPSERTS
        )

        doc = Document(
            text=f"Web Document Title: {data['title']}\nSource URL: {data['url']}\n\n{data['content']}",
            metadata={
                "workspace": workspace_name,
                "file_name": data["title"],
                "file_path": data["url"],
                "source": data["url"],
                "type": "web",
                "char_count": data["char_count"]
            },
            id_=f"web_{data['url']}"
        )

        pipeline.run(documents=[doc], show_progress=False)
        docstore.persist(persist_path=docstore_path)

        store.update_url_hash(url_id, data["title"], data["hash"])
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
    Deletes vectors associated with a specific web URL or root source URL in a workspace.
    """
    try:
        settings = AppSettings.load()
        db_save_path = settings.context.db_path if settings else "./context_db"
        collection_name = settings.context.collection_name if settings else "context_docs"
        if not os.path.exists(db_save_path):
            return True

        db = chromadb.PersistentClient(path=db_save_path)
        chroma_collection = db.get_or_create_collection(collection_name)
        
        # 1. Delete vectors by root_url match
        try:
            chroma_collection.delete(where={"$and": [{"workspace": workspace_name}, {"root_url": url}]})
        except Exception:
            pass

        # 2. Delete vectors by exact file_path match
        try:
            chroma_collection.delete(where={"$and": [{"workspace": workspace_name}, {"file_path": url}]})
        except Exception:
            pass

        # 3. Delete vectors by url match
        try:
            chroma_collection.delete(where={"$and": [{"workspace": workspace_name}, {"url": url}]})
        except Exception:
            pass
        return True
    except Exception as e:
        print(f"⚠️ Warning removing web vectors for '{url}': {e}")
        return False


def sync_workspace_web_urls(workspace_name: str) -> Dict[str, Any]:
    """
    Synchronizes / re-indexes all web URLs configured for a workspace.
    """
    store = WebSchedulerStore()
    urls = store.get_workspace_web_urls(workspace_name)
    results = []
    for u in urls:
        res = index_web_url_to_chromadb(workspace_name=workspace_name, url=u["url"], url_id=u["id"])
        results.append({"url": u["url"], "result": res})
    return {"workspace_name": workspace_name, "total_urls": len(urls), "synced": results}
