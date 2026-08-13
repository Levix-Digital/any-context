import sys
import json
import uuid
from typing import Dict, Any
from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.tools.search_tools import search_db
from any_context.core.agent import cli_agent
from any_context.memory import MemoryManager

def start_mcp_server():
    """
    Model Context Protocol (MCP) Server over Stdio JSON-RPC 2.0.
    Enables native integration with Claude Desktop, Cursor IDE, Antigravity, and external AI sidecars.
    """
    sys.stderr.write(f"🚀 AnyContext MCP Server v{__version__} starting on stdio...\n")
    sys.stderr.flush()

    tools_definitions = [
        {
            "name": "search_workspace_docs",
            "description": "Searches the vector database knowledge base for documents matching the query across configured workspaces.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "The search query string"},
                    "workspace": {"type": "string", "description": "Target workspace name (optional)"}
                },
                "required": ["query"]
            }
        },
        {
            "name": "query_anycontext_agent",
            "description": "Executes a query against the full AnyContext RAG agent with isolated workspace context and 3-level long-term memory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "User instruction or query for the AI agent"},
                    "workspace": {"type": "string", "description": "Active workspace name (optional)"}
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
                except Exception as e:
                    response = {
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32603,
                            "message": f"Internal tool execution error: {str(e)}"
                        }
                    }
                _send_json_response(response)

        except KeyboardInterrupt:
            break

def _send_json_response(response: Dict[str, Any]):
    sys.stdout.write(json.dumps(response) + "\n")
    sys.stdout.flush()
