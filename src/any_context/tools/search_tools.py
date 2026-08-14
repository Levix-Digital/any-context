import os
import chromadb
from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key
from llama_index.core import Settings, VectorStoreIndex
from llama_index.vector_stores.chroma import ChromaVectorStore
from llama_index.core.vector_stores import ExactMatchFilter, MetadataFilters
from llama_index.embeddings.openai import OpenAIEmbedding
from langchain.tools import tool

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

def configure_embedding_model():
    settings = AppSettings.load()
    local_openai_embedding_model = settings.models.local_openai_embedding_model if settings else "text-embedding-3-small"
    local_base_url = settings.models.local_base_url if settings else "https://api.openai.com/v1"
    model_provider = settings.models.model_provider if settings else "openai"
    api_key = get_api_key(provider=model_provider)

    try:
        embed_m = OpenAIEmbedding(
            model_name=local_openai_embedding_model,
            api_base=local_base_url,
            api_key=api_key
        )
        Settings.embed_model = embed_m
        return embed_m
    except Exception:
        embed_m = OpenAIEmbedding(
            model_name="text-embedding-3-small",
            api_base=local_base_url,
            api_key=api_key
        )
        Settings.embed_model = embed_m
        return embed_m



# Initial setup
configure_embedding_model()

@tool()
def search_db(prompt_text: str, search_session_memory: bool = False, top_k: int = 8, workspace: str = None):
    """
    Search for relevant information in the vector database based on the provided prompt text.
    Enforces strict workspace isolation to ensure total privacy between projects.

    Args:
        prompt_text (str): The text to search for.
        search_session_memory (bool): Set to True to search the user's past conversations/sessions memory. Set to False to search general workspace documents.
        top_k (int): The number of relevant document chunks to return (default: 8).
        workspace (str, optional): The specific workspace to filter searches by (enforces strict workspace privacy).

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
            safe_print(f"\n🔍 [Search] Searching strictly within Workspace: '{workspace}' (retrieving top {search_k} chunks)...")
            filters = MetadataFilters(
                filters=[ExactMatchFilter(key="workspace", value=workspace)]
            )
            retriever = index.as_retriever(similarity_top_k=search_k, filters=filters)
            nodes = retriever.retrieve(prompt_text)
        elif not search_session_memory:
            safe_print(f"\n🔍 [Search] Searching globally across all workspaces (retrieving top {search_k} chunks)...")
            retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
            nodes = retriever.retrieve(prompt_text)
        else:
            retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
            nodes = retriever.retrieve(prompt_text)

        results_list = []
        for i, node in enumerate(nodes):
            file_name = node.metadata.get('file_name', 'Unknown')
            file_path = node.metadata.get('file_path', file_name)
            ws_tag = node.metadata.get('workspace', 'Global')
            results_list.append(f"--- [Document Chunk {i+1} | Source: {file_name} | Workspace: {ws_tag}] ---\nPath: {file_path}\nContent:\n{node.text}")

        if not results_list:
            return f"No documents found for search in workspace '{workspace}'." if workspace else "No documents found."
            
        return "\n\n".join(results_list)

    except Exception as e:
        if search_session_memory:
            return "No long-term session memory entries found yet."
        return f"Error during database search: {str(e)}"


@tool()
def add_web_source(url: str, workspace: str = None, polling_interval_hours: int = 24) -> str:
    """
    Scrapes and indexes a website URL into the vector database for a workspace, setting up recurring polling.
    Allows the AI agent and user to query live web documentation, websites, and articles.

    Args:
        url (str): The web page URL to scrape and index (e.g. 'https://docs.python.org/3/').
        workspace (str, optional): The target workspace name. If omitted, uses active workspace.
        polling_interval_hours (int): Polling frequency in hours (default: 24).

    Returns:
        str: Success confirmation or error message.
    """
    from any_context.ingestion.web_scheduler import index_web_url_to_chromadb
    target_ws = workspace or "Default"
    safe_print(f"\n🌐 [Web Ingestion] Scraping and indexing '{url}' into workspace '{target_ws}'...")
    res = index_web_url_to_chromadb(workspace_name=target_ws, url=url)
    if res.get("status") == "success":
        return f"✅ Successfully scraped and indexed web source '{res.get('title')}' ({url}) into workspace '{target_ws}'. Total {res.get('char_count')} characters indexed."
    elif res.get("status") == "unchanged":
        return f"ℹ️ Web source '{url}' is already up-to-date in workspace '{target_ws}'."
    else:
        return f"❌ Failed to index web source: {res.get('message', 'Unknown error')}"


@tool()
def list_web_sources(workspace: str = None) -> str:
    """
    Lists all web URLs and documentation sites configured for scraping and polling in a workspace.

    Args:
        workspace (str, optional): Target workspace name.

    Returns:
        str: Markdown list of configured web sources.
    """
    from any_context.ingestion.web_scheduler import WebSchedulerStore
    store = WebSchedulerStore()
    target_ws = workspace or "Default"
    urls = store.get_workspace_web_urls(target_ws)
    if not urls:
        return f"No web sources configured yet for workspace '{target_ws}'."
    
    lines = [f"### 🌐 Web Sources for Workspace '{target_ws}':"]
    for u in urls:
        lines.append(f"- **{u.get('title') or u['url']}** (`{u['url']}`) - Interval: {u.get('polling_interval_hours', 24)}h | Last Scraped: {u.get('last_scraped_at') or 'Pending'}")
    return "\n".join(lines)


@tool()
def remove_web_source(url_or_id: str, workspace: str = None) -> str:
    """
    Removes a web URL from a workspace's scraping schedule and purges its indexed vectors from ChromaDB.

    Args:
        url_or_id (str): The URL or ID of the web source to remove.
        workspace (str, optional): Target workspace name.

    Returns:
        str: Confirmation message.
    """
    from any_context.ingestion.web_scheduler import WebSchedulerStore, remove_web_url_from_chromadb
    store = WebSchedulerStore()
    target_ws = workspace or "Default"
    
    # Check if id or url
    urls = store.get_workspace_web_urls(target_ws)
    matched = next((u for u in urls if u["id"] == url_or_id or u["url"] == url_or_id), None)
    if not matched:
        return f"Web source '{url_or_id}' not found in workspace '{target_ws}'."
    
    store.delete_web_url(matched["id"], workspace_name=target_ws)
    remove_web_url_from_chromadb(workspace_name=target_ws, url=matched["url"])
    return f"🗑️ Successfully removed web source '{matched['url']}' and purged its indexed vectors from workspace '{target_ws}'."
