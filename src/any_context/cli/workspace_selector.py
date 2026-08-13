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

def get_active_workspace() -> str:
    """
    Parses CLI arguments. Handles -v/--version, --config, --serve, --mcp, --update, --check-update flags or runs first-time setup if empty.
    """
    parser = argparse.ArgumentParser(description="Start the AnyContext AI Agent.")
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
        "--factory-reset", 
        action="store_true", 
        help="Wipe all settings, API keys, workspaces, and vector databases, resetting to factory defaults."
    )
    
    args, unknown = parser.parse_known_args()

    if args.version:
        print(f"AnyContext (actx) v{__version__} - Levix Digital")
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
    
    if args.workspace:
        return args.workspace
        
    return show_workspace_menu()
