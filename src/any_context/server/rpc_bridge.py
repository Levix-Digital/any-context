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

        try:
            from any_context.ingestion.orchestrator import BackgroundSyncManager
            self.bg_mgr = BackgroundSyncManager()
            self.bg_mgr.register_completion_listener(self._on_background_job_complete)
        except Exception:
            pass

    def _on_background_job_complete(self, notif: Dict[str, Any]):
        """Dispatches live notification and updated state across the RPC bridge when background crawl/sync finishes."""
        try:
            import time
            self._last_sync_timestamp = time.strftime("%H:%M:%S")
            _send_ndjson({
                "event": "notification",
                "level": "success" if notif.get("success") else "error",
                "message": notif.get("message", "Synchronization completed."),
                "workspace": notif.get("workspace"),
                "state": self.get_state()
            })
        except Exception:
            pass


    def _load_state(self):
        """Loads workspace settings and active configuration from SQLite."""
        try:
            from any_context.core.services.model_service import ModelService
            model_svc = ModelService(store=self.store)
            self._current_model = model_svc.get_current_model(workspace_name=self.active_workspace)
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
            from any_context.ingestion.orchestrator import BackgroundSyncManager
            bg_mgr = BackgroundSyncManager()
            if bg_mgr.is_syncing(self.active_workspace):
                sync_info = bg_mgr.format_progress_bar(self.active_workspace, width=8)
                is_syncing = True
            else:
                last_time = getattr(self, "_last_sync_timestamp", None)
                sync_info = f"Up to date ({last_time})" if last_time else "Up to date"
        except Exception:
            pass

        tier_name = "🌿 Community Edition"
        try:
            from any_context.billing import BillingManager
            b_mgr = BillingManager()
            status = b_mgr.get_status()
            tier_id = (status.active_tier_id or "community").lower().strip()
            if tier_id == "enterprise":
                tier_name = "🏢 Enterprise Edition"
            elif tier_id == "team":
                tier_name = "👥 Team Edition"
            elif tier_id == "pro":
                tier_name = "⭐ Pro Plan"
            elif tier_id == "starter":
                tier_name = "💼 Starter Plan"
            else:
                tier_name = "🌿 Community Edition"
        except Exception:
            pass

        from any_context.core.models_catalog import get_commercial_model_name
        model_display = get_commercial_model_name(self._current_model)

        from any_context.core.services.onboarding_service import OnboardingService
        onboarding_svc = OnboardingService(store=self.store)
        ob_state = onboarding_svc.check_status()

        return {
            "version": __version__,
            "workspace": self.active_workspace,
            "model": self._current_model,
            "model_display": model_display,
            "grounding_mode": self._grounding_mode,
            "web_search_enabled": self._web_search_enabled,
            "sync_info": sync_info,
            "is_syncing": is_syncing,
            "tier_name": tier_name,
            "needs_onboarding": ob_state.needs_onboarding,
            "onboarding_state": ob_state.model_dump()
        }

    def list_commands(self) -> list:
        """Returns metadata for all 23 available slash commands for the OpenTUI palette."""
        from any_context.commands.registry import COMMANDS_REGISTRY
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
        from any_context.observability import obs
        req_id = req.get("id")
        method = req.get("method")
        params = req.get("params") or {}
        obs.debug("RPC:RECV", f"Received method '{method}' (id={req_id})", {"id": req_id, "method": method, "params": params})

        try:
            if method == "ping":
                _send_ndjson({"id": req_id, "result": {"pong": True, "version": __version__}})
                obs.debug("RPC:RESP", f"Sent ping response (id={req_id})", {"pong": True})

            elif method == "get_state":
                state = self.get_state()
                _send_ndjson({"id": req_id, "result": state})
                obs.debug("RPC:RESP", f"Sent get_state response (id={req_id})", {"needs_onboarding": state.get("needs_onboarding")})

            elif method == "list_commands":
                cmds = self.list_commands()
                _send_ndjson({"id": req_id, "result": cmds})
                obs.debug("RPC:RESP", f"Sent list_commands response (id={req_id})", {"count": len(cmds)})

            elif method == "execute_command":
                from any_context.commands.dispatcher import dispatch_command
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
                    try:
                        from any_context.core.services.model_service import ModelService
                        model_svc = ModelService(store=self.store)
                        res = model_svc.set_model(new_model, workspace_name=self.active_workspace)
                        self._current_model = res["model"]
                    except Exception:
                        self._current_model = new_model
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
                from any_context.ingestion.orchestrator import BackgroundSyncManager
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

            elif method == "get_onboarding_status":
                from any_context.core.services.onboarding_service import OnboardingService
                onboarding_svc = OnboardingService(store=self.store)
                st = onboarding_svc.check_status()
                _send_ndjson({"id": req_id, "result": st.model_dump()})

            elif method == "complete_onboarding":
                choice = params.get("choice") or params.get("choice_id") or "openai"
                api_key = params.get("api_key")
                base_url = params.get("base_url")
                model_name = params.get("model_name")
                workspace_name = params.get("workspace_name") or self.active_workspace
                from any_context.core.services.onboarding_service import OnboardingService
                onboarding_svc = OnboardingService(store=self.store)
                res = onboarding_svc.complete_onboarding(
                    choice_id=choice,
                    api_key=api_key,
                    base_url=base_url,
                    model_name=model_name,
                    workspace_name=workspace_name
                )
                if res.success:
                    self.agent_instance = None
                    self._load_state()
                _send_ndjson({
                    "id": req_id,
                    "result": {
                        "success": res.success,
                        "message": res.message,
                        "error": res.error,
                        "state_updates": res.state_updates,
                        "state": self.get_state()
                    }
                })

            elif method == "get_options":
                opt_type = params.get("type", "grounding_mode")
                ws = params.get("workspace", self.active_workspace)
                from any_context.core.interaction.options_engine import OptionsEngine
                opts_engine = OptionsEngine()
                if opt_type == "onboarding":
                    from any_context.core.services.onboarding_service import OnboardingService
                    opts = OnboardingService(store=self.store).check_status().options_group
                elif opt_type == "grounding_mode":
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
                elif opt_type in ["delete_source", "ws_delete_source", "sources_delete"]:
                    opts = opts_engine.get_delete_source_options(current_workspace=ws)
                elif opt_type in ["confirm_delete_source", "confirm_delete_src"]:
                    source_info = params.get("source_info") or {}
                    opts = opts_engine.get_confirm_delete_source_options(source_info=source_info, current_workspace=ws)
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
                if opt_type == "onboarding":
                    from any_context.core.services.onboarding_service import OnboardingService
                    onboarding_svc = OnboardingService(store=self.store)
                    res = onboarding_svc.complete_onboarding(
                        choice_id=val,
                        api_key=params.get("api_key"),
                        base_url=params.get("base_url"),
                        workspace_name=ws
                    )
                elif opt_type == "grounding_mode":
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
                elif opt_type in ["delete_source", "ws_delete_source", "sources_delete", "confirm_delete_source", "confirm_delete_src"]:
                    res = opts_engine.execute_delete_source_option(option_id=val, current_workspace=ws, metadata=params.get("metadata"))
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
            err_str = str(e)
            _send_ndjson({"id": req_id, "type": "error", "message": err_str, "traceback": traceback.format_exc()})
            self.agent_instance = None
            if "tool_call_id" in err_str or "tool_calls" in err_str:
                try:
                    from any_context.core.agent import get_safe_checkpointer
                    chk = get_safe_checkpointer()
                    if hasattr(chk, "delete_thread"):
                        chk.delete_thread(thread_id)
                except Exception:
                    pass


