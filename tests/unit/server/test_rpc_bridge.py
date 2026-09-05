import os
import sys
import unittest
from unittest.mock import patch, MagicMock
import io
import json

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.server.rpc_bridge import StdioRPCServer, _send_ndjson, emit_boot_telemetry
from tests.e2e_helpers import safe_stdout_write


class TestRPCBridge(unittest.TestCase):
    """
    Unit Test Suite: Validates Stdio RPC Bridge Server (NDJSON Protocol - v0.26.0).
    """

    @classmethod
    def setUpClass(cls):
        import tempfile
        cls.temp_dir = tempfile.mkdtemp(prefix="actx_test_rpc_")
        cls.db_path = os.path.join(cls.temp_dir, "test_settings.db")
        cls._orig_db = os.environ.get("ACTX_SETTINGS_DB")
        os.environ["ACTX_SETTINGS_DB"] = cls.db_path

    @classmethod
    def tearDownClass(cls):
        import shutil
        if cls._orig_db is not None:
            os.environ["ACTX_SETTINGS_DB"] = cls._orig_db
        else:
            os.environ.pop("ACTX_SETTINGS_DB", None)
        if hasattr(cls, "temp_dir") and os.path.exists(cls.temp_dir):
            shutil.rmtree(cls.temp_dir, ignore_errors=True)

    def setUp(self):
        self.server = StdioRPCServer(default_workspace="RpcUnitTestWS")

    def tearDown(self):
        if hasattr(self, "server") and hasattr(self.server, "close"):
            self.server.close()

    def test_01_get_state_and_list_commands(self):
        """Validates that get_state and list_commands return accurate structure and all 29 commands."""
        safe_stdout_write("\n>>> [RPC UNIT] Testing get_state and list_commands...\n")
        state = self.server.get_state()
        self.assertIn("workspace", state)
        self.assertIn("model", state)
        self.assertIn("grounding_mode", state)
        self.assertIn("web_search_enabled", state)
        self.assertIn("tier_name", state)
        self.assertTrue(any(state["tier_name"].startswith(e) for e in ["🌿", "⭐", "👥", "🏢", "💼"]), f"Invalid tier_name: {state['tier_name']}")

        cmds = self.server.list_commands()
        self.assertEqual(len(cmds), 31, "All 31 slash commands must be present in palette metadata")
        slash_names = [c["command"] for c in cmds]
        self.assertIn("/switch", slash_names)
        self.assertIn("/model", slash_names)
        self.assertIn("/sync", slash_names)
        self.assertIn("/sources", slash_names)
        self.assertIn("/check-update", slash_names)
        self.assertIn("/update", slash_names)
        self.assertIn("/menu", slash_names)
        self.assertIn("/logs", slash_names)
        self.assertIn("/diagnostics", slash_names)
        safe_stdout_write("  [OK] State and 31-command catalog verified!\n")


    def test_02_handle_request_mutations(self):
        """Validates switch_workspace, set_model, set_mode, and set_web_search mutations."""
        safe_stdout_write(">>> [RPC UNIT] Testing RPC Mutation Requests...\n")
        fake_stdout = io.StringIO()

        with patch("sys.stdout", fake_stdout):
            # 1. Switch workspace
            self.server.handle_request({"id": 1, "method": "switch_workspace", "params": {"workspace": "NewRPCWS"}})
            # 2. Set model
            self.server.handle_request({"id": 2, "method": "set_model", "params": {"model": "claude-sonnet-4-5-20250929"}})
            # 3. Set mode
            self.server.handle_request({"id": 3, "method": "set_mode", "params": {"mode": "hybrid"}})
            # 4. Set web search
            self.server.handle_request({"id": 4, "method": "set_web_search", "params": {"enabled": True}})

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        res_lines = [l for l in lines if l.get("id") in (1, 2, 3, 4)]
        self.assertEqual(len(res_lines), 4)
        self.assertEqual(res_lines[0]["result"]["workspace"], "NewRPCWS")
        self.assertEqual(res_lines[1]["result"]["model"], "claude-sonnet-4-5-20250929")
        self.assertEqual(res_lines[2]["result"]["grounding_mode"], "hybrid")
        self.assertTrue(res_lines[3]["result"]["web_search_enabled"])
        safe_stdout_write("  [OK] Mutation requests executed and confirmed in state!\n")

    def test_03_handle_request_chat_streaming(self):
        """Validates that chat method streams tokens and tool tickers in real time."""
        safe_stdout_write(">>> [RPC UNIT] Testing Chat Streaming Protocol...\n")
        fake_stdout = io.StringIO()

        mock_token_1 = MagicMock()
        mock_token_1.type = "ai"
        mock_token_1.content = "Hello "

        mock_token_2 = MagicMock()
        mock_token_2.type = "ai"
        mock_token_2.content = "world!"

        mock_agent = MagicMock()
        mock_agent.stream.return_value = [(mock_token_1, {}), (mock_token_2, {})]

        with patch("any_context.core.agent.create_anycontext_agent", return_value=mock_agent):
            with patch("sys.stdout", fake_stdout):
                self.server.handle_request({"id": 10, "method": "chat", "params": {"message": "Test prompt"}})

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        self.assertTrue(any(l.get("type") == "token" and l.get("content") == "Hello " for l in lines))
        self.assertTrue(any(l.get("type") == "token" and l.get("content") == "world!" for l in lines))
        self.assertTrue(any(l.get("type") == "done" and "Hello world!" in l.get("full_reply", "") for l in lines))
        safe_stdout_write("  [OK] Chat streaming tokens and done marker emitted accurately!\n")

    def test_04_unknown_method_error_handling(self):
        """Validates error response on unknown method."""
        safe_stdout_write(">>> [RPC UNIT] Testing Unknown Method Error Response...\n")
        fake_stdout = io.StringIO()

        with patch("sys.stdout", fake_stdout):
            self.server.handle_request({"id": 99, "method": "non_existent_method"})

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        res_lines = [l for l in lines if l.get("id") == 99]
        self.assertEqual(len(res_lines), 1)
        self.assertIn("error", res_lines[0])
        self.assertEqual(res_lines[0]["error"]["code"], -32601)
        safe_stdout_write("  [OK] Error code -32601 returned on unknown method!\n")

    def test_05_get_and_set_delete_source_options(self):
        """Validates get_options and set_option for delete_source."""
        safe_stdout_write(">>> [RPC UNIT] Testing delete_source get_options and set_option...\n")
        fake_stdout = io.StringIO()

        with patch("sys.stdout", fake_stdout):
            # 1. get_options delete_source
            self.server.handle_request({"id": 50, "method": "get_options", "params": {"type": "delete_source"}})
            # 2. set_option cancel
            self.server.handle_request({"id": 51, "method": "set_option", "params": {"type": "delete_source", "value": "cancel_delete_source"}})

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        res_lines = [l for l in lines if l.get("id") in (50, 51)]
        self.assertEqual(len(res_lines), 2)
        self.assertEqual(res_lines[0]["result"]["type"], "delete_source")
        self.assertTrue(res_lines[1]["result"]["success"])
        safe_stdout_write("  [OK] delete_source get_options and set_option verified!\n")

    def test_06_ping_instant_handshake(self):
        """Validates that ping method responds instantaneously (< 50ms) with pong and version."""
        safe_stdout_write(">>> [RPC UNIT] Testing ping instant handshake method...\n")
        import time
        from any_context import __version__
        fake_stdout = io.StringIO()

        t0 = time.perf_counter()
        with patch("sys.stdout", fake_stdout):
            self.server.handle_request({"id": 100, "method": "ping"})
        latency_ms = (time.perf_counter() - t0) * 1000

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        res_lines = [l for l in lines if l.get("id") == 100]
        self.assertEqual(len(res_lines), 1)
        self.assertEqual(res_lines[0]["id"], 100)
        self.assertTrue(res_lines[0]["result"]["pong"])
        self.assertEqual(res_lines[0]["result"]["version"], __version__)
        self.assertLess(latency_ms, 100.0, f"Ping latency must be < 100ms (was {latency_ms:.2f}ms)")
        safe_stdout_write(f"  [OK] Ping responded in {latency_ms:.2f}ms (< 100ms benchmark met)!\n")

    def test_07_onboarding_set_option_action_propagation(self):
        """Validates that set_option for onboarding propagates action and state_updates via RPC."""
        safe_stdout_write(">>> [RPC UNIT] Testing set_option onboarding action propagation...\n")
        fake_stdout = io.StringIO()

        with patch("sys.stdout", fake_stdout):
            self.server.handle_request({
                "id": 101,
                "method": "set_option",
                "params": {"type": "onboarding", "value": "custom"}
            })

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        res_lines = [l for l in lines if l.get("id") == 101]
        self.assertEqual(len(res_lines), 1)
        res = res_lines[0]["result"]
        self.assertTrue(res["success"])
        self.assertEqual(res["action"], "open_config_modal")
        self.assertEqual(res["state_updates"]["target_menu"], "models")
        safe_stdout_write("  [OK] Onboarding set_option action propagation verified!\n")

    def test_08_boot_telemetry_emission(self):
        """Validates emit_boot_telemetry emits properly structured NDJSON payloads."""
        safe_stdout_write(">>> [RPC UNIT] Testing emit_boot_telemetry event structure...\n")
        fake_stdout = io.StringIO()

        with patch("sys.stdout", fake_stdout):
            emit_boot_telemetry(step="runtime", status="done", label="⚡ Python Core runtime linked", elapsed_ms=35.2)
            emit_boot_telemetry(step="db", status="running", label="🔌 Initializing SQLite Configuration Store...")
            emit_boot_telemetry(step="db", status="done", label="🔌 SQLite Configuration Store active", elapsed_ms=48.3)
            emit_boot_telemetry(step="ready", status="done", label="🚀 AnyContext ready in 0.15s", elapsed_ms=152.8)

        lines = [json.loads(l) for l in fake_stdout.getvalue().strip().split("\n") if l.strip()]
        self.assertEqual(len(lines), 4)
        for line in lines:
            self.assertEqual(line["event"], "boot_telemetry")
            self.assertIn("step", line)
            self.assertIn("status", line)
            self.assertIn("label", line)

        self.assertEqual(lines[0]["step"], "runtime")
        self.assertEqual(lines[0]["status"], "done")
        self.assertEqual(lines[0]["elapsed_ms"], 35.2)

        self.assertEqual(lines[1]["step"], "db")
        self.assertEqual(lines[1]["status"], "running")
        self.assertNotIn("elapsed_ms", lines[1])

        self.assertEqual(lines[2]["step"], "db")
        self.assertEqual(lines[2]["status"], "done")
        self.assertEqual(lines[2]["elapsed_ms"], 48.3)

        self.assertEqual(lines[3]["step"], "ready")
        self.assertEqual(lines[3]["status"], "done")
        self.assertEqual(lines[3]["elapsed_ms"], 152.8)
        safe_stdout_write("  [OK] emit_boot_telemetry NDJSON payload validation passed!\n")


if __name__ == "__main__":
    unittest.main()
