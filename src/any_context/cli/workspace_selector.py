import argparse
import sys
import questionary
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.cli.config_menu import run_first_time_wizard, show_config_menu
from any_context.cli.updater import check_for_updates, run_self_update

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
    Parses CLI arguments. Handles --config, --update, --check-update flags or runs first-time setup if empty.
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
        "--update", 
        action="store_true", 
        help="Update AnyContext to the latest released version."
    )
    parser.add_argument(
        "--check-update", 
        action="store_true", 
        help="Check if a newer version of AnyContext is available."
    )
    
    args, unknown = parser.parse_known_args()
    
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
