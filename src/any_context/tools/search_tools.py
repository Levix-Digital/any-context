import os
import chromadb
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key
from llama_index.core import Settings, VectorStoreIndex
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.core.vector_stores import ExactMatchFilter, MetadataFilters
from llama_index.embeddings.openai import OpenAIEmbedding
from langchain.tools import tool

def configure_embedding_model():
    settings = AppSettings.load()
    local_embedding_model = settings.models.local_embedding_model if settings else "text-embedding-multilingual-e5-small"
    local_openai_embedding_model = settings.models.local_openai_embedding_model if settings else "text-embedding-3-small"
    local_base_url = settings.models.local_base_url if settings else "http://localhost:1234/v1"
    model_provider = settings.models.model_provider if settings else "openai"
    api_key = get_api_key(provider=model_provider)

    Settings.embed_model = OpenAIEmbedding(
        model_name=local_embedding_model, 
        model=local_openai_embedding_model,
        api_base=local_base_url,
        api_key=api_key
    )

# Initial setup
configure_embedding_model()

@tool()
def search_db(prompt_text: str, search_session_memory: bool = False, top_k: int = 8, workspace: str = None):
    """
    Search for relevant information in the vector databases based on the provided prompt text.

    Args:
        prompt_text (str): The text to search for.
        search_session_memory (bool): Set to True to search the user's past conversations/sessions memory. Set to False to search general workspace documents.
        top_k (int): The number of relevant document chunks to return (default: 8).
        workspace (str, optional): The specific workspace to filter searches by.

    Returns:
        str: Relevant document content snippets or memory entries.
    """
    configure_embedding_model()
    settings = AppSettings.load()
    session_db_path = settings.session.db_path if settings else "./memory"
    session_collection_name = settings.session.collection_name if settings else "session_docs"
    folder_db_path = settings.context.db_path if settings else "./context_db"
    folder_collection_name = settings.context.collection_name if settings else "context_docs"

    if search_session_memory:
        db_path = session_db_path
        collection_name = session_collection_name
    else:
        db_path = folder_db_path
        collection_name = folder_collection_name

    try:
        os.makedirs(db_path, exist_ok=True)
        db = chromadb.PersistentClient(path=db_path)
        chroma_collection = db.get_or_create_collection(collection_name)

        if chroma_collection.count() == 0:
            if search_session_memory:
                return "No long-term session memory summaries found in database yet."
            return "No documents found in vector database."

        vector_store = ChromaVectorStore(chroma_collection=chroma_collection)
        index = VectorStoreIndex.from_vector_store(vector_store)

        search_k = max(top_k, 8)

        nodes = []
        filters = None

        if workspace and not search_session_memory:
            print(f"\n🔍 [Search] Filtering context by Workspace: '{workspace}' (retrieving top {search_k} chunks)...")
            filters = MetadataFilters(
                filters=[ExactMatchFilter(key="workspace", value=workspace)]
            )
            retriever = index.as_retriever(similarity_top_k=search_k, filters=filters)
            nodes = retriever.retrieve(prompt_text)

        # Fallback to global search if workspace filter returned 0 nodes or no workspace was provided
        if not nodes and not search_session_memory:
            if workspace:
                print(f"\n🔍 [Search] Workspace filter '{workspace}' returned 0 chunks. Performing global fallback search...")
            else:
                print(f"\n🔍 [Search] Searching globally across all workspaces (retrieving top {search_k} chunks)...")
            retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
            nodes = retriever.retrieve(prompt_text)
        elif search_session_memory:
            retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
            nodes = retriever.retrieve(prompt_text)

        results_list = []
        for i, node in enumerate(nodes):
            file_name = node.metadata.get('file_name', 'Unknown')
            file_path = node.metadata.get('file_path', file_name)
            ws_tag = node.metadata.get('workspace', 'Global')
            results_list.append(f"--- [Document Chunk {i+1} | Source: {file_name} | Workspace: {ws_tag}] ---\nPath: {file_path}\nContent:\n{node.text}")

        if not results_list:
            return "No documents found."
            
        return "\n\n".join(results_list)

    except Exception as e:
        if search_session_memory:
            return "No long-term session memory entries found yet."
        return f"Error during database search: {str(e)}"
