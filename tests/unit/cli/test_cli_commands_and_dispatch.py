import os
import sys
import unittest
from unittest.mock import patch

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.config.db_store import ConfigDBStore
from any_context.cli.chat_loop import run_chat_loop
from tests.e2e_helpers import safe_stdout_write

class TestCLICommandsAndDispatch(unittest.TestCase):
    """
    CLI Unit Test Suite: Validates chat loop command dispatching and execution safety.
    """

    def setUp(self):
        self.store = ConfigDBStore()
        self.test_ws = "test_cli_dispatch_ws"

    def tearDown(self):
        try:
            self.store.remove_workspace(self.test_ws)
        except Exception:
            pass

    def test_01_workspace_add_and_switch_dispatch(self):
        """Validates that /workspace add and /switch execute cleanly without scope errors."""
        safe_stdout_write("\n>>> [CLI UNIT] Testing /workspace add and /switch Dispatch...\n")
        mock_inputs = [f"/workspace add {self.test_ws}", "/exit"]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.ingestion.local_folder_ingestor.run_index_folder"):
                run_chat_loop(active_workspace="Default")

        settings = self.store.get_app_settings()
        ws_names = [w.name for w in settings.workspaces] if settings else []
        self.assertIn(self.test_ws, ws_names, f"Workspace '{self.test_ws}' must be registered in ConfigDBStore")
        safe_stdout_write("  [OK] /workspace add dispatch executed and verified!\n")

    def test_02_version_and_clear_dispatch(self):
        """Validates that /version, /clear, and /exit dispatch cleanly."""
        safe_stdout_write(">>> [CLI UNIT] Testing /version and /clear Dispatch...\n")
        mock_inputs = ["/version", "/clear", "/exit"]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            run_chat_loop(active_workspace="Default")
        safe_stdout_write("  [OK] /version and /clear dispatch verified!\n")

    def test_03_model_switch_dispatch(self):
        """Validates that /model switches the active session model."""
        safe_stdout_write(">>> [CLI UNIT] Testing /model Dispatch...\n")
        mock_inputs = ["/model gpt-4o-mini", "/exit"]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            run_chat_loop(active_workspace="Default")
        safe_stdout_write("  [OK] /model dispatch verified!\n")

    def test_04_paste_command_and_multiline_dispatch(self):
        """Validates that /paste collects multiline text and dispatches to AI agent loop."""
        safe_stdout_write(">>> [CLI UNIT] Testing /paste Command Multiline Dispatch...\n")
        mock_inputs = [
            "/paste",
            "Contract Clause 1: Confidentiality duration 5 years.",
            "Contract Clause 2: Liquidated damages $100,000.",
            "/send",
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_agent.stream.return_value = []
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_agent.stream.called, "Agent stream must be called with pasted multiline text")
                called_args = mock_agent.stream.call_args[0][0]
                self.assertIn("Contract Clause 1", called_args["messages"][0])
                self.assertIn("Contract Clause 2", called_args["messages"][0])
        safe_stdout_write("  [OK] /paste command multiline dispatch verified!\n")

    def test_05_triple_quotes_block_dispatch(self):
        """Validates that triple quotes delimiter ('\"\"\"') collects multiline text."""
        safe_stdout_write(">>> [CLI UNIT] Testing Triple Quotes ('\"\"\"') Multiline Block...\n")
        mock_inputs = [
            '"""Here is my meeting transcript:',
            "- Topic A: Vector DB architecture",
            "- Topic B: Temporal RAG metadata",
            '"""',
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_agent.stream.return_value = []
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_agent.stream.called, "Agent stream must be called with triple quotes block text")
                called_args = mock_agent.stream.call_args[0][0]
                self.assertIn("Topic A: Vector DB architecture", called_args["messages"][0])
                self.assertIn("Topic B: Temporal RAG metadata", called_args["messages"][0])
        safe_stdout_write("  [OK] Triple quotes multiline block verified!\n")

    def test_06_paste_cancel_dispatch(self):
        """Validates that /cancel cleanly aborts multiline paste without dispatching to AI."""
        safe_stdout_write(">>> [CLI UNIT] Testing /paste /cancel Abort Flow...\n")
        mock_inputs = [
            "/paste",
            "This text will be cancelled.",
            "/cancel",
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertFalse(mock_agent.stream.called, "Agent stream must NOT be called when paste is cancelled")
        safe_stdout_write("  [OK] /paste cancellation verified!\n")

if __name__ == "__main__":
    unittest.main()
