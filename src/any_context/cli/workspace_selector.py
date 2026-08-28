import argparse
import sys
from typing import Optional, List
import questionary
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.cli.config_menu import run_first_time_wizard, show_config_menu
from any_context.cli.updater import check_for_updates, run_self_update
from any_context.cli.spinner import Spinner
from any_context import __version__

def show_workspace_menu() -> Optional[str]:
    """
    Displays an interactive menu for the user to select an existing workspace or create a new one.
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
    choices = workspace_names + ["➕ Create New Workspace", "🔙 Cancel"]
    
    selected = None
    try:
        selected = questionary.select(
            "Select the active workspace:",
            choices=choices
        ).ask()
    except Exception:
        selected = None
    
    if not selected or selected.startswith("🔙"):
        return None

    if selected.startswith("➕"):
        new_name = questionary.text("Enter New Workspace Name:").ask()
        if new_name and new_name.strip():
            clean_name = new_name.strip()
            store.add_workspace(clean_name, paths=[])
            print(f"✅ Created workspace '{clean_name}'.\n")
            try:
                available_sources = store.list_all_available_shared_sources()
                if available_sources:
                    link_confirm = questionary.confirm("📚 Would you like to link reusable Shared Sources to this new workspace?").ask()
                    if link_confirm:
                        source_choices = [
                            f"[{s.get('type', 'folder').upper()}] {s.get('title', s.get('identifier'))} (from '{s.get('origin_workspace')}')"
                            for s in available_sources
                        ]
                        selected_links = questionary.checkbox("Select Shared Sources to link (Use Arrow Keys, Space to select, Enter to confirm):", choices=source_choices).ask()
                        if selected_links:
                            for ch in selected_links:
                                idx = source_choices.index(ch)
                                src = available_sources[idx]
                                store.link_shared_source_to_workspace(
                                    workspace_name=clean_name,
                                    source_type=src["type"],
                                    source_identifier=src["identifier"],
                                    title=src.get("title")
                                )
                            print(f"🔗 Linked {len(selected_links)} Shared Sources to '{clean_name}' ($0.00 cost).\n")
                        else:
                            print(f"ℹ️ No sources selected. Workspace '{clean_name}' created as empty. (Tip: Type '/link' anytime in chat to connect sources).\n")
            except Exception:
                pass
            return clean_name
        return None

    return selected

def ensure_api_key_configured():
    """
    Checks if first-time onboarding or API key configuration is required.
    Delegates to OnboardingService and presents interactive options via questionary.
    """
    from any_context.core.services.onboarding_service import OnboardingService
    onboarding_svc = OnboardingService()
    status = onboarding_svc.check_status()

    if not status.needs_onboarding:
        return

    print("\n======================================================================")
    print(status.title)
    print(status.description)
    print("======================================================================\n")

    choices = [opt.title for opt in status.options_group.items]
    choice = questionary.select(
        "How would you like to configure your AI Provider?",
        choices=choices
    ).ask()

    if not choice:
        return

    if choice.startswith("⚡"):
        entered_key = questionary.password("Enter your OpenAI API Key (sk-...):").ask()
        if entered_key and entered_key.strip():
            res = onboarding_svc.complete_onboarding("openai", api_key=entered_key.strip())
            if res.success:
                print(f"{res.message}\n")
            else:
                print(f"❌ Error: {res.error}\n")
        else:
            print("⚠️ Notice: No OpenAI API Key entered. Opening Custom Setup Menu...")
            from any_context.cli.config_menu import _manage_models
            _manage_models(onboarding_svc.store)
            onboarding_svc.store.set_onboarding_completed(True)
    elif choice.startswith("🏠"):
        res = onboarding_svc.complete_onboarding("local_offline")
        if res.success:
            print(f"{res.message}\n")
        else:
            print(f"❌ Error: {res.error}\n")
    elif choice.startswith("🛠️"):
        from any_context.cli.config_menu import _manage_models
        _manage_models(onboarding_svc.store)
        onboarding_svc.store.set_onboarding_completed(True)




def get_active_workspace() -> str:
    """
    Parses CLI arguments. Handles -v/--version, --config, --serve, --mcp, --update, --check-update, --help/-h flags.
    """
    if len(sys.argv) > 1:
        cli_str = " ".join(sys.argv[1:])
        from any_context.help import handle_command_help_interception
        if handle_command_help_interception(cli_str):
            sys.exit(0)

    # Intercept targeted update syntax: --update@0.15.2, -u@0.15.2, --update@latest
    for arg in sys.argv[1:]:
        if arg.startswith("--update@") or arg.startswith("-u@"):
            target_v = arg.split("@", 1)[1]
            from any_context.cli.updater import run_self_update
            is_close = "--close-instances" in sys.argv or "-c" in sys.argv
            is_bg = "--background" in sys.argv or "--force-background" in sys.argv
            is_force = "--force" in sys.argv or "-f" in sys.argv
            run_self_update(target_version=target_v, auto_close_instances=is_close, force_background=is_bg, force=is_force)
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
        "--rpc", 
        action="store_true", 
        help="Start the AnyContext Stdio RPC Bridge Server for OpenTUI."
    )

    parser.add_argument(
        "--update", "-u",
        dest="update",
        nargs="?",
        const="LATEST_FLAG_DEFAULT",
        default=None,
        help="Update AnyContext to the latest or specific version (e.g. --update, --update@0.15.2, --update 0.15.2, --update --list)."
    )
    parser.add_argument(
        "--to", "--target-version",
        dest="to_version",
        type=str,
        default=None,
        help="Target version to update or rollback to."
    )
    parser.add_argument(
        "--releases", "--list-releases",
        dest="list_releases",
        action="store_true",
        help="List available AnyContext releases from GitHub."
    )
    parser.add_argument(
        "--rollback",
        action="store_true",
        help="Interactive rollback to a previous AnyContext release."
    )
    parser.add_argument(
        "--force", "-f",
        dest="force",
        action="store_true",
        help="Force reinstall or overwrite current version during update."
    )
    parser.add_argument(
        "--check-update", 
        action="store_true", 
        help="Check if a newer version of AnyContext is available."
    )
    parser.add_argument(
        "--close-instances",
        action="store_true",
        help="Automatically terminate active AnyContext sessions during update."
    )
    parser.add_argument(
        "--background", "--force-background",
        dest="force_background",
        action="store_true",
        help="Perform update in background keeping active sessions running."
    )
    parser.add_argument(
        "--billing", "--plans",
        dest="billing",
        action="store_true",
        help="View and manage AnyContext subscription plan tiers, pricing, and capabilities."
    )
    parser.add_argument(
        "--keys", "--api-keys",
        dest="keys",
        action="store_true",
        help="Display the comprehensive guide on obtaining API keys for all AI providers."
    )
    parser.add_argument(
        "--web",
        action="store_true",
        help="Manage web sources and URL scraping for workspaces."
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

    if args.keys:
        from any_context.help.manager import display_help_page
        from any_context.help.registry import get_help_page
        page = get_help_page("api-keys")
        if page:
            display_help_page(page)
        sys.exit(0)

    if args.billing:
        from any_context.cli.config_menu import _manage_subscription
        _manage_subscription()
        sys.exit(0)

    if args.web:
        from any_context.cli.config_menu import _manage_workspace_web_urls
        _manage_workspace_web_urls(workspace_name=args.workspace)
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
    if getattr(args, "rpc", False) or "--rpc" in sys.argv:
        from any_context.server.rpc_bridge import run_rpc_server
        target_ws = args.workspace or (unknown[0] if unknown and not unknown[0].startswith("-") else "Default")
        run_rpc_server(default_workspace=target_ws)
        sys.exit(0)


    # 1. Releases listing or interactive rollback
    if args.list_releases or args.rollback or (args.update and str(args.update).lower() in ["list", "--list", "-l", "releases", "rollback", "--rollback", "-r"]):
        from any_context.cli.updater import display_available_releases, run_self_update
        picked = display_available_releases(interactive_select=True)
        if picked:
            run_self_update(
                target_version=picked,
                auto_close_instances=getattr(args, "close_instances", False),
                force_background=getattr(args, "force_background", False),
                force=getattr(args, "force", False)
            )
        sys.exit(0)

    # 2. Targeted or Latest self-update
    if args.update is not None or args.to_version is not None:
        target_v = None
        if args.update and args.update != "LATEST_FLAG_DEFAULT":
            target_v = args.update
        elif args.to_version:
            target_v = args.to_version

        from any_context.cli.updater import run_self_update
        run_self_update(
            target_version=target_v,
            auto_close_instances=getattr(args, "close_instances", False),
            force_background=getattr(args, "force_background", False),
            force=getattr(args, "force", False)
        )
        sys.exit(0)

    if args.check_update:
        from any_context.cli.updater import check_for_updates, run_self_update
        has_up, new_tag = check_for_updates(quiet_if_latest=False)
        if has_up:
            try:
                import questionary
                do_upgrade = questionary.confirm(
                    f"Would you like to download and install {new_tag} now?",
                    default=True
                ).ask()
                if do_upgrade:
                    run_self_update(
                        auto_close_instances=getattr(args, "close_instances", False),
                        force_background=getattr(args, "force_background", False),
                        force=getattr(args, "force", False)
                    )
            except Exception:
                pass
        sys.exit(0)

    if args.config:
        show_config_menu()
        sys.exit(0)

    store = ConfigDBStore()
    if store.is_empty():
        run_first_time_wizard()

    ensure_api_key_configured()
    
    if args.workspace and args.workspace.strip():
        clean_ws = args.workspace.strip()
        store.add_workspace(clean_ws, paths=[])
        return clean_ws
        
    return "Default"

