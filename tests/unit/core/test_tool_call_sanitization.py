import unittest
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage
from any_context.core.agent import sanitize_conversation_messages, _prune_messages_for_llm


class TestToolCallSanitization(unittest.TestCase):

    def test_complete_tool_calls_preserved(self):
        messages = [
            HumanMessage(content="Hello"),
            AIMessage(content="", tool_calls=[{"id": "call_1", "name": "search_db", "args": {}}]),
            ToolMessage(content="Found results", tool_call_id="call_1"),
            AIMessage(content="Here is your answer")
        ]
        sanitized = sanitize_conversation_messages(messages)
        self.assertEqual(len(sanitized), 4)
        self.assertEqual(sanitized[1].tool_calls[0]["id"], "call_1")
        self.assertEqual(sanitized[2].tool_call_id, "call_1")

    def test_orphan_tool_call_injected_with_synthetic_response(self):
        messages = [
            HumanMessage(content="Hello"),
            AIMessage(content="", tool_calls=[{"id": "call_orphan", "name": "search_db", "args": {}}]),
            HumanMessage(content="Next question")
        ]
        sanitized = sanitize_conversation_messages(messages)
        self.assertEqual(len(sanitized), 4)
        self.assertIsInstance(sanitized[1], AIMessage)
        self.assertIsInstance(sanitized[2], ToolMessage)
        self.assertEqual(sanitized[2].tool_call_id, "call_orphan")
        self.assertIn("interrupted", sanitized[2].content.lower())
        self.assertIsInstance(sanitized[3], HumanMessage)
        self.assertEqual(sanitized[3].content, "Next question")

    def test_partial_orphan_tool_calls_injected(self):
        messages = [
            AIMessage(
                content="",
                tool_calls=[
                    {"id": "call_A", "name": "search_db", "args": {}},
                    {"id": "call_B", "name": "search_db", "args": {}}
                ]
            ),
            ToolMessage(content="Result A", tool_call_id="call_A"),
            HumanMessage(content="Next")
        ]
        sanitized = sanitize_conversation_messages(messages)
        self.assertEqual(len(sanitized), 4)
        # Should have ToolMessage for call_A then synthetic ToolMessage for call_B
        tool_ids = [m.tool_call_id for m in sanitized if isinstance(m, ToolMessage)]
        self.assertIn("call_A", tool_ids)
        self.assertIn("call_B", tool_ids)

    def test_prune_messages_for_llm_sanitizes_orphans(self):
        messages = [
            HumanMessage(content="First turn"),
            AIMessage(content="", tool_calls=[{"id": "call_broken", "name": "search_db", "args": {}}]),
            HumanMessage(content="Second turn")
        ]
        pruned = _prune_messages_for_llm(messages, active_workspace="TestWorkspace")
        # Must contain a ToolMessage responding to call_broken
        tool_msgs = [m for m in pruned if getattr(m, "type", "") == "tool" or isinstance(m, ToolMessage)]
        self.assertEqual(len(tool_msgs), 1)
        self.assertEqual(tool_msgs[0].tool_call_id, "call_broken")


if __name__ == "__main__":
    unittest.main()
