import os
import dotenv
import chromadb
from any_context.config.app_settings import AppSettings
from llama_index.core import Settings, VectorStoreIndex
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.core.vector_stores import ExactMatchFilter, MetadataFilters
from llama_index.embeddings.openai import OpenAIEmbedding
from langchain.tools import tool

dotenv.load_dotenv()
LOCAL_API_KEY = os.getenv("LOCAL_API_KEY")

settings = AppSettings.load()
local_embedding_model = settings.models.local_embedding_model if settings else "text-embedding-multilingual-e5-small"
local_openai_embedding_model = settings.models.local_openai_embedding_model if settings else "text-embedding-3-small"
local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
session_db_path = settings.session.db_path if settings else "./memory"
session_collection_name = settings.session.collection_name if settings else "session_docs"
folder_db_path = settings.context.db_path if settings else "./context_db"
folder_collection_name = settings.context.collection_name if settings else "context_docs"

Settings.embed_model = OpenAIEmbedding(
    model_name=local_embedding_model, 
    model=local_openai_embedding_model,
    api_base=local_base_url,
    api_key=LOCAL_API_KEY
)

@tool()
def search_db(prompt_text: str, search_session_memory: bool = False, top_k: int = 3, workspace: str = None):
    """
    Search for relevant information in the vector databases based on the provided prompt text.

    Args:
        prompt_text (str): The text to search for.
        search_session_memory (bool): Set to True to search the user's past conversations/sessions memory. Set to False to search the general documents/knowledge base.
        top_k (int): The number of results to return.
        workspace (str, optional): The specific workspace to filter searches by (only applies when search_session_memory is False).

    Returns:
        list: A list of relevant information.
    """

    if search_session_memory:
        db_path = session_db_path
        collection_name = session_collection_name
    else:
        db_path = folder_db_path
        collection_name = folder_collection_name

    db = chromadb.PersistentClient(path=db_path)
    chroma_collection = db.get_collection(collection_name)
    vector_store = ChromaVectorStore(chroma_collection = chroma_collection)

    index = VectorStoreIndex.from_vector_store(vector_store)

    safe_top_k = min(top_k, 2)

    filters = None
    if workspace and not search_session_memory:
        print(f"\n🔍 [Search] Filtering context by Workspace: '{workspace}'")
        filters = MetadataFilters(
            filters=[ExactMatchFilter(key="workspace", value=workspace)]
        )
    elif not search_session_memory:
        print(f"\n🔍 [Search] Searching globally across all workspaces...")

    retriever = index.as_retriever(similarity_top_k=safe_top_k, filters=filters)
    nodes = retriever.retrieve(prompt_text)

    results_list = []
    for i, node in enumerate(nodes):
        file_name = node.metadata.get('file_name', 'Unknown')
        results_list.append(f"Source: {file_name}\nContent:\n{node.text}")

    if not results_list:
        return "No documents found."
        
    return "\n\n".join(results_list)
