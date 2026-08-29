import os
import unittest
import tempfile
import shutil

from any_context.vector_engine.models import ScoredChunk, RetrievalConfig, IngestionConfig
from any_context.vector_engine.filters import RelevanceFilter
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.enricher import ContextualEnricher, SemanticEnvelope


class TestVectorEngineModelsAndFilters(unittest.TestCase):
    def test_retrieval_config_presets(self):
        turbo = RetrievalConfig.from_preset("turbo")
        self.assertEqual(turbo.candidate_pool_k, 50)
        self.assertEqual(turbo.target_top_k, 10)
        self.assertEqual(turbo.max_chunks_per_source, 2)

        balanced = RetrievalConfig.from_preset("balanced")
        self.assertEqual(balanced.candidate_pool_k, 100)
        self.assertEqual(balanced.target_top_k, 20)
        self.assertEqual(balanced.max_chunks_per_source, 3)

        deep = RetrievalConfig.from_preset("deep_research")
        self.assertEqual(deep.candidate_pool_k, 150)
        self.assertEqual(deep.target_top_k, 40)
        self.assertEqual(deep.max_chunks_per_source, 5)

    def test_relevance_filter_thresholding(self):
        chunks = [
            ScoredChunk(text="High quality match", file_name="f1.pdf", file_path="/f1.pdf", workspace="W1", score=0.85),
            ScoredChunk(text="Medium match", file_name="f2.pdf", file_path="/f2.pdf", workspace="W1", score=0.60),
            ScoredChunk(text="Irrelevant noise", file_name="f3.pdf", file_path="/f3.pdf", workspace="W1", score=0.25)
        ]

        filtered = RelevanceFilter.apply_threshold(chunks, min_score=0.50)
        self.assertEqual(len(filtered), 2)
        self.assertEqual(filtered[0].file_name, "f1.pdf")
        self.assertEqual(filtered[1].file_name, "f2.pdf")

    def test_relevance_filter_source_diversification(self):
        # Monopolizing source A with 5 chunks, source B with 2 chunks, source C with 1 chunk
        chunks = [
            ScoredChunk(text="A1", file_name="A.pdf", file_path="/A.pdf", workspace="W1", score=0.95),
            ScoredChunk(text="A2", file_name="A.pdf", file_path="/A.pdf", workspace="W1", score=0.94),
            ScoredChunk(text="A3", file_name="A.pdf", file_path="/A.pdf", workspace="W1", score=0.93),
            ScoredChunk(text="A4", file_name="A.pdf", file_path="/A.pdf", workspace="W1", score=0.92),
            ScoredChunk(text="A5", file_name="A.pdf", file_path="/A.pdf", workspace="W1", score=0.91),
            ScoredChunk(text="B1", file_name="B.pdf", file_path="/B.pdf", workspace="W1", score=0.89),
            ScoredChunk(text="B2", file_name="B.pdf", file_path="/B.pdf", workspace="W1", score=0.88),
            ScoredChunk(text="C1", file_name="C.pdf", file_path="/C.pdf", workspace="W1", score=0.85),
        ]

        diversified = RelevanceFilter.apply_source_diversification(chunks, max_per_source=2, target_k=5)
        # Should pick A1, B1, C1 (pass 1), then A2, B2 (pass 2) -> total 5 items
        self.assertEqual(len(diversified), 5)
        files = [c.file_name for c in diversified]
        self.assertEqual(files, ["A.pdf", "B.pdf", "C.pdf", "A.pdf", "B.pdf"])


class TestLanceDBStore(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.store = LanceDBStore(db_path=self.test_dir)

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_upsert_and_search_vector(self):
        records = [
            {
                "id": "chunk_1",
                "vector": [1.0, 0.0, 0.0, 0.0],
                "text": "Immigration rules for minors traveling to Canada.",
                "file_name": "Regras_Menores.pdf",
                "file_path": "/docs/Regras_Menores.pdf",
                "workspace": "Immigration",
                "last_modified": "2026-08-22",
                "content_type": "Local Document",
                "document_summary": "Guide on minor consent and custody",
                "keywords": "minors, travel, consent",
                "content_hash": "hash_1"
            },
            {
                "id": "chunk_2",
                "vector": [0.0, 1.0, 0.0, 0.0],
                "text": "Server maintenance and IT database backup policy.",
                "file_name": "IT_Backup.pdf",
                "file_path": "/docs/IT_Backup.pdf",
                "workspace": "IT_Dept",
                "last_modified": "2026-08-22",
                "content_type": "Local Document",
                "document_summary": "Database backup schedules",
                "keywords": "database, backup, server",
                "content_hash": "hash_2"
            }
        ]

        self.store.upsert_records(records, dim=4)
        self.assertEqual(self.store.count_records(), 2)

        # Search matching chunk 1
        results = self.store.search_vector(
            query_vector=[1.0, 0.0, 0.0, 0.0],
            limit=2,
            workspace="Immigration"
        )
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0].file_name, "Regras_Menores.pdf")
        self.assertGreater(results[0].score, 0.8)

        # Zero-cost rename
        migrated = self.store.update_workspace_name("Immigration", "Immigration_CA")
        self.assertEqual(migrated, 1)
        self.assertEqual(self.store.count_records(workspace_name="Immigration_CA"), 1)
        self.assertEqual(self.store.count_records(workspace_name="Immigration"), 0)

        # Zero-cost transfer
        transferred = self.store.transfer_file("Immigration_CA", "Shared Sources", "/docs/Regras_Menores.pdf")
        self.assertEqual(transferred, 1)
        self.assertEqual(self.store.count_records(workspace_name="Shared Sources"), 1)

        # Delete by workspace
        self.store.delete_by_workspace("IT_Dept")
        self.assertEqual(self.store.count_records(), 1)


if __name__ == "__main__":
    unittest.main()
