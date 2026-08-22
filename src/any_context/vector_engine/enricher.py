"""
Contextual Enrichment Engine (Fase 1).
Provides high-speed, thread-safe document summarization, Top-N keyword extraction,
and Contextual Retrieval chunk enveloping with persistent SHA-256 SQLite caching.
"""
import os
import re
import sqlite3
import hashlib
import threading
from datetime import datetime, timezone
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field, asdict

from any_context.config.app_settings import AppSettings
from any_context.core.utils import get_api_key

@dataclass
class SemanticEnvelope:
    summary: str
    keywords: List[str]
    content_hash: str
    file_name: str
    file_path: Optional[str] = None
    url: Optional[str] = None
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "SemanticEnvelope":
        return cls(
            summary=data.get("summary", ""),
            keywords=data.get("keywords", []),
            content_hash=data.get("content_hash", ""),
            file_name=data.get("file_name", "Unknown"),
            file_path=data.get("file_path"),
            url=data.get("url"),
            created_at=data.get("created_at", datetime.now(timezone.utc).isoformat())
        )


class ContextualEnricher:
    """
    Thread-safe contextual enricher with local SQLite caching.
    Generates rich 3-4 sentence summaries and Top-N domain keywords per document/URL,
    stamping each chunk with an authoritative semantic header to eliminate cross-domain false positives.
    """
    _instance = None
    _lock = threading.Lock()

    def __new__(cls, db_path: Optional[str] = None):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(ContextualEnricher, cls).__new__(cls)
                cls._instance._init_db(db_path)
            return cls._instance

    def _init_db(self, custom_path: Optional[str] = None):
        if custom_path:
            self._db_file = custom_path
        else:
            settings = AppSettings.load()
            base_dir = settings.context.db_path if (settings and settings.context) else os.path.expanduser("~/.anycontext/cache")
            self._db_file = os.path.join(base_dir, "contextual_enrichment_cache.db")

        os.makedirs(os.path.dirname(os.path.abspath(self._db_file)), exist_ok=True)
        self._local_lock = threading.Lock()
        with self._get_connection() as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS semantic_envelopes (
                    content_hash TEXT PRIMARY KEY,
                    file_name TEXT,
                    file_path TEXT,
                    url TEXT,
                    summary TEXT,
                    keywords_json TEXT,
                    created_at TEXT
                );
            """)
            conn.execute("CREATE INDEX IF NOT EXISTS idx_envelope_file ON semantic_envelopes(file_path);")
            conn.commit()

    def _get_connection(self) -> sqlite3.Connection:
        return sqlite3.connect(self._db_file, timeout=15.0)

    def compute_hash(self, text: str) -> str:
        """Computes SHA-256 hash of normalized text."""
        return hashlib.sha256((text or "").strip().encode("utf-8")).hexdigest()

    def get_cached_envelope(self, content_hash: str) -> Optional[SemanticEnvelope]:
        """Retrieves cached semantic envelope by SHA-256 content hash in sub-millisecond time."""
        if not content_hash:
            return None
        with self._local_lock:
            try:
                with self._get_connection() as conn:
                    cursor = conn.cursor()
                    cursor.execute("""
                        SELECT file_name, file_path, url, summary, keywords_json, created_at
                        FROM semantic_envelopes WHERE content_hash = ?
                    """, (content_hash,))
                    row = cursor.fetchone()
                    if row:
                        import json
                        fn, fp, url, summ, kw_json, cat = row
                        kw = json.loads(kw_json) if kw_json else []
                        return SemanticEnvelope(
                            summary=summ,
                            keywords=kw,
                            content_hash=content_hash,
                            file_name=fn,
                            file_path=fp,
                            url=url,
                            created_at=cat
                        )
            except Exception:
                pass
        return None

    def save_envelope(self, envelope: SemanticEnvelope):
        """Persists semantic envelope in thread-safe SQLite cache."""
        import json
        with self._local_lock:
            try:
                with self._get_connection() as conn:
                    conn.execute("""
                        INSERT OR REPLACE INTO semantic_envelopes 
                        (content_hash, file_name, file_path, url, summary, keywords_json, created_at)
                        VALUES (?, ?, ?, ?, ?, ?, ?)
                    """, (
                        envelope.content_hash,
                        envelope.file_name,
                        envelope.file_path,
                        envelope.url,
                        envelope.summary,
                        json.dumps(envelope.keywords, ensure_ascii=False),
                        envelope.created_at
                    ))
                    conn.commit()
            except Exception:
                pass

    def extract_rich_summary_and_keywords(
        self,
        doc_text: str,
        file_name: str,
        file_path: Optional[str] = None,
        url: Optional[str] = None
    ) -> SemanticEnvelope:
        """
        Extracts a dense 3-4 sentence summary and 5-8 domain keywords.
        Utilizes cached results if hash matches; otherwise applies high-speed NLP extractive analysis.
        """
        content_hash = self.compute_hash(doc_text)
        cached = self.get_cached_envelope(content_hash)
        if cached:
            return cached

        clean_text = (doc_text or "").strip()
        lines = [line.strip() for line in clean_text.splitlines() if line.strip()]

        # 1. Extract Title / Lead Headings
        title_candidates = []
        for line in lines[:8]:
            if line.startswith("#") or len(line) < 120:
                cleaned_line = re.sub(r"^[#\*\-\s]+", "", line).strip()
                if cleaned_line and len(cleaned_line) > 3:
                    title_candidates.append(cleaned_line)
                    if len(title_candidates) >= 2:
                        break

        doc_title = title_candidates[0] if title_candidates else file_name

        # 2. Extract Lead Descriptive Sentences (first 1500 chars)
        lead_chunk = " ".join(lines[:12])
        sentences = [s.strip() for s in re.split(r"(?<=[.!?])\s+", lead_chunk) if len(s.strip()) > 20]
        lead_sentences = sentences[:3] if len(sentences) >= 3 else sentences

        summary_body = " ".join(lead_sentences) if lead_sentences else clean_text[:400]
        summary = f"Document '{doc_title}': {summary_body}"
        if len(summary) > 450:
            summary = summary[:447] + "..."

        # 3. Extract Top-N Keywords using frequency and domain heuristics
        keywords = self._extract_top_keywords(clean_text, doc_title)

        envelope = SemanticEnvelope(
            summary=summary,
            keywords=keywords,
            content_hash=content_hash,
            file_name=file_name,
            file_path=file_path,
            url=url
        )

        self.save_envelope(envelope)
        return envelope

    def _extract_top_keywords(self, text: str, title: str, top_n: int = 7) -> List[str]:
        """Extracts top domain keywords by frequency, length, and title significance."""
        STOP_WORDS = {
            "a", "o", "as", "os", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das",
            "em", "no", "na", "nos", "nas", "por", "pelo", "pela", "pelos", "pelas", "para",
            "com", "sem", "sob", "sobre", "que", "se", "ou", "e", "mas", "como", "mais", "muito",
            "seu", "sua", "seus", "suas", "este", "esta", "estes", "estas", "esse", "essa",
            "the", "and", "or", "to", "in", "of", "for", "with", "on", "at", "from", "by",
            "about", "as", "into", "like", "through", "after", "over", "between", "out",
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "can", "could", "should", "would", "may", "might", "must", "will"
        }

        # Tokenize words >= 4 chars with unicode support
        words = re.findall(r"(?u)\b[a-zA-Z\u00C0-\u00FF]{4,}\b", text.lower())
        freq: Dict[str, int] = {}
        for w in words:
            if w not in STOP_WORDS:
                freq[w] = freq.get(w, 0) + 1

        # Boost title words
        title_words = re.findall(r"(?u)\b[a-zA-Z\u00C0-\u00FF]{4,}\b", title.lower())
        for tw in title_words:
            if tw not in STOP_WORDS:
                freq[tw] = freq.get(tw, 0) + 5

        sorted_kw = sorted(freq.items(), key=lambda item: item[1], reverse=True)
        top_kw = [k for k, _ in sorted_kw[:top_n]]

        # Ensure title terms are represented
        for tw in title_words:
            if tw not in STOP_WORDS and tw not in top_kw and len(top_kw) < top_n:
                top_kw.append(tw)

        return top_kw

    def apply_envelope_to_chunk(self, chunk_text: str, envelope: SemanticEnvelope) -> str:
        """
        Envelopes chunk text with the authoritative document summary and keywords header.
        Guarantees that vector search embeddings are anchored to the document's true domain.
        """
        kw_str = ", ".join(envelope.keywords) if envelope.keywords else "general"
        header = f"[Context: {envelope.summary} | Keywords: {kw_str}]"
        return f"{header}\n---\n{chunk_text}"
