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


def create_workspace_prompt_session(history_file: str):
    """
    Creates a configured prompt_toolkit PromptSession supporting:
    - Bracketed Paste: Pasting multiline text does not prematurely submit.
    - Alt+Enter / Ctrl+J: Inserts manual newlines.
    - Enter: Submits the prompt buffer.
    - Continuation Prompt: '... ' for subsequent lines.
    """
    try:
        from any_context.cli.entrypoint import _patch_prompt_toolkit_for_git_bash
        _patch_prompt_toolkit_for_git_bash()
    except Exception:
        pass

    try:
        from prompt_toolkit import PromptSession
        from prompt_toolkit.history import FileHistory
        from prompt_toolkit.key_binding import KeyBindings

        kb = KeyBindings()

        # Alt+Enter (Escape followed by Enter) inserts a newline into the prompt buffer
        @kb.add("escape", "enter")
        def _insert_newline_alt_enter(event):
            event.current_buffer.insert_text("\n")

        # Ctrl+J inserts a newline into the prompt buffer
        @kb.add("c-j")
        def _insert_newline_ctrl_j(event):
            event.current_buffer.insert_text("\n")

        # Enter submits the buffer immediately
        @kb.add("enter")
        def _submit_buffer(event):
            event.current_buffer.validate_and_handle()

        from prompt_toolkit.formatted_text import ANSI

        def prompt_continuation(width, line_number, is_soft_wrap):
            return ANSI("\033[90m... \033[0m")

        return PromptSession(
            history=FileHistory(history_file),
            key_bindings=kb,
            multiline=True,
            prompt_continuation=prompt_continuation,
            enable_history_search=True
        )
    except Exception:
        return None


def get_default_prompt_style():
    """Returns elegant, modern theme colors for prompt_toolkit bottom toolbar."""
    try:
        from prompt_toolkit.styles import Style
        return Style.from_dict({
            "bottom-toolbar": "bg:#1a1b26 #a9b1d6",
            "bottom-toolbar.text": "#a9b1d6",
            "bottom-toolbar.ws": "fg:#e0af68 bold",
            "bottom-toolbar.model": "fg:#bb9af7 bold",
            "bottom-toolbar.mode": "fg:#7dcfff bold",
            "bottom-toolbar.sync": "bg:#e0af68 fg:#1a1b26 bold",
            "bottom-toolbar.dim": "fg:#565f89",
            "bottom-toolbar.cmd": "fg:#73daca bold",
        })
    except Exception:
        return None


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

        history_file = get_workspace_history_file(ws)
        sess = create_workspace_prompt_session(history_file)
        if sess:
            self._sessions[ws] = sess
        return sess

    def prompt(
        self,
        prompt_text: str,
        workspace_name: Optional[str] = None,
        bottom_toolbar: Optional[Any] = None,
        style: Optional[Any] = None
    ) -> Optional[str]:
        """
        Prompts user for input using the active workspace's PromptSession.
        Supports rich bottom_toolbar (docked at the bottom of the screen) and custom styling.
        Gracefully falls back to standard input() if prompt_toolkit or TTY is unavailable.
        """
        try:
            sess = self.get_session(workspace_name)
            if sess and sys.stdin.isatty():
                from prompt_toolkit.formatted_text import ANSI
                prompt_kwargs = {}
                if bottom_toolbar is not None:
                    prompt_kwargs["bottom_toolbar"] = bottom_toolbar
                if style is not None:
                    prompt_kwargs["style"] = style
                else:
                    def_style = get_default_prompt_style()
                    if def_style:
                        prompt_kwargs["style"] = def_style
                return sess.prompt(ANSI(prompt_text), **prompt_kwargs)
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


def safe_prompt_input(
    prompt_text: str,
    workspace_name: Optional[str] = None,
    bottom_toolbar: Optional[Any] = None,
    style: Optional[Any] = None
) -> Optional[str]:
    """
    Convenience wrapper to safely read user input with per-workspace history navigation and bottom toolbar.
    """
    return history_manager.prompt(
        prompt_text,
        workspace_name=workspace_name,
        bottom_toolbar=bottom_toolbar,
        style=style
    )
