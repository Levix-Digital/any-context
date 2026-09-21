"""
Unit Tests for Collaborative Dialogue Protocol, Turn-Based Epistemic Pruning,
and Dead-End Elimination (v0.30.29).
"""
import unittest
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage
from any_context.core.agent import _prune_messages_for_llm
from any_context.core.utils import get_system_prompt
from any_context.core.grounding_strategies import StrictGroundingStrategy
from any_context.skills.registry import SkillRegistry


class TestCollaborativeDialogueAndTurnPruning(unittest.TestCase):

    def test_full_turn_cycle_pruned_on_absence(self):
        """
        Ensures that when an earlier turn ended in factual absence,
        the ENTIRE turn (HumanMessage, AIMessage with tool_calls, ToolMessage, and AIMessage)
        is completely purged from the LLM payload, preventing orphan tool calls from blocking retries.
        """
        messages = [
            HumanMessage(content="Quais foram as entregas da IKEA?"),
            AIMessage(
                content="",
                tool_calls=[{"name": "search_db", "args": {"prompt_text": "entregas IKEA"}, "id": "call_1", "type": "tool_call"}]
            ),
            ToolMessage(content="[Prior chunks]", tool_call_id="call_1", name="search_db"),
            AIMessage(
                content="⚠️ Essa informação não consta nos documentos deste workspace.",
                additional_kwargs={"epistemic_state": "factual_absence"}
            ),
            # Active turn (same query re-sent by user):
            HumanMessage(content="Quais foram as entregas da IKEA?")
        ]

        pruned = _prune_messages_for_llm(
            messages,
            active_workspace="IKEAShipments",
            grounding_mode="strict",
            web_search_enabled=False
        )

        # The prior failed turn must be 100% eliminated so the model starts completely fresh
        self.assertEqual(len(pruned), 1)
        self.assertEqual(pruned[0].type, "human")
        self.assertIn("Quais foram as entregas da IKEA?", pruned[0].content)

    def test_collaborative_dialogue_skill_in_system_prompt(self):
        """
        Verifies that for human callers, the system prompt contains the explicit
        dead-end disclaimer prohibition and collaborative format alignment directives.
        """
        prompt = get_system_prompt(active_workspace="IKEAShipments", caller_type="human")
        self.assertIn("clarification-dialogue", prompt.lower())
        self.assertIn("Absolute Prohibition of Cold Absence Disclaimers", prompt)
        self.assertIn("Proactive Guiding Protocol When Topic is Genuinely Missing", prompt)

    def test_strict_grounding_turn_header_directives(self):
        """
        Verifies that StrictGroundingStrategy formats headers instructing the model
        to follow clarification dialogue rather than declaring total absence on broad queries.
        """
        strategy = StrictGroundingStrategy()
        header = strategy.format_turn_header(workspace_name="IKEAShipments", web_search_enabled=False)
        self.assertIn("BROAD OR MULTI-RECORD QUERIES", header)
        self.assertIn("TOPIC NOT FOUND / ZERO RESULTS", header)
        self.assertIn("NEVER declare absence", header)


if __name__ == "__main__":
    unittest.main()
