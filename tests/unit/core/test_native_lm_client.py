"""
Unit tests for Native Rust Language Model Façade (actx-lm via PyLmClient)
"""
import unittest
import json
import any_context_core_rs as core

class TestNativeLmClient(unittest.TestCase):
    def test_mock_provider_creation_and_quick_chat(self):
        client = core.PyLmClient("mock")
        self.assertEqual(client.provider_id(), "mock")

        reply = client.quick_chat("mock-model", "Test message")
        self.assertIn("Mock response to: Test message", reply)

    def test_mock_chat_complete_json(self):
        client = core.PyLmClient("mock")
        messages = [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "What is the capital of France?"}
        ]
        resp_json = client.chat_complete("mock-model", json.dumps(messages), temperature=0.5, max_tokens=100)
        resp = json.loads(resp_json)

        self.assertIn("content", resp)
        self.assertIn("Mock response to: What is the capital of France?", resp["content"])
        self.assertEqual(resp["model"], "mock-model")
        self.assertEqual(resp["finish_reason"], "stop")
        self.assertIsNotNone(resp.get("usage"))

    def test_mock_streaming_tokens(self):
        client = core.PyLmClient("mock")
        messages = [
            {"role": "user", "content": "Streaming token test"}
        ]
        tokens = client.chat_stream_collect("mock-model", json.dumps(messages))
        self.assertTrue(len(tokens) > 0)
        joined = "".join(tokens)
        self.assertIn("Mock response to: Streaming token test", joined)

    def test_ollama_local_client_instantiation_without_key(self):
        client = core.PyLmClient("ollama", base_url="http://localhost:11434/v1")
        self.assertEqual(client.provider_id(), "ollama")

    def test_openai_client_instantiation_with_key(self):
        client = core.PyLmClient("openai", api_key="sk-test-mock-key")
        self.assertEqual(client.provider_id(), "openai")

    def test_anthropic_client_instantiation_with_key(self):
        client = core.PyLmClient("anthropic", api_key="sk-ant-test-key")
        self.assertEqual(client.provider_id(), "anthropic")

    def test_gemini_client_instantiation_with_key(self):
        client = core.PyLmClient("gemini", api_key="AIzaSy-test-key")
        self.assertEqual(client.provider_id(), "gemini")

if __name__ == "__main__":
    unittest.main()
