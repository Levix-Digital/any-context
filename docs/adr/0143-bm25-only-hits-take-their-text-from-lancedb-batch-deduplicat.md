---
id: 143
title: BM25-only hits take their text from LanceDB; batch deduplication by chunk id
status: accepted
date: '2026-10-18'
version: v0.45.1
subsystems: [retrieval, ingestion]
supersedes: []
superseded_by: []
amends: [108, 142]
amended_by: []
---

# ADR-0143: BM25-only hits take their text from LanceDB; batch deduplication by chunk id

## Context

Up to v0.45.0 a chunk's text and its deduplication key depended on which list found it (BUG-011, BUG-016):

- Ingestion indexes in BM25 the enveloped text built by `apply_envelope_to_chunk` (`[Context: <summary> | Keywords: <keywords>]`, `---`, then the chunk), which is also the text embedded; LanceDB stores the raw chunk with the summary and keywords as separate columns.
- `search_single_async` took the text of a hit found only by BM25 from the BM25 record, so it reached the cross-encoder, the density budget and the prompt with the envelope, while a dense hit reached them without it. The same summary repeated in every BM25-only chunk of a file.
- The BM25-only metadata lookup of [ADR-0142](0142-chunk-provenance-columns-in-lancedb-added-in-place.md) already read those rows from LanceDB, keeping only the provenance.
- `content_hash` is the SHA-256 of the whole source file (of the page text for web pages), the same in every chunk of that file, and is used by incremental sync. `retrieve_hybrid_batch_async` (Deep Search) deduplicated on it ([ADR-0108](0108-reflective-deep-search-engine.md)), so every dense hit of one file collapsed into the first one with the scores of the others; BM25-only hits had an empty hash and were keyed by chunk id.
- The BM25 index re-tokenizes its stored text after every removal and its file is positional bincode, so its text must stay the indexed one and its record cannot gain fields without a reindex.

## Decision

- **Text of BM25-only hits**: `NativeHybridPipeline::lookup_stored_rows` runs the same single `id IN (...)` metadata query as before and returns the whole LanceDB row, keyed by chunk id and normalized file path. A BM25-only hit takes `text`, `content_hash` and provenance from it; name, path, workspace, content type and modification date still come from the BM25 record. Encrypted texts are decrypted by the existing step.
- **Fallback**: when no row is found (index out of step with LanceDB, lookup error) the BM25 text goes through `NativeContextualEnricher::strip_envelope`, which removes a leading envelope matched by an anchored regex built from the exact format of `apply_envelope_to_chunk` (keywords are letters only) and leaves any other text unchanged.
- **BM25 unchanged**: the BM25 index keeps indexing and storing the enveloped text, so contextual lexical matching does not change, and its file format is unchanged.
- **Deduplication**: `retrieve_hybrid_batch_async` keys its cross-query deduplication on `chunk_id` (then `file_path::text` when the id is empty), the key its final mapping already used. Files with identical content share chunk ids and are still merged. `content_hash` keeps meaning "hash of the source".

## Alternatives Considered

- **Store the raw text in the BM25 `DocRecord`**: breaks every existing `bm25_index.bin` (positional bincode, quarantine and reindex) for a value LanceDB already holds.
- **Only strip the envelope from the BM25 text**: keeps two text sources and silently fails on any text the pattern does not recognize; kept only as the fallback.
- **Stop enveloping the BM25 text**: loses keyword and summary matching from contextual retrieval ([ADR-0106](0106-full-rust-parity-python-purge.md)).
- **A per-chunk content hash column**: a schema change with no gain once the chunk id is the key.

## Consequences

- A chunk has the same text, content hash and provenance whichever list found it; the cross-encoder scores comparable texts and BM25-only hits no longer spend the density budget on repeated summaries.
- Deep Search returns distinct chunks of one file up to `max_chunks_per_source`, instead of one chunk carrying the sum of their scores.
- Existing indexes benefit at once; no reindex.
- Verified by `test_bm25_only_hits_take_raw_text_from_lancedb`, `test_bm25_only_hits_without_a_row_drop_the_envelope`, `test_batch_keeps_every_chunk_of_a_file` and `test_batch_merges_the_same_chunk_across_sub_queries` (`pipeline.rs`), `test_strip_envelope_round_trips_apply_envelope` and `test_strip_envelope_leaves_other_text_unchanged` (`enricher.rs`).
