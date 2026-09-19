import pytest
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage

from any_context.core.epistemic import (
    EpistemicState,
    classify_epistemic_state,
    is_pure_negative_disclaimer
)
from any_context.core.agent import _prune_messages_for_llm


def test_classify_epistemic_state_absence():
    # Pure absence disclaimers
    text_pt = "⚠️ Essa informação não consta nos documentos deste workspace. Não consegui encontrar detalhes."
    assert classify_epistemic_state(text_pt) == EpistemicState.FACTUAL_ABSENCE

    text_en = "⚠️ This information is not found in the documents of this workspace."
    assert classify_epistemic_state(text_en) == EpistemicState.FACTUAL_ABSENCE


def test_classify_epistemic_state_factual():
    # Substantive responses with sources or tables
    text_sources = (
        "Aqui estão os dados encontrados:\n- Remetente: SL Source\n- Destinatário: IKEA\n\n"
        "📄 Fontes Consultadas:\n- I.CMR_ONE_PICKUP.pdf"
    )
    assert classify_epistemic_state(text_sources) == EpistemicState.GROUNDED_FACTUAL

    text_table = (
        "| Campo | Valor |\n| --- | --- |\n| Remetente | IKEA Distribution |\n| Destino | Calgary |"
    )
    assert classify_epistemic_state(text_table) == EpistemicState.GROUNDED_FACTUAL


def test_classify_epistemic_state_clarification():
    text_clarif = (
        "Temos registros de 2025 e 2026. De qual ano você gostaria das informações?"
    )
    assert classify_epistemic_state(text_clarif) == EpistemicState.CLARIFICATION


def test_is_pure_negative_disclaimer_preserves_tables():
    # A rich CMR table where one cell says "Não consta" must NOT be treated as a pure negative disclaimer
    rich_table = (
        "### Detalhes do Envio\n"
        "| Campo | Valor |\n"
        "| --- | --- |\n"
        "| Remetente | SL Source Logistics |\n"
        "| Destinatário | IKEA Calgary |\n"
        "| Observações | não consta nos documentos deste workspace |\n"
    )
    assert is_pure_negative_disclaimer(rich_table) is False

    # A short disclaimer is a pure negative disclaimer
    disclaimer = "⚠️ Essa informação não consta nos documentos deste workspace."
    assert is_pure_negative_disclaimer(disclaimer) is True


def test_prune_purges_historical_absence_keeps_active():
    # Turn 1: Absence
    h1 = HumanMessage(content="Qual o relatório financeiro de 2029?")
    a1 = AIMessage(
        content="⚠️ Essa informação não consta nos documentos deste workspace.",
        additional_kwargs={"epistemic_state": EpistemicState.FACTUAL_ABSENCE.value}
    )
    # Turn 2: Real question
    h2 = HumanMessage(content="Quais foram as entregas da IKEA?")

    messages = [h1, a1, h2]
    pruned = _prune_messages_for_llm(messages, active_workspace="IKEAShipments", grounding_mode="strict")

    # a1 must be purged from historical messages
    assert a1 not in pruned
    # h2 is the active turn and must be present (with grounding header)
    assert any("Quais foram as entregas da IKEA?" in str(m.content) for m in pruned)


def test_prune_legacy_fallback():
    # Message without additional_kwargs (legacy session)
    h1 = HumanMessage(content="Arquivo inexistente?")
    a1 = AIMessage(content="⚠️ Essa informação não consta nos documentos deste workspace.")
    h2 = HumanMessage(content="Quais foram as entregas da IKEA?")

    messages = [h1, a1, h2]
    pruned = _prune_messages_for_llm(messages, active_workspace="IKEAShipments", grounding_mode="strict")

    # Legacy absence message must be purged via fallback heuristic
    assert a1 not in pruned


def test_deduplicate_consecutive_human_messages():
    h1 = HumanMessage(content="Quais foram as entregas da IKEA?")
    a1 = AIMessage(
        content="⚠️ Essa informação não consta nos documentos deste workspace.",
        additional_kwargs={"epistemic_state": EpistemicState.FACTUAL_ABSENCE.value}
    )
    h2 = HumanMessage(content="Quais foram as entregas da IKEA?")
    a2 = AIMessage(
        content="⚠️ Essa informação não consta nos documentos deste workspace.",
        additional_kwargs={"epistemic_state": EpistemicState.FACTUAL_ABSENCE.value}
    )
    h3 = HumanMessage(content="Quais foram as entregas da IKEA?")

    messages = [h1, a1, h2, a2, h3]
    pruned = _prune_messages_for_llm(messages, active_workspace="IKEAShipments", grounding_mode="strict")

    # After purging a1 and a2, h1 and h2 would be consecutive identical messages.
    # Deduplication collapses them so only 1 human question remains!
    human_messages = [m for m in pruned if getattr(m, "type", "") == "human" or m.__class__.__name__ == "HumanMessage"]
    assert len(human_messages) == 1
