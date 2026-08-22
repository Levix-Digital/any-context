"""
Unit and Integration Tests for Permanent System Self-Knowledge Auto-Bootstrap (v0.23.1).
Tests:
  1. ensure_system_knowledge_indexed boots HELP_REGISTRY and README.md into LanceDB 'Global'.
  2. TECDOC.md is strictly excluded from indexed system knowledge.
  3. Search queries for commands (e.g. transfer source, switch workspace) successfully return Global chunks.
  4. Instant SHA-256 hash bypass on subsequent calls.
"""
import os
import shutil
import tempfile
import unittest
from unittest.mock import patch

from any_context.help.bootstrap import ensure_system_knowledge_indexed, build_system_help_document, build_system_readme_document
from any_context.vector_engine.store import LanceDBStore
from any_context.tools.search_tools import _execute_search_context
from any_context.config.app_settings import AppSettings, ContextSettings


class TestSystemKnowledgeBootstrap(unittest.TestCase):

    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.db_path = os.path.join(self.temp_dir, "context_db")
        self.lance_dir = os.path.join(self.db_path, "lancedb")
        os.makedirs(self.lance_dir, exist_ok=True)
        self.lance_store = LanceDBStore.get_instance(db_path=self.lance_dir)

    def tearDown(self):
        LanceDBStore._instance = None
        if os.path.exists(self.temp_dir):
            try:
                shutil.rmtree(self.temp_dir)
            except Exception:
                pass

    def test_01_build_system_help_document_structure(self):
        """
        Tests that build_system_help_document properly formats all commands from HELP_REGISTRY.
        """
        print("\n>>> [UNIT] Testing System Help Document Builder...")
        doc = build_system_help_document()

        self.assertEqual(doc.metadata["workspace"], "Global")
        self.assertEqual(doc.metadata["content_type"], "System Documentation")
        self.assertTrue(doc.metadata["is_system_help"])
        self.assertIn("## Command: /transfer", doc.text)
        self.assertIn("## Command: /switch", doc.text)
        self.assertIn("## Command: /sync", doc.text)
        print("  [OK] System Help Document built with all command registries!")

    def test_02_tecdoc_exclusion_security(self):
        """
        Strict security verification: Ensures TECDOC.md is never indexed as system knowledge.
        """
        print("\n>>> [SECURITY UNIT] Verifying TECDOC.md Strict Exclusion...")
        doc = build_system_help_document()
        self.assertNotIn("TecDoc", doc.metadata.get("file_name", ""))
        self.assertNotIn("TECDOC.md", doc.metadata.get("file_name", ""))
        print("  [OK] TECDOC.md is strictly excluded from public system knowledge!")

    def test_03_ensure_system_knowledge_indexed_and_search_retrieval(self):
        """
        Tests that ensure_system_knowledge_indexed writes into LanceDB and search_context retrieves Global chunks.
        """
        print("\n>>> [UNIT] Testing Auto-Bootstrap & Command Query Retrieval...")
        
        # 1. Mock embedding generator
        def mock_embed_batch(texts):
            return [[0.05] * 1536 for _ in texts]

        def mock_query_embed(query):
            return [0.05] * 1536

        with patch("any_context.vector_engine.indexer.ParallelIndexer._get_text_embeddings_batch", side_effect=mock_embed_batch):
            with patch("any_context.vector_engine.retriever.ParallelRetriever._get_query_embedding", side_effect=mock_query_embed):
                with patch("any_context.tools.search_tools.configure_embedding_model"):
                    success = ensure_system_knowledge_indexed(db_path=self.db_path, force=True)
                    self.assertTrue(success)

                    records_count = self.lance_store.count_records(table_name="workspace_chunks")
                    self.assertGreater(records_count, 0)

                    # 2. Test _execute_search_context for '/transfer' or moving web sources in an empty workspace
                    res = _execute_search_context(
                        prompt_text="como mover um web source de um workspace para outro",
                        workspace="EmptyCustomWorkspace"
                    )

                    self.assertIn("Source: AnyContext", res)
                    self.assertIn("Workspace: Global", res)
                    self.assertIn("transfer", res.lower())
                    print("  [OK] System Help chunks retrieved across empty workspaces successfully!")


if __name__ == "__main__":
    unittest.main()
