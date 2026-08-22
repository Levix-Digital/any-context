import os
import uuid
from typing import List, Dict, Any, Optional
from any_context.config.app_settings import AppSettings
from any_context.memory.models import MemoryEntry, MemoryLevel
from any_context.vector_engine.store import LanceDBStore

class MemoryStore:
    """
    Isolated persistent storage manager for long-term vector memory powered by LanceDB.
    """

    def __init__(self, settings: AppSettings = None):
        self.settings = settings or AppSettings.load()
        self.db_path = self.settings.session.db_path if self.settings else "./memory"
        self.table_name = "session_memory"
        
        # Configure embedding model
        from any_context.tools.search_tools import configure_embedding_model
        configure_embedding_model()
        
        self.lance_store = LanceDBStore.get_instance(db_path=os.path.join(self.db_path, "lancedb"))

    def save_memory_entry(self, entry: MemoryEntry) -> str:
        """Saves a single memory entry (Level 1 or Level 3) to LanceDB"""
        from llama_index.core.settings import Settings
        from any_context.tools.search_tools import configure_embedding_model

        if Settings.embed_model is None:
            configure_embedding_model()

        emb = Settings.embed_model.get_text_embedding(entry.content)
        entry_id = str(uuid.uuid4())

        record = {
            "id": entry_id,
            "vector": emb,
            "text": entry.content,
            "file_name": f"Session Memory ({entry.level.value})",
            "file_path": f"memory://{entry.workspace or 'global'}/{entry_id}",
            "workspace": entry.workspace or "global",
            "last_modified": entry.timestamp,
            "content_type": f"Memory {entry.level.value}",
            "document_summary": entry.metadata.get("summary", ""),
            "keywords": entry.metadata.get("keywords", ""),
            "content_hash": ""
        }

        self.lance_store.upsert_records([record], table_name=self.table_name, dim=len(emb))
        return entry_id

    def get_entries_by_level(self, level: MemoryLevel, workspace: str = None) -> List[Dict[str, Any]]:
        """Retrieves stored entries matching memory level and workspace from LanceDB"""
        if not self.lance_store._has_table(self.table_name):
            return []

        try:
            table = self.lance_store._db.open_table(self.table_name)
            where_clauses = [f"content_type = 'Memory {level.value}'"]
            if workspace:
                clean_ws = workspace.replace("'", "''")
                where_clauses.append(f"workspace = '{clean_ws}'")
            where_expr = " AND ".join(where_clauses)
            results = table.search().where(where_expr).limit(1000).to_list()
            items = []
            for r in results:
                items.append({
                    "id": r.get("id"),
                    "content": r.get("text", ""),
                    "metadata": {
                        "workspace": r.get("workspace"),
                        "level": level.value,
                        "timestamp": r.get("last_modified")
                    }
                })
            return items
        except Exception:
            return []

    def delete_entries_by_ids(self, doc_ids: List[str]):
        """Deletes specified memory documents from LanceDB"""
        if not doc_ids:
            return
        for d_id in doc_ids:
            self.lance_store.delete_by_id(d_id, table_name=self.table_name)

    def reset_memory(self, workspace: Optional[str] = None) -> int:
        """
        Deletes memory entries from LanceDB for a specific workspace or all workspaces.
        Returns the count of deleted items.
        """
        if not self.lance_store._has_table(self.table_name):
            return 0

        try:
            table = self.lance_store._db.open_table(self.table_name)
            if workspace and workspace != "all":
                clean_ws = workspace.replace("'", "''")
                matching = table.search().where(f"workspace = '{clean_ws}'").limit(10000).to_list()
                count = len(matching)
                self.lance_store.delete_by_workspace(workspace, table_name=self.table_name)
                return count
            else:
                count = table.count_rows()
                self.lance_store.delete_all_records(table_name=self.table_name)
                return count
        except Exception:
            return 0
