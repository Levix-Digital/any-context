import sys
import os
import io

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
    High-speed CLI entrypoint. Prints signature ASCII banner in milliseconds
    before loading interactive UI, configuration, or RAG components.
    """
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

    # 2. Patch prompt_toolkit for Git Bash / MinGW compatibility
    _patch_prompt_toolkit_for_git_bash()

    # 3. Print banner IMMEDIATELY before importing anything heavy (suppressed for TUI, MCP, RPC, API)
    if "--mcp" not in sys.argv and "--rpc" not in sys.argv and "--tui" not in sys.argv:
        from any_context.cli.banner import print_banner, clear_terminal
        # Clear screen for interactive session unless user passed one-shot flags
        non_interactive_flags = {"--help", "-h", "--version", "-v", "--mcp", "--rpc", "--tui", "--server", "serve", "api"}
        if not any(arg.lower() in non_interactive_flags for arg in sys.argv[1:]):
            clear_terminal()
        print_banner()

    # 4. Load environment variables (.env) for LangSmith tracing, licenses, and API keys
    try:
        from any_context.core.utils import load_env
        load_env()
    except Exception:
        pass

    # 5. Configure logging silencers
    import logging
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("openai").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    logging.getLogger("llama_index").setLevel(logging.WARNING)
    logging.getLogger("transformers").setLevel(logging.ERROR)

    # 6. Now load chat loop and execute
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
