---
plan_id: any_context_section_and_line_metadata_are_lost_at_ingestion
project: any-context
title: Section and line metadata are lost at ingestion
kind: bug
version: v0.45.1
status: in_progress
author: Claude
created_at: '2026-10-18T08:31:37'
updated_at: '2026-10-18T08:43:31'
session_id: null
tags:
- ingestion
- lancedb
- retrieval
- prompt
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-010
milestones:
- name: M1 Storage and ingestion
  done: false
- name: M2 Retrieval and prompt
  done: false
- name: M3 Docs and manual tests
  done: false
- name: M4 Review and PR
  done: false
status_log:
- at: '2026-10-18T08:31:37'
  by: Claude
  status: draft
  note: Plan created
- at: '2026-10-18T08:33:33'
  by: Claude
  status: active
  note: Plan updated
- at: '2026-10-18T08:43:31'
  by: Claude
  status: in_progress
  note: Approved by the user on 2026-10-18
---

# Section and line metadata are lost at ingestion

Bug plan for BUG-010 (`dev-cycle-protocol` full path: it changes the LanceDB table schema with an in-place migration, a public struct shared by the core and the CLI, and the prompt envelope; one decision with alternatives becomes an ADR). Target: patch release v0.45.1, in its own pull request into `dev`.

## Backlog items

### BUG-010: Section and line metadata are lost at ingestion
- **Severity**: medium
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: When a document envelope exists, `embed_and_upsert_chunks` stores the document summary in place of the chunk's `header_path`, and start/end lines are not stored in LanceDB. The prompt header only shows the file name, and the Deep Search retriever passes `header_path`, `start_line` and `end_line` as `None`, although `SUMMARY_PROMPT` asks for `[path:lines]` citations. Related to the LanceDB schema changes planned in Frentes C and D (IMP-004).

Since BUG-009 (ADR-0141) the Deep Search synthesis goes through `ContextFormatter`, and `SUMMARY_PROMPT` no longer asks for `[path:lines]`; the canonical rule asks to cite source files or chunk ids. The defect remains: no surface can cite a section or a line range, and FEAT-016 (open a cited chunk at its file and line) depends on this.

Related items, deliberately out of scope: BUG-011 (chunk text depends on the retrieval path), BUG-014 (pseudo-vectors on failed embeddings), IMP-004 (per-table vector dimension), ADJ-005 (`rank` attribute), FEAT-016 (opening a citation).

## 1. Reproduction

No log is involved: the metadata is dropped in memory before the LanceDB write and the prompt is not logged. Reproduced by reading the code on `dev` at cc7604b and by unit tests on the write and read paths.

Scenario: a workspace with `docs/guide.md` (a `## Timeouts` section at lines 40-58) and `src/client.rs` (a `fn connect` at lines 12-30), synced with an AI envelope. `MarkdownHeaderChunker` and the Rust parser produce `ChunkPayload { header_path: Some("guide.md > Timeouts"), start_line: 40, end_line: 58, .. }` and `{ header_path: Some("client.rs > fn connect"), start_line: 12, end_line: 30, .. }`. The LanceDB rows hold `document_summary = "Document 'guide.md': ..."` and no section or line columns. A fast answer about timeouts receives:

```
<chunk id="..." rank="1" score="0.03" source="/home/ana/proj/docs/guide.md" file="guide.md" type="markdown">
  [Context: guide.md | Type: markdown | Modified: 2026-10-10T12:00:00+00:00]
  ...
```

The model can only cite `docs/guide.md`; Deep Search chunks carry `header_path`, `start_line` and `end_line` as `None`.

## 2. Evidence and root cause

Confirmed facts (code read on `dev` at cc7604b):

