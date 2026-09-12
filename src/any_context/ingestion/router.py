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
        return self._fallback_markdown_chunk(file_path, content)

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
        with open(file_path, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()
        return self._fallback_markdown_chunk(file_path, content)

    def _fallback_markdown_chunk(self, file_path: str, content: str) -> List[Dict[str, Any]]:
        """Pure-Python fallback when native Rust extension is not compiled."""
        clean = content.trim() if hasattr(content, "trim") else content.strip()
        if not clean:
            return []

        file_name = os.path.basename(file_path) or "unknown.md"
        sections = re.split(r"(?m)(?=^#{1,6}\s+)", clean)
        chunks = []

        for idx, sec in enumerate(sections):
            sec_clean = sec.strip()
            if not sec_clean:
                continue

            header_match = re.match(r"^(#{1,6})\s+(.+)$", sec_clean, re.MULTILINE)
            header_path = header_match.group(0) if header_match else None
            text = f"// Context: {header_path}\n---\n{sec_clean}" if header_path else sec_clean

            chunks.append({
                "id": f"{file_name}_{idx}",
                "text": text,
                "file_name": file_name,
                "file_path": file_path,
                "header_path": header_path,
                "start_line": 1,
                "end_line": len(sec_clean.splitlines()),
                "content_type": "markdown",
                "chunk_index": idx
            })

        return chunks
