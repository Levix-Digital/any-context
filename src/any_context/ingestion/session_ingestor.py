"""
Session Ingestor - Long-term memory vector indexing with ChromaDB.
"""

import os
from typing import Optional
from any_context.observability import obs


def index_session(session_summary: str):
    """
    Use this tool to save a summary of the current session into the long-term memory vector database.
    Call this tool whenever the user asks you to save the context, or at the end of an important conversation.
    """
    with obs.span("memory:index_session", length=len(session_summary)):
        from any_context.config.app_settings import AppSettings
        from any_context.tools.search_tools import configure_embedding_model
        import chromadb
        from llama_index.core import Settings, Document
        from llama_index.core.ingestion import IngestionPipeline
        from llama_index.core.node_parser import SentenceSplitter
        from llama_index.vector_stores.chroma import ChromaVectorStore

        settings = AppSettings.load()
        db_path = settings.session.db_path if settings and settings.session else "./memory"
        collection_name = settings.session.collection_name if settings and settings.session else "session_docs"

        configure_embedding_model()

        print("⚡ Connecting to Vector Memory Database...")
        os.makedirs(db_path, exist_ok=True)
        db = chromadb.PersistentClient(path=db_path)
        collection = db.get_or_create_collection(collection_name)
        vector_store = ChromaVectorStore(chroma_collection=collection)

        pipeline = IngestionPipeline(
            transformations=[
                SentenceSplitter(chunk_size=1024, chunk_overlap=200),
                Settings.embed_model
            ],
            vector_store=vector_store
        )

        print("⚡ Executing vector database session pipeline...")
        doc = Document(text=session_summary)
        nodes = pipeline.run(documents=[doc])

        print("🎉 Success! Session saved successfully into long-term memory!")
        return "Session summary saved successfully!"


if __name__ == "__main__":
    index_session("Test session")

