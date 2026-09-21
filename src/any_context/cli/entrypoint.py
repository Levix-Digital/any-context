import sys
import os
import io
import multiprocessing

multiprocessing.freeze_support()

def _patch_prompt_toolkit_for_git_bash():
    """
    Patches prompt_toolkit on Windows so that Git Bash (MINGW64/mintty) and pseudo-terminals
    automatically fall back to Vt100_Output without raising NoConsoleScreenBufferError.
    """
    try:
        import prompt_toolkit.output.defaults
        from prompt_toolkit.output.win32 import NoConsoleScreenBufferError
        from prompt_toolkit.output.vt100 import Vt100_Output

        orig_create = prompt_toolkit.output.defaults.create_output

        def robust_create_output(stdout=None, always_prefer_tty=False):
            target_out = stdout or sys.stdout
            try:
                return orig_create(stdout=target_out, always_prefer_tty=always_prefer_tty)
            except NoConsoleScreenBufferError:
                return Vt100_Output.from_pty(target_out)
            except Exception:
                try:
                    return Vt100_Output.from_pty(target_out)
                except Exception:
                    return orig_create(stdout=target_out, always_prefer_tty=always_prefer_tty)

        prompt_toolkit.output.defaults.create_output = robust_create_output
    except Exception:
        pass