1. **Summary in the header slot.** `embed_and_upsert_chunks` (`crates/any-context-core-rs/src/ingestion/orchestrator.rs:631-639`) sets `document_summary` to the envelope summary when there is an envelope, otherwise to `c.header_path`. The column therefore mixes two meanings, and the section path is lost whenever an envelope exists (every file once enrichment runs).
2. **No columns for lines or sections.** `VectorRecord`, `ScoredVectorResult` and `NativeLanceStore::get_schema` (`storage/lancedb.rs:38-66`, `:199-220`) have no `header_path`, `start_line` or `end_line`. `start_line`/`end_line` of `ChunkPayload` are never written.
3. **Nothing reads them either.** `HybridSearchResult` (`retrieval/pipeline.rs:156-170`) has no such fields; `format_xml_chunks` (`retrieval/prompt.rs:164-233`) writes `[Context: <file_name> | ...]`; `PipelineBatchRetriever` sets them to `None` (`crates/actx-cli/src/engine.rs:703-708`); `CanonicalSynthesisFormatter` drops them (`prompt.rs:380-401`). `document_summary` is never read back by retrieval.
4. **BM25 file format.** BM25-only hits are rebuilt from `DocRecord` (`retrieval/bm25.rs:15-24`), persisted with positional bincode (`save_to_file`/`load_from_file`). Adding fields to `DocRecord` makes every existing `bm25_index.bin` fail to load; `get_or_load_bm25` then quarantines it and starts empty (`pipeline.rs:361-397`), and an incremental sync does not re-add unchanged files. Older binaries would equally fail on a new file. Without an embedding key no query vector exists, so every hit is BM25-only.
5. **Line meaning differs per chunker.** Markdown, code, structured (JSON, YAML, TOML, XML) and plain text chunkers emit file line numbers. PDF and spatial form chunks use page-relative lines (`start_line: 1`, `end_line: line_count`, header `<file> > Page N`); Office uses lines of extracted text; Excel, CSV and OFX use row or transaction indexes; images use OCR lines; web pages use lines of the fetched Markdown. Their `header_path` already names the page, sheet, rows or section.
6. **Schema evolution is available.** `lancedb` 0.39 supports `table.add_columns().transform(NewColumnTransform::AllNulls(schema))`, and `table.add` accepts input batches that lack nullable table columns (filled with nulls; `lancedb/src/table/add_data.rs:237`).

Root cause: the ingestion write path was built around a single free-text `document_summary` column and never carried the chunker provenance into the store, so every read path had nothing to show.

## 3. Fix design (`software-design`)

Hexagonal style: core types and storage first, then the retrieval core, then the prompt adapter and the CLI composition.

### Storage (`crates/any-context-core-rs/src/storage/lancedb.rs`)

- Three nullable columns appended to `get_schema`: `header_path` (Utf8), `start_line` (UInt32), `end_line` (UInt32). `VectorRecord` and `ScoredVectorResult` gain `header_path: Option<String>`, `start_line: Option<u32>`, `end_line: Option<u32>` with `#[serde(default)]`.
- `header_path` is encrypted at rest like `document_summary` and `keywords` (it holds headings and symbol names, i.e. document content); `PLAINTEXT_ROWS_FILTER` and `encrypt_plaintext_rows` cover it. Lines stay in clear text.
- New private `ensure_provenance_columns(&Table)`: when the opened table lacks any of the three columns, add the missing ones with `NewColumnTransform::AllNulls`. Called by `get_or_create_table` (every write) and by `encrypt_plaintext_rows` before it reads rows. Idempotent: one schema read when the columns exist. A failure is returned as an error from the write (no silent fallback that would make `table.add` fail later with a less clear message).
- `extract_scored_results` and `extract_vector_records` read the three columns when present (`column_by_name` returns `None` on tables not yet migrated) and map nulls to `None`.

### Ingestion (`crates/any-context-core-rs/src/ingestion/orchestrator.rs`, `router.rs`)

- `embed_and_upsert_chunks` writes `document_summary` only from the envelope (`None` without one), `keywords` as today, and `header_path` from `c.header_path` always.
- Lines are stored only when they are file line numbers: new `IngestionRouter::has_file_lines(file_path) -> bool` (pure, extension based, mirroring `chunk_text_native`): true for local Markdown, code, structured and plain text files; false for web URLs, PDF, Office, images and tabular files (CSV, TSV, Excel, OFX). `embed_and_upsert_chunks` gets a `file_lines: bool` argument from its two callers (`true` per this function for files, `false` for web pages) and stores `start_line`/`end_line` only when it is true and `1 <= start_line <= end_line`. The page, sheet or row locator of the other formats stays in `header_path`.

### Retrieval (`crates/any-context-core-rs/src/retrieval/pipeline.rs`)

