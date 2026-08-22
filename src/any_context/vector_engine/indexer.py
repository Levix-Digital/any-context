"""
Parallel Vector Indexer (Fase 2).
Provides high-throughput concurrent document ingestion, contextual enrichment,
batch vector embeddings, and zero-copy columnar persistence into LanceDB.
"""
import os
import hashlib
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any, Optional

from any_context.vector_engine.models import IngestionConfig
from any_context.vector_engine.store import LanceDBStore
from any_context.vector_engine.enricher import ContextualEnricher


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

    def _embed_batch_with_retry(self, texts: List[str], max_retries: int = 4) -> List[List[float]]:
        """
        Embeds a batch of texts with exponential backoff and jitter upon rate limit (429) errors.
        """
        import time
        import random
        for attempt in range(max_retries):
            try:
                return self._get_text_embeddings_batch(texts)
            except Exception as e:
                err_str = str(e).lower()
                is_rate_limit = ("rate" in err_str and "limit" in err_str) or "429" in err_str or "tpm" in err_str
                if is_rate_limit and attempt < max_retries - 1:
                    sleep_time = (1.5 * (2 ** attempt)) + (random.uniform(0.1, 0.5))
                    time.sleep(sleep_time)
                else:
                    raise
        return []

    def index_documents(
        self,
        documents: List[Any],
        workspace_name: str = "Default",
        config: Optional[IngestionConfig] = None
    ) -> Dict[str, Any]:
        """
        Processes and indexes a list of LlamaIndex Document instances into LanceDB in parallel.
        """
        if not documents:
            return {"status": "empty", "indexed_chunks": 0}

        cfg = config or IngestionConfig()
        from llama_index.core.node_parser import SentenceSplitter
        splitter = SentenceSplitter(chunk_size=cfg.chunk_size, chunk_overlap=cfg.chunk_overlap)

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

        with ThreadPoolExecutor(max_workers=cfg.max_workers) as executor:
            enriched_results = list(executor.map(_enrich_single_doc, documents))

        # 2. Chunking
        raw_chunks = []
        for doc, _ in enriched_results:
            nodes = splitter.get_nodes_from_documents([doc])
            for node in nodes:
                raw_chunks.append({
                    "id": f"{workspace_name}_{hashlib.sha256(node.text.encode('utf-8')).hexdigest()[:20]}",
                    "text": node.text,
                    "file_name": node.metadata.get("file_name", "Unknown"),
                    "file_path": node.metadata.get("file_path", ""),
                    "workspace": workspace_name,
                    "last_modified": node.metadata.get("last_modified_date") or node.metadata.get("last_modified") or "",
                    "content_type": node.metadata.get("content_type", "Local Document"),
                    "document_summary": node.metadata.get("document_summary", ""),
                    "keywords": node.metadata.get("keywords", ""),
                    "content_hash": hashlib.sha256(node.text.encode("utf-8")).hexdigest()
                })

        if not raw_chunks:
            return {"status": "empty", "indexed_chunks": 0}

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
        with ThreadPoolExecutor(max_workers=min(5, max(1, len(batches)))) as executor:
            batch_results = list(executor.map(_process_embed_batch, batches))
            for res_list in batch_results:
                records_to_insert.extend(res_list)

        # 4. Columnar Persistence in LanceDB
        dim = len(records_to_insert[0]["vector"]) if records_to_insert else 1536
        self._store.upsert_records(records_to_insert, dim=dim)

        return {
            "status": "success",
            "indexed_documents": len(documents),
            "indexed_chunks": len(records_to_insert),
            "workspace": workspace_name
        }
