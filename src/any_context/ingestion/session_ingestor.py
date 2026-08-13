import os
import dotenv
import chromadb

from any_context.config.app_settings import AppSettings
from llama_index.core import Settings, Document
from llama_index.core.ingestion import IngestionPipeline
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding
from langchain.tools import tool

dotenv.load_dotenv()
LOCAL_API_KEY = os.getenv("LOCAL_API_KEY")

settings = AppSettings.load()

local_embedding_model = settings.models.local_embedding_model if settings else "text-embedding-multilingual-e5-small"
local_openai_embedding_model = settings.models.local_openai_embedding_model if settings else "text-embedding-3-small"
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
db_path = settings.session.db_path if settings else "./memory"
collection_name = settings.session.collection_name if settings else "session_docs"

Settings.embed_model = OpenAIEmbedding(
    model_name=local_embedding_model,
    model=local_openai_embedding_model,
    api_base=local_base_url,
    api_key=LOCAL_API_KEY
)

@tool()
def index_session(session_summary: str):
    """
    Use this tool to save a summary of the current session into the long-term memory vector database.
    Call this tool whenever the user asks you to save the context, or at the end of an important conversation.
    """
    print("⚡ Connecting to ChromaDB for Long-Term Memory...")
    os.makedirs(db_path, exist_ok=True)
    db = chromadb.PersistentClient(path=db_path)
    collection = db.get_or_create_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection=collection)

    pipeline = IngestionPipeline(
        transformations = [
            SentenceSplitter(chunk_size=512, chunk_overlap=50),
            Settings.embed_model
        ],
        vector_store = vector_store
    )

    print("⚡ Executing vector database session pipeline...")
    doc = Document(text=session_summary)
    nodes = pipeline.run(documents=[doc])

    print("🎉 Success! Session saved successfully into long-term memory!")
    return "Session summary saved successfully!"

if __name__ == "__main__":
    index_session()
