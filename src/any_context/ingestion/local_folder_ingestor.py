import os
import sys
import chromadb
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key
from llama_index.core import Settings, SimpleDirectoryReader
from llama_index.core.ingestion import IngestionPipeline, DocstoreStrategy
from llama_index.core.storage.docstore import SimpleDocumentStore
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding

from langchain.tools import tool

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

LOCAL_API_KEY = get_api_key()

settings = AppSettings.load()

db_save_path = settings.context.db_path if settings else "./context_db"
collection_name = settings.context.collection_name if settings else "context_docs"
local_embedding_model = settings.models.local_embedding_model if settings else "text-embedding-multilingual-e5-small"
local_openai_embedding_model = settings.models.local_openai_embedding_model if settings else "text-embedding-3-small"
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"

Settings.embed_model = OpenAIEmbedding(
    model_name=local_embedding_model,
    model=local_openai_embedding_model,
    api_base=local_base_url,
    api_key=LOCAL_API_KEY
)

def clear_context_vector_db():
    """
    Purges ChromaDB vector collection and docstore file to prevent dimension mismatch errors
    when embedding models are changed.
    """
    try:
        current_settings = AppSettings.load() or settings
        db_path = current_settings.context.db_path if current_settings else "./context_db"
        coll_name = current_settings.context.collection_name if current_settings else "context_docs"

        db = chromadb.PersistentClient(path=db_path)
        try:
            db.delete_collection(coll_name)
        except Exception:
            pass

        docstore_path = os.path.join(db_path, "docstore.json")
        if os.path.exists(docstore_path):
            os.remove(docstore_path)
        safe_print("🧹 Context vector collection and docstore successfully cleared for re-indexing!")
    except Exception as e:
        safe_print(f"⚠️ Warning during vector db clear: {e}")

@tool()
def index_folder(workspace_name: str = None):
    """
    Index documents in the vector database incrementally across all configured workspaces,
    or a specific workspace if provided.
    """
    current_settings = AppSettings.load() or settings
    if not current_settings or not current_settings.workspaces:
        safe_print("❌ Error: No workspaces configured in settings.")
        return

    safe_print("⚡ 1. Connecting to ChromaDB...")
    db = chromadb.PersistentClient(path=db_save_path)
    collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=collection)

    docstore_path = os.path.join(db_save_path, "docstore.json")
    if os.path.exists(docstore_path):
        docstore = SimpleDocumentStore.from_persist_path(docstore_path)
    else:
        docstore = SimpleDocumentStore()

    pipeline = IngestionPipeline(
        transformations = [
            SentenceSplitter(chunk_size=500, chunk_overlap=100),
            Settings.embed_model
        ],
        vector_store = vector_store,
        docstore = docstore,
        docstore_strategy = DocstoreStrategy.UPSERTS
    )

    all_documents = []
    
    workspaces_to_process = []
    for ws in current_settings.workspaces:
        if workspace_name and ws.name != workspace_name:
            continue
        workspaces_to_process.append(ws)
        safe_print(f"\n📂 Workspace: {ws.name}")
        for folder_path in ws.paths:
            if not os.path.exists(folder_path):
                safe_print(f"⚠️ Warning: Directory '{folder_path}' does not exist. (Its documents will be purged from DB)")
                continue
                
            safe_print(f"  🔍 Scanning: {folder_path}")
            try:
                reader = SimpleDirectoryReader(
                    input_dir=folder_path, 
                    recursive=True,
                    required_exts=[".pdf", ".docx", ".txt", ".md", ".csv", ".json", ".png", ".jpg", ".jpeg"]
                )
                docs = reader.load_data()
                
                for d in docs:
                    d.metadata["workspace"] = ws.name
                    if "file_path" in d.metadata:
                        d.id_ = d.metadata["file_path"]
                    
                all_documents.extend(docs)
                safe_print(f"  ✅ Found {len(docs)} files.")
            except ValueError:
                safe_print(f"  ⚠️ Warning: No valid files found in {folder_path}. Skipping...")
                continue
                
    if not all_documents:
        safe_print("❌ No valid documents found across any workspace.")
        
    safe_print("\n⚡ 2. Executing incremental check on vector database...")
    safe_print("⏳ Processing files and generating embeddings...")
    nodes = pipeline.run(documents = all_documents, show_progress=True)

    current_doc_ids = {doc.doc_id for doc in all_documents}
    processed_workspace_names = {ws.name for ws in workspaces_to_process}
    
    deleted_count = 0
    for node_id, node in list(docstore.docs.items()):
        node_workspace = node.metadata.get("workspace") if node.metadata else None
        
        if not node_workspace or node_workspace in processed_workspace_names:
            if getattr(node, "ref_doc_id", None) not in current_doc_ids and node.id_ not in current_doc_ids:
                docstore.delete_document(node_id)
                vector_store.delete(node_id)
                deleted_count += 1

    if deleted_count > 0:
        safe_print(f"🗑️ Purged {deleted_count} old file chunks from the database.")

    docstore.persist(persist_path=docstore_path)

    safe_print("🎉 Success! Incremental vectorial database updated!")

if __name__ == "__main__":
    index_folder()
