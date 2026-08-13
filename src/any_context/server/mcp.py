import sys
import json
import uuid
from typing import Dict, Any

from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.core.agent import cli_agent
from any_context.tools.search_tools import search_db
from any_context.memory import MemoryManager

def _send_json_response(response_dict: Dict[str, Any]):
    sys.stdout.write(json.dumps(response_dict) + "\n")
    sys.stdout.flush()

def start_mcp_server():
    """
    Launches AnyContext Model Context Protocol (MCP) Server over stdio JSON-RPC 2.0.
    """
    tools_definitions = [
        {
            "name": "search_workspace_docs",
            "description": "Searches ChromaDB vector database for documents relevant to the query in a specific workspace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query string"},
                    "workspace": {"type": "string", "description": "Target workspace name (optional)"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "query_anycontext_agent",
            "description": "Sends a prompt/question to AnyContext AI Agent with automatic RAG search and 3-level session memory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "User instruction or prompt"},
                    "workspace": {"type": "string", "description": "Target workspace name (optional)"}
                },
                "required": ["message"]
            }
        },
        {
            "name": "list_workspaces",
            "description": "Lists all workspaces and folder paths currently configured in AnyContext SQLite database.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "reset_workspace_memory",
            "description": "Resets long-term vector session memories for a specific workspace or globally.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace": {"type": "string", "description": "Target workspace name to reset memory for (optional)"}
                }
            }
        },
        {
            "name": "factory_reset_anycontext",
            "description": "Wipes all configured workspaces, folders, API keys, settings, users, access tokens, and vector databases, completely resetting AnyContext to factory defaults.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "get_anycontext_system_documentation",
            "description": "Retrieves the complete official AnyContext system documentation (README.md) including REST API endpoints, MCP setup, VPC deployment guide, slash commands, and architecture.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "setup_admin_user",
            "description": "Configures initial Administrator account for AnyContext server security.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Administrator full name"},
                    "email": {"type": "string", "description": "Administrator email address"},
                    "password": {"type": "string", "description": "Administrator password"}
                },
                "required": ["name", "email", "password"]
            }
        },
        {
            "name": "authenticate_user",
            "description": "Authenticates user credentials and returns active Bearer Access Token.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "email": {"type": "string", "description": "User email address"},
                    "password": {"type": "string", "description": "User password"}
                },
                "required": ["email", "password"]
            }
        },
        {
            "name": "create_user",
            "description": "Creates a new team user account with specific role and workspace permissions (Admin operation).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "User full name"},
                    "email": {"type": "string", "description": "User email address"},
                    "password": {"type": "string", "description": "User password"},
                    "role": {"type": "string", "description": "Role level: 'admin', 'analyst', or 'viewer'"},
                    "allowed_workspaces": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Allowed workspace names"
                    }
                },
                "required": ["name", "email", "password"]
            }
        },
        {
            "name": "list_users",
            "description": "Lists all configured team users in AnyContext SQLite database.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "create_access_token",
            "description": "Generates a new Bearer Security Access Token with role and workspace scope permissions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Token descriptive name (e.g. 'Dev Team', 'HR Bot')"},
                    "role": {"type": "string", "description": "Role level: 'admin', 'analyst', or 'viewer'"},
                    "allowed_workspaces": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Allowed workspace names or ['*'] for all workspaces"
                    }
                },
                "required": ["name"]
            }
        },
        {
            "name": "list_access_tokens",
            "description": "Lists all active Bearer Security Access Tokens stored in AnyContext SQLite database.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "revoke_access_token",
            "description": "Revokes/deletes a security access token by token ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "token_id": {"type": "string", "description": "Security token ID to revoke (e.g. 'actx_sec_...')"}
                },
                "required": ["token_id"]
            }
        }
    ]

    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break
            
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except Exception:
                continue

            req_id = request.get("id")
            method = request.get("method")
            params = request.get("params", {})

            if method == "initialize":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "AnyContext MCP Server",
                            "version": __version__
                        }
                    }
                }
                _send_json_response(response)

            elif method == "tools/list":
                response = {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": tools_definitions
                    }
                }
                _send_json_response(response)

            elif method == "tools/call":
                tool_name = params.get("name")
                arguments = params.get("arguments", {})

                try:
                    if tool_name == "search_workspace_docs":
                        query = arguments.get("query", "")
                        ws = arguments.get("workspace")
                        res = search_db.invoke({"query": query, "workspace": ws, "search_session_memory": False})
                        result_text = str(res)

                    elif tool_name == "query_anycontext_agent":
                        msg = arguments.get("message", "")
                        ws = arguments.get("workspace")
                        thread_id = f"mcp_chat_{uuid.uuid4()}"
                        config = {"configurable": {"thread_id": thread_id, "active_workspace": ws}}
                        
                        full_response = ""
                        for token, metadata in cli_agent.stream({"messages": [msg]}, stream_mode="messages", config=config):
                            if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                                if isinstance(token.content, str) and token.content:
                                    full_response += token.content
                        result_text = full_response.strip()

                    elif tool_name == "list_workspaces":
                        store = ConfigDBStore()
                        settings = store.get_app_settings()
                        workspaces = [{"name": w.name, "paths": w.paths} for w in settings.workspaces] if settings else []
                        result_text = json.dumps(workspaces, indent=2)

                    elif tool_name == "reset_workspace_memory":
                        ws = arguments.get("workspace")
                        memory_mgr = MemoryManager()
                        deleted = memory_mgr.reset_memory(workspace=ws)
                        result_text = f"Reset complete! Deleted {deleted} long-term memory entries."

                    elif tool_name == "factory_reset_anycontext":
                        store = ConfigDBStore()
                        store.factory_reset()
                        result_text = "Factory reset complete! All settings, workspaces, API keys, users, tokens, and vector databases have been wiped."

                    elif tool_name == "get_anycontext_system_documentation":
                        import os
                        readme_candidates = [
                            os.path.join(os.getcwd(), "README.md"),
                            os.path.join(os.path.dirname(__file__), "..", "config", "README.md"),
                            os.path.join(os.path.dirname(__file__), "..", "..", "..", "README.md")
                        ]
                        content = None
                        for cand in readme_candidates:
                            if os.path.exists(cand):
                                try:
                                    with open(cand, "r", encoding="utf-8") as f:
                                        content = f.read()
                                    break
                                except Exception:
                                    pass
                        result_text = content or "Error: Application documentation (README.md) not found."

                    elif tool_name == "setup_admin_user":
                        store = ConfigDBStore()
                        admin_info = store.setup_admin_user(
                            name=arguments.get("name", ""),
                            email=arguments.get("email", ""),
                            password=arguments.get("password", "")
                        )
                        result_text = json.dumps(admin_info, indent=2)

                    elif tool_name == "authenticate_user":
                        store = ConfigDBStore()
                        user_info = store.authenticate_user(
                            email=arguments.get("email", ""),
                            password=arguments.get("password", "")
                        )
                        if user_info:
                            result_text = json.dumps(user_info, indent=2)
                        else:
                            result_text = "Error: Invalid email or password."

                    elif tool_name == "create_user":
                        store = ConfigDBStore()
                        new_u = store.create_user(
                            name=arguments.get("name", ""),
                            email=arguments.get("email", ""),
                            password=arguments.get("password", ""),
                            role=arguments.get("role", "analyst"),
                            allowed_workspaces=arguments.get("allowed_workspaces", ["Default"])
                        )
                        result_text = json.dumps(new_u, indent=2)

                    elif tool_name == "list_users":
                        store = ConfigDBStore()
                        users = store.list_users()
                        result_text = json.dumps(users, indent=2)

                    elif tool_name == "create_access_token":
                        name = arguments.get("name", "Unnamed Token")
                        role = arguments.get("role", "viewer")
                        allowed_ws = arguments.get("allowed_workspaces", ["*"])
                        store = ConfigDBStore()
                        new_t = store.create_access_token(name=name, role=role, allowed_workspaces=allowed_ws)
                        result_text = json.dumps(new_t, indent=2)

                    elif tool_name == "list_access_tokens":
                        store = ConfigDBStore()
                        tokens = store.get_access_tokens()
                        result_text = json.dumps(tokens, indent=2)

                    elif tool_name == "revoke_access_token":
                        t_id = arguments.get("token_id", "")
                        store = ConfigDBStore()
                        deleted = store.delete_access_token(t_id)
                        if deleted:
                            result_text = f"Successfully revoked security token '{t_id}'."
                        else:
                            result_text = f"Error: Security token '{t_id}' not found."

                    else:
                        result_text = f"Error: Tool '{tool_name}' not found."

                    response = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": result_text
                                }
                            ]
                        }
                    }
                    _send_json_response(response)

                except Exception as e:
                    error_response = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32603,
                            "message": str(e)
                        }
                    }
                    _send_json_response(error_response)

        except Exception:
            break
