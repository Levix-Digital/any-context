import sys
import time
import threading
from typing import Optional

class Spinner:
    """
    A lightweight, zero-dependency CLI spinner for smooth terminal feedback.
    Safe across Windows UTF-8 and POSIX environments.
    """
    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    def __init__(self, message: str = "Loading...", done_message: Optional[str] = None):
        self.message = message
        self.done_message = done_message
        self._running = False
        self._thread: Optional[threading.Thread] = None

    def _spin(self):
        idx = 0
        while self._running:
            frame = self.SPINNER_FRAMES[idx % len(self.SPINNER_FRAMES)]
            try:
                sys.stdout.write(f"\r\033[96m{frame}\033[0m {self.message}   ")
                sys.stdout.flush()
            except Exception:
                pass
            time.sleep(0.08)
            idx += 1

    def start(self):
        self._running = True
        self._thread = threading.Thread(target=self._spin, daemon=True)
        self._thread.start()

    def stop(self, success: bool = True):
        self._running = False
        if self._thread:
            self._thread.join(timeout=0.3)
        try:
            sys.stdout.write("\r\033[K")
            if success:
                if self.done_message:
                    sys.stdout.write(f"\033[92m✔\033[0m {self.done_message}\n")
            else:
                sys.stdout.write(f"\033[91m✖\033[0m {self.message} (failed)\n")
            sys.stdout.flush()
        except Exception:
            pass

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.stop(success=(exc_type is None))
