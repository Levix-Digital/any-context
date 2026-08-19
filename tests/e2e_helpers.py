import os
import sys
import shutil
import tempfile
from typing import Dict, Any
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.core.utils import get_api_key

def safe_stdout_write(msg: str):
    """Writes message to stdout with fallback for Windows CP1252 / Charmap encoding."""
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

def setup_mock_embeddings_if_needed():
    """Configures deterministic MockEmbedding if no live OpenAI API key is present."""
    api_key = get_api_key()
    if not api_key or api_key.startswith("mock_") or "fake" in api_key.lower() or api_key == "sk-test":
        try:
            from llama_index.core import Settings
            from llama_index.core.embeddings.mock_embed_model import MockEmbedding
            Settings.embed_model = MockEmbedding(embed_dim=1536)
        except Exception:
            pass

def create_isolated_test_env(prefix: str = "anycontext_test") -> Dict[str, Any]:
    """
    Creates an isolated temporary directory, custom ChromaDB directory,
    and a clean SQLite database for deterministic test execution.
    """
    temp_dir = tempfile.mkdtemp(prefix=prefix)
    db_path = os.path.join(temp_dir, "context_db")
    memory_path = os.path.join(temp_dir, "memory")
    settings_db = os.path.join(temp_dir, "settings_test.db")
    
    os.makedirs(db_path, exist_ok=True)
    os.makedirs(memory_path, exist_ok=True)

    store = ConfigDBStore(db_path=settings_db)

    setup_mock_embeddings_if_needed()

    return {
        "temp_dir": temp_dir,
        "db_path": db_path,
        "memory_path": memory_path,
        "settings_db": settings_db,
        "store": store
    }

def cleanup_isolated_test_env(env: Dict[str, Any]):
    """Cleans up temporary directory and databases safely."""
    temp_dir = env.get("temp_dir")
    if temp_dir and os.path.exists(temp_dir):
        shutil.rmtree(temp_dir, ignore_errors=True)
