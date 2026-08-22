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
    preferred_engine: Optional[str] = None,
    max_results: int = 5,
    allow_fallback: bool = True
) -> List[Dict[str, str]]:
    """
    Executes web search across Tavily, Serper, or DuckDuckGo.
    Honors preferred_engine ('auto', 'tavily', 'serper', 'duckduckgo').
    If domains are provided, prioritizes domain-targeted queries.
    If domain-targeted search yields no results and allow_fallback is True,
    automatically cascades to the open global web.
    """
    clean_query = query.strip()
    if not clean_query:
        return []

    results = []

    from any_context.core.utils import get_api_key
    from any_context.config.db_store import ConfigDBStore

    pref = str(preferred_engine or "").strip().lower()
    if not pref or pref == "auto":
        try:
            store = ConfigDBStore()
            pref = store.get_default_search_engine()
        except Exception:
            pref = "auto"

    tavily_key = get_api_key("tavily") or os.getenv("TAVILY_API_KEY")
    serper_key = get_api_key("serper") or os.getenv("SERPER_API_KEY")

    def _try_tavily() -> List[Dict[str, str]]:
        if not (tavily_key and tavily_key.strip()):
            return []
        try:
            import httpx
            payload = {
                "api_key": tavily_key.strip(),
                "query": clean_query,
                "max_results": max_results,
            }
            if domains and len(domains) > 0:
                payload["include_domains"] = domains

            headers = {"Accept-Encoding": "identity", "Content-Type": "application/json"}
            resp = httpx.post("https://api.tavily.com/search", headers=headers, json=payload, timeout=10.0)
            if resp.status_code == 200:
                data = resp.json()
                t_res = []
                for item in data.get("results", []):
                    t_res.append({
                        "title": item.get("title", "Web Result"),
                        "url": item.get("url", ""),
                        "snippet": item.get("content", item.get("snippet", ""))
                    })
                return t_res
        except Exception:
            pass
        return []

    def _try_serper() -> List[Dict[str, str]]:
        if not (serper_key and serper_key.strip()):
            return []
        try:
            import httpx
            headers = {"X-API-KEY": serper_key.strip(), "Content-Type": "application/json", "Accept-Encoding": "identity"}
            q = clean_query
            if domains and len(domains) == 1:
                q = f"site:{domains[0]} {clean_query}"
            resp = httpx.post("https://google.serper.dev/search", headers=headers, json={"q": q, "num": max_results}, timeout=10.0)
            if resp.status_code == 200:
                data = resp.json()
                s_res = []
                for item in data.get("organic", []):
                    s_res.append({
                        "title": item.get("title", "Web Result"),
                        "url": item.get("link", ""),
                        "snippet": item.get("snippet", "")
                    })
                return s_res
        except Exception:
            pass
        return []

    def _try_duckduckgo() -> List[Dict[str, str]]:
        d_res = []
        try:
            try:
                from ddgs import DDGS
            except ImportError:
                from duckduckgo_search import DDGS

            with DDGS() as ddgs:
                if domains:
                    for dom in domains[:2]:
                        site_query = f"site:{dom} {clean_query}"
                        try:
                            ddg_res = list(ddgs.text(site_query, max_results=max_results))
                            for r in ddg_res:
                                d_res.append({
                                    "title": r.get("title", "Web Result"),
                                    "url": r.get("href", r.get("link", "")),
                                    "snippet": r.get("body", r.get("snippet", ""))
                                })
                        except Exception:
                            pass

                if len(d_res) < max_results:
                    needed = max_results - len(d_res)
                    try:
                        ddg_gen = list(ddgs.text(clean_query, max_results=needed))
                        for r in ddg_gen:
                            url = r.get("href", r.get("link", ""))
                            if not any(existing["url"] == url for existing in d_res):
                                d_res.append({
                                    "title": r.get("title", "Web Result"),
                                    "url": url,
                                    "snippet": r.get("body", r.get("snippet", ""))
                                })
                    except Exception:
                        pass
        except Exception:
            pass
        return d_res

    # Dispatch according to user's preferred search engine
    if pref == "tavily":
        results = _try_tavily()
        if not results and allow_fallback:
            results = _try_duckduckgo()
    elif pref == "serper":
        results = _try_serper()
        if not results and allow_fallback:
            results = _try_duckduckgo()
    elif pref in ["duckduckgo", "ddg"]:
        results = _try_duckduckgo()
    else:
        # Default Auto: Tavily (if key) -> Serper (if key) -> DuckDuckGo
        if tavily_key:
            results = _try_tavily()
        elif serper_key:
            results = _try_serper()
        if not results:
            results = _try_duckduckgo()

    # Automatic Cascade Fallback: If targeted domain returned no results, search open global web
    if not results and domains and allow_fallback:
        return execute_web_search(clean_query, domains=None, preferred_engine=pref, max_results=max_results, allow_fallback=False)

    return results[:max_results]


