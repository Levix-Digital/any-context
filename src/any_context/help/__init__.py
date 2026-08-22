"""
AnyContext Architectural Help & Documentation Module
"""

from any_context.help.models import HelpPage
from any_context.help.registry import HELP_REGISTRY, get_help_page

def __getattr__(name: str):
    if name in ["display_help_page", "show_interactive_help_menu", "handle_command_help_interception", "safe_print"]:
        from any_context.help import manager
        return getattr(manager, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

__all__ = [
    "HelpPage",
    "HELP_REGISTRY",
    "get_help_page",
    "display_help_page",
    "show_interactive_help_menu",
    "handle_command_help_interception"
]

