"""
AnyContext Terminal Viewport & Live Stream Presenter.
Provides smooth, cursor-aware streaming output and dynamic status line rendering
without disruptive DECSTBM scrolling margin resets, preserving full terminal
scrollback history and enabling natural top-down growth across conversation turns.
"""

import sys
import shutil
import unicodedata
import re
from typing import Optional


def _char_width(c: str) -> int:
    if unicodedata.east_asian_width(c) in ('F', 'W'):
        return 2
    if ord(c) > 0x2000 and unicodedata.category(c) in ('So', 'Sk'):
        return 2
    return 1


def _visible_len(s: str) -> int:
    clean = re.sub(r'\033\[[0-9;]*[a-zA-Z]', '', s)
    return sum(_char_width(c) for c in clean)


class PinnedBottomDock:
    """
    Cursor-aware live presentation context manager.
    Streams AI tokens and status tickers inline at the active cursor position,
    allowing the conversation to grow naturally from top to bottom.
    Preserves 100% of terminal scrollback history and prevents erratic screen jumps.
    """

    def __init__(
        self,
        workspace_name: str = "Default",
        model_name: str = "gpt-4o-mini",
        grounding_mode: str = "strict",
        web_search_enabled: bool = False,
        dock_height: int = 2
    ):
        self.workspace_name = workspace_name or "Default"
        self.model_name = model_name or "gpt-4o-mini"
        self.grounding_mode = (grounding_mode or "strict").capitalize()
        self.web_search_enabled = bool(web_search_enabled)
        self.dock_height = max(1, min(4, dock_height))
        self._last_status = ""
        self._has_active_status_line = False
        self._cols = 100

    def _get_terminal_cols(self) -> int:
        try:
            return shutil.get_terminal_size((100, 24)).columns
        except Exception:
            return 100

    def __enter__(self):
        self._cols = self._get_terminal_cols()
        self._has_active_status_line = False
        self._last_status = ""
        return self

    def write(self, text: str):
        """
        Safely writes streamed text to stdout.
        Clears any transient live status line before outputting new stream content.
        """
        if not text:
            return

        if self._has_active_status_line:
            try:
                sys.stdout.write("\r\033[K")
                sys.stdout.flush()
            except Exception:
                pass
            self._has_active_status_line = False

        try:
            sys.stdout.write(text)
            sys.stdout.flush()
        except Exception:
            try:
                sys.stdout.buffer.write(text.encode("utf-8", errors="replace"))
                sys.stdout.flush()
            except Exception:
                pass

    def update_status(self, status_text: str):
        """
        Dynamically renders an inline live status ticker at the current cursor line.
        """
        if not status_text:
            return

        self._last_status = status_text
        self._cols = self._get_terminal_cols()
        formatted_status = f"\r\033[K\033[90m⚡ \033[1;33m{status_text}\033[0m"

        try:
            sys.stdout.write(formatted_status)
            sys.stdout.flush()
            self._has_active_status_line = True
        except Exception:
            pass

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._has_active_status_line:
            try:
                sys.stdout.write("\r\033[K")
                sys.stdout.flush()
            except Exception:
                pass
            self._has_active_status_line = False
        return False
