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

def safe_stdout_write(msg: str):
    import sys
    try:
        sys.stdout.write(msg)
        sys.stdout.flush()
    except (UnicodeEncodeError, Exception):
        try:
            clean_msg = msg.encode("ascii", errors="ignore").decode("ascii")
            sys.stdout.write(clean_msg)
            sys.stdout.flush()
        except Exception:
            pass

def configure_embedding_model():
    import logging
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("openai").setLevel(logging.WARNING)
    logging.getLogger("llama_index").setLevel(logging.WARNING)

    settings = AppSettings.load()
    emb_model = settings.models.embedding_model if settings else "text-embedding-3-small"
    local_base_url = settings.models.local_base_url if settings else "https://api.openai.com/v1"
    model_provider = settings.models.model_provider if settings else "openai"
    api_key = get_api_key(provider=model_provider)

    is_mock = (
        not api_key
        or api_key.startswith("mock_")
        or api_key.startswith("sk-test")
        or api_key in ["lm-studio", "placeholder", "sk-placeholder", "test", "fake"]
        or "test" in api_key.lower()
        or "fake" in api_key.lower()
        or "mock" in api_key.lower()
    )
    if is_mock:
        try:
            from llama_index.core.embeddings.mock_embed_model import MockEmbedding
            embed_m = MockEmbedding(embed_dim=1536)
            Settings.embed_model = embed_m
            return embed_m
        except Exception:
            pass

    try:
        embed_m = OpenAIEmbedding(
            model_name=emb_model,
            api_base=local_base_url,
            api_key=api_key,
            embed_batch_size=32
        )
        Settings.embed_model = embed_m
        return embed_m
    except Exception:
        embed_m = OpenAIEmbedding(
            model_name="text-embedding-3-small",
            api_base=local_base_url,
            api_key=api_key,
            embed_batch_size=32
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

        import sys
        if workspace and not search_session_memory:
            safe_stdout_write(f"\r\033[K🔍 [Search] Searching strictly within Workspace: '{workspace}' (top {search_k} chunks)...")
            try:
                filters = MetadataFilters(
                    filters=[ExactMatchFilter(key="workspace", value=workspace)]
                )
                retriever = index.as_retriever(similarity_top_k=search_k, filters=filters)
                nodes = retriever.retrieve(prompt_text)
            except Exception:
                # Resilient Fallback: broad query with in-memory Python workspace isolation filter
                try:
                    retriever = index.as_retriever(similarity_top_k=search_k * 4, filters=None)
                    all_nodes = retriever.retrieve(prompt_text)
                    nodes = [n for n in all_nodes if n.metadata.get("workspace") == workspace][:search_k]
                except Exception:
                    nodes = []
        elif not search_session_memory:
            safe_stdout_write(f"\r\033[K🔍 [Search] Searching across workspaces (top {search_k} chunks)...")
            try:
                retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
                nodes = retriever.retrieve(prompt_text)
            except Exception:
                nodes = []
        else:
            try:
                retriever = index.as_retriever(similarity_top_k=search_k, filters=None)
                nodes = retriever.retrieve(prompt_text)
            except Exception:
                nodes = []

        results_list = []
        for i, node in enumerate(nodes):
            file_name = node.metadata.get('file_name', 'Unknown')
            file_path = node.metadata.get('file_path', file_name)
            ws_tag = node.metadata.get('workspace', 'Global')
            last_mod = node.metadata.get('last_modified_date') or node.metadata.get('last_modified') or node.metadata.get('creation_date')
            content_type = node.metadata.get('content_type') or ("Web Documentation" if str(file_path).startswith("http") else "Local Document")
            
            header_parts = [f"Source: {file_name}", f"Workspace: {ws_tag}"]
            if last_mod:
                header_parts.append(f"Last Modified: {last_mod}")
            if content_type:
                header_parts.append(f"Type: {content_type}")
                
            header_str = " | ".join(header_parts)
            results_list.append(f"--- [Document Chunk {i+1} | {header_str}] ---\nPath: {file_path}\nContent:\n{node.text}")

        if not results_list:
            if search_session_memory:
                return f"No session memory records found for query '{prompt_text}' in workspace '{workspace}'." if workspace else "No session memory records found."
            return f"No relevant documents found for query '{prompt_text}' in workspace '{workspace}'. (Search executed across {chroma_collection.count()} indexed chunks)." if workspace else f"No relevant documents found for query '{prompt_text}'."
            
        return "\n\n".join(results_list)

    except Exception as e:
        safe_stdout_write(f"\r\033[K⚠️ [Search] Vector search failed: {e}\n")
        if search_session_memory:
            return "No long-term session memory entries found yet."
        return f"DATABASE_SEARCH_FAILED: {str(e)}. The requested information could not be retrieved from the workspace vector database."


@tool()
def add_web_source(url: str, workspace: str = None, polling_interval_hours: int = 24, max_pages: int = 50) -> str:
    """
    Crawls and indexes a website or documentation portal into the vector database for a workspace.
    Automatically discovers and indexes sub-pages within the same section.

    Args:
        url (str): The website or documentation URL to scrape and index (e.g. 'https://docs.python.org/3/').
        workspace (str, optional): The target workspace name. If omitted, uses active workspace.
        polling_interval_hours (int): Polling frequency in hours (default: 24).
        max_pages (int): Maximum number of sub-pages to crawl (default: 50).

    Returns:
        str: Success confirmation or error message.
    """
    from any_context.ingestion.web_crawler import discover_site_urls, crawl_and_index_urls
    target_ws = workspace or "Default"
    
    disc = discover_site_urls(url)
    target_urls = disc.get("section_urls") or [url]
    if len(target_urls) > max_pages:
        target_urls = target_urls[:max_pages]

    res = crawl_and_index_urls(workspace_name=target_ws, urls=target_urls, max_workers=8)
    if res.get("status") == "success":
        return f"✅ Successfully crawled and indexed {res.get('indexed_count', len(target_urls))} web pages ({res.get('total_chars', 0):,} characters) for '{disc.get('title')}' ({url}) into workspace '{target_ws}'."
    else:
        return f"ℹ️ Ingested {res.get('indexed_count', 0)} web pages into workspace '{target_ws}'."


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
