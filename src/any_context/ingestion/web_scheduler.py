import sqlite3
import threading
import time
from datetime import datetime
from typing import List, Dict, Any, Optional
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_ingestor import scrape_url
from any_context.billing import BillingManager

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
                    created_at TEXT
                );
            """)
            conn.commit()

    def add_web_url(self, workspace_name: str, url: str, polling_interval_hours: int = 24) -> Dict[str, Any]:
        import uuid
        url_id = f"web_{uuid.uuid4().hex[:8]}"
        now_str = datetime.utcnow().isoformat()
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspace_web_urls (id, workspace_name, url, polling_interval_hours, created_at)
                VALUES (?, ?, ?, ?, ?)
            """, (url_id, workspace_name, url, polling_interval_hours, now_str))
            conn.commit()
        return {"id": url_id, "workspace_name": workspace_name, "url": url, "polling_interval_hours": polling_interval_hours}

    def get_workspace_web_urls(self, workspace_name: str) -> List[Dict[str, Any]]:
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT * FROM workspace_web_urls WHERE workspace_name = ?", (workspace_name,))
            return [dict(r) for r in cursor.fetchall()]

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


def index_web_url_to_chromadb(workspace_name: str, url: str, url_id: str) -> bool:
    """
    Scrapes a web URL, computes SHA-256 hash, and indexes content into ChromaDB if updated.
    Enforces feature gates via BillingManager.
    """
    b_mgr = BillingManager()
    if not b_mgr.can_ingest_source("web"):
        print(f"⚠️ Billing Gate: Web Scraping requires 'Web', 'Pro', 'Team', or 'Enterprise' tier.")
        return False

    try:
        data = scrape_url(url)
        store = WebSchedulerStore()
        urls = store.get_workspace_web_urls(workspace_name)
        curr_entry = next((u for u in urls if u["id"] == url_id or u["url"] == url), None)

        if curr_entry and curr_entry.get("last_hash") == data["hash"]:
            return False  # Unchanged

        # Index to ChromaDB
        from any_context.memory.store import MemoryVectorStore
        from llama_index.core import Document
        
        m_store = MemoryVectorStore(workspace_name=workspace_name)
        doc = Document(
            text=f"Web Document Title: {data['title']}\nSource URL: {data['url']}\n\n{data['content']}",
            metadata={"source": data["url"], "type": "web", "title": data["title"]}
        )
        m_store.add_documents([doc])
        store.update_url_hash(url_id, data["title"], data["hash"])
        return True
    except Exception as e:
        print(f"❌ Error scraping web URL '{url}': {e}")
        return False