def run_rpc_server(default_workspace: str = "Default"):
    """Starts the main stdio line-reading loop for NDJSON RPC."""
    from any_context.observability import obs
    obs.info("RPC:SERVER", "Stdio RPC bridge server started", {"workspace": default_workspace, "pid": os.getpid()})

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
    ready_state = server.get_state()
    _send_ndjson({"event": "ready", "state": ready_state})
    obs.info("RPC:READY", "Emitted ready event to OpenTUI", {"needs_onboarding": ready_state.get("needs_onboarding")})

    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                obs.info("RPC:EOF", "Stdin reached EOF. Shutting down RPC bridge server.")
                break
            clean_line = line.strip()
            if not clean_line:
                continue
            payload = json.loads(clean_line)
            server.handle_request(payload)
        except json.JSONDecodeError as e:
            obs.error("RPC:JSON_ERROR", f"Failed to decode JSON payload: {e}", exc=e)
            _send_ndjson({"error": {"code": -32700, "message": f"Parse error: {e}"}})
        except (KeyboardInterrupt, EOFError):
            obs.info("RPC:EXIT", "Received interrupt/EOF signal")
            break
        except Exception as e:
            obs.error("RPC:LOOP_ERROR", f"Unexpected error in RPC loop: {e}", exc=e)
            _send_ndjson({"error": {"code": -32000, "message": str(e)}})


if __name__ == "__main__":
    ws = sys.argv[1] if len(sys.argv) > 1 else "Default"
    run_rpc_server(default_workspace=ws)

