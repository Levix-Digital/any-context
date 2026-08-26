"""
CommandResult - Structured result object returned by the Command Dispatcher.
Decoupled: consumable by CLI, OpenTUI, RPC Bridge, REST API, and MCP tools.
"""

from dataclasses import dataclass, field
from typing import Dict, Any, Optional


@dataclass
class CommandResult:
    """Standardized output of command execution."""
    success: bool = True
    message: str = ""
    state_updates: Dict[str, Any] = field(default_factory=dict)
    action: Optional[str] = None  # e.g. "exit", "clear", "switch_workspace", "paste_mode"
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Converts the result to a JSON-serializable dictionary."""
        return {
            "success": self.success,
            "message": self.message,
            "state_updates": self.state_updates,
            "action": self.action,
            "error": self.error
        }