- `HybridSearchResult` gains `header_path: Option<String>`, `start_line: Option<u32>`, `end_line: Option<u32>` (`#[serde(default)]`), filled from the dense or metadata hit.
- BM25 format unchanged (fact 4). After fusion, the pipeline looks up the provenance of the BM25-only hits in one `search_metadata_async` call (`id IN (...)`, workspace filter, limit = number of ids) and matches rows on `id` and `file_path`. A lookup error leaves those hits without provenance (the answer still works; it is logged at debug level like other retrieval fallbacks). This also covers the no-embedding-key mode, where every hit is BM25-only.

### Prompt (`crates/any-context-core-rs/src/retrieval/prompt.rs`)

- `format_xml_chunks`: a `lines="<start>-<end>"` attribute on `<chunk>` when both lines are present; the header line becomes `[Context: <file> | Section: <header_path> | Type: <type> | Modified: <date>]`, with `Section` only when `header_path` is present and not empty (escaped and neutralized like the rest of the header).
- Isolation rule 4 example becomes "(e.g. docs/config.toml:10-24 or [chk_1])" and says to add the line range when the chunk has a `lines` attribute. The shared `isolation_rules` keeps one source; the deterministic, tool and Deep Search prompts change together.
- `CanonicalSynthesisFormatter` maps `header_path`, `start_line`, `end_line` both ways.

### Composition (`crates/actx-cli/src/engine.rs`)

- `PipelineBatchRetriever` fills `RetrievedChunk::header_path`, `start_line`, `end_line` (as `usize`) from `HybridSearchResult`.

### Error strategy

Only the schema migration is a new error path: it fails the write with "Adding provenance columns to LanceDB failed: ...", reported by sync like other storage errors. Missing provenance on read is never an error.

### Deliberately left out

- An automatic reindex of existing rows: they keep null provenance until the file changes or the user runs `/sync --force` (one embedding pass); the release notes say so. Re-embedding every workspace on upgrade would cost an embedding pass the user did not ask for.
- Changing the BM25 file format, and the envelope difference between BM25 and LanceDB text (BUG-011).
- Opening a citation at its line (FEAT-016) and page numbers as structured fields for PDFs.
- Line metadata for the ReAct `search_db` tool result beyond what `format_tool_result_payload` gets from `format_xml_chunks` (it benefits automatically).

### ADR

New ADR-0142 "Chunk provenance columns in LanceDB, added in place": the three nullable columns, `header_path` encrypted, `document_summary` only for the envelope summary, lines only for file-line chunkers, in-place `add_columns` migration without reindex, provenance of BM25-only hits looked up in LanceDB. Alternatives recorded: forced reindex on upgrade (embedding cost), new table with a full copy (doubles storage during the copy, larger migration), adding the fields to the BM25 `DocRecord` (breaks every existing `bm25_index.bin` and older binaries), storing lines for every chunker (page-relative and row numbers would produce wrong `file:lines` citations), keeping `header_path` in clear text (leaks headings and symbol names that ADR-0131 protects).

## 4. Affected surfaces and files

- `crates/any-context-core-rs/src/storage/lancedb.rs`: schema, records, migration, extraction, encryption filter, tests.
- `crates/any-context-core-rs/src/ingestion/orchestrator.rs`, `ingestion/router.rs`: write path, `has_file_lines`, callers, tests.
- `crates/any-context-core-rs/src/retrieval/pipeline.rs`: `HybridSearchResult` fields, provenance lookup for BM25-only hits, tests.
- `crates/any-context-core-rs/src/retrieval/prompt.rs`: `lines` attribute, `Section` header, rule 4, `CanonicalSynthesisFormatter` mapping, tests.
- `crates/actx-cli/src/engine.rs`: `PipelineBatchRetriever` mapping; other `HybridSearchResult` literals in tests.
- Surfaces `cli` and `tui`: no command, flag or setting changes. Stored data: the LanceDB table gains three nullable columns on the first write or migration after the upgrade. Answers can cite sections and line ranges.

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`):

- **Regression test** (`orchestrator.rs`) `test_upsert_keeps_header_path_and_lines_with_envelope`: with an envelope, the stored row has `document_summary` = envelope summary, `header_path` = the chunk's header path, and `start_line`/`end_line` of a Markdown chunk. Fails before the fix (header lost, no lines).
- `test_upsert_skips_lines_for_page_relative_chunkers`: a PDF-path chunk keeps `header_path` and has no lines; a web page has no lines.
- `router.rs` `test_has_file_lines`: Markdown, Rust, JSON, `.txt` true; PDF, DOCX, PNG, CSV, XLSX, OFX and `https://` false.
- `lancedb.rs` `test_provenance_columns_added_to_legacy_table`: a table created with the old schema (11 columns) and rows is migrated in place by the next `upsert_records`; old rows read back with `None` provenance, new rows with their values; the row count and vectors are unchanged.
- `lancedb.rs` `test_header_path_encrypted_at_rest`: the dataset files do not contain the header path in clear text; it decrypts on read; `encrypt_plaintext_rows` also encrypts a plaintext `header_path`.
- `pipeline.rs` `test_bm25_only_hits_get_provenance_from_lancedb`: a chunk in BM25 and LanceDB found with no query vector carries `header_path` and lines.
- `prompt.rs` `test_xml_chunk_shows_section_and_lines`: `lines="40-58"` and `Section: guide.md > Timeouts`; no attribute and no `Section` without provenance; a header path with `</chunk>` is neutralized; `test_reserved_tags_cover_the_envelope` still passes.
- `prompt.rs` `test_canonical_synthesis_formatter_maps_chunks` extended with section and lines.
- Existing tests updated for the new struct fields; `format_deterministic_prompt` equality test updated once for rule 4.

