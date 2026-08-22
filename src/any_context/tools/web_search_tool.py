"""
AnyContext Live Web Search Tool.
Provides real-time internet search with domain prioritization for workspace web portals.
"""
import os
import re
import warnings
from typing import Optional, List, Dict, Any
from urllib.parse import urlparse
from langchain.tools import tool

# Suppress RuntimeWarning from duckduckgo_search renaming/deprecation
warnings.filterwarnings("ignore", category=RuntimeWarning, module="duckduckgo_search.*")
warnings.filterwarnings("ignore", category=RuntimeWarning, module="ddgs.*")
warnings.filterwarnings("ignore", message=".*renamed to.*")
warnings.filterwarnings("ignore", message=".*duckduckgo_search.*")


def _extract_domain(url: str) -> Optional[str]:
    """Extracts base domain from a given URL (e.g., https://www.canada.ca/path -> canada.ca)."""
    try:
        parsed = urlparse(url)
        netloc = parsed.netloc or parsed.path
        netloc = netloc.split(":")[0]  # remove port if any
        if netloc.startswith("www."):
            netloc = netloc[4:]
        return netloc if netloc else None
    except Exception:
        return None


def execute_web_search(
    query: str,
    domains: Optional[List[str]] = None,
    max_results: int = 5
) -> List[Dict[str, str]]:
    """
    Executes web search across DuckDuckGo, Tavily, or Serper.
    If domains are provided, prioritizes domain-targeted queries.
    """
    clean_query = query.strip()
    if not clean_query:
        return []

    results = []

    from any_context.core.utils import get_api_key

    # 1. Check Tavily API (from SQLite DB or Environment)
    tavily_key = get_api_key("tavily") or os.getenv("TAVILY_API_KEY")
    if tavily_key and tavily_key.strip():
        try:
            import httpx
            payload = {
                "api_key": tavily_key.strip(),
                "query": clean_query,
                "max_results": max_results,
                "include_domains": domains if domains else []
            }
            headers = {"Accept-Encoding": "identity", "Content-Type": "application/json"}
            resp = httpx.post("https://api.tavily.com/search", headers=headers, json=payload, timeout=10.0)
            if resp.status_code == 200:
                data = resp.json()
                for item in data.get("results", []):
                    results.append({
                        "title": item.get("title", "Web Result"),
                        "url": item.get("url", ""),
                        "snippet": item.get("content", item.get("snippet", ""))
                    })
                if results:
                    return results[:max_results]
        except Exception:
            pass

    # 2. Check Serper API (from SQLite DB or Environment)
    serper_key = get_api_key("serper") or os.getenv("SERPER_API_KEY")
    if serper_key and serper_key.strip():
        try:
            import httpx
            headers = {"X-API-KEY": serper_key.strip(), "Content-Type": "application/json", "Accept-Encoding": "identity"}
            q = clean_query
            if domains and len(domains) == 1:
                q = f"site:{domains[0]} {clean_query}"
            resp = httpx.post("https://google.serper.dev/search", headers=headers, json={"q": q, "num": max_results}, timeout=10.0)
            if resp.status_code == 200:
                data = resp.json()
                for item in data.get("organic", []):
                    results.append({
                        "title": item.get("title", "Web Result"),
                        "url": item.get("link", ""),
                        "snippet": item.get("snippet", "")
                    })
                if results:
                    return results[:max_results]
        except Exception:
            pass

    # 3. DuckDuckGo Search (Default / Free)
    try:
        try:
            from ddgs import DDGS
        except ImportError:
            from duckduckgo_search import DDGS

        with DDGS() as ddgs:
            # First attempt: site-filtered query if domains are registered in workspace
            if domains:
                for dom in domains[:2]:
                    site_query = f"site:{dom} {clean_query}"
                    try:
                        ddg_res = list(ddgs.text(site_query, max_results=max_results))
                        for r in ddg_res:
                            results.append({
                                "title": r.get("title", "Web Result"),
                                "url": r.get("href", r.get("link", "")),
                                "snippet": r.get("body", r.get("snippet", ""))
                            })
                    except Exception:
                        pass

            # Second attempt: General open web search if needed
            if len(results) < max_results:
                needed = max_results - len(results)
                try:
                    ddg_gen = list(ddgs.text(clean_query, max_results=needed))
                    for r in ddg_gen:
                        url = r.get("href", r.get("link", ""))
                        if not any(existing["url"] == url for existing in results):
                            results.append({
                                "title": r.get("title", "Web Result"),
                                "url": url,
                                "snippet": r.get("body", r.get("snippet", ""))
                            })
                except Exception:
                    pass
    except Exception:
        # Fallback / Mock for offline and testing environments
        pass

    return results[:max_results]


def get_active_web_search_engine() -> str:
    """Returns the name of the currently active search engine (Tavily, Serper, or DuckDuckGo)."""
    from any_context.core.utils import get_api_key
    if get_api_key("tavily") or os.getenv("TAVILY_API_KEY"):
        return "Tavily Search API"
    elif get_api_key("serper") or os.getenv("SERPER_API_KEY"):
        return "Serper Google Search API"
    return "DuckDuckGo (Free Engine)"


@tool()
def live_web_search(query: str, workspace: Optional[str] = None, max_results: int = 5) -> str:
    """
    Performs real-time public web search on the internet when web search is enabled for the workspace.
    Prioritizes domain portals registered in the active workspace before searching the open web.

    CRITICAL CONVERSATIONAL SEARCH RULE:
    When the user confirms or grants permission to search the web (e.g. answering 'sim', 'yes', 'pode', 'ok', etc.),
    DO NOT pass the literal confirmation string 'sim' as the query!
    You MUST reconstruct and pass the full search topic/question previously discussed (e.g., query='previsão do tempo para Calgary amanhã').

    Args:
        query (str): The search query to look up online (e.g. 'previsão do tempo para Calgary amanhã').
        workspace (str, optional): Target workspace name to prioritize its registered web portals.
        max_results (int): Maximum number of search results to return (default: 5).

    Returns:
        str: Formatted Markdown string containing search results with URLs, titles, and snippets.
    """
    clean_query = query.strip()
    if not clean_query:
        return "⚠️ Please provide a non-empty search query."

    target_ws = (workspace or "Default").strip()

    # Discover registered domains in the active workspace
    domains = []
    try:
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        store = WebSchedulerStore()
        urls = store.get_workspace_web_urls(target_ws)
        for u in urls:
            dom = _extract_domain(u.get("url", ""))
            if dom and dom not in domains:
                domains.append(dom)
    except Exception:
        pass

    results = execute_web_search(clean_query, domains=domains if domains else None, max_results=max_results)
    engine_name = get_active_web_search_engine()

    if not results:
        return f"🔍 Nenhuma informação adicional encontrada na internet via {engine_name} para a busca: '{clean_query}'."

    lines = [f"### 🌐 Resultados da Busca Web (via {engine_name}) para '{clean_query}':"]
    if domains:
        lines.append(f"*(Priorizando portais do workspace: {', '.join(domains)})*\n")

    for i, r in enumerate(results, 1):
        title = r.get("title", "Resultado Web")
        url = r.get("url", "")
        snippet = r.get("snippet", "").strip()
        lines.append(f"{i}. **[{title}]({url})**\n   - **Fonte:** `{url}`\n   - **Trecho:** {snippet}\n")

    return "\n".join(lines)
