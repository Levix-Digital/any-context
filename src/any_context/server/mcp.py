import sys
import json
import uuid
from typing import Dict, Any, List, Optional

from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.core.agent import cli_agent
from any_context.tools.search_tools import search_db
from any_context.memory import MemoryManager

def _send_json_response(response_dict: Dict[str, Any]):
    sys.stdout.write(json.dumps(response_dict) + "\n")
    sys.stdout.flush()

MCP_TOOLS_DEFINITIONS = [
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
        "description": "Sends a prompt/question to AnyContext AI Agent with automatic RAG search, 3-level session memory, and optional on-the-fly model switching.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "message": {"type": "string", "description": "User instruction or prompt"},
                "workspace": {"type": "string", "description": "Target workspace name (optional)"},
                "model": {"type": "string", "description": "Optional inference model override on-the-fly (e.g. 'gpt-4o', 'claude-3-5-sonnet-20241022', 'deepseek-chat')"}
            },
            "required": ["message"]
        }
    },
    {
        "name": "list_available_models",
        "description": "Lists all available AI inference models verified against configured API keys, plus the current active default.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "list_workspaces",
        "description": "Lists all configured workspaces and their associated disk directory paths.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "reset_workspace_memory",
        "description": "Resets/purges long-term session memory summaries from ChromaDB for a specific workspace or globally.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace": {"type": "string", "description": "Workspace name to reset (e.g. 'Legal'). If omitted, resets globally across all workspaces."}
            },
            "required": []
        }
    },
    {
        "name": "factory_reset_anycontext",
        "description": "DANGER: Completely wipes all SQLite settings.db tables and deletes all local vector database directories ('./context_db' and './memory').",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "get_anycontext_system_documentation",
        "description": "Retrieves the complete AnyContext technical documentation, architecture guide, and operational instructions.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "setup_admin_user",
        "description": "Initial Admin setup wizard for first-time server security deployment.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Administrator full name"},
                "email": {"type": "string", "description": "Administrator email address"},
                "password": {"type": "string", "description": "Administrator secure password"}
            },
            "required": ["name", "email", "password"]
        }
    },
    {
        "name": "authenticate_user",
        "description": "Authenticates a user with email and password, returning user profile details and roles.",
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
        "description": "Creates a new user account with assigned role and allowed workspaces.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "User full name"},
                "email": {"type": "string", "description": "User email address"},
                "password": {"type": "string", "description": "User password"},
                "role": {"type": "string", "enum": ["admin", "analyst", "viewer"], "description": "User role (admin, analyst, viewer)"},
                "allowed_workspaces": {"type": "array", "items": {"type": "string"}, "description": "List of allowed workspace names or ['*'] for all"}
            },
            "required": ["name", "email", "password", "role"]
        }
    },
    {
        "name": "list_users",
        "description": "Lists all configured user accounts and their assigned roles.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "create_access_token",
        "description": "Generates a new Bearer Access Token (actx_sec_...) with assigned role and workspace scope restrictions.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Token descriptive name (e.g. 'VS Code Extension - Amanda')"},
                "role": {"type": "string", "enum": ["admin", "analyst", "viewer"], "description": "Assigned permission role (default: viewer)"},
                "allowed_workspaces": {"type": "array", "items": {"type": "string"}, "description": "Allowed workspace names or ['*'] for all"}
            },
            "required": ["name"]
        }
    },
    {
        "name": "list_access_tokens",
        "description": "Lists all active Bearer Access Tokens and their security scopes.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "revoke_access_token",
        "description": "Revokes/deletes a Bearer Access Token by its ID.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "token_id": {"type": "string", "description": "Token ID to revoke (e.g. 'actx_sec_...')"}
            },
            "required": ["token_id"]
        }
    },
    {
        "name": "create_workspace_share_invite",
        "description": "Generates a workspace share invite code (SHARE-WKS-XXXX) for team collaboration.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace_name": {"type": "string", "description": "Workspace name to share"},
                "access_level": {"type": "string", "enum": ["editor", "viewer"], "description": "Collaborator access level ('editor' or 'viewer')"},
                "max_uses": {"type": "integer", "description": "Maximum number of times this invite code can be used (default: 1)"}
            },
            "required": ["workspace_name"]
        }
    },
    {
        "name": "accept_workspace_share_invite",
        "description": "Accepts a workspace share invite code (SHARE-WKS-XXXX) to join a collaborative workspace.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "invite_code": {"type": "string", "description": "Workspace share invite code"},
                "user_email": {"type": "string", "description": "Collaborator user email address"}
            },
            "required": ["invite_code", "user_email"]
        }
    },
    {
        "name": "get_subscription_status",
        "description": "Retrieves the active AnyContext subscription tier (Community, Pro, Team, Enterprise) and capability limits.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "list_subscription_plans",
        "description": "Lists all available AnyContext subscription tiers and their full feature matrix.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "add_workspace_web_url",
        "description": "Adds and indexes a website or documentation URL into a workspace knowledge base.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace": {"type": "string", "description": "Target workspace name"},
                "url": {"type": "string", "description": "Website URL to scrape and embed"},
                "polling_interval_hours": {"type": "integer", "description": "Automated re-crawl polling interval in hours (default: 24)"}
            },
            "required": ["workspace", "url"]
        }
    },
    {
        "name": "list_workspace_web_urls",
        "description": "Lists all registered web URLs and their polling schedules for a specific workspace.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace": {"type": "string", "description": "Target workspace name"}
            },
            "required": ["workspace"]
        }
    },
    {
        "name": "remove_workspace_web_url",
        "description": "Removes a web URL from a workspace and purges its vector embeddings from ChromaDB.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace": {"type": "string", "description": "Target workspace name"},
                "url_or_id": {"type": "string", "description": "Target URL string or web_ ID to remove"}
            },
            "required": ["workspace", "url_or_id"]
        }
    },
    {
        "name": "sync_workspace_web_urls",
        "description": "Forces re-scraping and synchronization for all registered web URLs in a workspace.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace": {"type": "string", "description": "Target workspace name"}
            },
            "required": ["workspace"]
        }
    },
    {
        "name": "transfer_workspace_source",
        "description": "Transfers a local folder or crawled web portal and its existing vector chunks between workspaces in sub-50ms with zero API cost ($0.00).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "source_workspace": {"type": "string", "description": "Origin workspace name"},
                "target_workspace": {"type": "string", "description": "Destination workspace name"},
                "source_type": {"type": "string", "enum": ["folder", "web"], "description": "Type of source: 'folder' or 'web'"},
                "source_path_or_url": {"type": "string", "description": "Absolute folder path (e.g. 'C:\\Docs\\Legal') or website URL (e.g. 'https://canada.ca')"}
            },
            "required": ["source_workspace", "target_workspace", "source_type", "source_path_or_url"]
        }
    },
    {
        "name": "get_context_retrieval_settings",
        "description": "Retrieves the current multi-source RAG retrieval density settings (preset, top_k chunks, candidate pool size, max chunks per source).",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    },
    {
        "name": "set_context_retrieval_preset",
        "description": "Configures RAG retrieval density presets ('balanced', 'turbo', 'deep_research') or custom top_k / candidate pool parameters.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "preset": {"type": "string", "enum": ["balanced", "turbo", "deep_research", "custom"], "description": "Preset name: 'balanced', 'turbo', 'deep_research', or 'custom'"},
                "top_k": {"type": "integer", "description": "Custom target top_k diversified chunks to AI"},
                "candidate_pool_size": {"type": "integer", "description": "Custom candidate pool size retrieved from ChromaDB"},
                "max_chunks_per_source": {"type": "integer", "description": "Custom maximum chunks allowed per unique document or website"}
            },
            "required": []
        }
    }
]

