"""
Universal Command Adapter for AnyContext.
Translates slash commands into Core Application Service executions with rich structured results.
"""

from any_context.commands.registry import (
    CommandMeta,
    COMMANDS_REGISTRY,
    find_command_meta
)
from any_context.commands.result import CommandResult
from any_context.commands.dispatcher import (
    CommandDispatcher,
    dispatch_command,
    parse_args
)

__all__ = [
    "CommandMeta",
    "COMMANDS_REGISTRY",
    "find_command_meta",
    "CommandResult",
    "CommandDispatcher",
    "dispatch_command",
    "parse_args"
]
