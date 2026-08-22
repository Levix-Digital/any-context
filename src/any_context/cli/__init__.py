"""
AnyContext CLI Package
"""

def __getattr__(name: str):
    if name in ["run_chat_loop", "main", "main_cli"]:
        from any_context.cli.chat_loop import run_chat_loop, main, main_cli
        mapping = {"run_chat_loop": run_chat_loop, "main": main, "main_cli": main_cli}
        return mapping[name]
    elif name in ["show_workspace_menu", "get_active_workspace"]:
        from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
        mapping = {"show_workspace_menu": show_workspace_menu, "get_active_workspace": get_active_workspace}
        return mapping[name]
    elif name in ["format_sync_status_box", "format_pricing_plans_cli", "run_interactive_web_crawler", "display_help_page", "show_interactive_help_menu"]:
        from any_context.cli import formatters
        return getattr(formatters, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


__all__ = [
    "run_chat_loop",
    "main",
    "main_cli",
    "show_workspace_menu",
    "get_active_workspace",
    "Spinner",
    "format_sync_status_box",
    "format_pricing_plans_cli",
    "run_interactive_web_crawler",
    "display_help_page",
    "show_interactive_help_menu"
]
