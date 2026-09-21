"""
Unit and integration tests for Global system help retrieval across workspaces
and version-aware atomic re-indexing cache.
"""
import os
import shutil
import tempfile
import pytest
from unittest.mock import patch, MagicMock

from any_context import __version__
from any_context.help.bootstrap import ensure_system_knowledge_indexed
from any_context.vector_engine.store import LanceDBStore
from any_context.tools.search_tools import _execute_search_context
from llama_index.core import Settings
from llama_index.core.embeddings.mock_embed_model import MockEmbedding


@pytest.fixture
def temp_lancedb():
    temp_dir = tempfile.mkdtemp()
    db_path = os.path.join(temp_dir, "context_db")
    lance_dir = os.path.join(db_path, "lancedb")
    os.makedirs(lance_dir, exist_ok=True)
    Settings.embed_model = MockEmbedding(embed_dim=1536)
    yield db_path, lance_dir
    LanceDBStore._instance = None
    if os.path.exists(temp_dir):
        try:
            shutil.rmtree(temp_dir)
        except Exception:
            pass


def test_ensure_system_knowledge_version_cache(temp_lancedb):
    db_path, lance_dir = temp_lancedb

    with patch("any_context.tools.search_tools.configure_embedding_model"):
        # First run indexes
        indexed = ensure_system_knowledge_indexed(db_path=db_path)
        assert indexed is True

        cache_file = os.path.join(lance_dir, "system_help_cache.json")
        assert os.path.exists(cache_file)

        # Second run without changes hits fast cache
        indexed_again = ensure_system_knowledge_indexed(db_path=db_path)
        assert indexed_again is True


def test_search_context_includes_global_in_target_workspaces():
    with patch("any_context.vector_engine.retriever.ParallelRetriever.search") as mock_search:
        mock_search.return_value = []
        with patch("any_context.vector_engine.store.LanceDBStore.count_records", return_value=100):
            with patch("any_context.tools.search_tools.configure_embedding_model"):
                _execute_search_context(
                    prompt_text="como usar o comando /transfer",
                    workspace="IKEAShipments"
                )

                assert mock_search.called
                call_kwargs = mock_search.call_args[1]
                target_workspaces = call_kwargs.get("target_workspaces", [])
                assert "IKEAShipments" in target_workspaces
                assert "Global" in target_workspaces
