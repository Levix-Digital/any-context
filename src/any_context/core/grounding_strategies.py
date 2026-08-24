from abc import ABC, abstractmethod
from typing import Optional

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
    - Parametric Memory: FORBIDDEN (Zero speculation/hallucination)
    - Web Search: Permission-Gated (Asks user before querying web if facts missing locally)
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        if web_search_enabled:
            return (
                f"[GROUNDING: STRICT | Priority 0: VectorDB ONLY | Parametric Memory: FORBIDDEN | Web Search: PERMISSION-GATED | Workspace: '{ws}']\n"
                "- Answer strictly and exclusively from retrieved workspace documents. Zero speculation or outside facts.\n"
                "- If information is missing locally: declare '⚠️ Essa informação não consta nos documentos deste workspace.' and ask user if they want a web search.\n"
                "- NEVER call live_web_search autonomously without explicit confirmation."
            )
        return (
            f"[GROUNDING: STRICT | Priority 0: VectorDB ONLY | Parametric Memory: FORBIDDEN | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Answer strictly and exclusively from retrieved workspace documents. Zero speculation or outside facts.\n"
            "- If information is missing locally: declare '⚠️ Essa informação não consta nos documentos deste workspace.'"
        )


class HybridGroundingStrategy(GroundingStrategy):
    """
    Hybrid Mode Strategy (Balanced Default):
    - VectorDB: Priority 0 (Workspace facts first)
    - Parametric Memory: Priority 1
    - Web Search: Priority 1 (Autonomous execution; most recent source between Web & Parametric wins)
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        if web_search_enabled:
            return (
                f"[GROUNDING: HYBRID | Priority 0: VectorDB | Priority 1: Web Search & Parametric (Most Recent Wins) | Workspace: '{ws}']\n"
                "- Present local workspace facts first. If missing or complementary, use web search autonomously (most recent fact wins).\n"
                "- Clearly differentiate workspace facts ('### 📂 Informações do Workspace') from web/general knowledge ('### 🌐 Informações Complementares da Web')."
            )
        return (
            f"[GROUNDING: HYBRID | Priority 0: VectorDB | Priority 1: Parametric Memory (Labeled) | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Present local workspace facts first. Complement with labeled model knowledge ('De acordo com meus conhecimentos gerais...').\n"
            "- Clearly separate workspace facts from general model knowledge."
        )


class ProactiveGroundingStrategy(GroundingStrategy):
    """
    Proactive Mode Strategy (Research & Strategy):
    - VectorDB: Priority 0
    - Parametric Memory: Priority 0
    - Web Search: Priority 0 (Total real-time fusion; most recent fact across all sources wins)
    """

    def format_turn_header(self, workspace_name: Optional[str] = None, web_search_enabled: bool = False) -> str:
        ws = workspace_name or "Current"
        if web_search_enabled:
            return (
                f"[GROUNDING: PROACTIVE | All Sources Priority 0 (VectorDB + Web + Parametric) | Most Recent Wins | Workspace: '{ws}']\n"
                "- Total real-time fusion of workspace files, live web intelligence, and strategic domain knowledge.\n"
                "- Highlight temporal discrepancies, anticipate risks, provide actionable next steps, and suggest authoritative URLs to index."
            )
        return (
            f"[GROUNDING: PROACTIVE | All Sources Priority 0 (VectorDB + Parametric) | Forward-Looking | Web Search: DISABLED | Workspace: '{ws}']\n"
            "- Fuse workspace files and strategic domain knowledge. Anticipate risks and recommend forward-looking next steps."
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
