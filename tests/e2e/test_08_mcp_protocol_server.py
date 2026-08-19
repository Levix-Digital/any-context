import os
import json
import unittest
from any_context.server.mcp import dispatch_mcp_request, MCP_TOOLS_DEFINITIONS
from any_context.config.db_store import ConfigDBStore
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class Test08MCPProtocolServer(unittest.TestCase):
    """
    E2E Test Suite 08: Model Context Protocol (MCP) Server JSON-RPC 2.0 (actx --mcp), Tools List & Execution
    """

    @classmethod
    def setUpClass(cls):
        setup_mock_embeddings_if_needed()
        cls.store = ConfigDBStore()
        cls.ws = "E2E_Mod8_MCP"
        cls.store.add_workspace(cls.ws, [])

    @classmethod
    def tearDownClass(cls):
        try:
            cls.store.remove_workspace(cls.ws)
        except Exception:
            pass

    def test_01_mcp_initialize_handshake(self):
        """TC-8.1: Tests JSON-RPC initialize handshake returning server info and protocol version."""
        safe_stdout_write("\n>>> [MOD 8 / TC-8.1] Testing MCP Initialize Handshake...\n")
        req = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            }
        }
        res = dispatch_mcp_request(req)
        self.assertEqual(res["jsonrpc"], "2.0")
        self.assertEqual(res["id"], 1)
        self.assertIn("result", res)
        self.assertEqual(res["result"]["serverInfo"]["name"], "AnyContext MCP Server")
        safe_stdout_write("  [OK] MCP Initialize Handshake verified!\n")

    def test_02_mcp_tools_list(self):
        """TC-8.2: Tests tools/list returning all registered MCP tools."""
        safe_stdout_write(">>> [MOD 8 / TC-8.2] Testing MCP Tools List...\n")
        req = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        res = dispatch_mcp_request(req)
        self.assertEqual(res["id"], 2)
        tools = res["result"]["tools"]
        tool_names = [t["name"] for t in tools]
        self.assertIn("search_workspace_docs", tool_names)
        self.assertIn("query_anycontext_agent", tool_names)
        self.assertIn("list_workspaces", tool_names)
        self.assertIn("create_access_token", tool_names)
        self.assertIn("get_subscription_status", tool_names)
        safe_stdout_write("  [OK] MCP Tools List (20+ tools) verified!\n")

    def test_03_mcp_tools_call_execution(self):
        """TC-8.3: Tests tools/call invoking list_workspaces and get_subscription_status."""
        safe_stdout_write(">>> [MOD 8 / TC-8.3] Testing MCP Tools Call Execution...\n")
        # 1. Call list_workspaces
        req_ws = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "list_workspaces",
                "arguments": {}
            }
        }
        res_ws = dispatch_mcp_request(req_ws)
        self.assertIn("result", res_ws)
        content_ws = res_ws["result"]["content"][0]["text"]
        self.assertIn(self.ws, content_ws)

        # 2. Call get_subscription_status
        req_sub = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_subscription_status",
                "arguments": {}
            }
        }
        res_sub = dispatch_mcp_request(req_sub)
        self.assertIn("result", res_sub)
        content_sub = res_sub["result"]["content"][0]["text"]
        self.assertIn("active_tier", content_sub)
        safe_stdout_write("  [OK] MCP Tool execution verified!\n")

if __name__ == "__main__":
    unittest.main()
