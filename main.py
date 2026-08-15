import sys
import os
import io

os.environ["TRANSFORMERS_VERBOSITY"] = "error"
os.environ["HF_HUB_DISABLE_SYMLINKS_WARNING"] = "1"
os.environ["PYTHONWARNINGS"] = "ignore"

# Force Windows terminal to accept Emojis (UTF-8)
if sys.stdout.encoding != 'utf-8':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# Add src to path if running uninstalled
src_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src")
if src_dir not in sys.path:
    sys.path.insert(0, src_dir)

from any_context.cli.entrypoint import entrypoint

if __name__ == "__main__":
    try:
        entrypoint()
    except (KeyboardInterrupt, EOFError, SystemExit):
        sys.exit(0)
    except Exception as e:
        sys.exit(1)
