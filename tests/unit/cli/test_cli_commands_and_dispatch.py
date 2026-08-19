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
        from tests.e2e_helpers import setup_mock_embeddings_if_needed
        setup_mock_embeddings_if_needed()
        self.store = ConfigDBStore()
        self.test_ws = "test_cli_dispatch_ws"
        self.index_patcher = patch("any_context.ingestion.local_folder_ingestor.run_index_folder")
        self.mock_run_index = self.index_patcher.start()

    def tearDown(self):
        try:
            self.index_patcher.stop()
        except Exception:
            pass
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

    def test_07_trailing_backslash_continuation_dispatch(self):
        """Validates that trailing backslash (\\) accumulates lines and dispatches to AI agent."""
        safe_stdout_write(">>> [CLI UNIT] Testing Trailing Backslash (\\) Line Continuation...\n")
        mock_inputs = [
            "Quero saber sobre:\\",
            "Startup Visa Program Canada\\",
            "and all eligibility requirements.",
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_agent.stream.return_value = []
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_agent.stream.called, "Agent stream must be called with continued multiline prompt")
                called_args = mock_agent.stream.call_args[0][0]
                self.assertIn("Quero saber sobre:", called_args["messages"][0])
                self.assertIn("Startup Visa Program Canada", called_args["messages"][0])
                self.assertIn("and all eligibility requirements.", called_args["messages"][0])
        safe_stdout_write("  [OK] Trailing backslash line continuation verified!\n")

    def test_08_transfer_command_dispatch(self):
        """Validates that /transfer dispatches correctly and transfers folder between workspaces."""
        safe_stdout_write(">>> [CLI UNIT] Testing /transfer Command Dispatch...\n")
        src_ws = "cli_transfer_src"
        tgt_ws = "cli_transfer_tgt"
        test_dir = os.path.abspath(os.path.join(os.getcwd(), "test_cli_transfer_folder"))
        os.makedirs(test_dir, exist_ok=True)

        self.store.add_workspace(src_ws, paths=[test_dir])
        self.store.add_workspace(tgt_ws, paths=[])

        try:
            mock_inputs = [
                f"/transfer {src_ws} {tgt_ws} {test_dir}",
                "/exit"
            ]
            with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
                run_chat_loop(active_workspace="Default")

            settings = self.store.get_app_settings()
            src_obj = next((w for w in settings.workspaces if w.name == src_ws), None)
            tgt_obj = next((w for w in settings.workspaces if w.name == tgt_ws), None)

            self.assertNotIn(test_dir, [os.path.abspath(p) for p in src_obj.paths])
            self.assertIn(test_dir, [os.path.abspath(p) for p in tgt_obj.paths])
            safe_stdout_write("  [OK] /transfer CLI command dispatch verified!\n")
        finally:
            self.store.remove_workspace(src_ws)
            self.store.remove_workspace(tgt_ws)
            try:
                os.rmdir(test_dir)
            except Exception:
                pass

    def test_09_multiline_inline_send_dispatch(self):
        """Validates that /send typed inline at the end of a multiline prompt terminates and dispatches cleanly."""
        safe_stdout_write(">>> [CLI UNIT] Testing Multiline Inline /send Dispatch...\n")
        mock_inputs = [
            '"""',
            "Gostei desse de alberta",
            "Quais sao os pre requisitos? /send",
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_agent.stream.return_value = []
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_agent.stream.called, "Agent stream must be called when /send is typed inline")
                called_args = mock_agent.stream.call_args[0][0]
                prompt_sent = called_args["messages"][0]
                self.assertIn("Gostei desse de alberta", prompt_sent)
                self.assertIn("Quais sao os pre requisitos?", prompt_sent)
                self.assertNotIn("/send", prompt_sent, "/send keyword must be stripped before sending to AI")
        safe_stdout_write("  [OK] Multiline inline /send termination and dispatch verified!\n")

    def test_10_slash_palette_dispatch(self):
        """Validates that typing '/' launches the interactive Slash Commands Palette and dispatches selected command."""
        safe_stdout_write(">>> [CLI UNIT] Testing '/' Slash Command Palette Dispatch...\n")
        mock_inputs = ["/", "/exit"]
        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.cli.chat_loop.show_slash_commands_palette", return_value="/version") as mock_palette:
                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_palette.called, "show_slash_commands_palette must be called when '/' is entered")
        safe_stdout_write("  [OK] '/' Slash Command Palette dispatch verified!\n")

    def test_11_trailing_slash_is_regular_prompt(self):
        """Validates that typing 'text /' is sent directly as standard prompt (not line continuation)."""
        safe_stdout_write(">>> [CLI UNIT] Testing Trailing '/' As Regular Prompt...\n")
        mock_inputs = [
            "Interessante /",
            "/exit"
        ]

        with patch("any_context.cli.chat_loop.safe_prompt_input", side_effect=mock_inputs):
            with patch("any_context.core.agent.create_anycontext_agent") as mock_create_agent:
                mock_agent = unittest.mock.MagicMock()
                mock_agent.stream.return_value = []
                mock_create_agent.return_value = mock_agent

                run_chat_loop(active_workspace="Default")
                self.assertTrue(mock_agent.stream.called)
                called_args = mock_agent.stream.call_args[0][0]
                prompt_sent = called_args["messages"][0]
                self.assertEqual(prompt_sent, "Interessante /")
        safe_stdout_write("  [OK] Trailing '/' treated as normal text prompt verified!\n")


if __name__ == "__main__":
    unittest.main()


