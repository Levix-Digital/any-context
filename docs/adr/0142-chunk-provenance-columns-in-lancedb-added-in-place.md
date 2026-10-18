---
id: 142
title: Chunk provenance columns in LanceDB, added in place
status: accepted
date: '2026-10-18'
version: v0.45.1
subsystems: [storage, retrieval, ingestion]
supersedes: []
superseded_by: []
amends: [131]
amended_by: [143]
---

# ADR-0142: Chunk provenance columns in LanceDB, added in place

## Context

Chunkers emit provenance with every chunk: a `header_path` (section, symbol, page, sheet or row locator) and `start_line`/`end_line` (`ChunkPayload`). Up to v0.45.0 none of it reached the model (BUG-010):

- `embed_and_upsert_chunks` wrote the envelope summary into `document_summary` when an envelope existed and the `header_path` otherwise, so the section was lost for every enriched file and the column held two meanings.
- The LanceDB table (`workspace_chunks`) had no column for the section or the lines; `HybridSearchResult` had no such fields; `format_xml_chunks` showed only the file name; the Deep Search retriever set them to `None`.
- The BM25 index (`bm25_index.bin`) is positional bincode: adding fields to `DocRecord` makes every existing file fail to load (quarantine, empty index until a full reindex) and older binaries fail on new files. Without an embedding key every hit comes from BM25 only.
- Line numbers mean file lines only for Markdown, code, structured data and plain text. PDF and spatial forms use page-relative lines, Office the lines of the extracted text, images OCR lines, Excel, CSV and OFX row or transaction indexes, web pages the lines of the fetched text.
- `lancedb` 0.39 can add all-null columns to an existing table (`add_columns` with `NewColumnTransform::AllNulls`) without rewriting rows, and appends accept batches that lack nullable table columns.

## Decision

- **Columns**: `workspace_chunks` gains three nullable columns after `content_hash`: `header_path` (Utf8, encrypted like `text`, `document_summary` and `keywords`, amending [ADR-0131](0131-field-level-encryption-on-every-lancedb-write.md)), `start_line` and `end_line` (UInt32, clear text). `VectorRecord`, `ScoredVectorResult` and `HybridSearchResult` carry them as `Option` fields.
- **Migration in place**: `NativeLanceStore::ensure_provenance_columns` adds the missing columns as all-null columns. It runs at the start of every sync, inside `get_or_create_table` (every write) and inside `encrypt_plaintext_rows` (whose plaintext filter covers `header_path`). Nothing is rewritten or re-embedded; existing rows keep null provenance until their file changes or a `/sync --force`. Reads use the columns when present, so tables not migrated yet still read.
- **Write path**: `document_summary` holds only the envelope summary. `header_path` is always stored. `start_line`/`end_line` are stored only when `IngestionRouter::has_file_lines(file_path)` is true (local Markdown, code, structured and plain text files) and the range is valid; the other formats keep their page, sheet or row locator in `header_path`.
- **BM25-only hits**: the BM25 file format is unchanged. After fusion, `NativeHybridPipeline` reads the provenance of the hits that came only from BM25 in one `id IN (...)` metadata query and matches rows on chunk id and file path; a lookup error leaves them without provenance.
- **Prompt**: `format_xml_chunks` adds `lines="<start>-<end>"` to `<chunk>` and `| Section: <header_path>` to the header line when known. Isolation rules 1 and 4 describe the attribute and ask for `file:start-end` citations (e.g. `docs/config.toml:10-24`). `PipelineBatchRetriever` and `CanonicalSynthesisFormatter` pass the provenance through Deep Search.

## Alternatives Considered

- **Force a reindex on upgrade**: every workspace would pay an embedding pass the user did not ask for; `/sync --force` stays the explicit way.
- **Copy into a new table with the new schema**: doubles storage during the copy and is a larger, riskier migration for three nullable columns.
- **Add the fields to the BM25 `DocRecord`**: breaks every existing `bm25_index.bin` and older binaries (positional bincode).
- **Store lines for every chunker**: page-relative lines and row indexes would produce wrong `file:lines` citations.
- **Keep `header_path` in clear text**: section headings and symbol names are document content that ADR-0131 protects at rest.

## Consequences

- Fast, `search_db` and Deep Search prompts can cite a section and a file line range; FEAT-016 (open a citation at its line) has the data it needs.
- Files indexed before v0.45.1 have no provenance until they change or the user runs `/sync --force` once; the release notes say so.
- Older versions on the same data keep working: they read by column name and their appends leave the new columns null.
- Later nullable columns (Frente D `parent_id`, level) are added the same way in `provenance_fields`; the vector dimension change (IMP-004) still needs its own reindex.
- One extra metadata query per search when there are BM25-only hits.
- Verified by `test_provenance_columns_added_to_legacy_table`, `test_header_path_encrypted_at_rest` and `test_encrypt_plaintext_rows_migrates_legacy_rows` (`lancedb.rs`), `test_upsert_keeps_header_path_and_lines_with_envelope` and `test_upsert_skips_lines_for_page_relative_chunkers` (`orchestrator.rs`), `test_has_file_lines` (`router.rs`), `test_bm25_only_hits_get_provenance_from_lancedb` (`pipeline.rs`), `test_xml_chunk_shows_section_and_lines` and `test_canonical_synthesis_formatter_maps_chunks` (`prompt.rs`).
