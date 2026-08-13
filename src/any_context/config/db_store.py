import os
import sys
import json
import sqlite3
from typing import Optional, List, Dict
from any_context.config.app_settings import (
    AppSettings,
    WorkspaceSettings,
    ContextSettings,
    SessionSettings,
    ModelSettings,
    MemorySettings
)

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

class ConfigDBStore:
    """
    SQLite-backed Configuration Storage Manager
    Handles persistent CRUD operations for Workspaces, Models, Database paths, Memory settings, and API Keys.
    """

    def __init__(self, db_path: Optional[str] = None):
        self.db_path = db_path or self.find_db_file("settings.db")
        self._init_db()
        self._auto_migrate_if_needed()

    @classmethod
    def find_db_file(cls, filename: str = "settings.db") -> str:
        """Resolves the settings.db SQLite file location"""
        candidates = [
            os.path.join(os.getcwd(), "config", filename),
            os.path.join(os.getcwd(), filename),
            os.path.expanduser(os.path.join("~", ".config", "any-context", filename)),
        ]

        if sys.platform == "win32" and "APPDATA" in os.environ:
            candidates.append(os.path.join(os.environ["APPDATA"], "any-context", filename))

        for candidate in candidates:
            if os.path.exists(candidate):
                return os.path.abspath(candidate)

        target_dir = os.path.join(os.getcwd(), "config")
        os.makedirs(target_dir, exist_ok=True)
        return os.path.abspath(os.path.join(target_dir, filename))

    def _get_connection(self) -> sqlite3.Connection:
        os.makedirs(os.path.dirname(self.db_path), exist_ok=True)
        conn = sqlite3.connect(self.db_path, check_same_thread=False)
        conn.row_factory = sqlite3.Row
        return conn

    def _init_db(self):
        """Creates configuration tables if they do not exist"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS workspaces (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT UNIQUE NOT NULL,
                    paths_json TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS models (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    local_embedding_model TEXT NOT NULL,
                    local_openai_embedding_model TEXT NOT NULL,
                    inference_model TEXT NOT NULL,
                    summary_model TEXT NOT NULL,
                    model_provider TEXT NOT NULL,
                    local_base_url TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS context_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    db_path TEXT NOT NULL,
                    collection_name TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS session_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    db_path TEXT NOT NULL,
                    collection_name TEXT NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS memory_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    short_term_buffer_size INTEGER NOT NULL,
                    rolling_window_messages INTEGER NOT NULL,
                    meta_summary_threshold INTEGER NOT NULL,
                    meta_summary_batch_size INTEGER NOT NULL
                )
            """)

            cursor.execute("""
                CREATE TABLE IF NOT EXISTS api_keys (
                    provider TEXT PRIMARY KEY,
                    api_key TEXT NOT NULL
                )
            """)
            conn.commit()

    def is_empty(self) -> bool:
        """Returns True if no workspaces are configured in SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT COUNT(*) FROM workspaces")
            count = cursor.fetchone()[0]
            return count == 0

    def _auto_migrate_if_needed(self):
        """Auto-migrates existing settings.json into SQLite if DB is empty"""
        if not self.is_empty():
            return

        json_path = AppSettings.find_config_file("settings.json")
        if json_path and os.path.exists(json_path):
            try:
                settings = AppSettings.load(json_path)
                if settings:
                    safe_print(f"🔄 Auto-migrating settings from {json_path} into SQLite database ({self.db_path})...")
                    self.save_app_settings(settings)
                    safe_print("✅ Auto-migration complete!")
            except Exception as e:
                safe_print(f"⚠️ Warning: Auto-migration from JSON failed: {e}")

    def get_app_settings(self) -> Optional[AppSettings]:
        """Loads and returns an AppSettings instance from SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()

            cursor.execute("SELECT name, paths_json FROM workspaces")
            ws_rows = cursor.fetchall()
            workspaces = [
                WorkspaceSettings(name=row["name"], paths=json.loads(row["paths_json"]))
                for row in ws_rows
            ]

            cursor.execute("SELECT * FROM models WHERE id = 1")
            m_row = cursor.fetchone()
            if m_row:
                models = ModelSettings(
                    local_embedding_model=m_row["local_embedding_model"],
                    local_openai_embedding_model=m_row["local_openai_embedding_model"],
                    inference_model=m_row["inference_model"],
                    summary_model=m_row["summary_model"],
                    model_provider=m_row["model_provider"],
                    local_base_url=m_row["local_base_url"]
                )
            else:
                models = ModelSettings(
                    local_embedding_model="text-embedding-multilingual-e5-small",
                    local_openai_embedding_model="text-embedding-3-small",
                    inference_model="gpt-4o-mini",
                    summary_model="google/gemma-4-e2b",
                    model_provider="openai",
                    local_base_url="http://localhost:1234/v1"
                )

            cursor.execute("SELECT * FROM context_settings WHERE id = 1")
            c_row = cursor.fetchone()
            context = ContextSettings(
                db_path=c_row["db_path"] if c_row else "./context_db",
                collection_name=c_row["collection_name"] if c_row else "context_docs"
            )

            cursor.execute("SELECT * FROM session_settings WHERE id = 1")
            s_row = cursor.fetchone()
            session = SessionSettings(
                db_path=s_row["db_path"] if s_row else "./memory",
                collection_name=s_row["collection_name"] if s_row else "session_docs"
            )

            cursor.execute("SELECT * FROM memory_settings WHERE id = 1")
            mem_row = cursor.fetchone()
            if mem_row:
                memory = MemorySettings(
                    short_term_buffer_size=mem_row["short_term_buffer_size"],
                    rolling_window_messages=mem_row["rolling_window_messages"],
                    meta_summary_threshold=mem_row["meta_summary_threshold"],
                    meta_summary_batch_size=mem_row["meta_summary_batch_size"]
                )
            else:
                memory = MemorySettings()

            return AppSettings(
                workspaces=workspaces,
                context=context,
                session=session,
                models=models,
                memory=memory
            )

    def save_app_settings(self, settings: AppSettings):
        """Saves a complete AppSettings object into SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()

            cursor.execute("DELETE FROM workspaces")
            for ws in settings.workspaces:
                cursor.execute(
                    "INSERT INTO workspaces (name, paths_json) VALUES (?, ?)",
                    (ws.name, json.dumps(ws.paths))
                )

            cursor.execute("""
                INSERT OR REPLACE INTO models 
                (id, local_embedding_model, local_openai_embedding_model, inference_model, summary_model, model_provider, local_base_url)
                VALUES (1, ?, ?, ?, ?, ?, ?)
            """, (
                settings.models.local_embedding_model,
                settings.models.local_openai_embedding_model,
                settings.models.inference_model,
                settings.models.summary_model,
                settings.models.model_provider,
                settings.models.local_base_url
            ))

            cursor.execute("""
                INSERT OR REPLACE INTO context_settings (id, db_path, collection_name)
                VALUES (1, ?, ?)
            """, (settings.context.db_path, settings.context.collection_name))

            cursor.execute("""
                INSERT OR REPLACE INTO session_settings (id, db_path, collection_name)
                VALUES (1, ?, ?)
            """, (settings.session.db_path, settings.session.collection_name))

            cursor.execute("""
                INSERT OR REPLACE INTO memory_settings 
                (id, short_term_buffer_size, rolling_window_messages, meta_summary_threshold, meta_summary_batch_size)
                VALUES (1, ?, ?, ?, ?)
            """, (
                settings.memory.short_term_buffer_size,
                settings.memory.rolling_window_messages,
                settings.memory.meta_summary_threshold,
                settings.memory.meta_summary_batch_size
            ))

            conn.commit()

    def add_workspace(self, name: str, paths: List[str]):
        """Adds or updates a workspace in SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT INTO workspaces (name, paths_json) VALUES (?, ?)
                ON CONFLICT(name) DO UPDATE SET paths_json=excluded.paths_json
            """, (name, json.dumps(paths)))
            conn.commit()

    def remove_workspace(self, name: str):
        """Removes a workspace from SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("DELETE FROM workspaces WHERE name = ?", (name,))
            conn.commit()

    def set_api_key(self, provider: str, api_key: str):
        """Saves or updates an API key for a specific provider in SQLite"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("""
                INSERT OR REPLACE INTO api_keys (provider, api_key)
                VALUES (?, ?)
            """, (provider.lower().strip(), api_key.strip()))
            conn.commit()

    def get_api_key(self, provider: str = "openai") -> Optional[str]:
        """Retrieves stored API key for provider"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT api_key FROM api_keys WHERE provider = ?", (provider.lower().strip(),))
            row = cursor.fetchone()
            return row["api_key"] if row else None

    def get_all_api_keys(self) -> Dict[str, str]:
        """Retrieves all stored API keys"""
        with self._get_connection() as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT provider, api_key FROM api_keys")
            rows = cursor.fetchall()
            return {row["provider"]: row["api_key"] for row in rows}