def get_active_web_search_engine(workspace: Optional[str] = None) -> str:
    """Returns the name of the currently active search engine (Tavily, Serper, or DuckDuckGo) based on preferences and configured keys."""
    from any_context.core.utils import get_api_key
    from any_context.config.db_store import ConfigDBStore

    pref = "auto"
    try:
        store = ConfigDBStore()
        pref = store.get_default_search_engine(workspace_name=workspace)
    except Exception:
        pass

    tavily_key = get_api_key("tavily") or os.getenv("TAVILY_API_KEY")
    serper_key = get_api_key("serper") or os.getenv("SERPER_API_KEY")

    if pref == "tavily" and tavily_key:
        return "Tavily Search API"
    elif pref == "serper" and serper_key:
        return "Serper Google Search API"
    elif pref in ["duckduckgo", "ddg"]:
        return "DuckDuckGo (Free Engine)"

    # Default Auto: Tavily -> Serper -> DuckDuckGo
    if tavily_key:
        return "Tavily Search API (Auto)"
    elif serper_key:
        return "Serper Google Search API (Auto)"
    return "DuckDuckGo (Free Engine • Auto)"


@tool()
def live_web_search(
    query: str,
    target_domain: Optional[str] = None,
    workspace: Optional[str] = None,
    max_results: int = 5
) -> str:
    """
    Performs real-time public web search on the internet when web search is enabled for the workspace.

    SMART DOMAIN ROUTING RULE:
    - If the user's question specifically relates to a website or organization registered in the workspace (especially SPAs), pass its domain in `target_domain` (e.g. target_domain='flyingsquirrelsports.ca').
    - If the user's question is about general knowledge, current events, weather, stock prices, or external topics, do NOT pass `target_domain` (leave it empty) to search the global open web.

    CRITICAL CONVERSATIONAL SEARCH RULE:
    When the user confirms or grants permission to search the web (e.g. answering 'sim', 'yes', 'pode', 'ok', etc.),
    DO NOT pass the literal confirmation string 'sim' as the query!
    You MUST reconstruct and pass the full search topic/question previously discussed (e.g., query='previsão do tempo para Calgary amanhã').

    Args:
        query (str): The search query to look up online (e.g. 'previsão do tempo para Calgary amanhã').
        target_domain (str, optional): Target specific website domain when query is about that portal (e.g. 'flyingsquirrelsports.ca').
        workspace (str, optional): Active workspace name.
        max_results (int): Maximum number of search results to return (default: 5).

    Returns:
        str: Formatted Markdown string containing search results with URLs, titles, and snippets.
    """
    clean_query = query.strip()
    if not clean_query:
        return "⚠️ Please provide a non-empty search query."

    domains = []
    if target_domain and target_domain.strip():
        extracted = _extract_domain(target_domain.strip())
        domains.append(extracted if extracted else target_domain.strip())

    results = execute_web_search(clean_query, domains=domains if domains else None, max_results=max_results, allow_fallback=True)
    engine_name = get_active_web_search_engine()

    if not results:
        return f"🔍 Nenhuma informação adicional encontrada na internet via {engine_name} para a busca: '{clean_query}'."

    prioritized_str = f" • Portal: {', '.join(domains)}" if domains else ""
    lines = [f"### 🌐 Resultados da Busca Web (via {engine_name}{prioritized_str}) para '{clean_query}':"]

    for i, r in enumerate(results, 1):
        title = r.get("title", "Resultado Web")
        url = r.get("url", "")
        snippet = r.get("snippet", "").strip()
        lines.append(f"{i}. **[{title}]({url})**\n   - **Fonte:** `{url}`\n   - **Trecho:** {snippet}\n")

    return "\n".join(lines)
