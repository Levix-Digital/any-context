import os
import chromadb
from typing import List, Dict, Any, Optional
from any_context.config.app_settings import AppSettings
from any_context.memory.models import MemoryEntry, MemoryLevel
from llama_index.core import Settings, Document
from llama_index.core.ingestion import IngestionPipeline
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding
from any_context.core.utils import get_api_key

class MemoryStore:
    """
    Isolated persistent storage manager for long-term vector memory (ChromaDB)
    """

    def __init__(self, settings: AppSettings = None):
        self.settings = settings or AppSettings.load()
        self.db_path = self.settings.session.db_path if self.settings else "./memory"
        self.collection_name = self.settings.session.collection_name if self.settings else "session_docs"
        
        # Configure embedding model
        from any_context.tools.search_tools import configure_embedding_model
        configure_embedding_model()


    def get_collection(self):
        os.makedirs(self.db_path, exist_ok=True)
        db = chromadb.PersistentClient(path=self.db_path)
        return db.get_or_create_collection(self.collection_name)

    def save_memory_entry(self, entry: MemoryEntry) -> str:
        """Saves a single memory entry (Level 1 or Level 3) to ChromaDB"""
        collection = self.get_collection()
        vector_store = ChromaVectorStore(chroma_collection=collection)

        pipeline = IngestionPipeline(
            transformations=[
                SentenceSplitter(chunk_size=1024, chunk_overlap=200),
                Settings.embed_model
            ],
            vector_store=vector_store
        )

        metadata = {
            "level": entry.level.value,
            "workspace": entry.workspace or "global",
            "thread_id": entry.thread_id or "",
            "timestamp": entry.timestamp,
            **entry.metadata
        }

        doc = Document(text=entry.content, metadata=metadata)
        pipeline.run(documents=[doc])
        return doc.doc_id

    def get_entries_by_level(self, level: MemoryLevel, workspace: str = None) -> List[Dict[str, Any]]:
        """Retrieves raw stored entries matching memory level and workspace"""
        collection = self.get_collection()
        where_clause = {"level": level.value}
        if workspace:
            where_clause = {
                "$and": [
                    {"level": level.value},
                    {"workspace": workspace}
                ]
            }

        results = collection.get(where=where_clause)
        items = []
        if results and "ids" in results and results["ids"]:
            for idx, doc_id in enumerate(results["ids"]):
                items.append({
                    "id": doc_id,
                    "content": results["documents"][idx] if "documents" in results and results["documents"] else "",
                    "metadata": results["metadatas"][idx] if "metadatas" in results and results["metadatas"] else {}
                })
        return items

    def delete_entries_by_ids(self, doc_ids: List[str]):
        """Deletes specified memory documents from ChromaDB"""
        if not doc_ids:
            return
        collection = self.get_collection()
        collection.delete(ids=doc_ids)

    def reset_memory(self, workspace: Optional[str] = None) -> int:
        """
        Deletes memory entries from ChromaDB for a specific workspace or all workspaces.
        Returns the count of deleted items.
        """
        collection = self.get_collection()
        where_clause = {"workspace": workspace} if (workspace and workspace != "all") else None

        if where_clause:
            results = collection.get(where=where_clause)
        else:
            results = collection.get()

        if results and "ids" in results and results["ids"]:
            doc_ids = results["ids"]
            collection.delete(ids=doc_ids)
            return len(doc_ids)
        return 0
