"""
AnyContext Per-Workspace Input History Manager.
Provides isolated, persistent command and prompt history per workspace with Arrow Up / Down navigation.
"""
import os
import sys
from typing import Optional, List, Dict, Any


def get_history_dir() -> str:
    """Returns the directory where per-workspace history files are stored."""
    history_dir = os.path.expanduser("~/.any_context/history")
    os.makedirs(history_dir, exist_ok=True)
    return history_dir


def get_workspace_history_file(workspace_name: Optional[str]) -> str:
    """
    Returns the absolute path to the .history file for a given workspace.
    Sanitizes workspace names to prevent filesystem collision or invalid characters.
    """
    ws = (workspace_name or "Global").strip()
    safe_name = "".join(c if (c.isalnum() or c in ("-", "_")) else "_" for c in ws)
    return os.path.join(get_history_dir(), f"{safe_name}.history")


def get_workspace_history_entries(workspace_name: Optional[str], limit: int = 50) -> List[str]:
    """
    Returns the most recent input history entries for a given workspace in chronological order.
    """
    history_file = get_workspace_history_file(workspace_name)
    if not os.path.exists(history_file):
        return []
    try:
        from prompt_toolkit.history import FileHistory
        h = FileHistory(history_file)
        # load_history_strings yields newest first; reverse to preserve chronological order
        entries = list(reversed(list(h.load_history_strings())))
        return entries[-limit:] if limit else entries
    except Exception:
        return []


def clear_workspace_history(workspace_name: Optional[str]) -> bool:
    """
    Clears the history file for a specific workspace.
    """
    history_file = get_workspace_history_file(workspace_name)
    if os.path.exists(history_file):
        try:
            os.remove(history_file)
            return True
        except Exception:
            return False
    return True


class WorkspaceHistoryManager:
    """
    Manages prompt_toolkit PromptSession instances keyed by workspace name.
    Ensures that Arrow Up / Down navigates strictly within the active workspace's past prompts.
    """
    def __init__(self):
        self._sessions: Dict[str, Any] = {}

    def get_session(self, workspace_name: Optional[str]):
        ws = (workspace_name or "Global").strip()
        if ws in self._sessions:
            return self._sessions[ws]

        try:
            from prompt_toolkit import PromptSession
            from prompt_toolkit.history import FileHistory
            
            history_file = get_workspace_history_file(ws)
            sess = PromptSession(history=FileHistory(history_file))
            self._sessions[ws] = sess
            return sess
        except Exception:
            return None

    def prompt(self, prompt_text: str, workspace_name: Optional[str] = None) -> Optional[str]:
        """
        Prompts user for input using the active workspace's PromptSession.
        Gracefully falls back to standard input() if prompt_toolkit or TTY is unavailable.
        """
        try:
            sess = self.get_session(workspace_name)
            if sess and sys.stdin.isatty():
                from prompt_toolkit.formatted_text import ANSI
                return sess.prompt(ANSI(prompt_text))
            return input(prompt_text)
        except (KeyboardInterrupt, EOFError):
            print()
            try:
                confirm_ans = input("\033[93m❓ Are you sure you want to exit AnyContext? [y/N]:\033[0m ").strip().lower()
                if confirm_ans in ["y", "yes", "s", "sim"]:
                    return "/exit"
                print("↩️ Resuming session...\n")
                return None
            except (KeyboardInterrupt, EOFError):
                return "/exit"


# Global singleton instance
history_manager = WorkspaceHistoryManager()


def safe_prompt_input(prompt_text: str, workspace_name: Optional[str] = None) -> Optional[str]:
    """
    Convenience wrapper to safely read user input with per-workspace history navigation.
    """
    return history_manager.prompt(prompt_text, workspace_name=workspace_name)
