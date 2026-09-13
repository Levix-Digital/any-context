"""
AnyContext Ingestion Router Bridge (v0.30.0).
Provides a Pythonic gateway to the high-performance Rust any-context-core-rs IngestionRouter,
with graceful pure-Python fallback.
"""
import os
import re
from typing import List, Dict, Any, Optional

try:
    import any_context_core_rs
    _RUST_CORE_AVAILABLE = True
except ImportError:
    _RUST_CORE_AVAILABLE = False


class IngestionRouter:
    """
    Polymorphic Ingestion Router.
    Routes documents across all workspace sources (Local Folders, Cloud Drives, Web)
    to specialized high-speed chunkers.
    """
    def __init__(self, max_chunk_chars: int = 1800, overlap_chars: int = 200):
        self._max_chunk_chars = max_chunk_chars
        self._overlap_chars = overlap_chars
        self._rust_router = (
            any_context_core_rs.IngestionRouter(max_chunk_chars, overlap_chars)
            if _RUST_CORE_AVAILABLE
            else None
        )

    @property
    def is_rust_accelerated(self) -> bool:
        return self._rust_router is not None

    def supports_file(self, file_path: str) -> bool:
        if self._rust_router:
            return self._rust_router.supports_file(file_path)
        ext = os.path.splitext(file_path)[1].lower()
        return ext in {".md", ".markdown", ".rst", ".mdown"}

    def chunk_text(self, file_path: str, content: str) -> List[Dict[str, Any]]:
        if self._rust_router:
            chunks = self._rust_router.chunk_text(file_path, content)
            return [
                {
                    "id": c.id,
                    "text": c.text,
                    "file_name": c.file_name,
                    "file_path": c.file_path,
                    "header_path": c.header_path,
                    "start_line": c.start_line,
                    "end_line": c.end_line,
                    "content_type": c.content_type,
                    "chunk_index": c.chunk_index
                }
                for c in chunks
            ]
        raise RuntimeError("any_context_core_rs native library is required for text chunking.")

    def chunk_file(self, file_path: str) -> List[Dict[str, Any]]:
        if self._rust_router:
            chunks = self._rust_router.chunk_file(file_path)
            return [
                {
                    "id": c.id,
                    "text": c.text,
                    "file_name": c.file_name,
                    "file_path": c.file_path,
                    "header_path": c.header_path,
                    "start_line": c.start_line,
                    "end_line": c.end_line,
                    "content_type": c.content_type,
                    "chunk_index": c.chunk_index
                }
                for c in chunks
            ]
        raise RuntimeError("any_context_core_rs native library is required for file chunking.")

    def chunk_bytes(self, file_path: str, bytes_data: bytes) -> List[Dict[str, Any]]:
        if self._rust_router and hasattr(self._rust_router, "chunk_bytes"):
            chunks = self._rust_router.chunk_bytes(file_path, bytes_data)
            return [
                {
                    "id": c.id,
                    "text": c.text,
                    "file_name": c.file_name,
                    "file_path": c.file_path,
                    "header_path": c.header_path,
                    "start_line": c.start_line,
                    "end_line": c.end_line,
                    "content_type": c.content_type,
                    "chunk_index": c.chunk_index
                }
                for c in chunks
            ]
        raise RuntimeError("any_context_core_rs native library is required for byte chunking.")
