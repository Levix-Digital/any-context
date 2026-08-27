"""
Unit tests for Stdio RPC Bridge interaction endpoints.
"""

import unittest
from unittest.mock import patch
from any_context.server.rpc_bridge import StdioRPCServer


class TestRPCBridgeInteraction(unittest.TestCase):

    def test_rpc_bridge_interaction_methods(self):
        sent = []
        def fake_send(data):
            sent.append(data)

        with patch("any_context.server.rpc_bridge._send_ndjson", fake_send):
            server = StdioRPCServer(default_workspace="Default")

            # 1. get_menu_tree
            server.handle_request({
                "id": "1",
                "method": "get_menu_tree",
                "params": {"menu_id": "main", "workspace": "Default"}
            })
            self.assertEqual(len(sent), 1)
            self.assertEqual(sent[0]["id"], "1")
            self.assertEqual(sent[0]["result"]["menu_id"], "main")
            self.assertEqual(len(sent[0]["result"]["items"]), 11)

            # 2. get_options
            server.handle_request({
                "id": "2",
                "method": "get_options",
                "params": {"type": "grounding_mode", "workspace": "Default"}
            })
            self.assertEqual(len(sent), 2)
            self.assertEqual(sent[1]["id"], "2")
            self.assertEqual(sent[1]["result"]["type"], "grounding_mode")

            # 3. set_option
            server.handle_request({
                "id": "3",
                "method": "set_option",
                "params": {"type": "grounding_mode", "value": "strict", "workspace": "Default"}
            })
            self.assertEqual(len(sent), 3)
            self.assertEqual(sent[2]["id"], "3")
            self.assertTrue(sent[2]["result"]["success"])

            # 4. execute_menu_action
            server.handle_request({
                "id": "4",
                "method": "execute_menu_action",
                "params": {"action_id": "set_grounding_hybrid", "workspace": "Default"}
            })
            self.assertEqual(len(sent), 4)
            self.assertEqual(sent[3]["id"], "4")
            self.assertTrue(sent[3]["result"]["success"])


if __name__ == "__main__":
    unittest.main()
