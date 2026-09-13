import os
import unittest
import tempfile
import shutil

from any_context.vector_engine.models import ScoredChunk, RetrievalConfig, IngestionConfig
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.enricher import ContextualEnricher, SemanticEnvelope
import any_context_core_rs


class TestVectorEngineModelsAndFilters(unittest.TestCase):
    def test_retrieval_config_presets(self):
        turbo = RetrievalConfig.from_preset("turbo")
        self.assertEqual(turbo.candidate_pool_k, 50)
        self.assertEqual(turbo.target_top_k, 10)
        self.assertEqual(turbo.max_chunks_per_source, 2)
        self.assertEqual(turbo.rrf_k, 60)

        balanced = RetrievalConfig.from_preset("balanced")
        self.assertEqual(balanced.candidate_pool_k, 100)
        self.assertEqual(balanced.target_top_k, 20)
        self.assertEqual(balanced.max_chunks_per_source, 3)
        self.assertEqual(balanced.rrf_k, 60)

        deep = RetrievalConfig.from_preset("deep_research")
        self.assertEqual(deep.candidate_pool_k, 150)
        self.assertEqual(deep.target_top_k, 40)
        self.assertEqual(deep.max_chunks_per_source, 5)
        self.assertEqual(deep.rrf_k, 60)

    def test_rust_hybrid_engine_bm25_and_diversification(self):
        engine = any_context_core_rs.HybridRetrieverEngine()

        # Monopolizing source A with 5 chunks, source B with 2 chunks, source C with 1 chunk
        dense_results = [
            {"id": "A1", "score": 0.95, "file_name": "A.pdf", "file_path": "/A.pdf", "text": "A1", "workspace": "W1", "content_type": "PDF"},
            {"id": "A2", "score": 0.94, "file_name": "A.pdf", "file_path": "/A.pdf", "text": "A2", "workspace": "W1", "content_type": "PDF"},
            {"id": "A3", "score": 0.93, "file_name": "A.pdf", "file_path": "/A.pdf", "text": "A3", "workspace": "W1", "content_type": "PDF"},
            {"id": "A4", "score": 0.92, "file_name": "A.pdf", "file_path": "/A.pdf", "text": "A4", "workspace": "W1", "content_type": "PDF"},
            {"id": "A5", "score": 0.91, "file_name": "A.pdf", "file_path": "/A.pdf", "text": "A5", "workspace": "W1", "content_type": "PDF"},
            {"id": "B1", "score": 0.89, "file_name": "B.pdf", "file_path": "/B.pdf", "text": "B1", "workspace": "W1", "content_type": "PDF"},
            {"id": "B2", "score": 0.88, "file_name": "B.pdf", "file_path": "/B.pdf", "text": "B2", "workspace": "W1", "content_type": "PDF"},
            {"id": "C1", "score": 0.85, "file_name": "C.pdf", "file_path": "/C.pdf", "text": "C1", "workspace": "W1", "content_type": "PDF"},
        ]

        diversified = engine.fuse_and_diversify(
            dense_results=dense_results,
            query="test",
            workspace="W1",
            candidate_pool_k=10,
            target_top_k=5,
            max_per_source=2,
            max_density_chars=40000,
            rrf_k=60
        )
        # Should pick A1, B1, C1 (pass 1), then A2, B2 (pass 2) -> total 5 items
        self.assertEqual(len(diversified), 5)
        files = [c["file_name"] for c in diversified]
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

    def test_search_metadata(self):
        records = [
            {
                "id": "chunk_date_1",
                "vector": [0.1, 0.2, 0.3, 0.4],
                "text": "Shipment checklist 1.",
                "file_name": "015-TSO-1.pdf",
                "file_path": "C:/docs/2026/09/01/all_results/015-TSO-1.pdf",
                "workspace": "IKEA",
                "last_modified": "2026-09-01",
                "content_type": "Local Document",
                "document_summary": "Checklist 1",
                "keywords": "shipment, 2026-09-01",
                "content_hash": "hash_d1"
            },
            {
                "id": "chunk_date_2",
                "vector": [0.1, 0.2, 0.3, 0.4],
                "text": "Old report from January.",
                "file_name": "CMR_Jan.pdf",
                "file_path": "C:/docs/2026/01/15/CMR_Jan.pdf",
                "workspace": "IKEA",
                "last_modified": "2026-01-15",
                "content_type": "Local Document",
                "document_summary": "CMR report",
                "keywords": "cmr, shipment",
                "content_hash": "hash_d2"
            }
        ]
        self.store.upsert_records(records, dim=4)
        results = self.store.search_metadata(
            where_clause="file_path LIKE '%2026/09/01%'",
            workspace="IKEA"
        )
        self.assertEqual(len(results), 1)
        self.assertEqual(results[0].file_name, "015-TSO-1.pdf")
        self.assertGreaterEqual(results[0].score, 0.9)


if __name__ == "__main__":
    unittest.main()
