import os
import dotenv
import chromadb

from config.app_settings import AppSettings
from llama_index.core import Settings, Document
from llama_index.core.ingestion import IngestionPipeline
from llama_index.core.node_parser import SentenceSplitter
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.embeddings.openai import OpenAIEmbedding
from langchain.tools import tool

dotenv.load_dotenv()
LOCAL_API_KEY = os.getenv("LOCAL_API_KEY")

settings = AppSettings.load()

local_embedding_model = settings.models.local_embedding_model
local_openai_embedding_model = settings.models.local_openai_embedding_model
local_base_url = settings.models.local_base_url
db_path = settings.session.db_path
collection_name = settings.session.collection_name

# --------------------------------------------------------------------------
# 🎯 DEFININING THE EMBEDDING MODEL
# --------------------------------------------------------------------------
# OPTION A: Local Model via OpenAI Compatible API (e.g., LM Studio)
Settings.embed_model = OpenAIEmbedding(
    model_name=local_embedding_model,
    model=local_openai_embedding_model, # OBRIGATÓRIO SER UM NOME DA OPENAI PARA O LLAMAINDEX NÃO TRAVAR. O LM STUDIO IGNORA.
    api_base=local_base_url,
    api_key=LOCAL_API_KEY
)

# OPTION B: If you prefer OpenAI (Requires OPENAI_API_KEY in .env):
# Settings.embed_model = OpenAIEmbedding(model="text-embedding-3-small")
# --------------------------------------------------------------------------

# --------------------------------------------------------------------------
# 🎯 VECTOR DATABASE PIPELINE
# --------------------------------------------------------------------------

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
    # LlamaIndex expects Document objects, not raw strings!
    doc = Document(text=session_summary)
    nodes = pipeline.run(documents=[doc])

    print("🎉 Success! Session saved successfully into long-term memory!")
    return "Session summary saved successfully!"

if __name__ == "__main__":
    index_session()