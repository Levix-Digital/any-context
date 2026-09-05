"""
AnyContext CLI Package
"""

def __getattr__(name: str):
    if name == "launch_opentui":
        from any_context.cli.tui_launcher import launch_opentui
        return launch_opentui
    elif name in ["main", "main_cli", "entrypoint"]:
        from any_context.cli.entrypoint import entrypoint
        return entrypoint
    elif name in ["safe_stdout_write", "format_session_error"]:
        from any_context.cli.utils import safe_stdout_write, format_session_error
        mapping = {"safe_stdout_write": safe_stdout_write, "format_session_error": format_session_error}
        return mapping[name]
    elif name in ["show_workspace_menu", "get_active_workspace"]:
        from any_context.cli.workspace_selector import show_workspace_menu, get_active_workspace
        mapping = {"show_workspace_menu": show_workspace_menu, "get_active_workspace": get_active_workspace}
        return mapping[name]
    elif name in ["format_sync_status_box", "format_pricing_plans_cli", "run_interactive_web_crawler", "display_help_page", "show_interactive_help_menu"]:
        from any_context.cli import formatters
        return getattr(formatters, name)
    elif name == "TwoStageProgressRenderer":
        from any_context.cli.progress import TwoStageProgressRenderer
        return TwoStageProgressRenderer
    elif name == "Spinner":
        from any_context.cli.spinner import Spinner
        return Spinner
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


__all__ = [
    "launch_opentui",
    "entrypoint",
    "main",
    "main_cli",
    "safe_stdout_write",
    "format_session_error",
    "show_workspace_menu",
    "get_active_workspace",
    "Spinner",
    "TwoStageProgressRenderer",
    "format_sync_status_box",
    "format_pricing_plans_cli",
    "run_interactive_web_crawler",
    "display_help_page",
    "show_interactive_help_menu",
]
