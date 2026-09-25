"""
Unit tests for Native Rust ReAct Agent Engine (actx-agent via PyAgentEngine)
"""
import os
import tempfile
import unittest
import json
import any_context_core_rs as core


class TestNativeAgentEngine(unittest.TestCase):
    def test_native_agent_creation_with_mock_provider(self):
        engine = core.PyAgentEngine(
            provider="mock",
            default_model="mock-model",
            system_prompt="You are a helpful assistant.",
            max_turns=5,
            temperature=0.0,
            execution_mode="react",
            search_mode="auto"
        )
        self.assertIsNotNone(engine)
        self.assertEqual(engine.tools_count(), 0)

    def test_native_agent_from_lm_client(self):
        client = core.PyLmClient("mock")
        engine = core.PyAgentEngine(
            client=client,
            default_model="mock-model",
            system_prompt="Test system prompt",
            max_turns=4
        )
        self.assertIsNotNone(engine)

    def test_native_agent_register_and_query_tools(self):
        engine = core.PyAgentEngine(provider="mock", default_model="mock-model")

        def dummy_calc(args_str):
            data = json.loads(args_str) if args_str else {}
            a = data.get("a", 0)
            b = data.get("b", 0)
            return str(a + b)

        schema = json.dumps({
            "type": "object",
            "properties": {
                "a": {"type": "integer"},
                "b": {"type": "integer"}
            },
            "required": ["a", "b"]
        })

        engine.register_tool(
            "calculator",
            "Adds two numbers together",
            schema,
            dummy_calc
        )

        self.assertTrue(engine.contains_tool("calculator"))
        self.assertFalse(engine.contains_tool("nonexistent_tool"))
        self.assertEqual(engine.tools_count(), 1)

    def test_native_agent_run_direct(self):
        engine = core.PyAgentEngine(
            provider="mock",
            default_model="mock-model",
            system_prompt="You are an AI assistant."
        )
        resp = engine.run("What is the meaning of life?")
        self.assertIsNotNone(resp)
        self.assertIn("Mock response to: What is the meaning of life?", resp.content)
        self.assertGreaterEqual(resp.total_turns, 1)
        self.assertEqual(resp.tool_calls_count, 0)
        self.assertEqual(resp.finish_reason, "Stop")
        self.assertGreaterEqual(resp.execution_time_ms, 0)

        # Verify to_dict conversion
        d = resp.to_dict()
        self.assertIsInstance(d, dict)
        self.assertEqual(d["content"], resp.content)
        self.assertEqual(d["total_turns"], resp.total_turns)

    def test_native_agent_run_with_event_callback(self):
        engine = core.PyAgentEngine(
            provider="mock",
            default_model="mock-model"
        )
        collected_events = []

        def on_event(ev):
            collected_events.append(ev)

        resp = engine.run_with_callback("Tell me a story", on_event)
        self.assertIsNotNone(resp)
        self.assertTrue(len(collected_events) > 0)

        # Inspect events
        event_types = [ev.event_type for ev in collected_events]
        self.assertTrue("Delta" in event_types or "Done" in event_types)

        # Check PyAgentEvent helper methods
        for ev in collected_events:
            d = ev.to_dict()
            self.assertIn("type", d)
            self.assertIn("payload", d)
            if ev.event_type == "Done":
                self.assertTrue(ev.is_done())

    def test_native_agent_sqlite_session_persistence(self):
        with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
            temp_db = f.name

        try:
            engine = core.PyAgentEngine(
                provider="mock",
                default_model="mock-model",
                db_path=temp_db
            )
            # Run first turn in session
            resp1 = engine.run("Hello from turn 1", session_id="test_sess_01")
            self.assertIn("Mock response", resp1.content)

            # Run second turn in same session
            resp2 = engine.run("Hello from turn 2", session_id="test_sess_01")
            self.assertIn("Mock response", resp2.content)
        finally:
            if os.path.exists(temp_db):
                try:
                    os.remove(temp_db)
                except Exception:
                    pass


    def test_create_native_anycontext_agent_factory_and_stream(self):
        from any_context.core.agent import create_native_anycontext_agent, NativeAgentWrapper

        agent = create_native_anycontext_agent(
            active_workspace="Default",
            model_override="mock-model",
            provider_override="mock"
        )
        self.assertIsNotNone(agent)
        self.assertIsInstance(agent, NativeAgentWrapper)

        # Test streaming
        chunks = list(agent.stream({"messages": ["Test factory prompt"]}))
        self.assertTrue(len(chunks) > 0)
        token, meta = chunks[0]
        self.assertEqual(meta["langgraph_node"], "agent")
        self.assertIn("Mock response", token.content)
        self.assertEqual(token.type, "ai")

    def test_native_agent_wrapper_invoke(self):
        from any_context.core.agent import create_native_anycontext_agent

        agent = create_native_anycontext_agent(
            active_workspace="Default",
            model_override="mock-model",
            provider_override="mock"
        )
        self.assertIsNotNone(agent)

        res = agent.invoke({"messages": ["Test invoke message"]})
        self.assertIn("messages", res)
        self.assertEqual(len(res["messages"]), 1)
        msg = res["messages"][0]
        self.assertIn("Mock response", msg.content)
        self.assertEqual(msg.type, "ai")


if __name__ == "__main__":
    unittest.main()
