"""
Epistemic State Machine for AnyContext Agent.

Tracks, categorizes, and controls the epistemic state of conversational turns
to completely eliminate Self-Consistency Bias / Attention Echo Chambers in LLM inference.
"""

from enum import Enum
import re
from typing import Optional, List, Any


class EpistemicState(str, Enum):
    """
    First-class epistemic status of an agent conversational turn.
    """
    GROUNDED_FACTUAL = "grounded_factual"   # Chunks retrieved and substantive factual answer produced with sources
    FACTUAL_ABSENCE  = "factual_absence"    # Tool returned 0 docs or model produced legitimate absence declaration
    CLARIFICATION    = "clarification"      # Model asked for clarification (clarification-dialogue skill)
    CONVERSATIONAL   = "conversational"     # Chit-chat, greetings, direct conversational dialogue


def is_pure_negative_disclaimer(content: str) -> bool:
    """
    Detects whether an assistant message represents a pure boilerplate negative disclaimer
    (indicating that requested information was absent at that moment) without substantive factual content.
    """
    if not isinstance(content, str) or not content:
        return False
    c = content.strip()
    
    # Check for canonical absence prefixes or sentences
    absence_patterns = [
        r"⚠️\s*Essa informação não consta nos documentos deste workspace",
        r"⚠️\s*This information is not found in the documents of this workspace",
        r"⚠️\s*Esta información no se encuentra en los documentos de este espacio",
        r"não consta nos documentos deste workspace",
        r"not found in the documents of this workspace"
    ]
    
    has_absence_indicator = any(re.search(p, c, flags=re.IGNORECASE) for p in absence_patterns)
    if not has_absence_indicator:
        return False

    # A pure disclaimer lacks rich tabular data and is compact
    has_table = "|" in c and c.count("|") >= 4
    has_substantive_sections = "###" in c and len(c) > 450
    if has_table or has_substantive_sections:
        return False

    return len(c) < 500


def classify_epistemic_state(
    content: str,
    tool_messages: Optional[List[Any]] = None,
    tool_calls: Optional[List[Any]] = None
) -> EpistemicState:
    """
    Deterministically classifies the EpistemicState of a completed assistant turn.
    """
    c = (content or "").strip()

    # 1. Pure absence disclaimer check
    if is_pure_negative_disclaimer(c):
        return EpistemicState.FACTUAL_ABSENCE

    # 2. Check if search tool was called and returned empty
    if tool_messages:
        for tm in tool_messages:
            tm_c = str(getattr(tm, "content", ""))
            if "Nenhum documento encontrado" in tm_c or "No documents found" in tm_c:
                if "⚠️" in c or "não consta" in c.lower() or "not found" in c.lower():
                    return EpistemicState.FACTUAL_ABSENCE

    # 3. Check for clarification dialogue cues
    clarification_patterns = [
        r"você gostaria de",
        r"de qual .* você gostaria",
        r"poderia especificar",
        r"qual ano você gostaria",
        r"qual período",
        r"would you like to",
        r"could you specify",
        r"which year",
        r"which period"
    ]
    if any(re.search(p, c, flags=re.IGNORECASE) for p in clarification_patterns) and "?" in c:
        return EpistemicState.CLARIFICATION

    # 4. Check for grounded factual response (citations, sources, or tables)
    has_citations = any(kw in c for kw in ["Fontes Consultadas", "Sources Consulted", "Document Chunk", "📄", "🌐"])
    has_table = "|" in c and c.count("|") >= 4
    if has_citations or has_table or (tool_messages and len(tool_messages) > 0):
        return EpistemicState.GROUNDED_FACTUAL

    return EpistemicState.CONVERSATIONAL
