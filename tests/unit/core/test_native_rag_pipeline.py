"""
Unit and integration tests for Native Rust RAG Pipeline and RFC-042 Batch Retrieval.
Validates PyHybridPipeline, PyHybridSearchRequest, PyHybridSearchResult, and
ParallelRetriever's zero-copy integration.
"""
import os
import tempfile
import pytest

import any_context_core_rs
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.retriever import ParallelRetriever
from any_context.vector_engine.models import RetrievalConfig


def test_native_pipeline_basic_search():
    """Validates PyHybridPipeline creation, BM25 indexing, and single query retrieval."""
    with tempfile.TemporaryDirectory() as tmpdir:
        pipeline = any_context_core_rs.PyHybridPipeline(tmpdir)
        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-1",
            "Rust async Tokio runtime with high performance and zero GIL overhead",
            "runtime.rs",
            "/src/runtime.rs",
            "Default",
            "Local Document"
        )
        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-2",
            "Python asyncio event loop with GIL lock contention",
            "loop.py",
            "/src/loop.py",
            "Default",
            "Local Document"
        )

        req = any_context_core_rs.PyHybridSearchRequest(
            query_text="Tokio runtime performance",
            top_k=5
        )
        results = pipeline.search(req)

        assert len(results) >= 1
        assert results[0].chunk_id == "chunk-1"
        assert "Tokio" in results[0].text
        assert results[0].sparse_score is not None

        d = results[0].to_dict()
        assert d["chunk_id"] == "chunk-1"
        assert d["file_name"] == "runtime.rs"


def test_native_pipeline_batch_cross_query_deduplication():
    """
    RFC-042: Validates that when multiple orthogonal sub-queries are dispatched in batch,
    chunks matching more than one sub-query are deduplicated by content_hash/chunk_id,
    receive an accumulated RRF boost, and preserve provenance in matched_subqueries.
    """
    with tempfile.TemporaryDirectory() as tmpdir:
        pipeline = any_context_core_rs.PyHybridPipeline(tmpdir)
        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-jwt",
            "JWT token cryptographic verification and RS256 signature validation",
            "jwt.rs",
            "/src/auth/jwt.rs",
            "Default",
            "Local Document"
        )
        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-session",
            "HTTP session cookie expiration and Redis storage adapter",
            "session.rs",
            "/src/auth/session.rs",
            "Default",
            "Local Document"
        )

        req1 = any_context_core_rs.PyHybridSearchRequest(
            query_text="JWT token verification",
            sub_query_id="subq-1-jwt"
        )
        req2 = any_context_core_rs.PyHybridSearchRequest(
            query_text="signature validation RS256",
            sub_query_id="subq-2-sig"
        )

        batch_results = pipeline.retrieve_hybrid_batch([req1, req2])

        assert len(batch_results) >= 1
        top = batch_results[0]
        assert top.chunk_id == "chunk-jwt"
        # Since chunk-jwt matched BOTH sub-queries, matched_subqueries must track both
        assert "subq-1-jwt" in top.matched_subqueries
        assert "subq-2-sig" in top.matched_subqueries
        # Score must be accumulated (two RRF additions: 1/(60+1) + 1/(60+1) = 2/61 ≈ 0.0328)
        assert top.score > 0.03


def test_parallel_retriever_native_dispatch():
    """Validates ParallelRetriever delegating search and batch retrieval to native Rust."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = os.path.join(tmpdir, "lancedb")
        os.makedirs(db_path, exist_ok=True)
        store = LanceDBStore(db_path=db_path)

        engine = store.get_hybrid_engine()
        engine.add_chunk(
            "doc-1",
            "PostgreSQL connection pooling and read replica failover",
            "db.py",
            "/src/db.py",
            "Default",
            "Local Document"
        )
        store.save_hybrid_engine(engine)

        retriever = ParallelRetriever(store=store)
        assert retriever._native_pipeline is not None

        # Test single search through retriever
        results = retriever.search("PostgreSQL failover", config=RetrievalConfig.from_preset("fast"))
        assert len(results) >= 1
        assert results[0].chunk_id == "doc-1"
        assert results[0].metadata["file_name"] == "db.py"

        # Test retrieve_batch through retriever
        batch_results = retriever.retrieve_batch([
            {"query": "PostgreSQL connection", "sub_query_id": "q1"},
            {"query": "replica failover", "sub_query_id": "q2"}
        ])
        assert len(batch_results) >= 1
        top = batch_results[0]
        assert top.chunk_id == "doc-1"
        assert "q1" in top.metadata.get("matched_subqueries", [])
        assert "q2" in top.metadata.get("matched_subqueries", [])
