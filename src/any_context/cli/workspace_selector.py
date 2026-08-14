import argparse
import sys
import questionary
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.cli.config_menu import run_first_time_wizard, show_config_menu
from any_context.cli.updater import check_for_updates, run_self_update
from any_context import __version__

def show_workspace_menu() -> str:
    """
    Displays an interactive menu for the user to select a workspace.
    Runs first-time wizard if database is empty.
    """
    store = ConfigDBStore()
    if store.is_empty():
        run_first_time_wizard()

    settings = store.get_app_settings()
    if not settings or not settings.workspaces:
        print("❌ Error: No workspaces found in configuration.")
        sys.exit(1)

    workspace_names = [ws.name for ws in settings.workspaces]
    
    selected = questionary.select(
        "Select the active workspace:",
        choices=workspace_names
    ).ask()
    
    if not selected:
        print("❌ No workspace selected. Exiting...")
        sys.exit(0)
        
    return selected

def ensure_api_key_configured():
    """
    Checks if an OpenAI API key is required and missing.
    Presents an interactive explanation allowing the user to provide an OpenAI key
    OR switch seamlessly to a Local Offline Server (LM Studio / Ollama) or Custom Setup.
    """
    store = ConfigDBStore()
    settings = store.get_app_settings()
    if not settings or not settings.models:
        return

    provider = settings.models.model_provider
    if provider == "openai":
        if "localhost" in settings.models.local_base_url or "127.0.0.1" in settings.models.local_base_url:
            settings.models.local_base_url = "https://api.openai.com/v1"
            store.save_app_settings(settings)

        api_key = store.get_api_key("openai")
        if not api_key or api_key == "lm-studio":
            from any_context.core.utils import get_api_key
            api_key = get_api_key(provider="openai")

        if not api_key or api_key == "lm-studio":
            print("\n======================================================================")
            print("🤖 Welcome to AnyContext AI Setup!")
            print("By default, AnyContext uses OpenAI Cloud models (gpt-4o-mini &")
            print("text-embedding-3-small) for fast reasoning and semantic search.")
            print("======================================================================\n")

            choice = questionary.select(
                "How would you like to configure your AI Provider?",
                choices=[
                    "⚡ OpenAI Cloud (Enter OpenAI API Key - Recommended)",
                    "🏠 Local Offline Server (LM Studio / Ollama - 100% Free & Offline)",
                    "🛠️ Custom Setup (Configure custom models, base URL & keys)"
                ]
            ).ask()

            if choice and choice.startswith("⚡"):
                entered_key = questionary.password("Enter your OpenAI API Key (sk-...):").ask()
                if entered_key and entered_key.strip():
                    store.set_api_key("openai", entered_key.strip())
                    print("✅ OpenAI API Key saved successfully!\n")
                else:
                    print("⚠️ Notice: No OpenAI API Key entered. Opening Custom Setup Menu...")
                    from any_context.cli.config_menu import _manage_models
                    _manage_models(store)
            elif choice and (choice.startswith("🏠") or choice.startswith("🛠️")):
                from any_context.cli.config_menu import _manage_models
                _manage_models(store)




def get_active_workspace() -> str:
    """
    Parses CLI arguments. Handles -v/--version, --config, --serve, --mcp, --update, --check-update, --help/-h flags.
    """
    if len(sys.argv) > 1:
        cli_str = " ".join(sys.argv[1:])
        from any_context.help import handle_command_help_interception
        if handle_command_help_interception(cli_str):
            sys.exit(0)

    parser = argparse.ArgumentParser(description="Start the AnyContext AI Agent.", add_help=False)

    parser.add_argument(
        "-w", "--workspace", 
        type=str, 
        help="Specify the active workspace to use for this session.", 
        default=None
    )
    parser.add_argument(
        "-c", "--config", 
        action="store_true", 
        help="Open the interactive configuration management menu."
    )
    parser.add_argument(
        "-v", "--version", 
        action="store_true", 
        help="Show AnyContext version information."
    )
    parser.add_argument(
        "--serve", "--server",
        dest="serve",
        action="store_true", 
        help="Start the AnyContext REST API Server for external app connections."
    )
    parser.add_argument(
        "--port", 
        type=int, 
        default=8000, 
        help="Port to listen on for REST API server (default: 8000)."
    )
    parser.add_argument(
        "--host", 
        type=str, 
        default="127.0.0.1", 
        help="Host address to bind REST API server (default: 127.0.0.1)."
    )
    parser.add_argument(
        "--mcp", 
        action="store_true", 
        help="Start the AnyContext Model Context Protocol (MCP) Server on stdio."
    )
    parser.add_argument(
        "--update", 
        action="store_true", 
        help="Update AnyContext to the latest released version."
    )
    parser.add_argument(
        "--check-update", 
        action="store_true", 
        help="Check if a newer version of AnyContext is available."
    )
    parser.add_argument(
        "--billing", "--plans",
        dest="billing",
        action="store_true",
        help="View and manage AnyContext subscription plan tiers, pricing, and capabilities."
    )
    parser.add_argument(
        "--factory-reset", 
        action="store_true", 
        help="Wipe all settings, API keys, workspaces, and vector databases, resetting to factory defaults."
    )
    parser.add_argument(
        "--reset-models", 
        action="store_true", 
        help="Reset AI model settings and API keys to OpenAI factory defaults while preserving workspaces and vector history."
    )
    
    args, unknown = parser.parse_known_args()

    if args.version:
        print(f"AnyContext (actx) v{__version__} - Levix Digital")
        sys.exit(0)

    if args.billing:
        from any_context.cli.config_menu import _manage_subscription
        _manage_subscription()
        sys.exit(0)

    if args.reset_models:
        store = ConfigDBStore()
        store.reset_model_settings_to_default()
        print("\n🧹 Model settings and API keys reset to OpenAI factory defaults!")
        print("📂 Workspaces and vector history preserved.")
        sys.exit(0)

    if args.factory_reset:


        confirm = questionary.confirm(
            "⚠️ DANGER: Are you sure you want to reset AnyContext to Factory Defaults?\n  This will erase ALL workspaces, folders, API keys, configuration settings, and vector memory databases!"
        ).ask()
        if confirm:
            store = ConfigDBStore()
            store.factory_reset()
            print("\n🎉 AnyContext has been completely reset to factory defaults!")
            print("Run 'actx' again anytime to launch the first-time setup wizard.\n")
        sys.exit(0)


    if args.serve or "server" in unknown or "serve" in unknown:
        from any_context.server.api import start_api_server
        start_api_server(host=args.host, port=args.port)
        sys.exit(0)

    if args.mcp:
        from any_context.server.mcp import start_mcp_server
        start_mcp_server()
        sys.exit(0)
    
    if args.update:
        run_self_update()
        sys.exit(0)

    if args.check_update:
        check_for_updates(quiet_if_latest=False)
        sys.exit(0)

    if args.config:
        show_config_menu()
        sys.exit(0)

    store = ConfigDBStore()
    if store.is_empty():
        run_first_time_wizard()

    ensure_api_key_configured()
    
    if args.workspace:
        return args.workspace
        
    return show_workspace_menu()

