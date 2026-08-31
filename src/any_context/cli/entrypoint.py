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
    if "-v" in sys.argv or "--version" in sys.argv:
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

    # 2. Print banner IMMEDIATELY before importing anything heavy (suppressed for TUI, MCP, RPC, API, Diagnostics)
    non_interactive_flags = {"--help", "-h", "--version", "-v", "--mcp", "--rpc", "--tui", "--server", "serve", "api", "--diagnostics", "--diag", "--logs"}
    if not any(arg.lower() in non_interactive_flags for arg in sys.argv[1:]):
        # Patch prompt_toolkit for Git Bash / MinGW compatibility
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

    from any_context.observability import obs, collect_diagnostic_report, format_diagnostic_report, format_recent_logs
    obs.debug("CLI:BOOT", "AnyContext entrypoint invoked", {"argv": sys.argv})

    # 4. Fast-path dispatch for non-interactive flags (sub-10ms response, avoids loading unused modules)
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

    if "--tui" in sys.argv:
        obs.info("CLI:DISPATCH", "Dispatching to OpenTUI", {"argv": sys.argv})
        from any_context.cli.chat_loop import launch_opentui
        ws = "Default"
        for i, a in enumerate(sys.argv):
            if a in ["-w", "--workspace"] and i + 1 < len(sys.argv):
                ws = sys.argv[i + 1]
            elif not a.startswith("-") and a != sys.argv[0] and a != "--tui":
                ws = a
        launched = launch_opentui(ws)
        if launched:
            sys.exit(0)
        else:
            sys.exit(1)

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

    # 6. Configure logging silencers
    import logging
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("openai").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    logging.getLogger("llama_index").setLevel(logging.WARNING)
    logging.getLogger("transformers").setLevel(logging.ERROR)

    # 7. Now load chat loop and execute
    try:
        from any_context.cli.chat_loop import main_cli
        main_cli()
    except (KeyboardInterrupt, EOFError):
        print("\n\n👋 AnyContext closed.\n")
        sys.exit(0)
    except Exception as e:
        print(f"\n❌ Unexpected error starting AnyContext: {e}\n")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    entrypoint()
