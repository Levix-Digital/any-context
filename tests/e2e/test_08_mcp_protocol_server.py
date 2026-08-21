import os
import sys
import json
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

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
        self.assertIn("get_workspace_sources", tool_names)
        self.assertIn("create_access_token", tool_names)
        self.assertIn("get_subscription_status", tool_names)
        safe_stdout_write("  [OK] MCP Tools List (20+ tools) verified!\n")

    def test_03_mcp_tools_call_execution(self):
        """TC-8.3: Tests tools/call invoking list_workspaces, get_workspace_sources and get_subscription_status."""
        safe_stdout_write(">>> [MOD 8 / TC-8.3] Testing MCP Tools Call Execution...\n")
        # Add web source to workspace
        from any_context.ingestion.web_scheduler import WebSchedulerStore
        web_store = WebSchedulerStore()
        web_store.add_or_update_root_web_source(
            workspace_name=self.ws,
            root_url="https://python.org",
            title="Python Portal",
            page_count=5
        )

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
        self.assertIn("https://python.org", content_ws)

        # 2. Call get_workspace_sources
        req_src = {
            "jsonrpc": "2.0",
            "id": 35,
            "method": "tools/call",
            "params": {
                "name": "get_workspace_sources",
                "arguments": {"workspace": self.ws}
            }
        }
        res_src = dispatch_mcp_request(req_src)
        self.assertIn("result", res_src)
        content_src = res_src["result"]["content"][0]["text"]
        self.assertIn(self.ws, content_src)
        self.assertIn("https://python.org", content_src)

        # 3. Call get_subscription_status
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

    def test_04_mcp_transfer_workspace_source_tool(self):
        """TC-8.4: Tests tools/call invoking transfer_workspace_source tool."""
        safe_stdout_write(">>> [MOD 8 / TC-8.4] Testing MCP transfer_workspace_source Tool Execution...\n")
        src_ws = "E2E_MCP_Transfer_Src"
        tgt_ws = "E2E_MCP_Transfer_Tgt"
        test_dir = os.path.abspath(os.path.join(os.getcwd(), "test_mcp_transfer_folder"))
        os.makedirs(test_dir, exist_ok=True)

        self.store.add_workspace(src_ws, paths=[test_dir])
        self.store.add_workspace(tgt_ws, paths=[])

        try:
            req_transfer = {
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "transfer_workspace_source",
                    "arguments": {
                        "source_workspace": src_ws,
                        "target_workspace": tgt_ws,
                        "source_type": "folder",
                        "source_path_or_url": test_dir
                    }
                }
            }
            res = dispatch_mcp_request(req_transfer)
            self.assertIn("result", res)
            content = json.loads(res["result"]["content"][0]["text"])
            self.assertTrue(content["success"])
            self.assertEqual(content["source_workspace"], src_ws)
            self.assertEqual(content["target_workspace"], tgt_ws)

            settings = self.store.get_app_settings()
            src_obj = next((w for w in settings.workspaces if w.name == src_ws), None)
            tgt_obj = next((w for w in settings.workspaces if w.name == tgt_ws), None)
            self.assertNotIn(test_dir, [os.path.abspath(p) for p in src_obj.paths])
            self.assertIn(test_dir, [os.path.abspath(p) for p in tgt_obj.paths])
            safe_stdout_write("  [OK] MCP transfer_workspace_source tool verified!\n")
        finally:
            self.store.remove_workspace(src_ws)
            self.store.remove_workspace(tgt_ws)
            try:
                os.rmdir(test_dir)
            except Exception:
                pass

    def test_05_mcp_context_retrieval_presets_tools(self):
        """TC-8.5: Tests tools/call invoking get_context_retrieval_settings and set_context_retrieval_preset."""
        safe_stdout_write(">>> [MOD 8 / TC-8.5] Testing MCP Context Retrieval Settings Tools Execution...\n")

        # 1. get_context_retrieval_settings
        req_get = {
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "get_context_retrieval_settings",
                "arguments": {}
            }
        }
        res_get = dispatch_mcp_request(req_get)
        self.assertIn("result", res_get)
        data_get = json.loads(res_get["result"]["content"][0]["text"])
        self.assertIn("retrieval_preset", data_get)
        self.assertIn("top_k", data_get)

        # 2. set_context_retrieval_preset (Deep Research)
        req_set = {
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "set_context_retrieval_preset",
                "arguments": {
                    "preset": "deep_research"
                }
            }
        }
        res_set = dispatch_mcp_request(req_set)
        self.assertIn("result", res_set)
        data_set = json.loads(res_set["result"]["content"][0]["text"])
        self.assertEqual(data_set["status"], "success")
        self.assertEqual(data_set["retrieval_preset"], "deep_research")
        self.assertEqual(data_set["top_k"], 60)
        self.assertEqual(data_set["candidate_pool_size"], 150)

        # 3. Reset back to balanced
        req_reset = {
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "set_context_retrieval_preset",
                "arguments": {
                    "preset": "balanced"
                }
            }
        }
        res_reset = dispatch_mcp_request(req_reset)
        self.assertIn("result", res_reset)
        data_reset = json.loads(res_reset["result"]["content"][0]["text"])
        self.assertEqual(data_reset["retrieval_preset"], "balanced")
        self.assertEqual(data_reset["top_k"], 40)
        safe_stdout_write("  [OK] MCP context retrieval settings tools verified!\n")

    def test_06_mcp_rename_workspace_tool(self):
        """TC-8.6: Tests tools/call invoking rename_workspace tool."""
        safe_stdout_write(">>> [MOD 8 / TC-8.6] Testing MCP rename_workspace Tool Execution...\n")
        mcp_src = "mcp_rename_src"
        mcp_tgt = "mcp_rename_tgt"

        self.store.add_workspace(mcp_src, paths=[])

        try:
            req = {
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "name": "rename_workspace",
                    "arguments": {
                        "old_name": mcp_src,
                        "new_name": mcp_tgt
                    }
                }
            }
            res = dispatch_mcp_request(req)
            self.assertIn("result", res)
            self.assertFalse(res["result"].get("isError", False))
            data = json.loads(res["result"]["content"][0]["text"])
            self.assertTrue(data.get("success", False))
            self.assertEqual(data["old_workspace"], mcp_src)
            self.assertEqual(data["new_workspace"], mcp_tgt)

            # Verify in DB
            settings = self.store.get_app_settings()
            ws_names = [w.name for w in settings.workspaces]
            self.assertNotIn(mcp_src, ws_names)
            self.assertIn(mcp_tgt, ws_names)
            safe_stdout_write("  [OK] MCP rename_workspace tool verified!\n")
        finally:
            self.store.remove_workspace(mcp_src)
            self.store.remove_workspace(mcp_tgt)

    def test_07_mcp_grounding_mode_tools(self):
        """TC-8.7: Tests tools/call invoking get_grounding_mode and set_grounding_mode MCP tools."""
        safe_stdout_write(">>> [MOD 8 / TC-8.7] Testing MCP Grounding Mode Tools Execution...\n")

        # 1. get_grounding_mode
        req_get = {
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "get_grounding_mode",
                "arguments": {}
            }
        }
        res_get = dispatch_mcp_request(req_get)
        self.assertIn("result", res_get)
        data_get = json.loads(res_get["result"]["content"][0]["text"])
        self.assertIn("grounding_mode", data_get)
        self.assertIn("available_modes", data_get)

        # 2. set_grounding_mode (Strict)
        req_set = {
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "set_grounding_mode",
                "arguments": {
                    "mode": "strict"
                }
            }
        }
        res_set = dispatch_mcp_request(req_set)
        self.assertIn("result", res_set)
        data_set = json.loads(res_set["result"]["content"][0]["text"])
        self.assertEqual(data_set["status"], "success")
        self.assertEqual(data_set["grounding_mode"], "strict")
        self.assertEqual(self.store.get_grounding_mode(), "strict")

        # 3. set_grounding_mode (Reset to Hybrid)
        req_reset = {
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "set_grounding_mode",
                "arguments": {
                    "mode": "hybrid"
                }
            }
        }
        res_reset = dispatch_mcp_request(req_reset)
        self.assertIn("result", res_reset)
        data_reset = json.loads(res_reset["result"]["content"][0]["text"])
        self.assertEqual(data_reset["grounding_mode"], "hybrid")
        self.assertEqual(self.store.get_grounding_mode(), "hybrid")
        safe_stdout_write("  [OK] MCP get_grounding_mode and set_grounding_mode tools verified!\n")

    def test_08_mcp_shared_sources_tools(self):
        """TC-8.8: Tests list_available_shared_sources, link_shared_source_to_workspace, unlink_shared_source_from_workspace tools."""
        safe_stdout_write(">>> [MOD 8 / TC-8.8] Testing MCP Shared Sources Tools Execution...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_target = "mcp_shared_target"
        test_folder = os.path.abspath(os.path.join(temp_d, "mcp_shared_folder"))
        os.makedirs(test_folder, exist_ok=True)

        try:
            self.store.add_folder_to_workspace("Shared Sources", test_folder)
            self.store.add_workspace(ws_target, paths=[])

            # 1. list_available_shared_sources
            req_list = {
                "jsonrpc": "2.0",
                "id": 13,
                "method": "tools/call",
                "params": {
                    "name": "list_available_shared_sources",
                    "arguments": {}
                }
            }
            res_list = dispatch_mcp_request(req_list)
            self.assertIn("result", res_list)
            data_list = json.loads(res_list["result"]["content"][0]["text"])
            self.assertIn("sources", data_list)

            # 2. link_shared_source_to_workspace
            req_link = {
                "jsonrpc": "2.0",
                "id": 14,
                "method": "tools/call",
                "params": {
                    "name": "link_shared_source_to_workspace",
                    "arguments": {
                        "workspace_name": ws_target,
                        "source_type": "folder",
                        "source_identifier": test_folder,
                        "title": "MCP Shared Framework"
                    }
                }
            }
            res_link = dispatch_mcp_request(req_link)
            self.assertIn("result", res_link)
            data_link = json.loads(res_link["result"]["content"][0]["text"])
            self.assertEqual(data_link["status"], "success")

            # Verify in DB
            sources = self.store.get_workspace_sources(ws_target)
            self.assertEqual(sources["total_sources"], 1)
            self.assertTrue(sources["sources"][0]["details"].get("is_shared_link"))

            # 3. unlink_shared_source_from_workspace
            req_unlink = {
                "jsonrpc": "2.0",
                "id": 15,
                "method": "tools/call",
                "params": {
                    "name": "unlink_shared_source_from_workspace",
                    "arguments": {
                        "workspace_name": ws_target,
                        "source_type": "folder",
                        "source_identifier": test_folder
                    }
                }
            }
            res_unlink = dispatch_mcp_request(req_unlink)
            self.assertIn("result", res_unlink)
            data_unlink = json.loads(res_unlink["result"]["content"][0]["text"])
            self.assertEqual(data_unlink["status"], "success")

        finally:
            self.store.remove_folder_from_workspace("Shared Sources", test_folder)
            self.store.remove_workspace(ws_target)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

    def test_09_mcp_broadcast_source_linking_tool(self):
        """TC-8.9: Tests add_workspace_folder MCP tool with link_to_workspaces broadcast."""
        safe_stdout_write(">>> [MOD 8 / TC-8.9] Testing MCP add_workspace_folder Broadcast Tool Execution...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_prim = "mcp_broadcast_prim"
        ws_sub = "mcp_broadcast_sub"
        test_folder = os.path.abspath(os.path.join(temp_d, "mcp_broadcast_folder"))
        os.makedirs(test_folder, exist_ok=True)

        try:
            self.store.add_workspace(ws_prim, paths=[])
            self.store.add_workspace(ws_sub, paths=[])

            req = {
                "jsonrpc": "2.0",
                "id": 16,
                "method": "tools/call",
                "params": {
                    "name": "add_workspace_folder",
                    "arguments": {
                        "workspace": ws_prim,
                        "folder_path": test_folder,
                        "link_to_workspaces": [ws_sub]
                    }
                }
            }
            res = dispatch_mcp_request(req)
            self.assertIn("result", res)
            data = json.loads(res["result"]["content"][0]["text"])
            self.assertEqual(data["status"], "success")
            self.assertEqual(data["total_linked"], 1)

            # Verify in DB
            sources_sub = self.store.get_workspace_sources(ws_sub)
            self.assertEqual(sources_sub["total_sources"], 1)
            self.assertTrue(sources_sub["sources"][0]["details"].get("is_shared_link"))

            safe_stdout_write("  [OK] MCP add_workspace_folder broadcast tool verified!\n")
        finally:
            self.store.remove_workspace(ws_prim)
            self.store.remove_workspace(ws_sub)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

    def test_10_mcp_sync_tools(self):
        """TC-8.10: Tests check_workspace_sync_status and sync_workspace_folders MCP tools."""
        safe_stdout_write(">>> [MOD 8 / TC-8.10] Testing MCP Workspace Sync Tools Execution...\n")
        import tempfile
        import shutil
        temp_d = tempfile.mkdtemp()
        ws_test = "mcp_sync_test"
        test_folder = os.path.abspath(os.path.join(temp_d, "mcp_sync_folder"))
        os.makedirs(test_folder, exist_ok=True)
        doc_f = os.path.join(test_folder, "mcp_doc.md")
        with open(doc_f, "w", encoding="utf-8") as f:
            f.write("# MCP Sync Test Document\nTesting MCP folder sync tools.")

        try:
            self.store.add_workspace(ws_test, paths=[test_folder])

            # 1. check_workspace_sync_status tool
            req_status = {
                "jsonrpc": "2.0",
                "id": 17,
                "method": "tools/call",
                "params": {
                    "name": "check_workspace_sync_status",
                    "arguments": {
                        "workspace": ws_test
                    }
                }
            }
            res_status = dispatch_mcp_request(req_status)
            self.assertIn("result", res_status)
            data_status = json.loads(res_status["result"]["content"][0]["text"])
            self.assertEqual(data_status["workspace_name"], ws_test)
            self.assertIn("is_up_to_date", data_status)

            # 2. sync_workspace_folders tool
            req_sync = {
                "jsonrpc": "2.0",
                "id": 18,
                "method": "tools/call",
                "params": {
                    "name": "sync_workspace_folders",
                    "arguments": {
                        "workspace": ws_test,
                        "force_full": False
                    }
                }
            }
            res_sync = dispatch_mcp_request(req_sync)
            self.assertIn("result", res_sync)
            data_sync = json.loads(res_sync["result"]["content"][0]["text"])
            self.assertIn(data_sync.get("status"), ["completed", "updated", "up_to_date"])

            safe_stdout_write("  [OK] MCP sync tools execution verified!\n")
        finally:
            self.store.remove_workspace(ws_test)
            try:
                shutil.rmtree(temp_d, ignore_errors=True)
            except Exception:
                pass

if __name__ == "__main__":
    unittest.main()


