---
title: Document Chunkers
summary: Turns each supported file type into chunks with line provenance and a content type.
code: [crates/any-context-core-rs/src/ingestion/chunkers, crates/any-context-core-rs/src/ingestion/router.rs]
---

# Document Chunkers

Turns each supported file type into chunks with line provenance and a content type.

## Responsibilities

- Parse files of each supported format in pure Rust (no Python, no external services) and split them into `ChunkPayload`s small enough to embed.
- Keep provenance on every chunk: file name and path, a header path or breadcrumb, start and end lines, content type and chunk index. Markdown, code, structured data and plain text chunkers report file line numbers; PDF and spatial forms report lines within a page, Office lines of the extracted text, images OCR lines, tabular chunkers row or transaction indexes, so only the first group's lines are stored as a file line range (`IngestionRouter::has_file_lines`, see [Ingestion and Sync](ingestion.md)).
- Prefix chunk text with a `// Context: <file> > <location>` breadcrumb so a chunk is understandable on its own.

Choosing which chunks to keep (quality gate) and writing them is the job of [Ingestion and Sync](ingestion.md).

## Structure

`IngestionRouter` (`ingestion/router.rs`) owns one instance of each chunker. Its default is `IngestionRouter::new(1800, 200)`: 1,800 characters per chunk and 200 characters of overlap; the PDF chunker uses at least 8,000 characters.

| Chunker | Module | Formats | Content types |
|---|---|---|---|
| `MarkdownHeaderChunker` | `chunkers/markdown.rs` | `md`, `markdown`, `rst`, `mdown`, web pages with headings | Markdown sections |
| `ASTCodeChunker` | `chunkers/code/` | Python, TypeScript/JavaScript, Java, C#, Go, Rust, C/C++, Kotlin, Swift, Ruby, PHP, Lua, Dart | code symbols |
| `StructuredDataChunker` | `chunkers/structured/` | `xml`, `json`, `jsonl`, `ndjson`, `yaml`, `yml`, `toml` | `xml`, `json`, `yaml`, `toml` |
| `TabularChunker` | `chunkers/tabular/` | `csv`, `tsv`, `ofx`, `xlsx`, `xls`, `ods` | `csv`, `tsv`, `ofx`, `excel`, `ods` |
| `PdfChunker` | `chunkers/pdf.rs` | `pdf` | `pdf`, `pdf_scan` |
| `OfficeChunker` | `chunkers/office.rs` | `docx`, `pptx` | `Word Document`, `PowerPoint Presentation` |
| `ImageChunker` | `chunkers/image.rs` | `png`, `jpg`, `jpeg`, `webp` | `image_ocr`, `visual_diagram` |
| `TextChunker` | `chunkers/text.rs` | `txt`, `log`, `sql`, shell scripts, configuration files, `Dockerfile`/`Makefile`-style names, HTML, and anything else | `text`, `sql`, `shell`, `config`, `web` |
| `SpatialFormChunker` | `chunkers/spatial_form.rs` | dense forms (called by the ingestion model router) | `document_form` |

## Behavior

### Routing

`IngestionRouter::chunk_file_native` handles binary formats first (spreadsheets, PDF, Office, images) and reads everything else as text. `chunk_text_native` then dispatches by extension: Markdown, AST code, structured data, tabular, PDF, and finally the text chunker as the default. Web URLs go to the Markdown chunker when the text contains headings and to the text chunker otherwise. `chunk_bytes_native` offers the same dispatch for in-memory bytes.

### Formats

- **Markdown**: splits by headings, tracking the header stack so each chunk carries its heading path; long sections are split with overlap.
- **Code**: one tree-sitter grammar per language (`tree-sitter-*` crates). Each `LanguageASTParser` emits symbol-level chunks (functions, classes, methods) with breadcrumbs like `// Context: <file> > <symbol> [lines a-b]`. Oversized symbols are split by `split_oversized_code` in three tiers (paragraphs, lines, then a character window); every slice uses `safe_floor_char_boundary` so a split never lands inside a multi-byte UTF-8 character.
- **Structured data**: JSON, YAML, TOML and XML are split along their structure (keys, array items, sections, elements; JSON Lines in line batches) with a path breadcrumb such as `<file> > <key> > items [a..b]`.
- **Tabular**: CSV/TSV rows become Markdown tables that repeat the header row in every chunk (`<file> > rows a..b`); Excel and ODS sheets are read with `calamine`; OFX financial statements have their own parser (`tabular/ofx.rs`).
- **PDF**: pages are decoded with `lopdf`; text spans are extracted with their 2D coordinates (including nested Form XObjects) and `reconstruct_spatial_layout` groups them into lines and columns, rebuilding tables and forms as Markdown with a `// Context: <file>.pdf > Page <N>` breadcrumb. A page with fewer than 50 characters of text is emitted as `pdf_scan`.
- **Office**: `docx` and `pptx` are unzipped in memory and their OpenXML (`word/document.xml`, `ppt/slides/slide*.xml`) is parsed into text with breadcrumbs.
- **Images**: dimensions and format are decoded natively. If a Tesseract binary is found (first `%LOCALAPPDATA%\actx\bin\tesseract\`, then the system), OCR text of at least 30 characters becomes an `image_ocr` chunk; otherwise the image becomes a `visual_diagram` chunk describing resolution and format.
- **Spatial forms**: `SpatialFormChunker` pairs labels with values using typography, punctuation (`:`, `-`, `___`) and 2D proximity, without domain vocabularies, and emits `document_form` chunks.

## Interfaces

- **Called by**: `NativeSyncOrchestrator` (files and web pages) and the ingestion model router (spatial forms), see [Ingestion and Sync](ingestion.md).
- **Contract**: `ingestion::traits::Chunker` (`chunk(file_path, content) -> Vec<ChunkPayload>`), with `ChunkPayload` defined in `crates/any-context-core-rs/src/models.rs`.
- **External tools**: an optional Tesseract executable for OCR.

## Decisions

- [ADR-0102](../adr/0102-scoped-folder-ingestion-utf8-boundaries.md): UTF-8 safe slicing in the code splitter.
- [ADR-0105](../adr/0105-native-rust-ingestion-sync-pipeline.md): tree-sitter AST, Markdown, PDF and tabular chunking in native Rust.
- [ADR-0106](../adr/0106-full-rust-parity-python-purge.md): Office (`docx`/`pptx`) chunker ported to Rust.
- [ADR-0110](../adr/0110-ingestion-model-router-quality-gate.md): `SpatialFormChunker` for domain-agnostic form reconstruction.
- [ADR-0113](../adr/0113-tabular-chunker-markdown-table-preservation.md): conservative key-value pairing in tables.
- [ADR-0142](../adr/0142-chunk-provenance-columns-in-lancedb-added-in-place.md): which chunkers' line numbers are stored as file line ranges.
