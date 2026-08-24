"""
AnyContext Terminal Viewport & Pinned Bottom Dock.
Provides a persistent bottom status dock and scrolling region (DECSTBM)
so that the status toolbar and user prompt line remain anchored and visible
during AI token generation, search execution, and streaming responses.
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
    Context manager that pins a status toolbar dock at the bottom of the terminal screen
    using ANSI Scrolling Margins (DECSTBM: \\033[top;bottom r).
    Ensures that streamed tokens and tool activity scroll smoothly in the upper region
    without erasing, pushing off, or blinking the bottom status dock.
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
        self._is_active = False
        self._rows = 24
        self._cols = 100

    def _render_dock_lines(self, status_override: Optional[str] = None):
        """Builds ANSI strings for the horizontal divider and status dock bar."""
        cols = self._cols
        divider_char = "─"
        divider_line = f"\033[90m{divider_char * max(cols, 20)}\033[0m"

        search_badge = (
            "\033[1;36m🌐 Search: ON\033[0m"
            if self.web_search_enabled
            else "\033[90m🌐 Search: OFF\033[0m"
        )

        status_badge = ""
        if status_override:
            status_badge = f"  \033[90m│\033[0m  \033[1;33m{status_override}\033[0m"

        left_content = (
            f" \033[1;33m📂 {self.workspace_name}\033[0m  "
            f"\033[90m│\033[0m  "
            f"\033[1;35m🤖 {self.model_name}\033[0m  "
            f"\033[90m│\033[0m  "
            f"\033[1;34m🛡️ {self.grounding_mode}\033[0m  "
            f"\033[90m│\033[0m  "
            f"{search_badge}"
            f"{status_badge}"
        )

        right_content = "\033[1;31m🚪 /exit\033[0m "

        vis_left = _visible_len(left_content)
        vis_right = _visible_len(right_content)
        pad_count = max(2, cols - vis_left - vis_right - 1)
        padding = " " * pad_count

        status_line = f"{left_content}{padding}{right_content}"
        return divider_line, status_line

    def __enter__(self):
        try:
            is_tty = hasattr(sys.stdout, "isatty") and sys.stdout.isatty()
            if not is_tty:
                return self

            sz = shutil.get_terminal_size((100, 24))
            self._cols = sz.columns
            self._rows = sz.lines

            if self._rows < 10 or self._cols < 30:
                # Terminal too small for margins, fallback to standard stream
                return self

            scroll_bottom = self._rows - self.dock_height
            divider_line, status_line = self._render_dock_lines()

            # 1. Save cursor position
            sys.stdout.write("\033[s")
            # 2. Draw divider on line (rows - 1)
            sys.stdout.write(f"\033[{self._rows - 1};1H\033[K{divider_line}")
            # 3. Draw status toolbar on line (rows)
            sys.stdout.write(f"\033[{self._rows};1H\033[K{status_line}")
            # 4. Set scrolling region for top content (lines 1 to rows - 2)
            sys.stdout.write(f"\033[1;{scroll_bottom}r")
            # 5. Restore cursor or place in content area
            sys.stdout.write(f"\033[{scroll_bottom};1H")
            sys.stdout.flush()

            self._is_active = True
        except Exception:
            self._is_active = False

        return self

    def write(self, text: str):
        """Safely writes streamed text to stdout."""
        if not text:
            return
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
        """Dynamically updates the pinned bottom status bar without disrupting the stream."""
        if not self._is_active:
            return
        try:
            _, updated_status_line = self._render_dock_lines(status_override=status_text)
            # Save cursor, jump to bottom row, redraw status, restore cursor
            sys.stdout.write(f"\033[s\033[{self._rows};1H\033[K{updated_status_line}\033[u")
            sys.stdout.flush()
        except Exception:
            pass

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._is_active:
            try:
                # Reset terminal scrolling margins to full screen
                sys.stdout.write("\033[r")
                sys.stdout.flush()
            except Exception:
                pass
            self._is_active = False
        return False
