"""
Parallel Vector Indexer (Fase 2).
Provides high-throughput concurrent document ingestion, contextual enrichment,
batch vector embeddings, and zero-copy columnar persistence into LanceDB.
"""
import os
import hashlib
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any, Optional, Callable

from any_context.vector_engine.models import IngestionConfig
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.enricher import ContextualEnricher
from llama_index.core import Document


class ParallelIndexer:
    """
    High-throughput parallel vector ingestion engine.
    Encapsulates document parsing, contextual enrichment, batch embedding, and LanceDB insertion.
    """

    def __init__(self, store: Optional[LanceDBStore] = None, enricher: Optional[ContextualEnricher] = None):
        self._store = store or LanceDBStore.get_instance()
        self._enricher = enricher or ContextualEnricher()

    def _get_text_embeddings_batch(self, texts: List[str]) -> List[List[float]]:
        """Generates embeddings in batch using LlamaIndex / OpenAI configured model."""
        from llama_index.core.settings import Settings
        from any_context.tools.search_tools import configure_embedding_model

        if Settings.embed_model is None:
            configure_embedding_model()

        return Settings.embed_model.get_text_embedding_batch(texts)

    def _embed_batch_with_retry(self, texts: List[str], max_retries: int = 6) -> List[List[float]]:
        """
        Embeds a batch of texts with thread-isolated exponential backoff and randomized jitter.
        Each concurrent worker independently retries without blocking or slowing other threads.
        """
        import time
        import random
        for attempt in range(max_retries):
            try:
                return self._get_text_embeddings_batch(texts)
            except Exception as e:
                err_str = str(e).lower()
                is_quota = "quota" in err_str or "billing" in err_str or "insufficient_quota" in err_str
                if is_quota:
                    raise RuntimeError(
                        "❌ OpenAI Quota Exceeded: Sua chave da OpenAI atingiu o limite de saldo ou cota de uso ($0.00 disponível). "
                        "Acesse https://platform.openai.com/billing para recarregar créditos ou configure um provedor local/offline."
                    ) from e

                is_rate_limit = ("rate" in err_str and "limit" in err_str) or "429" in err_str or "tpm" in err_str
                if is_rate_limit and attempt < max_retries - 1:
                    # Thread-isolated backoff with desynchronized jitter to avoid thundering herd
                    sleep_time = (1.5 * (1.8 ** attempt)) + random.uniform(0.4, 1.8)
                    time.sleep(sleep_time)
                else:
                    raise
        return []

    def index_documents(
        self,
        documents: List[Any],
        workspace_name: str = "Default",
        config: Optional[IngestionConfig] = None,
        progress_callback: Optional[Callable[[int, int, str, str], None]] = None
    ) -> Dict[str, Any]:
        """
        Processes and indexes a list of LlamaIndex Document instances into LanceDB in parallel.
        Emits live real-time progress callbacks for both contextual enrichment and batch vector embeddings.
        """
        if not documents:
            return {"status": "empty", "indexed_chunks": 0}

        cfg = config or IngestionConfig()
        total_docs = len(documents)

        # 1. Parallel Contextual Enrichment
        def _enrich_single_doc(doc):
            if doc.metadata.get("is_system_help"):
                return doc, None
            fp = doc.metadata.get("file_path") or getattr(doc, "id_", getattr(doc, "doc_id", ""))
            fn = doc.metadata.get("file_name") or os.path.basename(str(fp))
            envelope = self._enricher.extract_rich_summary_and_keywords(
                doc_text=doc.text,
                file_name=fn,
                file_path=str(fp),
                url=doc.metadata.get("url")
            )
            doc.metadata["document_summary"] = envelope.summary
            doc.metadata["keywords"] = ", ".join(envelope.keywords)
            new_text = self._enricher.apply_envelope_to_chunk(doc.text, envelope)
            if hasattr(doc, "set_content"):
                doc.set_content(new_text)
            else:
                doc = Document(text=new_text, metadata=dict(doc.metadata), id_=getattr(doc, "id_", getattr(doc, "doc_id", None)))
            return doc, envelope

        enriched_results = []
        with ThreadPoolExecutor(max_workers=cfg.max_workers) as executor:
            future_to_doc = {executor.submit(_enrich_single_doc, doc): doc for doc in documents}
            completed_enrich = 0
            for future in as_completed(future_to_doc):
                doc, env = future.result()
                enriched_results.append((doc, env))
                completed_enrich += 1
                if progress_callback:
                    fn = doc.metadata.get("file_name") or os.path.basename(str(doc.metadata.get("file_path", "")))
                    progress_callback(completed_enrich, total_docs, "enriching", str(fn))

        # 2. Chunking (100% Native Rust Polymorphic Ingestion Router)
        from any_context.ingestion.router import IngestionRouter
        router = IngestionRouter(max_chunk_chars=cfg.chunk_size, overlap_chars=cfg.chunk_overlap)

        raw_chunks = []
        for doc, _ in enriched_results:
            fp = str(doc.metadata.get("file_path") or getattr(doc, "id_", getattr(doc, "doc_id", "")))
            node_ws = doc.metadata.get("workspace") or getattr(doc, "metadata", {}).get("workspace") or workspace_name

            fp_lower = fp.lower()
            if fp_lower.endswith((".xlsx", ".xls", ".ods", ".pdf", ".png", ".jpg", ".jpeg", ".webp")):
                rust_chunks = router.chunk_file(fp)
            else:
                rust_chunks = router.chunk_text(fp, doc.text)

            for rc in rust_chunks:
                ct = rc.get("content_type")
                if ct == "python" or fp_lower.endswith((".py", ".pyw", ".pyi")):
                    content_type_str = "Python Source Code"
                elif ct == "typescript" or fp_lower.endswith((".ts", ".tsx")):
                    content_type_str = "TypeScript Source Code"
                elif ct == "javascript" or fp_lower.endswith((".js", ".jsx", ".mjs", ".cjs")):
                    content_type_str = "JavaScript Source Code"
                elif ct == "java" or fp_lower.endswith(".java"):
                    content_type_str = "Java Source Code"
                elif ct == "csharp" or fp_lower.endswith(".cs"):
                    content_type_str = "C# Source Code"
                elif ct == "go" or fp_lower.endswith(".go"):
                    content_type_str = "Go Source Code"
                elif ct == "rust" or fp_lower.endswith(".rs"):
                    content_type_str = "Rust Source Code"
                elif ct == "c" or fp_lower.endswith((".c", ".h")):
                    content_type_str = "C Source Code"
                elif ct == "cpp" or fp_lower.endswith((".cpp", ".hpp", ".cc", ".cxx", ".c++", ".hh", ".hxx")):
                    content_type_str = "C++ Source Code"
                elif ct == "kotlin" or fp_lower.endswith((".kt", ".kts")):
                    content_type_str = "Kotlin Source Code"
                elif ct == "swift" or fp_lower.endswith(".swift"):
                    content_type_str = "Swift Source Code"
                elif ct == "ruby" or fp_lower.endswith(".rb"):
                    content_type_str = "Ruby Source Code"
                elif ct == "php" or fp_lower.endswith((".php", ".phtml")):
                    content_type_str = "PHP Source Code"
                elif ct == "lua" or fp_lower.endswith(".lua"):
                    content_type_str = "Lua Source Code"
                elif ct == "dart" or fp_lower.endswith(".dart"):
                    content_type_str = "Dart Source Code"
                elif ct == "markdown" or fp_lower.endswith((".md", ".markdown", ".rst", ".mdown")):
                    content_type_str = "Markdown Document"
                elif ct == "xml" or fp_lower.endswith(".xml"):
                    content_type_str = "XML Structured Document"
                elif ct in ("json", "jsonl") or fp_lower.endswith((".json", ".jsonl", ".ndjson")):
                    content_type_str = "JSON Structured Data"
                elif ct in ("yaml", "yml") or fp_lower.endswith((".yaml", ".yml")):
                    content_type_str = "YAML Configuration"
                elif ct == "toml" or fp_lower.endswith(".toml"):
                    content_type_str = "TOML Configuration"
                elif ct in ("csv", "tsv") or fp_lower.endswith((".csv", ".tsv")):
                    content_type_str = "CSV / Delimited Data"
                elif ct == "excel" or fp_lower.endswith((".xlsx", ".xls")):
                    content_type_str = "Excel Spreadsheet"
                elif ct == "ods" or fp_lower.endswith(".ods"):
                    content_type_str = "OpenDocument Spreadsheet"
                elif ct == "ofx" or fp_lower.endswith(".ofx"):
                    content_type_str = "OFX Financial Statement"
                elif ct == "pdf" or fp_lower.endswith(".pdf"):
                    content_type_str = "PDF Document (Digital Layout)"
                elif ct == "pdf_scan":
                    content_type_str = "PDF Scanned Document (OCR)"
                elif ct == "image_ocr":
                    content_type_str = "Image Document (OCR Scan)"
                elif ct == "visual_diagram":
                    content_type_str = "Visual Diagram / Image (Vision AI)"
                elif ct == "sql" or fp_lower.endswith(".sql"):
                    content_type_str = "SQL Script"
                elif ct == "shell" or fp_lower.endswith((".sh", ".bash", ".ps1", ".bat", ".cmd")):
                    content_type_str = "Shell Script"
                elif ct == "config" or fp_lower.endswith((".env", ".ini", ".cfg", ".conf", ".properties")) or os.path.basename(fp_lower).startswith(".env"):
                    content_type_str = "Configuration / Env"
                elif ct == "build" or os.path.basename(fp_lower) in ("dockerfile", "makefile") or fp_lower.endswith((".dockerfile", ".makefile")):
                    content_type_str = "Container / Build Definition"
                elif ct == "web" or fp_lower.startswith(("http://", "https://")) or fp_lower.endswith((".html", ".htm")):
                    content_type_str = "Web Documentation"
                elif ct == "text" or fp_lower.endswith((".txt", ".text", ".log")):
                    content_type_str = "Plain Text Document"
                else:
                    content_type_str = doc.metadata.get("content_type", "Document")

                raw_chunks.append({
                    "id": f"{node_ws}_{hashlib.sha256(rc['text'].encode('utf-8')).hexdigest()[:20]}",
                    "text": rc["text"],
                    "file_name": rc.get("file_name") or doc.metadata.get("file_name", "Unknown"),
                    "file_path": fp,
                    "workspace": node_ws,
                    "last_modified": doc.metadata.get("last_modified_date") or doc.metadata.get("last_modified") or "",
                    "content_type": content_type_str,
                    "document_summary": doc.metadata.get("document_summary", ""),
                    "keywords": doc.metadata.get("keywords", ""),
                    "content_hash": doc.metadata.get("content_hash") or hashlib.sha256(rc["text"].encode("utf-8")).hexdigest()
                })

        if not raw_chunks:
            return {"status": "empty", "indexed_chunks": 0}

        # 2.5 Fail-safe token ceiling barrier per embedding model specification
        from any_context.tools.search_tools import get_embedding_token_limit
        token_limit = get_embedding_token_limit()
        safe_token_ceiling = max(256, int(token_limit * 0.95))

        sanitized_chunks = []
        for rc in raw_chunks:
            text = rc["text"]
            if len(text) > safe_token_ceiling * 3:
                try:
                    import tiktoken
                    enc = tiktoken.get_encoding("cl100k_base")
                    tokens = enc.encode(text)
                    if len(tokens) > safe_token_ceiling:
                        truncated_text = enc.decode(tokens[:safe_token_ceiling])
                        rc = dict(rc)
                        rc["text"] = truncated_text
                except Exception:
                    if len(text) > safe_token_ceiling * 4:
                        rc = dict(rc)
                        rc["text"] = text[:safe_token_ceiling * 4]
            sanitized_chunks.append(rc)
        raw_chunks = sanitized_chunks

        # 3. Parallel Batch Vector Embeddings
        total_chunks = len(raw_chunks)
        batch_size = cfg.batch_embed_size
        batches = [raw_chunks[i:i + batch_size] for i in range(0, total_chunks, batch_size)]

        def _process_embed_batch(batch):
            texts = [c["text"] for c in batch]
            embeddings = self._embed_batch_with_retry(texts)
            batch_records = []
            for chunk_data, emb in zip(batch, embeddings):
                item = dict(chunk_data)
                item["vector"] = emb
                batch_records.append(item)
            return batch_records

        records_to_insert = []
        with ThreadPoolExecutor(max_workers=min(cfg.max_workers, max(1, len(batches)))) as executor:
            future_to_batch = {executor.submit(_process_embed_batch, b): b for b in batches}
            completed_chunks = 0
            for future in as_completed(future_to_batch):
                res_list = future.result()
                records_to_insert.extend(res_list)
                completed_chunks += len(res_list)
                if progress_callback:
                    progress_callback(completed_chunks, total_chunks, "embedding", f"{completed_chunks}/{total_chunks} chunks")

        # 4. Columnar Persistence in LanceDB
        dim = len(records_to_insert[0]["vector"]) if records_to_insert else 1536
        self._store.upsert_records(records_to_insert, dim=dim)
        if progress_callback:
            progress_callback(total_chunks, total_chunks, "persisting", "LanceDB")

        return {
            "status": "success",
            "indexed_documents": len(documents),
            "indexed_chunks": len(records_to_insert),
            "workspace": workspace_name
        }
