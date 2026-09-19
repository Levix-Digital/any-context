import unittest
from unittest.mock import MagicMock, patch
from langchain_core.messages import HumanMessage, AIMessage, ToolMessage

from any_context.core.agent import (
    _prune_messages_for_llm,
    ResilientSqliteSaver,
    MAX_ACTIVE_SESSION_TURNS,
    _active_summarizing_threads,
    _summarizing_lock
)


class TestSessionCascadeAndSlidingWindow(unittest.TestCase):
    def setUp(self):
        with _summarizing_lock:
            _active_summarizing_threads.clear()

    def tearDown(self):
        with _summarizing_lock:
            _active_summarizing_threads.clear()

    def test_01_prune_messages_sliding_window_large_history(self):
        """Validates that _prune_messages_for_llm enforces max_history_messages (default 10) on prior turns."""
        messages = []
        for i in range(1, 26):
            messages.append(HumanMessage(content=f"Human turn {i}"))
            messages.append(AIMessage(content=f"Assistant turn {i}"))

        # Active turn has user query + search_db tool message
        messages.append(HumanMessage(content="Qual o status final do carregamento?"))
        messages.append(ToolMessage(content="[Chunk] Documento de transporte confirmado.", tool_call_id="call_99"))

        # Total messages = 25 * 2 + 2 = 52 messages
        self.assertEqual(len(messages), 52)

        pruned = _prune_messages_for_llm(
            messages,
            max_history_messages=10,
            active_workspace="LogisticsWS",
            grounding_mode="strict"
        )

        # Output should be at most 10 historical messages + active turn (Human + ToolMessage) = 12 messages
        self.assertLessEqual(len(pruned), 12)

        # The first message in the pruned history MUST be a HumanMessage (clean turn boundary)
        self.assertEqual(pruned[0].__class__.__name__, "HumanMessage")
        self.assertIn("Human turn", pruned[0].content)

        # The active turn HumanMessage must have the grounding header injected
        active_human = [m for m in pruned if getattr(m, "type", "") == "human"][-1]
        self.assertIn("[GROUNDING: STRICT", active_human.content)
        self.assertIn("Qual o status final do carregamento?", active_human.content)

        # The active turn ToolMessage must be preserved intact
        active_tool = pruned[-1]
        self.assertEqual(getattr(active_tool, "tool_call_id", None), "call_99")
        self.assertIn("Documento de transporte confirmado", active_tool.content)

    def test_02_prune_messages_short_history_untouched(self):
        """Validates that sessions under max_history_messages are not truncated."""
        messages = [
            HumanMessage(content="Pergunta 1"),
            AIMessage(content="Resposta 1"),
            HumanMessage(content="Pergunta 2"),
        ]

        pruned = _prune_messages_for_llm(messages, max_history_messages=10)
        self.assertEqual(len(pruned), 3)
        self.assertEqual(pruned[0].content, "Pergunta 1")
        self.assertEqual(pruned[1].content, "Resposta 1")

    def test_03_resilient_sqlite_saver_15_turn_rollover(self):
        """Validates that ResilientSqliteSaver limits checkpoints to 15 turns and dispatches older turns."""
        saver = ResilientSqliteSaver(conn=MagicMock())

        # Create 20 turns (40 messages)
        msgs = []
        for i in range(1, 21):
            msgs.append(HumanMessage(content=f"User turn {i}"))
            msgs.append(AIMessage(content=f"AI turn {i}"))

        mock_tuple = MagicMock()
        mock_tuple.checkpoint = {
            "channel_values": {
                "messages": msgs
            }
        }

        config = {
            "configurable": {
                "thread_id": "rpc_session_Logistics",
                "active_workspace": "Logistics"
            }
        }

        with patch.object(ResilientSqliteSaver, "_dispatch_background_summarization") as mock_dispatch:
            with patch("langgraph.checkpoint.sqlite.SqliteSaver.get_tuple", return_value=mock_tuple):
                res = saver.get_tuple(config)

                self.assertIsNotNone(res)
                retained = res.checkpoint["channel_values"]["messages"]

                # Should retain exactly 15 turns (30 messages)
                human_count = sum(1 for m in retained if getattr(m, "type", "") == "human")
                self.assertEqual(human_count, MAX_ACTIVE_SESSION_TURNS)
                self.assertEqual(len(retained), 30)
                self.assertEqual(retained[0].content, "User turn 6")
                self.assertEqual(retained[-1].content, "AI turn 20")

                # Dispatched older messages should be 5 turns (10 messages: turns 1 to 5)
                mock_dispatch.assert_called_once()
                args, kwargs = mock_dispatch.call_args
                dispatched_msgs = args[0]
                self.assertEqual(len(dispatched_msgs), 10)
                self.assertEqual(dispatched_msgs[0].content, "User turn 1")
                self.assertEqual(dispatched_msgs[-1].content, "AI turn 5")
                self.assertEqual(kwargs.get("workspace"), "Logistics")
                self.assertEqual(kwargs.get("thread_id"), "rpc_session_Logistics")

    def test_04_resilient_sqlite_saver_under_15_turns_no_dispatch(self):
        """Validates that sessions with <= 15 turns do not trigger rollover summarization."""
        saver = ResilientSqliteSaver(conn=MagicMock())

        msgs = []
        for i in range(1, 10):
            msgs.append(HumanMessage(content=f"User turn {i}"))
            msgs.append(AIMessage(content=f"AI turn {i}"))

        mock_tuple = MagicMock()
        mock_tuple.checkpoint = {
            "channel_values": {
                "messages": msgs
            }
        }

        config = {"configurable": {"thread_id": "session_test"}}

        with patch.object(ResilientSqliteSaver, "_dispatch_background_summarization") as mock_dispatch:
            with patch("langgraph.checkpoint.sqlite.SqliteSaver.get_tuple", return_value=mock_tuple):
                res = saver.get_tuple(config)
                self.assertIsNotNone(res)
                retained = res.checkpoint["channel_values"]["messages"]
                self.assertEqual(len(retained), 18)
                mock_dispatch.assert_not_called()

    def test_05_background_summarization_concurrency_lock(self):
        """Validates that duplicate background threads are not launched for the same active thread_id."""
        with _summarizing_lock:
            _active_summarizing_threads.add("busy_thread")

        with patch("threading.Thread") as mock_thread:
            ResilientSqliteSaver._dispatch_background_summarization(
                messages=[HumanMessage(content="Hi")],
                workspace="TestWS",
                thread_id="busy_thread"
            )
            mock_thread.assert_not_called()

    def test_06_memory_manager_strips_grounding_headers_on_summarization(self):
        """Validates that MemoryManager strips injected grounding headers before sending to compressor."""
        from any_context.memory.manager import MemoryManager

        with patch("any_context.memory.compressor.MemoryCompressor.summarize_chat_block") as mock_compress:
            mock_compress.return_value = "Summary of session"
            with patch("any_context.memory.store.MemoryStore.save_memory_entry"):
                mgr = MemoryManager()

                messages = [
                    HumanMessage(content="[GROUNDING: STRICT (WORKSPACE KNOWLEDGE BASE ONLY)]\n- Follow instructions\n\nQual o prazo de entrega?"),
                    AIMessage(content="O prazo de entrega é de 5 dias úteis.")
                ]

                mgr.process_session_messages(messages, workspace="TestWS", thread_id="test_thread")

                mock_compress.assert_called_once()
                chat_block = mock_compress.call_args[0][0]
                self.assertNotIn("[GROUNDING:", chat_block)
                self.assertIn("USER: Qual o prazo de entrega?", chat_block)
                self.assertIn("ASSISTANT: O prazo de entrega é de 5 dias úteis.", chat_block)


if __name__ == "__main__":
    unittest.main()
