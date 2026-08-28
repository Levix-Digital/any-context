"""
AnyContext Stdio RPC Bridge Server.
Provides a high-speed, zero-network-port NDJSON (Newline-Delimited JSON)
communication protocol between the Python Core and external interactive TUIs (OpenTUI).
"""

import sys
import os
import io
import json
import logging
import traceback
from typing import Dict, Any, Optional

# Suppress noisy library loggers to prevent stdout/stderr contamination in RPC bridge mode
for _log_name in ["llama_index", "chromadb", "httpx", "httpcore", "urllib3", "openai", "tenacity"]:
    logging.getLogger(_log_name).setLevel(logging.ERROR)

from any_context import __version__
from any_context.config.db_store import ConfigDBStore
from any_context.core.models_catalog import get_available_models, validate_model_key_availability
from any_context.ingestion.local_folder_ingestor import BackgroundSyncManager, check_workspace_changes
from any_context.commands import COMMANDS_REGISTRY, dispatch_command


def _send_ndjson(data: Dict[str, Any]):
    """Sends a single JSON payload terminated with newline to stdout and flushes immediately."""
    try:
        line = json.dumps(data, ensure_ascii=False)
        sys.stdout.write(line + "\n")
        sys.stdout.flush()
    except Exception:
        pass


class StdioRPCServer:
    """
    Stdio RPC Server managing LangGraph agent executions, configuration state,
    slash command catalog, and real-time streaming over stdin/stdout.
    """

    def __init__(self, default_workspace: str = "Default"):
        caller_cwd = os.getenv("ACTX_CALLER_CWD")
        if caller_cwd and os.path.exists(caller_cwd):
            try:
                os.chdir(caller_cwd)
            except Exception:
                pass

        self.store = ConfigDBStore()
        self.active_workspace = default_workspace or "Default"
        self.agent_instance = None
        self._current_model = "gpt-4o-mini"
        self._grounding_mode = "strict"
        self._web_search_enabled = False
        self._load_state()

    def _load_state(self):
        """Loads workspace settings and active configuration from SQLite."""
        try:
            settings = self.store.get_app_settings()
            if settings and settings.models and settings.models.inference_model:
                from any_context.core.models_catalog import normalize_model_id
                self._current_model = normalize_model_id(settings.models.inference_model)
            self._grounding_mode = self.store.get_grounding_mode(workspace_name=self.active_workspace) or "strict"
            self._web_search_enabled = self.store.get_web_search_status(workspace_name=self.active_workspace) or False
        except Exception:
            pass

    def get_state(self) -> Dict[str, Any]:
        """Returns the current runtime state."""
        self._load_state()
        sync_info = "Up to date"
        is_syncing = False
        try:
            bg_mgr = BackgroundSyncManager()
            if bg_mgr.is_syncing(self.active_workspace):
                sync_info = bg_mgr.format_progress_bar(self.active_workspace, width=8)
                is_syncing = True
            else:
                last_time = getattr(self, "_last_sync_timestamp", None)
                sync_info = f"Up to date ({last_time})" if last_time else "Up to date"
        except Exception:
            pass

        tier_name = "Community Edition"
        try:
            from any_context.core.services.billing_service import BillingService
            billing_svc = BillingService()
            b_info = billing_svc.get_billing_info()
            tier_name = b_info.get("active_tier_name", "Community Edition")
        except Exception:
            pass

        return {
            "version": __version__,
            "workspace": self.active_workspace,
            "model": self._current_model,
            "grounding_mode": self._grounding_mode,
            "web_search_enabled": self._web_search_enabled,
            "sync_info": sync_info,
            "is_syncing": is_syncing,
            "tier_name": tier_name
        }

    def list_commands(self) -> list:
        """Returns metadata for all 23 available slash commands for the OpenTUI palette."""
        return [
            {
                "command": c.name,
                "args": c.args,
                "description": c.description,
                "category": c.category,
                "direct_execution": c.direct_execution,
                "aliases": c.aliases,
            }
            for c in COMMANDS_REGISTRY
        ]

    def handle_request(self, req: Dict[str, Any]):
        """Dispatches an incoming NDJSON request and returns responses or streams."""
        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params") or {}

        try:
            if method == "get_state":
                state = self.get_state()
                _send_ndjson({"id": req_id, "result": state})

            elif method == "list_commands":
                cmds = self.list_commands()
                _send_ndjson({"id": req_id, "result": cmds})

            elif method == "execute_command":
                cmd_line = params.get("command", "")
                result = dispatch_command(cmd_line, active_workspace=self.active_workspace)
                if result.state_updates:
                    if "workspace" in result.state_updates:
                        self.active_workspace = result.state_updates["workspace"]
                    if "model" in result.state_updates:
                        self._current_model = result.state_updates["model"]
                    if "grounding_mode" in result.state_updates:
                        self._grounding_mode = result.state_updates["grounding_mode"]
                    if "web_search_enabled" in result.state_updates:
                        self._web_search_enabled = result.state_updates["web_search_enabled"]
                    if "model" in result.state_updates or "workspace" in result.state_updates or "grounding_mode" in result.state_updates:
                        self.agent_instance = None
                    self._load_state()

                _send_ndjson({
                    "id": req_id,
                    "result": {
                        "success": result.success,
                        "message": result.message,
                        "action": result.action,
                        "error": result.error,
                        "state": self.get_state()
                    }
                })

            elif method == "switch_workspace":
                target_ws = params.get("workspace", "Default").strip()
                self.active_workspace = target_ws
                if not self.store.get_workspace_meta(target_ws):
                    try:
                        self.store.add_workspace(name=target_ws, paths=[])
                    except Exception:
                        pass
                self.agent_instance = None
                self._load_state()
                _send_ndjson({"id": req_id, "result": self.get_state()})

            elif method == "set_model":
                new_model = params.get("model", "").strip()
                if new_model:
                    self._current_model = new_model
                    try:
                        settings = self.store.get_app_settings()
                        if settings and settings.models:
                            models = settings.models
                            models.inference_model = new_model
                            self.store.update_model_settings(models)
                    except Exception:
                        pass
                    self.agent_instance = None
                _send_ndjson({"id": req_id, "result": self.get_state()})

            elif method == "set_mode":
                new_mode = params.get("mode", "strict").lower().strip()
                if new_mode in ["strict", "hybrid", "proactive"]:
                    self._grounding_mode = new_mode
                    try:
                        self.store.set_grounding_mode(mode=new_mode, workspace_name=self.active_workspace, apply_global=(self.active_workspace.lower() == "default"))
                    except Exception:
                        pass
                    self.agent_instance = None
                _send_ndjson({"id": req_id, "result": self.get_state()})

            elif method == "set_web_search":
                val = bool(params.get("enabled", False))
                self._web_search_enabled = val
                self.store.set_web_search_status(workspace_name=self.active_workspace, enabled=val)
                self.agent_instance = None
                _send_ndjson({"id": req_id, "result": self.get_state()})

            elif method == "start_sync":
                force = bool(params.get("force", False))
                bg_mgr = BackgroundSyncManager()
                bg_mgr.start_background_sync(workspace_name=self.active_workspace, force_full=force, verbose=False)
                _send_ndjson({"id": req_id, "result": {"started": True, "workspace": self.active_workspace}})

            elif method == "list_sources":
                all_sources = self.store.get_workspace_sources(workspace_name=self.active_workspace)
                _send_ndjson({"id": req_id, "result": all_sources})

            elif method == "get_menu_tree":
                menu_id = params.get("menu_id", "main")
                ws = params.get("workspace", self.active_workspace)
                from any_context.core.interaction.config_engine import ConfigEngine
                cfg_engine = ConfigEngine()
                tree = cfg_engine.get_menu_tree(menu_id=menu_id, workspace=ws)
                _send_ndjson({"id": req_id, "result": tree.model_dump()})

            elif method == "execute_menu_action":
                action_id = params.get("action_id", "")
                act_params = params.get("params", {})
                ws = params.get("workspace", self.active_workspace)
                from any_context.core.interaction.config_engine import ConfigEngine
                cfg_engine = ConfigEngine()
                action_res = cfg_engine.execute_action(action_id=action_id, params=act_params, workspace=ws)
                if action_res.state_updates:
                    if "workspace" in action_res.state_updates:
                        self.active_workspace = action_res.state_updates["workspace"]
                    if "model" in action_res.state_updates:
                        self._current_model = action_res.state_updates["model"]
                    if "grounding_mode" in action_res.state_updates:
                        self._grounding_mode = action_res.state_updates["grounding_mode"]
                    if "web_search_enabled" in action_res.state_updates:
                        self._web_search_enabled = action_res.state_updates["web_search_enabled"]
                    self.agent_instance = None
                    self._load_state()
                _send_ndjson({"id": req_id, "result": action_res.model_dump()})

            elif method == "get_options":
                opt_type = params.get("type", "grounding_mode")
                ws = params.get("workspace", self.active_workspace)
                from any_context.core.interaction.options_engine import OptionsEngine
                opts_engine = OptionsEngine()
                if opt_type == "grounding_mode":
                    opts = opts_engine.get_grounding_mode_options(workspace=ws)
                elif opt_type == "workspace":
                    opts = opts_engine.get_workspace_options(current_workspace=ws)
                elif opt_type in ["inference_model", "model"]:
                    opts = opts_engine.get_inference_model_options()
                elif opt_type == "retrieval_density":
                    opts = opts_engine.get_retrieval_density_options()
                elif opt_type == "update":
                    target_v = params.get("target_version")
                    opts = opts_engine.get_update_options(target_version=target_v)
                elif opt_type in ["delete_workspace", "ws_delete"]:
                    opts = opts_engine.get_delete_workspace_options(current_workspace=ws)
                elif opt_type in ["confirm_delete_workspace", "confirm_delete_ws"]:
                    target_ws = params.get("target_workspace") or ws
                    is_active = (ws.lower() == target_ws.lower())
                    opts = opts_engine.get_confirm_delete_workspace_options(workspace_to_delete=target_ws, is_active=is_active)
                else:
                    opts = opts_engine.get_grounding_mode_options(workspace=ws)
                _send_ndjson({"id": req_id, "result": opts.model_dump()})

            elif method == "set_option":
                opt_type = params.get("type", "grounding_mode")
                val = params.get("value", "")
                ws = params.get("workspace", self.active_workspace)
                apply_global = bool(params.get("apply_global", False))
                from any_context.core.interaction.options_engine import OptionsEngine
                opts_engine = OptionsEngine()
                if opt_type == "grounding_mode":
                    res = opts_engine.set_grounding_mode(mode=val, workspace=ws, apply_global=apply_global)
                elif opt_type == "workspace":
                    res = opts_engine.set_workspace(workspace_name=val)
                elif opt_type in ["inference_model", "model"]:
                    res = opts_engine.set_inference_model(model_name=val)
                elif opt_type == "retrieval_density":
                    res = opts_engine.set_retrieval_density_preset(preset=val)
                elif opt_type == "update":
                    res = opts_engine.execute_update_option(option_id=val, is_tui=True)
                elif opt_type in ["delete_workspace", "ws_delete", "confirm_delete_workspace", "confirm_delete_ws"]:
                    res = opts_engine.execute_delete_workspace_option(option_id=val, current_workspace=ws)
                else:
                    res = opts_engine.set_grounding_mode(mode=val, workspace=ws, apply_global=apply_global)

                if res.state_updates:
                    if "workspace" in res.state_updates:
                        self.active_workspace = res.state_updates["workspace"]
                    if "model" in res.state_updates:
                        self._current_model = res.state_updates["model"]
                    if "grounding_mode" in res.state_updates:
                        self._grounding_mode = res.state_updates["grounding_mode"]
                    self.agent_instance = None
                    self._load_state()

                _send_ndjson({"id": req_id, "result": res.model_dump()})

            elif method == "chat":
                prompt_text = params.get("message", "")
                self._stream_chat(req_id, prompt_text)

            else:
                _send_ndjson({"id": req_id, "error": {"code": -32601, "message": f"Method '{method}' not found"}})

        except Exception as e:
            _send_ndjson({"id": req_id, "error": {"code": -32000, "message": str(e), "traceback": traceback.format_exc()}})


    def _stream_chat(self, req_id: Any, prompt_text: str):
        """Streams LangGraph agent tokens and tool execution tickers in real-time."""
        from any_context.core.agent import create_anycontext_agent

        try:
            thread_id = f"rpc_session_{self.active_workspace}"
            config = {
                "configurable": {
                    "thread_id": thread_id,
                    "active_workspace": self.active_workspace,
                    "grounding_mode": self._grounding_mode,
                    "web_search_enabled": self._web_search_enabled
                }
            }

            if self.agent_instance is None:
                self.agent_instance = create_anycontext_agent(
                    active_workspace=self.active_workspace,
                    model_override=self._current_model,
                    grounding_mode=self._grounding_mode,
                    web_search_enabled=self._web_search_enabled
                )

            full_reply = ""
            for token, metadata in self.agent_instance.stream(
                {"messages": [prompt_text]},
                stream_mode="messages",
                config=config
            ):
                if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"]:
                    content_chunk = ""
                    if isinstance(token.content, str) and token.content:
                        content_chunk = token.content
                    elif isinstance(token.content, list):
                        parts = [p if isinstance(p, str) else p.get("text", "") for p in token.content if isinstance(p, (str, dict))]
                        content_chunk = "".join(parts)

                    if content_chunk:
                        full_reply += content_chunk
                        _send_ndjson({"id": req_id, "type": "token", "content": content_chunk})

                elif hasattr(token, "type") and token.type in ["tool", "ToolMessage", "ToolMessageChunk"]:
                    t_name = str(getattr(token, "name", "") or "")
                    if "web" in t_name.lower():
                        ticker = "🌐 Searching web in real-time..."
                    else:
                        ticker = "📚 Reading indexed context documents..."
                    _send_ndjson({"id": req_id, "type": "ticker", "content": ticker})

            _send_ndjson({"id": req_id, "type": "done", "full_reply": full_reply})

        except Exception as e:
            _send_ndjson({"id": req_id, "type": "error", "message": str(e), "traceback": traceback.format_exc()})
            self.agent_instance = None


def run_rpc_server(default_workspace: str = "Default"):
    """Starts the main stdio line-reading loop for NDJSON RPC."""
    # Force UTF-8 on Windows stdin/stdout
    if hasattr(sys.stdin, "reconfigure"):
        try:
            sys.stdin.reconfigure(encoding="utf-8")
        except Exception:
            pass
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8")
        except Exception:
            pass

    server = StdioRPCServer(default_workspace=default_workspace)
    # Notify TUI on startup that RPC server is ready
    _send_ndjson({"event": "ready", "state": server.get_state()})

    for line in sys.stdin:
        clean_line = line.strip()
        if not clean_line:
            continue
        try:
            payload = json.loads(clean_line)
            server.handle_request(payload)
        except json.JSONDecodeError as e:
            _send_ndjson({"error": {"code": -32700, "message": f"Parse error: {e}"}})
        except Exception as e:
            _send_ndjson({"error": {"code": -32000, "message": str(e)}})


if __name__ == "__main__":
    ws = sys.argv[1] if len(sys.argv) > 1 else "Default"
    run_rpc_server(default_workspace=ws)

