import os
import unittest
import tempfile
import shutil

import any_context_core_rs
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.retriever import ParallelRetriever
from any_context.vector_engine.models import RetrievalConfig, ScoredChunk


class TestRustHybridRetriever(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.store = LanceDBStore(db_path=self.test_dir)

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_native_rust_bm25_exact_code_tokenization(self):
        engine = any_context_core_rs.HybridRetrieverEngine()

        engine.add_chunk(
            id="chunk_env",
            text="DATABASE_URL=postgres://app_user:secret_pass@db.internal:5432/production_db",
            file_name=".env",
            file_path="/app/.env",
            workspace="Backend",
            content_type="Configuration / Env"
        )

        engine.add_chunk(
            id="chunk_auth",
            text="export function verifyToken(token: string): boolean { return true; }",
            file_name="jwt.ts",
            file_path="/app/src/auth/jwt.ts",
            workspace="Backend",
            content_type="TypeScript Source Code"
        )

        # 1. Exact match for DATABASE_URL
        res_env = engine.search_bm25("DATABASE_URL", 5, "Backend")
        self.assertTrue(len(res_env) >= 1)
        self.assertEqual(res_env[0][0], "chunk_env")

        # 2. Exact match for verifyToken
        res_auth = engine.search_bm25("verifyToken", 5, "Backend")
        self.assertTrue(len(res_auth) >= 1)
        self.assertEqual(res_auth[0][0], "chunk_auth")

        # 3. Sub-token matching for verify
        res_sub = engine.search_bm25("verify", 5, "Backend")
        self.assertTrue(len(res_sub) >= 1)
        self.assertEqual(res_sub[0][0], "chunk_auth")

    def test_rrf_scoring_mathematical_precision(self):
        engine = any_context_core_rs.HybridRetrieverEngine()

        engine.add_chunk(
            id="doc_exact",
            text="AUTH_HEADER_SECRET_TOKEN=xyz123",
            file_name="config.env",
            file_path="/config.env",
            workspace="Default",
            content_type="Configuration / Env"
        )

        # Mock dense vector search results:
        # doc_sem ranked #1 in dense, doc_exact ranked #2 in dense
        dense_candidates = [
            {
                "id": "doc_sem",
                "score": 0.85,
                "text": "Authentication guidelines for authorization headers",
                "file_name": "auth.md",
                "file_path": "/docs/auth.md",
                "workspace": "Default",
                "content_type": "Markdown Document"
            },
            {
                "id": "doc_exact",
                "score": 0.80,
                "text": "AUTH_HEADER_SECRET_TOKEN=xyz123",
                "file_name": "config.env",
                "file_path": "/config.env",
                "workspace": "Default",
                "content_type": "Configuration / Env"
            }
        ]

        # Query matches AUTH_HEADER_SECRET_TOKEN in BM25 (doc_exact is #1 in BM25)
        fused = engine.fuse_and_diversify(
            dense_results=dense_candidates,
            query="AUTH_HEADER_SECRET_TOKEN",
            workspace="Default",
            candidate_pool_k=10,
            target_top_k=5,
            max_per_source=3,
            max_density_chars=40000,
            rrf_k=60
        )

        self.assertTrue(len(fused) >= 2)
        # doc_exact is rank 2 in dense (1/62) and rank 1 in sparse (1/61) -> score ≈ 0.016129 + 0.016393 = 0.03252
        # doc_sem is rank 1 in dense (1/61) and not in sparse (0) -> score ≈ 0.016393
        # Therefore, doc_exact must win the top slot via RRF!
        self.assertEqual(fused[0]["id"], "doc_exact")
        self.assertEqual(fused[1]["id"], "doc_sem")
        self.assertGreater(fused[0]["score"], fused[1]["score"])

    def test_bm25_binary_persistence_and_store_integration(self):
        records = [
            {
                "id": "c1",
                "vector": [1.0, 0.0, 0.0, 0.0],
                "text": "Deploy script using docker build and alpine image",
                "file_name": "deploy.sh",
                "file_path": "/scripts/deploy.sh",
                "workspace": "DevOps",
                "content_type": "Shell Script"
            },
            {
                "id": "c2",
                "vector": [0.0, 1.0, 0.0, 0.0],
                "text": "Database migrations and SQL table schema definition",
                "file_name": "schema.sql",
                "file_path": "/db/schema.sql",
                "workspace": "DevOps",
                "content_type": "SQL Script"
            }
        ]

        # Upsert into store should automatically populate and persist BM25 index to disk
        self.store.upsert_records(records, dim=4)

        bm25_file = self.store.get_bm25_path()
        self.assertTrue(os.path.exists(bm25_file))

        # Create new store instance pointing to same path to verify reload
        fresh_store = LanceDBStore(db_path=self.test_dir)
        engine = fresh_store.get_hybrid_engine()
        self.assertEqual(engine.count_bm25_chunks(), 2)

        bm25_res = engine.search_bm25("alpine", 5, "DevOps")
        self.assertEqual(len(bm25_res), 1)
        self.assertEqual(bm25_res[0][0], "c1")

        # Test deletion synchronization
        fresh_store.delete_by_file("/scripts/deploy.sh")
        updated_engine = fresh_store.get_hybrid_engine()
        self.assertEqual(updated_engine.count_bm25_chunks(), 1)


if __name__ == "__main__":
    unittest.main()
