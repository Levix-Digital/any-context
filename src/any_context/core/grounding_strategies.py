"""
AnyContext AI Grounding Strategies & Source Precedence Matrix.
Encapsulates Grounding Strategy implementations (Strict, Hybrid, Proactive)
with Priority 0 for Registered Workspace Web Portals and Universal Temporal Recency Rules.
"""

from abc import ABC, abstractmethod
from typing import Optional, List


def _get_workspace_registered_domains(workspace_name: Optional[str]) -> List[str]:
    """Retrieves unique base domain names for registered web sources in a workspace."""
    if not workspace_name:
        return []
    try:
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        from any_context.tools.web_search_tool import _extract_domain
        web_store = WebSchedulerStore()
        urls = web_store.get_workspace_web_urls(workspace_name)
        domains = []
        for u in urls:
            raw_url = u.get("url") if isinstance(u, dict) else str(u)
            dom = _extract_domain(raw_url)
            if dom and dom not in domains:
                domains.append(dom)
        return domains
    except Exception:
        return []


class GroundingStrategy(ABC):
    """
    Abstract Strategy Interface for AI Grounding and Turn-Level Directive Injection.
    Encapsulates source precedence matrices (0 vs 1) and temporal recency rules per mode.
    """

    @abstractmethod
    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        """Generates an ultra-compact, token-efficient prompt header for the active turn."""
        pass


class StrictGroundingStrategy(GroundingStrategy):
    """
    Strict Mode Strategy (Audit / Legal):
    - VectorDB: Priority 0 (Sole source of truth)
    - Registered Workspace Portals: Priority 0 for live unindexed data (Permission-gated)
    - Parametric Memory: FORBIDDEN (Zero speculation/hallucination)
    - Open Web Search: Priority 1 fallback (Permission-gated)
    - Universal Recency Rule: If information is found in multiple sources, most recent source always prevails.
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        domains = _get_workspace_registered_domains(workspace_name)
        dom_clause = f" & Registered Portals ({', '.join(domains)})" if domains else ""
        dom_inst = f"- If user confirms web search, query registered portal ({', '.join(domains)}) FIRST before open web.\n" if domains else ""

        if web_search_enabled:
            return (
                f"[GROUNDING: STRICT | Priority 0: VectorDB{dom_clause} | Parametric Memory: FORBIDDEN | Web Search: PERMISSION-GATED | Workspace: '{ws}']\n"
                "- Answer strictly and exclusively from retrieved workspace documents. Zero speculation or outside facts.\n"
                "- If information is missing locally: you MUST respond EXACTLY with: '⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre \"[tópico]\"?' and STOP. Do NOT guess or invent facts.\n"
                f"{dom_inst}"
                "- NEVER call live_web_search autonomously without explicit confirmation.\n"
                "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data."
            )
        return (
            f"[GROUNDING: STRICT | Priority 0: VectorDB ONLY | Parametric Memory: FORBIDDEN | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Answer strictly and exclusively from retrieved workspace documents. Zero speculation or outside facts.\n"
            "- If information is missing locally: declare '⚠️ Essa informação não consta nos documentos deste workspace.'\n"
            "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data."
        )


class HybridGroundingStrategy(GroundingStrategy):
    """
    Hybrid Mode Strategy (Balanced Default):
    - VectorDB: Priority 0 (Workspace facts first)
    - Registered Workspace Portals: Priority 0 (Autonomous targeted search for unindexed/live content)
    - Parametric Memory & Open Web Search: Priority 1
    - Universal Recency Rule: If information is found in multiple sources, most recent source always prevails.
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        domains = _get_workspace_registered_domains(workspace_name)
        dom_clause = f" & Registered Portals ({', '.join(domains)})" if domains else ""
        dom_inst = f"- For live web search, query registered workspace portal ({', '.join(domains)}) FIRST (target_domain) before open web.\n" if domains else ""

        if web_search_enabled:
            return (
                f"[GROUNDING: HYBRID | Priority 0: VectorDB{dom_clause} | Priority 1: Open Web & Parametric | Workspace: '{ws}']\n"
                "- Present local workspace facts first. If missing or complementary, use web search autonomously.\n"
                f"{dom_inst}"
                "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data.\n"
                "- Clearly differentiate workspace facts ('### 📂 Informações do Workspace') from web/general knowledge ('### 🌐 Informações Complementares da Web')."
            )
        return (
            f"[GROUNDING: HYBRID | Priority 0: VectorDB | Priority 1: Parametric Memory (Labeled) | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Present local workspace facts first. Complement with labeled model knowledge ('De acordo com meus conhecimentos gerais...').\n"
            "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data.\n"
            "- Clearly separate workspace facts from general model knowledge."
        )


class ProactiveGroundingStrategy(GroundingStrategy):
    """
    Proactive Mode Strategy (Research & Strategy):
    - VectorDB: Priority 0
    - Registered Workspace Portals: Priority 0 (Real-time live portal search)
    - Parametric Memory & Web Search: Priority 0
    - Universal Recency Rule: Total real-time fusion; most recent source across all channels always wins.
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        domains = _get_workspace_registered_domains(workspace_name)
        dom_inst = f"- For web search, prioritize registered workspace portals ({', '.join(domains)}) first.\n" if domains else ""

        if web_search_enabled:
            return (
                f"[GROUNDING: PROACTIVE | All Sources Priority 0 (VectorDB + Registered Portals + Web + Parametric) | Workspace: '{ws}']\n"
                "- Total real-time fusion of workspace files, registered web portals, live web intelligence, and strategic domain knowledge.\n"
                f"{dom_inst}"
                "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data.\n"
                "- Highlight temporal discrepancies, anticipate risks, provide actionable next steps, and suggest authoritative URLs to index."
            )
        return (
            f"[GROUNDING: PROACTIVE | All Sources Priority 0 (VectorDB + Parametric) | Forward-Looking | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Fuse workspace files and strategic domain knowledge. Anticipate risks and recommend forward-looking next steps.\n"
            "- RECENCY RULE (SAME PRIORITY): If multiple sources within the same priority tier contain differing facts, the most recent source ALWAYS prevails and supersedes older data."
        )


_STRATEGY_REGISTRY = {
    "strict": StrictGroundingStrategy(),
    "hybrid": HybridGroundingStrategy(),
    "proactive": ProactiveGroundingStrategy(),
}


def get_grounding_strategy(mode: Optional[str] = None) -> GroundingStrategy:
    """
    Factory function to retrieve the appropriate GroundingStrategy singleton instance.
    Defaults safely to HybridGroundingStrategy if unspecified.
    """
    clean_mode = (mode or "hybrid").lower().strip()
    return _STRATEGY_REGISTRY.get(clean_mode, _STRATEGY_REGISTRY["hybrid"])
