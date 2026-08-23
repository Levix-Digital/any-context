"""
Universal CLI Two-Stage Progress Renderer (v0.24.2).
Provides a unified, thread-safe, and decoupled terminal progress bar and live spinner ticker
for all AnyContext data ingestion pipelines (Local Folders, Web Portals, Cloud Drives).
Part of the Hexagonal Architecture CLI Presentation Adapter layer.
"""
import sys
import threading
from typing import Optional, Callable


def safe_stdout_write(msg: str):
    """Safely writes strings to stdout handling legacy Windows CP1252 encoding errors."""
    try:
        sys.stdout.write(msg)
        sys.stdout.flush()
    except (UnicodeEncodeError, Exception):
        try:
            clean_msg = msg.encode("ascii", errors="ignore").decode("ascii")
            sys.stdout.write(clean_msg)
            sys.stdout.flush()
        except Exception:
            pass


class TwoStageProgressRenderer:
    """
    Unified dual-stage progress bar and live ticker for the CLI.
    
    Stage 1: Collection / Discovery / Crawling / Download
      - Displays: [1/2 <Stage>] [████░░░░] 45/100 (45%) • 40 new, 5 cached • item.txt
    
    Stage 2: AI Contextual Enrichment & Vector Embeddings
      - Displays: [2/2 Embedding] [██████░░░░] 60/100 (180/300 chunks) (60%) • Vector Knowledge Base
    """
    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    def __init__(
        self,
        stage1_label: str = "Collecting",
        bar_len: int = 14,
        auto_hide_cursor: bool = True
    ):
        self.stage1_label = stage1_label
        self.bar_len = bar_len
        self.auto_hide_cursor = auto_hide_cursor
        self._lock = threading.Lock()
        self._frame_idx = 0
        self._is_active = False

    def __enter__(self):
        self._is_active = True
        if self.auto_hide_cursor:
            safe_stdout_write("\033[?25l")
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.finish()

    def update_stage1(
        self,
        current: int,
        total: int,
        indexed: int = 0,
        skipped: int = 0,
        item_name: str = "",
        stage_name: Optional[str] = None
    ):
        """Renders Stage 1 progress update (Discovery, Crawling, File Scanning, Cloud Download)."""
        with self._lock:
            self._frame_idx += 1
            frame = self.SPINNER_FRAMES[self._frame_idx % len(self.SPINNER_FRAMES)]
            pct = int((current / total) * 100) if total > 0 else (100 if current > 0 else 0)
            filled = int((pct / 100) * self.bar_len)
            bar = "█" * filled + "░" * (self.bar_len - filled)
            st_label = stage_name or self.stage1_label

            display_item = item_name or ""
            if len(display_item) > 30:
                display_item = display_item[:12] + "..." + display_item[-15:]

            status_parts = []
            if indexed > 0 or skipped > 0:
                if indexed > 0:
                    status_parts.append(f"{indexed} new")
                if skipped > 0:
                    status_parts.append(f"{skipped} cached")
            status_text = f" • \033[93m{', '.join(status_parts)}\033[0m" if status_parts else ""
            item_text = f" • \033[90m{display_item}\033[0m" if display_item else ""

            safe_stdout_write(
                f"\r\033[K\033[96m{frame}\033[0m [1/2 {st_label}] [{bar}] {current}/{total} ({pct}%){status_text}{item_text}"
            )

    def update_stage2(
        self,
        current: int,
        total: int,
        chunk_curr: int = 0,
        chunk_total: int = 0,
        stage_detail: str = "Vector Knowledge Base"
    ):
        """Renders Stage 2 progress update (Enrichment, Vector Embeddings, LanceDB persistence)."""
        with self._lock:
            self._frame_idx += 1
            frame = self.SPINNER_FRAMES[self._frame_idx % len(self.SPINNER_FRAMES)]
            pct = int((current / total) * 100) if total > 0 else (100 if current > 0 else 0)
            filled = int((pct / 100) * self.bar_len)
            bar = "█" * filled + "░" * (self.bar_len - filled)

            chunks_label = f" ({chunk_curr}/{chunk_total} chunks)" if chunk_total > 0 else ""
            safe_stdout_write(
                f"\r\033[K\033[95m{frame}\033[0m [2/2 Embedding] [{bar}] {current}/{total} items{chunks_label} ({pct}%) • \033[92m{stage_detail}\033[0m"
            )

    def create_stage1_callback(self, stage_name: Optional[str] = None) -> Callable:
        """Returns a standardized progress callback for Stage 1 ingestors."""
        st_name = stage_name or self.stage1_label

        def _callback(current: int, total: int, indexed: int = 0, skipped: int = 0, item_name: str = "", detail: str = ""):
            # Flexible signature handling both 4-arg (current, total, stage, item) and 6-arg web formats
            if isinstance(indexed, str) and not isinstance(skipped, int):
                # Called as (current, total, stage, item)
                stage = indexed
                item = skipped or item_name
                self.update_stage1(current, total, item_name=str(item), stage_name=str(stage))
            else:
                self.update_stage1(current, total, indexed=indexed, skipped=skipped, item_name=item_name, stage_name=st_name)

        return _callback

    def create_stage2_callback(self, total_items: int) -> Callable:
        """Returns a standardized progress callback for ParallelIndexer (Stage 2)."""
        def _callback(current: int, total: int, stage: str, detail: str = ""):
            if stage == "enriching":
                curr_docs = min(current, total_items)
                self.update_stage2(curr_docs, total_items, 0, 0, stage_detail="Enriching Context")
            elif stage in ["embedding", "persisting"]:
                curr_docs = min(int((current / max(total, 1)) * total_items), total_items)
                self.update_stage2(curr_docs, total_items, current, total, stage_detail="Vector Knowledge Base")

        return _callback

    def finish(self, final_message: Optional[str] = None):
        """Clears the live progress line and restores terminal cursor visibility."""
        if self._is_active:
            self._is_active = False
            if self.auto_hide_cursor:
                safe_stdout_write("\033[?25h")
            safe_stdout_write("\r\033[K")
            if final_message:
                safe_stdout_write(final_message + "\n")
