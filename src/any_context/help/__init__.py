"""
AnyContext Architectural Help & Documentation Module
"""

from any_context.help.models import HelpPage
from any_context.help.registry import HELP_REGISTRY, get_help_page
from any_context.help.manager import (
    display_help_page,
    show_interactive_help_menu,
    handle_command_help_interception
)

__all__ = [
    "HelpPage",
    "HELP_REGISTRY",
    "get_help_page",
    "display_help_page",
    "show_interactive_help_menu",
    "handle_command_help_interception"
]
