import sys
import os
import io

# Force Windows terminal to accept Emojis (UTF-8)
if sys.stdout.encoding != 'utf-8':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# Ensure the root directory is in the Python path
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

import logging
# Suppress noisy HTTP request logs that interleave with tqdm progress bar
logging.getLogger("httpx").setLevel(logging.WARNING)
logging.getLogger("httpcore").setLevel(logging.WARNING)

from cli.workspace_selector import get_active_workspace
from cli.chat_loop import run_chat_loop

if __name__ == "__main__":
    workspace = get_active_workspace()
    run_chat_loop(active_workspace=workspace)
