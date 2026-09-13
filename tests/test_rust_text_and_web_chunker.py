"""
Tests for 100% Native Rust Text, Script, Configuration, and Web URL Ingestion (v0.30.15).
Validates that Plain Text (.txt, .log), Scripts (.sql, .sh), Configurations (.env, .ini),
Build files (Dockerfile, Makefile), and Web URLs are chunked purely in Rust with zero Python fallbacks.
"""
import os
import pytest
from any_context.ingestion.router import IngestionRouter
from any_context.vector_engine.indexer import ParallelIndexer
from any_context.vector_engine.store import LanceDBStore
from llama_index.core import Document


def test_ingestion_router_supports_new_types():
    router = IngestionRouter()
    assert router.is_rust_accelerated is True

    # Scripts
    assert router.supports_file("schema.sql") is True
    assert router.supports_file("deploy.sh") is True
    assert router.supports_file("setup.ps1") is True
    assert router.supports_file("run.bat") is True
    assert router.supports_file("task.cmd") is True

    # Configs
    assert router.supports_file(".env") is True
    assert router.supports_file(".env.local") is True
    assert router.supports_file("settings.ini") is True
    assert router.supports_file("server.cfg") is True
    assert router.supports_file("application.properties") is True

    # Build & Container
    assert router.supports_file("Dockerfile") is True
    assert router.supports_file("Makefile") is True

    # Web URLs & Text
    assert router.supports_file("https://docs.anycontext.ai/intro") is True
    assert router.supports_file("http://localhost:8000/docs") is True
    assert router.supports_file("notes.txt") is True
    assert router.supports_file("system.log") is True


def test_ingestion_router_chunks_sql_natively():
    router = IngestionRouter()
    sql_content = """-- Database Migration
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL
);

CREATE INDEX idx_users_email ON users(email);
"""
    chunks = router.chunk_text("migrations/001_init.sql", sql_content)
    assert len(chunks) >= 1
    chunk = chunks[0]
    assert chunk["content_type"] == "sql"
    assert "// Context: 001_init.sql > lines" in chunk["text"]
    assert "CREATE TABLE users" in chunk["text"]


def test_ingestion_router_chunks_web_url_natively():
    router = IngestionRouter()
    web_markdown = """# Architecture Overview
AnyContext uses a native Rust core for vector ingestion and chunking.

## Performance
Throughput reaches 5,000+ chunks/sec.
"""
    chunks = router.chunk_text("https://docs.anycontext.ai/arch", web_markdown)
    assert len(chunks) >= 1
    # Markdown headings in web documentation route to markdown chunker
    assert chunks[0]["content_type"] in ("markdown", "web")

    web_plain = "Simple web paragraph without headings.\nAnother line of web text."
    plain_chunks = router.chunk_text("https://example.com/about", web_plain)
    assert len(plain_chunks) >= 1
    assert plain_chunks[0]["content_type"] == "web"
    assert "// Context: https://example.com/about > Section" in plain_chunks[0]["text"]


def test_parallel_indexer_zero_python_sentence_splitter(tmp_path):
    store = LanceDBStore.get_instance(db_path=str(tmp_path / "lancedb"))
    indexer = ParallelIndexer(store=store)
    # Mock embedding batch to allow offline vector indexing test
    indexer._get_text_embeddings_batch = lambda texts: [[0.05] * 1536 for _ in texts]

    docs = [
        Document(text="SELECT 1 FROM dual;", metadata={"file_path": "query.sql", "workspace": "test_ws"}),
        Document(text="echo 'hello world'", metadata={"file_path": "script.sh", "workspace": "test_ws"}),
        Document(text="API_KEY=secret123\nDEBUG=true", metadata={"file_path": ".env", "workspace": "test_ws"}),
        Document(text="FROM python:3.13-slim\nCMD ['actx']", metadata={"file_path": "Dockerfile", "workspace": "test_ws"}),
        Document(text="Some documentation scraped from the web", metadata={"file_path": "https://anycontext.ai", "workspace": "test_ws"}),
    ]

    result = indexer.index_documents(documents=docs, workspace_name="test_ws")
    assert result["indexed_chunks"] >= 5

    # Check taxonomies recorded in LanceDB
    table = store._db.open_table("workspace_chunks")
    df = table.search().where("workspace = 'test_ws'").to_pandas()
    content_types = set(df["content_type"].unique())

    assert "SQL Script" in content_types
    assert "Shell Script" in content_types
    assert "Configuration / Env" in content_types
    assert "Container / Build Definition" in content_types
    assert "Web Documentation" in content_types