def dispatch_mcp_request(request: Dict[str, Any]) -> Dict[str, Any]:
    """Processes a single JSON-RPC 2.0 MCP request and returns the response dictionary."""
    req_id = request.get("id")
    method = request.get("method")
    params = request.get("params", {})

    if method == "initialize":
        return {
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

    elif method == "tools/list":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": MCP_TOOLS_DEFINITIONS
            }
        }

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
                model_req = arguments.get("model")
                thread_id = f"mcp_chat_{uuid.uuid4()}"
                config = {
                    "configurable": {
                        "thread_id": thread_id, 
                        "active_workspace": ws,
                        "model": model_req,
                        "model_override": model_req
                    }
                }
                
                try:
                    full_response = ""
                    for token, metadata in cli_agent.stream({"messages": [msg]}, stream_mode="messages", config=config):
                        if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                            if isinstance(token.content, str) and token.content:
                                full_response += token.content
                    result_text = full_response.strip()
                except Exception as e:
                    from any_context.core.models_catalog import format_inference_error
                    err_info = format_inference_error(e, model_req or "default")
                    result_text = (
                        f"❌ Inference Error ({err_info['title']}):\n"
                        f"What happened: {err_info['cause']}\n"
                        f"Action: {err_info['action']}"
                    )

            elif tool_name == "list_available_models":
                from any_context.core.models_catalog import get_available_models
                store = ConfigDBStore()
                settings = store.get_app_settings()
                active_default = settings.models.inference_model if (settings and settings.models) else "gpt-4o-mini"
                models = get_available_models()
                result_text = json.dumps({
                    "active_default": active_default,
                    "available_models": models
                }, indent=2)

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

            elif tool_name == "create_workspace_share_invite":
                from any_context.workspace_sharing import WorkspaceSharingStore
                ws_name = arguments.get("workspace_name", "")
                acc_lvl = arguments.get("access_level", "viewer")
                max_u = arguments.get("max_uses", 1)
                store = WorkspaceSharingStore()
                invite = store.create_share_invite(workspace_name=ws_name, access_level=acc_lvl, created_by_email="mcp@system", max_uses=max_u)
                result_text = json.dumps(invite.dict(), indent=2)

            elif tool_name == "accept_workspace_share_invite":
                from any_context.workspace_sharing import WorkspaceSharingStore
                inv_code = arguments.get("invite_code", "")
                u_email = arguments.get("user_email", "")
                store = WorkspaceSharingStore()
                perm = store.accept_share_invite(invite_code=inv_code, user_email=u_email)
                result_text = json.dumps(perm.dict(), indent=2)

            elif tool_name == "get_subscription_status":
                from any_context.billing import BillingManager
                mgr = BillingManager()
                result_text = json.dumps(mgr.get_status().dict(), indent=2)

            elif tool_name == "list_subscription_plans":
                from any_context.billing import BillingManager, get_all_plans
                mgr = BillingManager()
                plans = [p.dict() for p in get_all_plans()]
                result_text = json.dumps({"plans": plans, "pricing_table": mgr.format_pricing_table_markdown()}, indent=2)

            elif tool_name == "add_workspace_web_url":
                from any_context.ingestion.web_scheduler import index_web_url_to_chromadb
                ws_target = arguments.get("workspace", "Default")
                url_target = arguments.get("url", "")
                poll_int = arguments.get("polling_interval_hours", 24)
                res = index_web_url_to_chromadb(workspace_name=ws_target, url=url_target, force=True)
                result_text = json.dumps(res, indent=2)

            elif tool_name == "list_workspace_web_urls":
                from any_context.ingestion.web_scheduler import WebSchedulerStore
                ws_target = arguments.get("workspace", "Default")
                store = WebSchedulerStore()
                urls = store.get_workspace_web_urls(ws_target)
                result_text = json.dumps({"workspace": ws_target, "web_urls": urls}, indent=2)

            elif tool_name == "remove_workspace_web_url":
                from any_context.ingestion.web_scheduler import WebSchedulerStore, remove_web_url_from_chromadb
                ws_target = arguments.get("workspace", "Default")
                url_or_id = arguments.get("url_or_id", "")
                store = WebSchedulerStore()
                urls = store.get_workspace_web_urls(ws_target)
                matched = next((u for u in urls if u["id"] == url_or_id or u["url"] == url_or_id), None)
                if matched:
                    store.delete_web_url(matched["id"], workspace_name=ws_target)
                    remove_web_url_from_chromadb(workspace_name=ws_target, url=matched["url"])
                    result_text = json.dumps({"status": "success", "message": f"Web URL '{matched['url']}' removed."}, indent=2)
                else:
                    result_text = json.dumps({"status": "error", "message": "Web URL not found in workspace."}, indent=2)

            elif tool_name == "sync_workspace_web_urls":
                from any_context.ingestion.web_scheduler import sync_workspace_web_urls
                ws_target = arguments.get("workspace", "Default")
                sync_res = sync_workspace_web_urls(workspace_name=ws_target)
                result_text = json.dumps(sync_res, indent=2)

            elif tool_name == "transfer_workspace_source":
                src_ws = arguments.get("source_workspace", "").strip()
                tgt_ws = arguments.get("target_workspace", "").strip()
                src_type = arguments.get("source_type", "folder").strip().lower()
                src_item = arguments.get("source_path_or_url", "").strip()

                if not src_ws or not tgt_ws or not src_item:
                    raise ValueError("source_workspace, target_workspace, and source_path_or_url are required.")

                if src_type in ["web", "url", "site", "portal"] or src_item.startswith("http://") or src_item.startswith("https://"):
                    from any_context.ingestion.web_scheduler import WebSchedulerStore
                    web_store = WebSchedulerStore()
                    res = web_store.transfer_web_source(source_ws=src_ws, target_ws=tgt_ws, url_or_root=src_item)
                else:
                    store = ConfigDBStore()
                    res = store.transfer_local_folder_source(source_ws=src_ws, target_ws=tgt_ws, folder_path=src_item)

                result_text = json.dumps(res, indent=2)

            elif tool_name == "get_context_retrieval_settings":
                store = ConfigDBStore()
                settings = store.get_app_settings()
                ctx = settings.context if settings else None
                if not ctx:
                    result_text = json.dumps({"error": "Could not load context settings."}, indent=2)
                else:
                    result_text = json.dumps({
                        "retrieval_preset": ctx.retrieval_preset,
                        "top_k": ctx.top_k,
                        "candidate_pool_size": ctx.candidate_pool_size,
                        "max_chunks_per_source": ctx.max_chunks_per_source,
                        "chunk_size": ctx.chunk_size,
                        "chunk_overlap": ctx.chunk_overlap
                    }, indent=2)

            elif tool_name == "set_context_retrieval_preset":
                store = ConfigDBStore()
                settings = store.get_app_settings()
                ctx = settings.context if settings else None
                if not ctx:
                    result_text = json.dumps({"error": "Could not load context settings."}, indent=2)
                else:
                    preset = arguments.get("preset")
                    top_k_val = arguments.get("top_k")
                    pool_val = arguments.get("candidate_pool_size")
                    max_src_val = arguments.get("max_chunks_per_source")

                    if preset:
                        ctx.apply_preset(preset)
                    if top_k_val is not None:
                        ctx.top_k = int(top_k_val)
                        ctx.retrieval_preset = "custom"
                    if pool_val is not None:
                        ctx.candidate_pool_size = int(pool_val)
                        ctx.retrieval_preset = "custom"
                    if max_src_val is not None:
                        ctx.max_chunks_per_source = int(max_src_val)
                        ctx.retrieval_preset = "custom"

                    store.update_context_settings(ctx)
                    result_text = json.dumps({
                        "status": "success",
                        "message": f"Updated context retrieval settings to preset '{ctx.retrieval_preset}'.",
                        "retrieval_preset": ctx.retrieval_preset,
                        "top_k": ctx.top_k,
                        "candidate_pool_size": ctx.candidate_pool_size,
                        "max_chunks_per_source": ctx.max_chunks_per_source
                    }, indent=2)

            else:
                result_text = f"Error: Tool '{tool_name}' not found."

            return {
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
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32603,
                    "message": str(e)
                }
            }

    return {
        "jsonrpc": "2.0",
        "id": req_id,
        "error": {
            "code": -32601,
            "message": f"Method '{method}' not found"
        }
    }

def start_mcp_server():
    """
    Launches AnyContext Model Context Protocol (MCP) Server over stdio JSON-RPC 2.0.
    """
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

            response = dispatch_mcp_request(request)
            _send_json_response(response)
        except Exception:
            break
