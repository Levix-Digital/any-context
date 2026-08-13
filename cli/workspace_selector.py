import argparse
import sys
import questionary
from config.app_settings import AppSettings

def show_workspace_menu() -> str:
    """
    Displays an interactive menu for the user to select a workspace.
    """
    settings = AppSettings.load()
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
    Parses CLI arguments. If a workspace is provided, it uses it.
    Otherwise, it prompts the user to select one.
    """
    parser = argparse.ArgumentParser(description="Start the AnyContext AI Agent.")
    parser.add_argument(
        "-w", "--workspace", 
        type=str, 
        help="Specify the active workspace to use for this session.", 
        default=None
    )
    
    args = parser.parse_args()
    
    if args.workspace:
        return args.workspace
        
    return show_workspace_menu()