def entrypoint():
    """
    High-speed CLI entrypoint. Fast-paths non-interactive flags in < 1ms
    before loading interactive UI, configuration, or RAG components.
    """
    # 0. Instant fast-path for version check (sub-1ms response)
    if "-v" in sys.argv or "--version" in sys.argv or "-V" in sys.argv:
        from any_context import __version__
        print(f"AnyContext (actx) v{__version__} - Levix Digital")
        sys.exit(0)

    # 1. Force UTF-8 on Windows terminal while preserving TTY handles
    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8")
        except Exception:
            pass
    elif sys.stdout.encoding != "utf-8":
        try:
            sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")
        except Exception:
            pass

    if hasattr(sys.stderr, "reconfigure"):
        try:
            sys.stderr.reconfigure(encoding="utf-8")
        except Exception:
            pass

    # 2. Print banner only when legacy CLI mode is explicitly requested
    if "--cli" in sys.argv:
        _patch_prompt_toolkit_for_git_bash()
        from any_context.cli.banner import print_banner, clear_terminal
        clear_terminal()
        print_banner()

    # 3. Load environment variables (.env) for LangSmith tracing, licenses, and API keys
    try:
        from any_context.core.utils import load_env
        load_env()
    except Exception:
        pass

    try:
        from any_context.help.bootstrap import async_ensure_system_knowledge_indexed
        async_ensure_system_knowledge_indexed()
    except Exception:
        pass

    from any_context.observability import obs, collect_diagnostic_report, format_diagnostic_report, format_recent_logs
    obs.debug("CLI:BOOT", "AnyContext entrypoint invoked", {"argv": sys.argv})

    # 4. Fast-path dispatch for non-interactive flags (sub-10ms response, avoids loading unused modules)
    if any(arg in ["--version", "-v", "-V"] for arg in sys.argv[1:]):
        from any_context import __version__
        print(f"AnyContext (actx) v{__version__} - Levix Digital")
        sys.exit(0)

    if any(arg in ["--help", "-h"] for arg in sys.argv[1:]):
        cli_str = " ".join(sys.argv[1:])
        from any_context.help import handle_command_help_interception
        if handle_command_help_interception(cli_str):
            sys.exit(0)
        from any_context.help.manager import display_help_page
        from any_context.help.registry import get_help_page
        page = get_help_page("commands")
        if page:
            display_help_page(page)
        else:
            print("Run 'actx' to launch the interactive terminal or 'actx --help <command>' for specific command help.")
        sys.exit(0)

    has_cli_mgmt_flag = any(
        arg in ["--update", "-u", "--check-update", "--releases", "--list-releases", "--rollback", "--config", "-c", "--keys", "--billing", "--reset-models", "--factory-reset"]
        or arg.startswith("--update@") or arg.startswith("-u@")
        for arg in sys.argv[1:]
    )
    if has_cli_mgmt_flag:
        obs.info("CLI:DISPATCH", "Dispatching to CLI workspace/management selector", {"argv": sys.argv})
        from any_context.cli.workspace_selector import get_active_workspace
        get_active_workspace()
        sys.exit(0)

    if any(arg in ["--serve", "--server", "serve", "api"] for arg in sys.argv[1:]):
        obs.info("CLI:DISPATCH", "Dispatching to REST API Server", {"argv": sys.argv})
        from any_context.server.api import start_api_server
        port = 8000
        host = "127.0.0.1"
        for i, a in enumerate(sys.argv):
            if a == "--port" and i + 1 < len(sys.argv) and sys.argv[i + 1].isdigit():
                port = int(sys.argv[i + 1])
            elif a == "--host" and i + 1 < len(sys.argv):
                host = sys.argv[i + 1]
        start_api_server(host=host, port=port)
        sys.exit(0)

    if "--diagnostics" in sys.argv or "--diag" in sys.argv:
        report = collect_diagnostic_report()
        print(format_diagnostic_report(report))
        sys.exit(0)

    if "--logs" in sys.argv:
        from any_context.observability import ObservabilityStorage
        storage = ObservabilityStorage()
        limit = 50
        for i, a in enumerate(sys.argv):
            if a == "--limit" and i + 1 < len(sys.argv) and sys.argv[i + 1].isdigit():
                limit = int(sys.argv[i + 1])
        logs = storage.get_recent_logs(limit=limit)
        print(format_recent_logs(logs, limit=limit))
        sys.exit(0)

    if "--rpc" in sys.argv:
        obs.info("CLI:DISPATCH", "Dispatching to Stdio RPC Server", {"argv": sys.argv})
        from any_context.server.rpc_bridge import run_rpc_server
        target_ws = "Default"
        for i, a in enumerate(sys.argv):
            if a in ["-w", "--workspace"] and i + 1 < len(sys.argv):
                target_ws = sys.argv[i + 1]
            elif not a.startswith("-") and a != sys.argv[0] and a != "--rpc":
                target_ws = a
        run_rpc_server(default_workspace=target_ws)
        sys.exit(0)

    if "--mcp" in sys.argv:
        obs.info("CLI:DISPATCH", "Dispatching to MCP Server", {"argv": sys.argv})
        from any_context.server.mcp import start_mcp_server
        start_mcp_server()
        sys.exit(0)

    # 5. Resolve active workspace from arguments
    ws = "Default"
    for i, a in enumerate(sys.argv):
        if a in ["-w", "--workspace"] and i + 1 < len(sys.argv):
            ws = sys.argv[i + 1]
        elif not a.startswith("-") and a != sys.argv[0] and a not in ["--tui", "--cli"]:
            ws = a

    # 6. Check for direct one-shot prompt (e.g. actx "my question" or actx -p "question" or piped stdin)
    p_flag_idx = -1
    for flag in ["-p", "--prompt", "-q", "--query"]:
        if flag in sys.argv:
            p_flag_idx = sys.argv.index(flag)
            break

    direct_prompt = None
    if p_flag_idx != -1 and p_flag_idx + 1 < len(sys.argv):
        direct_prompt = sys.argv[p_flag_idx + 1].strip()
    elif not any(arg.lower() in ["--help", "-h", "--version", "-v", "--update", "-u", "--mcp", "--server", "serve", "api", "--tui", "--cli"] for arg in sys.argv[1:]):
        filtered_args = []
        skip_next = False
        for a in sys.argv[1:]:
            if skip_next:
                skip_next = False
                continue
            if a in ["-w", "--workspace"]:
                skip_next = True
                continue
            if not a.startswith("-"):
                filtered_args.append(a)
        if filtered_args:
            direct_prompt = " ".join(filtered_args).strip()

    if not direct_prompt and not sys.stdin.isatty():
        try:
            piped = sys.stdin.read().strip()
            if piped:
                direct_prompt = piped
        except Exception:
            pass

    if direct_prompt:
        from any_context.core.agent import create_anycontext_agent
        from any_context.config.db_store import ConfigDBStore
        store = ConfigDBStore()
        settings = store.get_app_settings()
        model = settings.models.inference_model if settings and settings.models else "gpt-4o-mini"
        mode = store.get_grounding_mode(workspace_name=ws) or "strict"
        search_enabled = store.get_web_search_status(workspace_name=ws) or False

        agent = create_anycontext_agent(
            model_name=model,
            workspace_name=ws,
            grounding_mode=mode,
            web_search_enabled=search_enabled
        )
        config = {
            "configurable": {
                "thread_id": f"batch_{ws}",
                "active_workspace": ws,
                "grounding_mode": mode,
                "web_search_enabled": search_enabled
            }
        }
        for token, meta in agent.stream({"messages": [direct_prompt]}, stream_mode="messages", config=config):
            if hasattr(token, "type") and token.type in ["ai", "AIMessageChunk", "AIMessage"] and token.content:
                if isinstance(token.content, str):
                    sys.stdout.write(token.content)
                    sys.stdout.flush()
        sys.stdout.write("\n")
        sys.stdout.flush()
        sys.exit(0)

    # 7. DEFAULT USER EXPERIENCE: OpenTUI Interactive Terminal
    from any_context.cli.tui_launcher import launch_opentui
    obs.info("CLI:DISPATCH", "Defaulting to OpenTUI interface", {"workspace": ws})
    launched = launch_opentui(ws)
    if launched:
        sys.exit(0)
    sys.exit(1)


if __name__ == "__main__":
    entrypoint()
