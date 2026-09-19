import pytest
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage
from any_context.core.agent import _prune_historical_tool_messages


def test_prune_preserves_current_turn_tool_messages():
    """
    Validates that _prune_historical_tool_messages strictly prunes ONLY
    ToolMessages from prior turns (idx < last_human_idx) while preserving
    100% of the active turn's ToolMessages (idx > last_human_idx), preventing
    zero-context regressions on multi-turn conversations.
    """
    # Simulate a conversation with 3 turns
    messages = [
        # Turn 1
        HumanMessage(content="Qual foi o shipment de 01/09?"),
        AIMessage(content="", tool_calls=[{"name": "search_db", "args": {}, "id": "call_1", "type": "tool_call"}]),
        ToolMessage(content="Heavy prior turn context " * 50, tool_call_id="call_1", name="search_db"),
        AIMessage(content="O shipment de 01/09 foi 015-TSO-S10000536381."),
        
        # Turn 2
        HumanMessage(content="E quem foi o motorista?"),
        AIMessage(content="", tool_calls=[{"name": "search_db", "args": {}, "id": "call_2", "type": "tool_call"}]),
        ToolMessage(content="Another heavy prior context " * 50, tool_call_id="call_2", name="search_db"),
        AIMessage(content="O motorista foi John Doe."),
        
        # Turn 3 (Active turn awaiting LLM synthesis)
        HumanMessage(content="O que diz o arquivo I.CMR_ONE_PICKUP.pdf do dia 02/09?"),
        AIMessage(content="", tool_calls=[{"name": "search_db", "args": {}, "id": "call_3", "type": "tool_call"}]),
        ToolMessage(content="FRESH ATOMIC CMR DOCUMENT CONTEXT " * 100, tool_call_id="call_3", name="search_db"),
    ]

    assert len(messages) == 11
    assert len(messages[2].content) > 300
    assert len(messages[6].content) > 300
    assert len(messages[10].content) > 300

    # Execute pruning
    pruned = _prune_historical_tool_messages(messages)

    # 1. Total message count unchanged (no messages dropped)
    assert len(pruned) == 11

    # 2. Prior tool messages (Turn 1 and Turn 2) are compacted
    assert pruned[2].content == "[Prior workspace context retrieved and synthesized in conversation history]"
    assert pruned[6].content == "[Prior workspace context retrieved and synthesized in conversation history]"

    # 3. Current turn ToolMessage (Turn 3) is 100% PRESERVED INTACT
    assert pruned[10].content == "FRESH ATOMIC CMR DOCUMENT CONTEXT " * 100

    # 4. All HumanMessage and AIMessage dialogues are 100% preserved
    assert pruned[0].content == "Qual foi o shipment de 01/09?"
    assert pruned[3].content == "O shipment de 01/09 foi 015-TSO-S10000536381."
    assert pruned[4].content == "E quem foi o motorista?"
    assert pruned[7].content == "O motorista foi John Doe."
    assert pruned[8].content == "O que diz o arquivo I.CMR_ONE_PICKUP.pdf do dia 02/09?"
    assert pruned[10].content.startswith("FRESH ATOMIC CMR")


def test_prune_noop_on_short_conversations():
    """Short conversations with 1 or 2 messages should not be altered."""
    msgs = [HumanMessage(content="Hi")]
    res = _prune_historical_tool_messages(msgs)
    assert res == msgs

    msgs2 = [HumanMessage(content="Hi"), AIMessage(content="Hello")]
    res2 = _prune_historical_tool_messages(msgs2)
    assert res2 == msgs2
