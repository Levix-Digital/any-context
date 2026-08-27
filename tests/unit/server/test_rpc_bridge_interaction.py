"""
Unit tests for Stdio RPC Bridge interaction endpoints.
"""

import json
from any_context.server.rpc_bridge import StdioRPCServer


def test_rpc_bridge_interaction_methods(monkeypatch):
    sent = []
    def fake_send(data):
        sent.append(data)

    monkeypatch.setattr("any_context.server.rpc_bridge._send_ndjson", fake_send)

    server = StdioRPCServer(default_workspace="Default")

    # 1. get_menu_tree
    server.handle_request({
        "id": "1",
        "method": "get_menu_tree",
        "params": {"menu_id": "main", "workspace": "Default"}
    })
    assert len(sent) == 1
    assert sent[0]["id"] == "1"
    assert sent[0]["result"]["menu_id"] == "main"
    assert len(sent[0]["result"]["items"]) == 11

    # 2. get_options
    server.handle_request({
        "id": "2",
        "method": "get_options",
        "params": {"type": "grounding_mode", "workspace": "Default"}
    })
    assert len(sent) == 2
    assert sent[1]["id"] == "2"
    assert sent[1]["result"]["type"] == "grounding_mode"

    # 3. set_option
    server.handle_request({
        "id": "3",
        "method": "set_option",
        "params": {"type": "grounding_mode", "value": "strict", "workspace": "Default"}
    })
    assert len(sent) == 3
    assert sent[2]["id"] == "3"
    assert sent[2]["result"]["success"] is True

    # 4. execute_menu_action
    server.handle_request({
        "id": "4",
        "method": "execute_menu_action",
        "params": {"action_id": "set_grounding_hybrid", "workspace": "Default"}
    })
    assert len(sent) == 4
    assert sent[3]["id"] == "4"
    assert sent[3]["result"]["success"] is True