Manual (`tests/MANUAL_TESTS.md`):

- New **MT-ANS-007: Answers cite the section and line range**: after `/sync --force`, a fast and a deep question about a Markdown section and a code function cite `file:start-end`; a PDF answer cites the file and page, not line numbers.
- Update **MT-SYNC-002** expected result: after upgrading, unchanged files keep working (answers without line ranges) and a changed file is cited with lines after the next sync.

Acceptance criteria: provenance stored for new and changed files; existing tables migrated in place without reindex and readable by the previous version; fast, tool and Deep Search prompts show `Section` and `lines` when known; no wrong line ranges for page or row based formats.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/storage.md`: LanceDB columns, `header_path` encryption, in-place provenance migration.
- `docs/architecture/ingestion.md`: what `embed_and_upsert_chunks` stores; `has_file_lines`.
- `docs/architecture/retrieval.md`: `HybridSearchResult` provenance, BM25-only lookup, `lines` attribute and `Section` header.
- `docs/architecture/chunkers.md`: which chunkers produce file lines.
- ADR-0142 in Decisions of storage and retrieval; `build_index.py`, `check_docs.py`.
- README: Troubleshooting row "Answers do not cite line ranges for older files: run `/sync --force` once"; `check_readme.py`.

## 7. Release, migration and rollback (`devops-release`)

- Version: part of v0.45.1 (patch, with BUG-009). No CI workflow change.
- Migration: additive, backward compatible. The first write (sync) or `encrypt_plaintext_rows` after the upgrade adds three nullable columns; no row is rewritten and nothing is re-embedded. Release notes: "Run `/sync --force` once for line citations on files indexed before v0.45.1."
- Older versions on the same data: they read by column name and ignore the new columns; their appends omit the columns, which LanceDB fills with nulls. Their `encrypt_plaintext_rows` only runs on plaintext rows, which v0.45.1 never writes.
- BM25 index file unchanged.
- Roadmap fit: the post-v0.45.0 roadmap wants the Frente C dimension migration and the Frente D `parent_id`/level columns combined so LanceDB is not migrated twice. Adding nullable columns with `AllNulls` is a metadata-only commit (no row rewrite, no re-embedding), so it does not create a second costly migration; `ensure_provenance_columns` is the place where Frente D adds its nullable columns. The Frente C dimension change still needs its own reindex plan (IMP-004).
- Rollback: revert through a pull request and ship the next patch; the extra columns stay and are harmless to the previous version.
- Observability: the migration is logged once in the sync log ("LanceDB: added provenance columns header_path, start_line, end_line"), covered by the migration test through the returned flag.

## 8. Execution steps and milestones

1. **M1 Storage and ingestion**: schema, records, migration, encryption, write path, `has_file_lines`, tests.
2. **M2 Retrieval and prompt**: `HybridSearchResult` provenance, BM25-only lookup, `format_xml_chunks`, rule 4, Deep Search mapping, tests.
3. **M3 Docs and manual tests**: ADR-0142, architecture pages, README, MT-ANS-007, MT-SYNC-002; all checkers clean.
4. **M4 Review and PR**: code review, `check_design.py`, PR into `dev`, CI green.
