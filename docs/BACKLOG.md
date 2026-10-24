# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-024: [ingestion] Secret files (.env, .tfvars, .key) are indexed in clear
- **Severity**: critical
- **Status**: planned
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: any_context_ingestion_secret_files_env_tfvars_key_are_indexed_in_clear
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7 "Personal data, sensitive data and secrets (audit P13)". The scanner accepts `.env` and `.env.*` (ingestion/scanner.rs:102-106) and the `env` and `tfvars` extensions (scanner.rs:186, 192); `key` is accepted as Keynote (scanner.rs:146), but a PEM `server.key` falls to the plain-text chunker (ingestion/router.rs:157-160). Their secrets are indexed in clear and can reach a cloud embedding model or LLM. Expected: secret values are detected and never indexed (`[SECRET REDACTED]`), with no configurable exception, while the rest of the file stays searchable.

### BUG-022: [ingestion] Folder-scoped sync purges the other folders' index
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-19, IMP-004 M1 (recorded in the backlog on 2026-10-23)
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3 "Incremental diff". A sync with a target folder scans only that folder (ingestion/orchestrator.rs:339-341) but diffs it against the stat cache of the whole workspace (orchestrator.rs:379-383), so every file of the other folders counts as deleted and its chunks are purged (orchestrator.rs:409-413). Adding a folder starts such a sync (commands/engine.rs:1039), so adding a second folder silently empties the index of the first. A fix (`is_within_folder` in scanner.rs, with a regression test) exists only on the unmerged branch feat/imp-004-embedding-model (commit 4042b89). Expected: a scoped sync only touches files inside its folder.

### BUG-025: [ingestion] An unreachable folder's files are purged from the index
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3 "Incremental diff" (an unreachable source goes offline, it is not deleted). A missing folder is skipped (ingestion/scanner.rs:265-273), so its cached files count as deleted (scanner.rs:308-312) and their chunks are purged (ingestion/orchestrator.rs:409-413); the warning comes after the purge and only in verbose mode (orchestrator.rs:461-462). Walk errors are dropped too (`filter_map(|e| e.ok())`, scanner.rs:276), so a subfolder without read permission is purged the same way. Violates "nothing disappears silently". Expected: an unreachable folder or subfolder is reported as offline, its index is kept, and it is reconciled when it comes back.

### BUG-026: [ingestion] Path healer silently redirects folders and rewrites the saved path
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3 "Incremental diff" (no silent path healing). When a folder is missing, storage/path_healer.rs:17-69 guesses another one; its last step (path_healer.rs:57-66) takes any folder with the same name in the grandparent, so a missing `/a/projX/docs` becomes an unrelated `/a/docs`. The scan uses the guess (ingestion/scanner.rs:268) and `get_workspace_folders` writes it over the saved folder (storage/sqlite.rs:585-596), with no message. Content the user never chose can be indexed and sent to a cloud model. Violates "explicit action" and "nothing disappears silently". Expected: no automatic redirection; a missing folder is reported and the user relocates it explicitly.

### BUG-027: [ingestion] Sync indexes the working directory when a workspace has no folders
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1 "Source selection". When a workspace has no folders, sync indexes the process's current directory (ingestion/orchestrator.rs:342-348), which can be the home folder, and its content can reach a cloud embedding model. Violates "every source comes from an explicit user action". Expected: a workspace without sources indexes nothing and says so.

### BUG-028: [connectors] The scan downloads online-only cloud placeholders
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 21. The scanner does not check the online-only attributes of OneDrive, Google Drive or Dropbox placeholders (Cloud Files on Windows, File Provider on macOS) (ingestion/scanner.rs:275-286), and reading the file (ingestion/router.rs:158) forces its download. Violates "explicit action": downloads and disk use the user never asked for. Expected: online-only files are skipped and counted in the source report; files already on disk are indexed normally.

### BUG-029: [retrieval] Web search without a key scrapes DuckDuckGo result pages
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 24 and 32. Without a Tavily or Serper key, web search falls back to DuckDuckGo (retrieval/web_search.rs:114-115): the Instant Answer API, which is not a search API, and then scraping html.duckduckgo.com (web_search.rs:281-293, parser at 318). DuckDuckGo has no official search API, so this breaks the official-APIs-only rule. Expected: web search uses only official APIs, or third-party APIs that are allowed and worth their cost; with none available it says so instead of scraping.

## Features

### FEAT-020: [ingestion] Single local file and single web page as sources
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1, Decisions. Besides a recursive folder and a crawled site, the user can add a single local file and a single web page (only that URL, no links followed). A file selected on its own ignores the exclude filters. Supersedes FEAT-006.

### FEAT-021: [ingestion] Recursive website scope: host and path prefix, robots.txt, 3000-page ceiling
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1, Recursive website. A crawled site stays on the same host and inside the path prefix of the given URL and honors robots.txt. Default ceiling of 3000 pages, configurable globally and per source; when the site has more, the user is told how many were indexed and how to raise the ceiling of that source.

### FEAT-022: [ingestion] Embedded headless Chromium, sandboxed and shipped in the installer
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, Chromium: distribution and security. An embedded headless Chromium renders pages so SPA sites arrive with content. It ships in the installer and is updated with every AnyContext release; it runs with the sandbox on, a disposable profile per run, downloads disabled and a timeout per page.

### FEAT-023: [ingestion] Render-or-fetch calibration per site and per page
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, When to render. Per-site calibration on a small sample (static fetch versus rendering, static when the fetch brings at least about 95% of the text), per-page check for empty or JavaScript-app pages, recalibration on each sync with a sample page, and the mode shown in the source status with an option to force rendering. When in doubt it renders.

### FEAT-024: [models] ONNX content-block classifier for web pages
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, Web: rendering and conversion to Markdown. A small ONNX classification model (not generative) marks each DOM block as content or noise (menu, footer, advertising), distributed through the verified model catalog. Without it, the Readability-style heuristic of IMP-019 plays the same role.

### FEAT-025: [ingestion] Incremental web re-sync with conditional requests and linked documents
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, Decisions. Web re-sync uses ETag, Last-Modified and the sitemap lastmod to skip unchanged pages; pages that disappear from the site leave the index; documents linked from the site within the source scope (PDF, DOCX...) are downloaded and go through the pipeline like any file.

### FEAT-026: [ingestion] Exact duplicates stored once with occurrences
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3, Duplicates and versions (audit P17). A file with the same hash stores and embeds its whole parent-child tree once, each location being an occurrence pointing to it. An identical passage in different documents reuses the vector through the per-chunk diff but still belongs to its own document and parent.

### FEAT-027: [ingestion] Version families by text fingerprint, with user split and join
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3, Duplicates and versions (audit P17). A language-independent fingerprint (MinHash/SimHash) groups near-duplicate documents into a version family at the document level. The current version is the one with the most recent document date (mtime as tiebreaker); previous ones stay indexed and marked. The user can split wrong groups and join undetected versions. Retrieval use belongs to the retrieval batch.

### FEAT-028: [ingestion] Parsers for legacy Office, ODF and RTF
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Decisions. Real parsers for .doc, .ppt, .xls, .odt, .odp, .ods and .rtf. Today .doc/.ppt are opened as ZIP and fail, and .odt and .rtf are indexed as raw text or fail.

### FEAT-029: [ingestion] Parsers for e-mail (.eml, .msg) and e-book (.epub)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Decisions. E-mail body and headers parsed, with attachments going back into the pipeline as files; EPUB parsed by chapter. Today .eml, .msg and .epub are indexed as raw text or fail.

### FEAT-030: [ingestion] Per-page PDF state: digital, scanned or broken text
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Decisions (audit P9). Each PDF page is classified as digital, scanned or digital with broken text (fonts without Unicode mapping, CID), the third treated as scanned. Detection is language-independent (replacement, private-use and control characters, improbable sequences), checked against a quick OCR when in doubt. Mixed PDFs send each page down the right path.

### FEAT-031: [ingestion] Source report: unindexed files and discarded chunks with the reason
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Decisions, and section 6, Decisions. Password-protected, corrupted or unsupported files, blocked container items and every chunk discarded by the quality gate are reported with the reason in the sync report and an inspection command, instead of a count or nothing. Principle: nothing disappears silently.

### FEAT-032: [ingestion] Archives opened recursively as child documents
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Containers and archives (audit P1). ZIP, 7z, TAR, GZ, BZ2 and XZ are opened recursively; each inner item is a child document with chained provenance and goes through the normal pipeline. Depth, uncompressed size and compression ratio limits, rejection of `../` paths, blocked items in the report. The container and each child have their own hash; source filters apply inside; citations show the inner path and opening the original extracts the item to a temporary folder.

### FEAT-033: [ingestion] Mailboxes as containers: MBOX and PST/OST
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Containers and archives (audit P1). MBOX and Outlook PST/OST are opened as containers: each message is a child document with provenance (`office.pst > Inbox > Re: contract > draft.docx`) and its attachments go back into the pipeline.

### FEAT-034: [ingestion] RAR archives, read-only
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Containers and archives (audit P1). RAR is read like the other archives, if the decompressor's license allows it; otherwise it is refused with a clear message. Depends on the open RAR license question.

### FEAT-035: [ingestion] Unlock protected files with passwords from the personal vault
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Vault of file passwords. Passwords per source or file live in the user's personal vault; settings.db keeps only a secret_id. Only the ingestion worker reads a password, in memory, wiped right after; never in logs, reports, indexes or prompts, and no agent tool reads it. Option to always ask; a wrong password fails the file and is reported, with no guessing. Depends on the personal vault (identity-and-governance-v2.md, section 2).

### FEAT-036: [ingestion] Hierarchical parent and child chunks per content type
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Decisions. Small chunks are embedded and searched; each keeps a link to a stored, not embedded parent that can be handed to the LLM: section for Markdown, Office, web and e-book; page for PDF; function, class or module for code; sheet plus row range for tables; message for e-mail.

### FEAT-037: [models] Multilingual ONNX OCR replacing Tesseract
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Decisions. OCR runs through an ONNX model (candidate PaddleOCR exported to ONNX, alternative docTR) that detects the page's writing system and uses the matching recognition model, distributed through the model catalog. Replaces the external Tesseract looked up on the PATH.

### FEAT-038: [ingestion] Full DOCX: headings, tracked changes, comments, headers and notes
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Full Office and PDF (audit P6 and P7). Heading styles become the section path; tables and lists preserved. The main text is the current version; each tracked change is its own item with author, date and passage; comments are items linked to the commented passage. Headers and footers indexed once per document; footnotes and endnotes linked to their section.

### FEAT-039: [ingestion] Full PPTX and XLSX: notes, values and formulas, merges, hidden sheets
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Full Office and PDF (audit P6 and P7). PPTX: slide titles as the section path and speaker notes with the slide; comments as items. XLSX: the computed value is indexed and the formula kept as metadata, merged cells expanded, named ranges as labels, hidden sheets indexed and marked hidden; comments as items.

### FEAT-040: [ingestion] Full PDF: outline, annotations, form fields, attachments, signatures
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Full Office and PDF (audit P6 and P7). The outline becomes the section path; annotations are items with author and date; AcroForm fields are key-value pairs; attached files go through the containers (FEAT-032); digital signatures give signer, date and whether the document changed after signing, validated offline with the OS root certificates or marked not verified.

### FEAT-041: [ingestion] Calendar, contacts and MHTML parsers
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Full Office and PDF, other formats level 1. Parsers for .ics (events), .vcf (contacts) and MHTML (through the HTML converter).

### FEAT-042: [ingestion] Visio, XPS and Apple iWork parsers
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Full Office and PDF, other formats level 2. Parsers for Visio (.vsdx), XPS and Apple iWork (Pages, Numbers, Keynote; proprietary and harder format).

### FEAT-043: [ingestion] Academic formats: LaTeX, bibliographies, notebooks, Parquet, Arrow
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Academic, engineering, GIS and health formats (audit P3). LaTeX by section with equations kept in LaTeX and \cite linked to the reference; BibTeX, RIS and CSL-JSON as records; Jupyter, R Markdown and Quarto with prose cells, code through the code parser and text outputs (images through the vision model); Parquet and Arrow as queryable tables. SPSS and Stata come later.

### FEAT-044: [ingestion] Engineering formats: DXF, IFC, STEP/IGES headers; DWG refused
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Academic, engineering, GIS and health formats (audit P4). DXF texts, layers and block attributes with the title block as fields; IFC with site > building > storey > space as the section path and each element with its properties; STEP and IGES by the header only; DWG refused with a message suggesting export to DXF or PDF.

### FEAT-045: [ingestion] GIS formats: GeoJSON, KML/KMZ, Shapefile, GeoPackage
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Academic, engineering, GIS and health formats (audit P4). Each feature becomes a record with its attributes; coordinate system and covered area stored as metadata.

### FEAT-046: [ingestion] Health formats: DICOM, HL7 v2, FHIR, CDA
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Academic, engineering, GIS and health formats (audit P5). DICOM tags, structured reports and embedded PDFs, with the medical image off by default; HL7 v2 with each segment as a record; FHIR with each resource as an item and references preserved; CDA by its clinical sections.

### FEAT-047: [ingestion] Local audio and video transcription, existing captions first
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Audio and video (audit P2). Audio MP3, WAV, M4A, OGG, FLAC, Opus and the audio track of MP4, MKV, MOV, WebM read with Symphonia; local multilingual transcription with a Whisper-family ONNX model per profile, language detection and VAD. Embedded captions or a sibling .srt/.vtt are used directly. Chunks follow sentences and turns, locators are start and end in seconds, parents are blocks of a few minutes or chapters. It is the Audio and video connector of the Pro plan (connectors-v2.md, decisions 22 and 28).

### FEAT-048: [ingestion] Speaker identification in transcripts
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Audio and video (audit P2). Optional per profile and on by default in the legal pack: speakers come out as Speaker 1, 2... and the user can name them for the whole file.

### FEAT-049: [ingestion] Video key frames and slides through FFmpeg, vision and OCR
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Audio and video (audit P2). Key frames at scene changes are described by the vision model and lecture or meeting slides go through OCR. FFmpeg ships in the installer as a separate library to comply with the LGPL.

### FEAT-050: [ingestion] Long jobs: estimate, confirmation and progressive indexing
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Long jobs. Before a large job: expected chunks and time on this machine, from previous syncs with the same embedding target. Above a threshold (e.g. 10 minutes) the user chooses to index everything, part or skip. The job runs in the background, what is indexed is searchable at once, and an interruption loses no work. User files are never split or altered.

### FEAT-051: [models] Benchmark of scanned-page extraction and vision models per profile
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Page extraction and Vision model per profile. Measure Granite-Docling, PaddleOCR-VL and the OCR + layout cascade on a real set of scanned pages, tables, forms and handwriting, and the vision model candidates on real photos, diagrams and charts: quality and time on CPU per profile, plus the license of each model. The results inform the model choice, which stays the user's decision (open benchmark question).

### FEAT-052: [ingestion] Scanned-page extraction per profile
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Page extraction (audit P8) and Local Document AI via ONNX. The text layer of a digital page is always the truth. Scanned, broken-text, photo or handwritten pages go to the page parsing model on profiles that can afford it, or to OCR + layout model + 2D heuristic on light profiles; a vision LLM only by explicit configuration. Acceptance: the Document AI components are wired in production (today IngestionModelRouter::default() at ingestion/orchestrator.rs:203 and 215 leaves vision disabled and no layout extractor or scan sentinel, so every candidate falls to the 2D heuristic). Supersedes the Document AI part of IMP-006.

### FEAT-053: [ingestion] Known-layout forms extracted by LSIE as field records and tables
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Forms as data (LSIE). A known layout is recognized by geometry and fixed labels and its stored map drives LSIE for deterministic extraction. On a digital page every value must exist in the text layer. Each field becomes field number + label + value; the form is a record and forms of the same model become a queryable table. A test corpus per sector runs on every change, starting with the fictitious CMR in crates/any-context-core-rs/tests/fixtures/forms/. The shared LSIE crate belongs to the platform batch.

### FEAT-054: [ingestion] Automatic layout mapping (phase 0) per profile with a local anomaly gate
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Forms as data (LSIE). For a new layout, phase 0 generates the map (field, box, anchor label): box and typography heuristics on light profiles, a small label-value model on medium ones, the page parsing model on high ones (field model license part of the benchmark). An anomaly gate with a local model flags suspicious values; cloud only by explicit configuration. The map review screen belongs to the desktop UI batch.

### FEAT-055: [ingestion] Layout maps shared with DocMapper
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Forms as data (LSIE). The layout map format is common to AnyContext and DocMapper, so a layout mapped in one serves the other; AnyContext can hand over to DocMapper what belongs to it (advanced visual mapping, PDF stamping, field export). Local integration, no cloud; AnyContext works without DocMapper.

### FEAT-056: [ingestion] Image descriptions by the profile's local vision model
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Decisions, and Vision model per profile. Photos, diagrams and charts get an English description from the profile's local ONNX vision model, indexed with the OCR text and metadata. The image is searchable first through OCR and gets the description in the background. Model per profile as listed, to be confirmed by FEAT-051.

### FEAT-057: [ingestion] Precise locators per content type
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Precise locators (audit P22). Each chunk stores a locator for its type: page + region for PDF and images, sheet + cell range, start and end seconds, lines, section + paragraph for Office, URL + anchor, container path + item locator, field + region. The child has the precise locator and the parent the extent covering its children. Enables FEAT-016.

### FEAT-058: [ingestion] Document metadata: language, document date, type, title, authors
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Document metadata and entities (audit P12). Each document gets its language, the document date extracted from the content (not the mtime), type, title and authors, usable as filters.

### FEAT-059: [ingestion] Zero-shot entity extraction with types per workspace
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Document metadata and entities (audit P12). Entities with types configurable per workspace (case number, patient, sheet number, tax identifier...) extracted by a zero-shot multilingual ONNX model (candidate GLiNER; license of each checkpoint to be confirmed). They become filters and facets in retrieval.

### FEAT-060: [ingestion] Personal and sensitive data tagging
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Personal data, sensitive data and secrets (audit P13). The entity model tags generic personal data categories (name, identity document, health, financial, contact), with no rule tied to one country. Nothing is masked in the index or for the user. Reversible pseudonymization for external endpoints belongs to the retrieval batch.

### FEAT-061: [ingestion] Hierarchical summaries of documents and folders
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Hierarchical summaries (audit P14). Document and folder summaries generated by the profile's local LLM, stored as their own embedded and searchable nodes (never prefixed to chunks), regenerated only when the content hash changes, in the background. Document summary in its language; mixed-language folder summary in English. No summaries without a local LLM unless a cloud LLM is explicitly authorized.

### FEAT-062: [ingestion] Late chunking with a long-context local embedding model
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Late chunking (audit P15). With a local long-context embedding model, the model reads the whole parent and each child gets a vector with that context; an edit re-embeds only the children of that parent. On by default only when the workspace evaluation set shows a gain.

### FEAT-063: [ingestion] Contextual retrieval sentences by the profile's local LLM
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Contextual retrieval (local LLM). On profiles with room for a local LLM (on by default there, changeable by the user), the profile's LLM writes one context sentence per chunk into the embedded header, in the background after the chunk is already searchable. The context model identity is part of the chunk key, so switching it warns and reindexes the affected chunks. Never an implicit external service.

### FEAT-064: [ingestion] Base entity and document graph without an LLM
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Entity and relation graph (audit P16). Nodes are documents, entities and sources; edges are entity mentions and structural relations known for certain (attachment of, version of, replies to, cites). Entity resolution by spelling and embedding similarity, correctable by the user. One graph per workspace, encrypted with the workspace key, on by default only with a measured gain.

### FEAT-065: [ingestion] Typed relations by the local LLM with cited evidence
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Entity and relation graph (audit P16). On profiles with a local LLM, in the background, relations typed by the domain pack (e.g. party in case, drug prescribed to patient), each keeping the passage it came from and a confidence; a relation without evidence does not get in. Use in retrieval belongs to the retrieval batch.

### FEAT-066: [ingestion] Page-image embeddings for visually rich pages
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Visual page retrieval (audit P10). Only visually rich pages (charts, diagrams, drawing sheets, handwritten forms) get multi-vector page-image embeddings (ColPali family), per profile via ONNX, chosen by benchmark and permissive license, stored at full size. Off on light profiles. Retrieval use belongs to the retrieval batch.

### FEAT-067: [models] Local multilingual ONNX embedding models per profile as the default
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 8, Decisions. The default embedding model is local, multilingual and runs via ONNX, varying by profile; permissive candidates: multilingual-e5-small/base, BGE-M3, Qwen3-Embedding-0.6B. Cloud providers only by explicit choice.

### FEAT-068: [storage] Learned sparse vectors in the lexical index
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 9, Decisions (audit P19). Only when the profile's embedding model already produces sparse vectors in the same pass (e.g. BGE-M3): stored in the lexical index as a weighted field, per workspace and encrypted, as an extra RRF list, on by default only with a measured gain.

### FEAT-069: [storage] Encrypted storage layer for LanceDB and Tantivy with the workspace data key
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 10, Decisions, and section 9, Decisions. LanceDB and Tantivy files are encrypted on disk through storage layers built by AnyContext (a Tantivy Directory, a LanceDB storage layer) with the workspace data key of identity-and-governance-v2.md, section 2. Acceptance: the machine-id key with a fixed salt (security/mod.rs) and the clear lexical index are gone. Depends on the key hierarchy. Supersedes IMP-012.

### FEAT-070: [storage] Approximate vector index from 100 thousand chunks
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 10, Decisions. Exhaustive search below 100 thousand chunks per table (configurable); above it an ANN index is built automatically in the background and rebuilt after many changes, with new chunks not yet in it found by exhaustive search and fused.

### FEAT-071: [storage] Compact vectors only when needed and without measurable loss
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 10, Compact vectors (audit P18). No compaction by default; int8 or binary quantization only when the index does not fit memory or search gets slow, fetching more candidates and re-ranking with the full vectors, kept only with at most 0.5 points of hit-rate loss and switched off automatically when a loss appears. Matryoshka truncation is never used.

### FEAT-072: [ingestion] Deterministic extraction quality signals per file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: ingestion self-evaluation (audit H1 and H2), item 1. On every profile, without an LLM: an extraction quality score per file from text per page, empty pages, broken text, OCR confidence, filled form fields, discarded chunks and parser errors, shown in the source report.

### FEAT-073: [ingestion] Findability probing with synthetic questions and the evaluation set
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: ingestion self-evaluation (audit H1 and H2), items 2, 6 and 7. On profiles with a local LLM, per source, a sample of chunks gets a synthetic question in the document's language; retrieval runs and the hit rate is the source's findability score. Background, low priority, local only, questions stored encrypted. The questions form the per-workspace retrieval evaluation set (part of IMP-008).

### FEAT-074: [ingestion] Automatic reprocessing with alternative strategies and strategy memory
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: ingestion self-evaluation (audit H1 and H2), items 3 to 5. A low-scoring file is reprocessed with an alternative strategy (OCR instead of the text layer, rendering, another form mapping, another chunk size) and the best one stays; the winning strategy is remembered per source and file type. The user can ask for reprocessing from the source report.

### FEAT-075: [ingestion] Domain pack format, distribution and selection per workspace
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: domain packs (audit H3). A declarative pack (entity types, metadata and filters, pseudonymization types, heavy processors on by default, question templates, answer guidelines, layout maps) distributed through the verified catalog; several packs per workspace and custom packs; internal names in English, labels in the UI language. Parsers always work without a pack.

### FEAT-076: [ingestion] Content of the 11 launch domain packs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: domain packs (audit H3). Personal (default), legal, health, engineering, accounting, logistics, HR, academic, IT, public sector and real estate packs.

### FEAT-077: [ingestion] Automatic sync of the chosen sources while AnyContext is open
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: automatic sync of sources (audit P20), items 1 to 6. File system watching only on chosen sources with batching, periodic fallback scan for network folders and removable drives, a recovery diff when AnyContext opens, daily web recrawl configurable per source, low priority; manual sync stays and automatic sync can be turned off per source.

### FEAT-078: [ingestion] Optional background sync service
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: automatic sync of sources (audit P20), item 7. With AnyContext closed nothing runs by default; an optional background service keeps sources up to date for whoever chooses it.

### FEAT-079: [ingestion] Machine resource governor
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: machine resource governance (audit P21). One governor for all heavy work in ingestion and search: question > base indexing > enrichments > self-evaluation; adaptation to battery, load, temperature and metered networks; memory budget per profile with models loaded on demand; disk reserve checked before large jobs; only a pause button and a status indicator for the user; enterprise windows and CPU ceiling by policy.

### FEAT-080: [governance] Per-source access control with permission scopes
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, Cross-cutting: per-source access control (audit P23). Each chunk stores the permission scope ID of its document; scopes per workspace and directory users and groups live in separate SQLite databases; origin identities link through SSO and SCIM and what does not link gets no access; permission changes need no reprocessing; unknown permission denies by default. Acceptance: pre-filter by readable scopes in every retriever and aggregates per scope.

### FEAT-081: [retrieval] Follow-up rewriting for search only
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. A question that depends on the conversation is rewritten as a standalone question before searching; the original stays in the history and is what the answering LLM sees. The profile's local LLM rewrites; without it, the chat LLM, respecting trust level and pseudonymization; without a local LLM and with an external chat, the previous user question is added to the search. Any language. Supersedes IMP-009.

### FEAT-082: [retrieval] Help and status as agent tools
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. `anycontext_help` (AnyContext documentation, the built-in source of IMP-017) and `workspace_status` (sync and source state) are tools the LLM calls in any language. Acceptance: the fixed PT/EN status keywords (actx-agent/src/router.rs:88-91) and the mixing of AnyContext's documentation into every workspace search are gone.

### FEAT-083: [ui] Drag a file into the chat to add it to the workspace as a single source
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. There is no temporary chat attachment: dropping a file into the chat adds it to the workspace as a single file source, with a short confirmation, visible indexing progress and the time estimate for large files. The file stays for the next conversations.

### FEAT-084: [retrieval] Question queue per workspace chat
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. New questions sent while the agent answers are queued and processed in order, one at a time (follow-up rewriting uses the previous answer). The queue is visible; questions not yet started can be edited, removed or reordered; 'stop the current one and send now'; it survives closing the app; each question counts toward policy token quotas; stopping the current question keeps the queue.

### FEAT-085: [retrieval] Stop and edit a running question
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. Stopping (button or Esc) cancels search, Deep turns and streaming; the question returns to the composer to edit and resend, and the interrupted question and partial answer leave the history.

### FEAT-086: [ui] Conversation branches
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. Editing an answered question creates a new branch from it and keeps the previous one; the edited message shows a switcher between versions (e.g. 2/3). Builds on IMP-036.

### FEAT-087: [retrieval] Question planner: search text, filters and question type
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding. In the same LLM call as follow-up rewriting, the planner returns the search text (and Deep sub-questions), filters (source, document date in any language, type, entities, version, connector metadata) and the question type (point, overview, aggregate, comparison, enumerative, whole document, help or status). Explicit filters restrict and inferred ones only boost; a filter that empties the result is relaxed with a warning; with an external chat the question is pseudonymized first. Acceptance: no invented scores (today cited files enter at 1.0 and dates at 0.95); a quoted passage is searched as an exact phrase or identifier (Tantivy phrase query) as a restricting filter, which supersedes FEAT-017.

### FEAT-088: [retrieval] LLM-free filters: @source, multilingual dates and the entity index
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding, and step 1, Question input. Without an LLM planner: `@source` restricts the search to a source inside the workspace, dates come from a multilingual parser (today English month names only) and entities from the entity index.

### FEAT-089: [retrieval] Enumerative questions answered exhaustively, with the count checked
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding. 'Which', 'all the', 'list' questions skip top-k: entity and metadata filters or SQL over tables and forms; without structure, a Deep sweep verifying candidate documents in batches. The answer says how many documents were checked; large sweeps use the long-job estimate and confirmation.

### FEAT-090: [retrieval] Version-aware search: current version by default, family for change questions
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding, and Items inherited from ingestion. Search prefers the current version of a version family (FEAT-027) and uses the whole family for questions about changes between versions, usually in Deep.

### FEAT-091: [retrieval] Entity and metadata filters and facets
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding, and Items inherited from ingestion. Document metadata (FEAT-058) and entities (FEAT-059) become search filters and facets, with tolerance to different spellings of an entity.

### FEAT-092: [retrieval] Table and form questions by read-only SQL
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search. For aggregate questions the LLM writes read-only SQL over the schema of the workspace's tables and forms, executed by DataFusion: only that workspace's tables, time and row limits, the query and rows used shown in the citation, fallback to normal search with a warning on failure or ambiguity.

### FEAT-093: [retrieval] Overview questions answered from document and folder summaries
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search, and step 7, Selection. Overview questions search the document and folder summaries (FEAT-061), which go through the reranker and enter the context as summaries.

### FEAT-094: [retrieval] Search stratified by source with dynamic parallelism
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search. In a workspace with several sources, search runs per source in parallel with a smaller list each, so every source secures slots; parallelism depends on the workspace size and the machine's free capacity (resource governor, FEAT-079). Single search with one source or few documents.

### FEAT-095: [retrieval] Two-stage search in large workspaces
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search. Above a number of documents, the most relevant documents are found first (summaries and chunk aggregation per document), then the chunks inside them.

### FEAT-096: [retrieval] Page-image and sparse-vector lists in search
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search, and step 5, Fusion. Page-image embeddings (FEAT-066) and learned sparse vectors (FEAT-068) enter as extra lists in the same RRF; the result of a page-image hit is the page. With a multimodal chat model the page image goes as evidence, never to an external endpoint when pseudonymization is active.

### FEAT-097: [retrieval] Entity graph as a search list and a relations tool
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search, and Items inherited from ingestion. Entities in the question pull the linked documents as an extra list; a 'query relations' tool (useful in Deep) and the graph as a structured source for enumerative questions (FEAT-064, FEAT-065).

### FEAT-098: [retrieval] Small-to-big: selected chunks expanded to their parent
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 7, Selection. Each selected child is replaced by its parent (section or page, FEAT-036) when it fits the budget; children of the same parent bring it once; a parent too large gives the child with its immediate neighbors. The citation points to the child.

### FEAT-099: [retrieval] Deep coverage: later turns exclude documents already found
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 7, Selection. Each Deep turn's evaluation sees which documents already appeared and can ask for others; the next search excludes them. The evaluation set measures how many of the right documents reach the context and calibrates stratification, the per-document cap and two-stage search.

### FEAT-100: [retrieval] Reversible pseudonymization for external LLM endpoints
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 8, Context assembly, and step 9, Generation and citations. When the LLM endpoint is external, personal identifiers tagged at ingestion (FEAT-060) become consistent markers ([PERSON_1]) per the domain pack; the mapping stays local for the whole conversation and the markers are replaced by the real values during streaming, including markers split across chunks of the stream. The user never sees a marker; with a local or corporate endpoint nothing is pseudonymized by default.

### FEAT-101: [retrieval] Checked citations linked to their locators
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 9, Generation and citations. The LLM cites by evidence ids; AnyContext removes ids that do not exist in the context and turns each citation into a link to its locator (page and region, cell, minute) that opens the original at the exact point in the desktop UI. Part of FEAT-016.

### FEAT-102: [retrieval] Citation support checked by the local reranker
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 9, Generation and citations. Each cited sentence is compared with the cited chunk by the local reranker; a weak citation is marked discreetly with a hint that the chunk does not directly confirm the sentence.

### FEAT-103: [retrieval] Answer receipt: citations with file version, mode, tools and tokens
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation. Each answer stores its citations with locator and file hash, the mode used (Fast, Deep, escalated), the tools called and the tokens consumed (for the status bar and the Admin Hub quotas, which receive only numbers). Reopened conversations keep working citations and show when a document changed since the answer. Part of FEAT-010.

### FEAT-104: [retrieval] Answer feedback that stays on the machine
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation. Thumbs up or down per answer with an optional reason (wrong information, wrong citation, missing information), stored locally; in enterprise mode the Admin Hub receives at most counts, never content.

### FEAT-105: [retrieval] Self-tuning loop per workspace
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation. Each question records quality signals locally (reranker bands, escalation, document coverage, weak citations, feedback); with them and the evaluation set AnyContext tunes per workspace, invisibly: RRF weights, reranker depth, band thresholds, Deep turns, stratification and compaction.

### FEAT-106: [retrieval] Regression evaluation with automatic revert
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation. The synthetic question set (FEAT-073), enriched with the questions that got feedback, runs after every change that may worsen quality (model switch, source ingestion change, AnyContext update); a regression reverts the automatic adjustment that caused it. In development the test corpus (starting with the fictitious CMR) runs on every code change. With FEAT-073, supersedes IMP-008.

### FEAT-107: [retrieval] Conversation retention
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 10, History and evaluation. The user can delete conversations; deleting a workspace asks what to do with its conversations; in enterprise mode the policy can set a retention period.

### FEAT-108: [retrieval] Whole-document questions with @file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 3, Question understanding (whole document). An overview or analysis question with @file reads the whole document, its parent sections in document order, instead of top-k chunks; a document larger than the token budget is summarized in parts and the parts combined. Supersedes FEAT-007.

### FEAT-109: [retrieval] Source tools: list sources, browse folders and read files
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input (source tools). Agent tools, in both modes, to list the workspace's sources, browse their folders and files, and read a file or a range of it. Read-only and limited to the workspace's registered sources; no general file system or web browsing. Supersedes FEAT-015.

### FEAT-110: [storage] AnyContext Server exposing the knowledge base port (mode 3)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 1, Storage port, and Deployment modes. A server in the company VPC runs the three stores on local disk and exposes the knowledge base port over an API; no LanceDB directly on S3 or MinIO. Workspace data keys in the server's vault or key service, TLS with mutual authentication, enrolled and governed by the Admin Hub like a device. The ingestion worker runs on the server and clients only enqueue and query. Enterprise plan; depends on the users module and per-source access control (FEAT-080).

### FEAT-111: [models] Native vendor adapters: OpenAI, Azure OpenAI, Anthropic, Gemini API, Vertex AI
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Each vendor through its native API, which covers features the OpenAI-compatible layers miss (embedding task types, multimodal inputs, context caching): OpenAI and Azure OpenAI, Anthropic Messages, Google Gemini API and Vertex AI (generateContent, embedContent) in the customer's own GCP project.

### FEAT-112: [models] Generic OpenAI-compatible adapter, including /rerank and /audio/transcriptions
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. For servers that use the OpenAI-compatible format (vLLM, Ollama, LM Studio), plus the de facto formats for reranking (/rerank) and transcription (/audio/transcriptions).

### FEAT-113: [models] AnyContext Model Server for roles without a standard (mode 2)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role, and Deployment modes. The desktop's ONNX runtime packaged as a service for OCR, page parsing, form fields and entities, with the same model and the same result as locally. Enables mode 2: local index, heavy models in the VPC.

### FEAT-114: [models] Adapters for cloud services of the ONNX roles
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Adapters for services customers already pay for (e.g. Google Document AI and Speech-to-Text, and the Microsoft and AWS equivalents), following the endpoint trust level (FEAT-117) and the official-APIs-only rule (connectors-v2.md, decision 32).

### FEAT-115: [models] Local LLM per profile via ONNX
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role, and ingestion-pipeline-v2.md, Vision model per profile. The context and summary LLM role runs locally on profiles that can afford it, used by contextual retrieval, summaries, follow-up rewriting, the planner and self-evaluation. Starting point: Gemma 4 E2B/E4B, the only sizes with official ONNX, which are multi-graph and multi-shard (E2B q4 above 3.6 GB); needs a multi-shard downloader, a generative loop (tokenization, KV cache, sampling) and tier recalibration; the actx-onnx tiers (Compact, 27b, 32b) do not exist. Supersedes FEAT-002.

### FEAT-116: [models] GPU execution providers for local ONNX roles
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role, and ingestion-pipeline-v2.md, machine resource governance. Local ONNX roles can run on the GPU (DirectML on Windows, CUDA on Linux) with the matching ONNX Runtime package, one heavy model at a time on the GPU under the resource governor (FEAT-079). Care with DLL dependencies: static DirectML linking once prevented actx.exe from starting. Supersedes FEAT-003.

### FEAT-117: [governance] Endpoint trust levels: local, corporate, external
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 3, Trust level of each endpoint. Each endpoint is local (the machine), corporate (VPC, Model Server, AnyContext Server, services in the company's own account) or external (third-party public APIs). Without Admin Hub the user classifies endpoints (localhost local, public API external by default); with Admin Hub the policy defines the levels and the user cannot raise one. Pseudonymization applies only to external; Admin Hub modes become rules over levels.

### FEAT-118: [governance] Confidential workspace ceiling
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 3, Trust level of each endpoint. A workspace marked confidential never sends anything to an external endpoint, even when the general policy allows it. Acceptance: web search (the question sent to a search engine) and applications that consume AnyContext through the MCP server count as external exits, with pseudonymization and the workspace ceiling.

### FEAT-119: [models] Embedding target identity with a fingerprint test
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 4, Embedding model identity. The embedding target is what produces the vector: model and weights version (catalog hash), dimension, precision and preprocessing (tokenizer version, prefixes); the same model and precision local or on a server is the same target. The catalog ships fixed texts and reference vectors; when a binding is configured and periodically, AnyContext embeds them and compares (e.g. similarity above 0.999 means the same target), catching silent provider changes and precision differences.

### FEAT-125: [identity] Sealed inbox: sync with the app closed using the public key
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, Sync with the app closed — sealed inbox. The optional background service (FEAT-078) holds only the user's public key: with the app closed it detects changes and runs extraction, OCR, transcription, chunking and local embedding, writing the result to an inbox encrypted with the public key; it can never decrypt what is indexed. On sign-in the app merges the inbox. With cloud embedding, embedding waits for sign-in.

### FEAT-131: [identity] Workspace key rotation when a member leaves
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 2, item 3, and topic 3, item 5. A member leaving rotates the workspace key: new data uses the new key and old data is re-encrypted in the background. The member's conversations stay with them but citations stop opening; in enterprise mode the policy can require deleting them.

### FEAT-133: [identity] Workspace roles (owner, editor, reader) and a private chat per user
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 3, Roles and sharing, item 1, and deployment-architecture-v2.md, Workspace model. Each user creates their own workspaces and is the sole owner. Permissions: everyone asks, opens cited sources and gives feedback; owner and editor manage sources, filters, reprocessing, the workspace prompt and the domain pack; only the owner marks confidential, sets privacy policies, shares, changes roles and deletes. An editor does not pass access on. Each user has their own chat in a workspace and nobody reads anyone else's. Acceptance: the confidential ceiling applies to every member.

### FEAT-134: [identity] Workspace definition export and import (.actx-workspace)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 3, Roles and sharing, item 3. Without Admin Hub, the definition (name, sources, filters, domain pack, prompt) is exported to a file and imported by a colleague, who indexes the same sources on their own machine; never documents, chunks, vectors, API keys or passwords. Within one environment only; the corporate policy can accept only definitions signed by the organization. Acceptance: a connector source carries the connector and the target ID, never the credential; each member signs in with their own account (connectors-v2.md, decision 5).

### FEAT-135: [identity] Sharing a workspace between accounts on the same machine
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 3, Roles and sharing, and topic 2, item 3. The owner shares a workspace with another account on the same machine by wrapping its data key with that member's public key, with a UI warning that members see the indexed content (ingestion-pipeline-v2.md, per-source access control).

### FEAT-136: [identity] Enterprise sharing: groups, organization workspaces, curators
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 3, Roles and sharing, items 2 and 3, and deployment-architecture-v2.md, Workspace model. In enterprise mode, sharing with groups from SSO or the Admin Hub directory; the admin creates organization workspaces (admin owner, everyone reader) and appoints curators from a department as editors without giving up ownership; workspace definitions are distributed by the Admin Hub as metadata only.

### FEAT-142: [governance] Source telemetry level set by the policy
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 5, Admin Hub integration, item 3. Source telemetry at the level the policy sets: counts only; anonymized identifiers (a hash that says 'same source' without the name), the default; or full paths when the company needs to audit and informs users.

### FEAT-143: [release] Updates, models, packs and Chromium from the Admin Hub internal mirror
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 5, Admin Hub integration, item 4. Enrolled devices without internet receive AnyContext updates, ONNX models, domain packs and Chromium from the Admin Hub mirror, verified by the Levix signature, at the version IT releases to the fleet.

### FEAT-146: [identity] Minimal user profile: role and answer style as a user preference
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 7, item 1. The display name and language come from the personal account; the user's role and preferred answer style are a preference in the personal vault that enters the prompt as the user layer, below the workspace prompt, never overriding the base rules (retrieval-pipeline-v2.md, step 8). Supersedes FEAT-013.

### FEAT-147: [retrieval] Explicit user memories
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 7, item 2. Only what the user asks to keep or confirms when the agent proposes it becomes a memory; nothing is saved silently. A visible, editable list in the personal vault, scoped globally or per workspace, entering the prompt's user layer within the token budget. Supersedes FEAT-014.

### FEAT-148: [retrieval] AnyContext MCP server
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 7, item 3. Off by default, turned on explicitly per workspace. Read-only: asking and searching over one workspace per call, never across workspaces, only while the user's session is unlocked. Its consumers are external exits: a confidential workspace is not reachable and the policy governs it. Supersedes FEAT-018.

### FEAT-149: [connectors] actx-connectors crate with the single connector contract
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 1, 2, 8 and 9. A Levix-only `actx-connectors` crate in Rust, one trait and one module per connector, no third-party plugins. Contract: list the target's items (single or recursive), read the content as a stream, changes since the last cursor, permission scopes, metadata (author, dates, link to open at the origin). A connector only acquires; from the diff on it is the normal pipeline. Each item has a stable native ID that replaces the hash for rename and move detection. Indexed only, the question is never sent to the origin. No copy of the original is kept: the reader shows the indexed text and offers 'open at origin'; downloads stream within the governor's disk reserve.

### FEAT-150: [connectors] Shared HTTP client with corporate proxy and company certificates
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 9. All connectors use one HTTP client that honors the corporate proxy and trusts company certificate authorities read from the OS certificate store and from the policy.

### FEAT-151: [connectors] OAuth 2.0 with PKCE through the system browser, tokens in the personal vault
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 3 and 4. Sign-in through the system browser redirecting to 127.0.0.1, never through a Levix server, with the smallest read-only scopes. Tokens live in the user's personal vault (FEAT-130), belonging to the person, not the workspace. Personal environment: public OAuth app registered by Levix; corporate environment: the company's own app (Entra ID, Google Workspace) distributed through the policy; AnyContext Server: a company service account with the per-reader permission filter.

### FEAT-152: [connectors] Sync through the origin's change feed
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 6. Sync uses the origin's change feed (Drive changes, Graph delta, Dropbox cursor, IMAP CONDSTORE) instead of scanning: polling with the cursor while AnyContext is open, a recovery diff on opening, the optional background service, webhooks on the AnyContext Server; exponential backoff on rate limits; on a metered network the resource governor holds connector sync and the user can release it per source or for now.

### FEAT-153: [connectors] Explicit target selection by browsing the origin's tree
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 10 and 12. The user browses the origin and picks a folder (recursive), a single file or an origin-specific target (shared drive, SharePoint library, mail folder or label, channel, playlist); everything selected and accessible is indexed and nothing outside it. Include and exclude filters apply to remote paths. The source stores account_ref and target_id (IMP-015); the change cursor lives in the workspace sync state.

### FEAT-154: [connectors] Native cloud formats and wiki pages converted for the existing parsers
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 7. Google Docs, Sheets and Slides are exported to DOCX, XLSX and PPTX and handed to the existing parsers; wiki pages are converted from their block tree to deterministic Markdown; cloud shortcuts and links are resolved and deduplicated as duplicates.

### FEAT-155: [connectors] Typed connector metadata, with live metadata outside the embedded text
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 13 and 14. Each connector declares a typed metadata schema (e.g. YouTube view_count, published_at; e-mail from, labels) usable as retrieval filters. Values that change without the content changing live only in filterable columns, outside the embedded text and the chunk header; a change updates the columns without re-embedding.

### FEAT-156: [connectors] Source states: deleted at origin, disconnected, item access revoked
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 18. An item deleted at the origin leaves the index; without technical access (expired or revoked token, network down) the source is shown disconnected with 'reconnect account' and its content stays searchable; an item whose access was revoked for the user leaves the index and appears in the source report with the reason.

### FEAT-157: [connectors] High-volume noisy sources indexed by relevance with a ceiling
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 15. For sources such as comments, the most relevant set is indexed (e.g. most liked comments and their replies), each item as its own chunk with author and relevance in the header; the source report says how much was left out and the user can change the ceiling. Comment authors are third-party personal data under pseudonymization for external endpoints.

### FEAT-158: [connectors] Extraction maps signed by Levix and reviewed by a person
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 19. Origins without a stable API use a declarative selector file distributed through the verified catalog, signed by Levix and reviewed by a person before publishing; never generated and published automatically. Only within the official-APIs-only rule (decision 32).

### FEAT-159: [governance] Connector policy: allowed connectors, account domains, volume ceiling
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 11 and 27. In the corporate environment connectors are a selection of features the Admin Hub policy turns on or off, with allowed account domains (e.g. only @company.com) and a volume ceiling per source, enforced by the governance engine (FEAT-138); a block says which rule blocked.

### FEAT-160: [connectors] Product model: base, source connectors and capability connectors
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 22 and 23. The base searches local and internal-network files with no external call; everything else is a connector the user plugs explicitly: source connectors (web, drives, YouTube, e-mail, Obsidian) and capability connectors (local audio and video transcription). Without a connector, what it would cover appears in the source report ('12 videos not indexed — enable the Video connector'). The system help is not a connector.

### FEAT-161: [connectors] Web search connector through an allowed API
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 24 and 32. Web search at question time through an official or allowed third-party search API whose terms and cost-benefit fit (Jina is the cost reference, about US$ 0.0005 per search on a paid key; its free key is non-commercial only); the provider is chosen when the item is planned. The question is pseudonymized and a confidential workspace never searches the web (FEAT-118). Replaces the DuckDuckGo scraping of BUG-029.

### FEAT-162: [connectors] YouTube connector through the official Data API
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 16. Metadata and comments of any public video through the official YouTube Data API; the transcript through the official caption only when the user owns the channel (the API hands captions only to whoever can edit the video). A third-party video is transcribed locally by the Audio and video capability from a file the user legitimately has; never InnerTube, client impersonation or a third-party transcription service. Enterprise feature (decision 28).

### FEAT-163: [connectors] Obsidian connector: wikilinks, frontmatter, embedded notes, aliases, canvas
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 17 and 23. On top of the base indexing of the vault as a Markdown folder, the connector adds wikilinks as graph relations, frontmatter as metadata, embedded notes, aliases and canvas. Acceptance for the base Markdown parser: `.obsidian/` ignored by default and YAML frontmatter read as metadata. Enterprise feature (decision 28).

### FEAT-164: [connectors] Google Drive connector through the official API (Enterprise)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 20 and 30. Full Google Drive through the official API with the read scope over the whole Drive (no file-picker restriction). The corporate environment uses the company's internal app, exempt from Google's security assessment (CASA); for the personal environment it comes only if the assessment fits. Enterprise candidate: the local sync folders already cover personal use.

### FEAT-165: [connectors] OneDrive and SharePoint connector through Microsoft Graph (Enterprise)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 6, 10 and 30. OneDrive and SharePoint libraries through Microsoft Graph with delta sync, the company's Entra ID app and the origin's permissions captured for per-source access control (FEAT-080). Enterprise candidate.

### FEAT-166: [connectors] Dropbox connector (Enterprise)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 6 and 30. Dropbox through its official API with cursor-based sync. Enterprise candidate: the local sync folder already covers personal use.

### FEAT-168: [governance] Free and Pro plan limits
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 28. Free: local and internal-network files only, up to 3 workspaces. Pro: Free plus the Web and the Audio and video connectors built in, unlimited workspaces. No per-connector sales and no capability combinations sold as extra plans.

### FEAT-169: [governance] Downgrade and lapsed plan deactivate, never delete
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 26 and 29. Indexes the plan no longer covers stay on disk, out of search and sync, and can be deleted explicitly to free disk. Over the Free limit the user chooses once which 3 workspaces stay active, with no swapping afterwards. When the plan returns, the sources come back and only the incremental diff is reprocessed.

### FEAT-170: [connectors] Web search fair-use daily ceiling in Pro
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 31. Pro carries a high daily ceiling of web searches and page reads that normal use never reaches; reaching it pauses web search until the next day with a clear message while local search keeps working. The exact value is open and set from real usage data.

### FEAT-171: [connectors] Generic IMAP e-mail connector (Enterprise)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/connectors-v2.md, decision 33. One read-only connector for any provider over IMAP, Gmail included: folders and labels as targets, change detection through CONDSTORE, attachments back into the pipeline as files. Sign-in prefers the provider's OAuth (XOAUTH2); otherwise an app password in the personal vault. Gmail OAuth uses Google's restricted full-mail scope (security assessment for a public app; the corporate environment uses the company's app); confirm Google's scope and app-password rules in its documentation when planning. Enterprise feature (decision 28).

### FEAT-172: [ui] Tauri 2 desktop app shell: three columns and the top bar
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Three-column structure. A Tauri 2 app with the Context Explorer (200/260/420 px), the Canvas Hub (min 480 px, flexible) and the Workspace Inspector (240/280/480 px), resizable by dragging and collapsible with Cmd/Ctrl+B and Cmd/Ctrl+I, under a fixed 38 px top bar with the app name, quick search and the global menu. Tauri commands call the core; worker progress arrives as native events (IMP-061). No Tauri app exists in the repository today.

### FEAT-173: [ui] Design system tokens
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Design system. Semantic colors, typography, spacing, radii and elevations from the token specification: dark theme with surfaces from #0B0C0E to #1E2330, primary accent #2563EB, source status colors (synced, syncing, error), native sans font and mono for code, 4 px base spacing.

### FEAT-174: [ui] Context Explorer: workspace tree with sources and their status
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Three-column structure, and Rules inherited from the architecture decisions. Tree of workspaces with their sources (expandable folders, sites expandable into pages, connectors) and the status of each source; shared and organization workspaces show the user's role; a source the plan no longer covers shows as deactivated, never disappears; the system help is not in the tree. '+ New' button; footer with the user profile and shortcuts.

### FEAT-175: [ui] Canvas Hub tabs: one chat per workspace, one reader per open source
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior. Clicking a workspace opens or focuses its chat tab; clicking a source opens or focuses its reader tab; tabs never repeat.

### FEAT-176: [ui] Chat tab: streaming, clickable citations, composer with quick options
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior. Streaming history with clickable citations that open the source at the cited point; multiline composer with a fixed bar of quick options (model, Fast or Deep, web search, sync, send) saved per workspace. Shows the question queue (FEAT-084), stop and edit (FEAT-085), feedback (FEAT-104) and weak-citation marks (FEAT-102).

### FEAT-177: [ui] Reader tab opening the original at the cited locator
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior. Read-only document with syntax highlighting, line numbers, indexing metadata and 'reprocess this document'. A citation opens it at its locator (FEAT-057) with the passage highlighted: page and region, cell range, minute, lines, section. With FEAT-057 and FEAT-101, supersedes FEAT-016.

### FEAT-178: [ui] Browse folders, files and sites without leaving AnyContext
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior. Opening files, browsing a source's folders and viewing sites inside the desktop UI, read-only and limited to the workspace's sources (the same scope as the source tools, FEAT-109).

### FEAT-179: [ui] Workspace Inspector with only what belongs to the user
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior. The inspector follows the active tab (workspace or document): basic section exposed, advanced in an accordion, only what is the user's (name and description, sources and filters, chat model, domain pack, privacy policies, crawl ceiling). Acceptance: no Chunk Size, Overlap, Top-K, Score Threshold, Neural Reranker or Max Depth settings.

### FEAT-180: [ui] Quick search (Cmd/Ctrl+K)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Three-column structure. Quick search from the top bar over workspaces, sources, open tabs and commands.

### FEAT-181: [ui] Token usage and context window fill in the chat tab
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior (decided 2026-10-23), and retrieval-pipeline-v2.md, step 10. Tokens used per model and how full the chat model's context window is, shown discreetly next to the composer, with no separate status bar; fed by the answer receipt (FEAT-103), including custom models outside the catalog (window unknown shows only tokens). With FEAT-103, supersedes FEAT-010.

### FEAT-182: [ui] Layout map review and correction screen
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior, and ingestion-pipeline-v2.md, section 6, Forms as data (LSIE), item 3. A screen in the style of DocMapper's Mapping Studio to review and correct a form layout map (fields, boxes, anchor labels) generated by phase 0 (FEAT-054); the corrected map is used by LSIE for the next documents of the layout.

### FEAT-183: [ui] Entity view in the inspector, with merge and split
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior, and ingestion-pipeline-v2.md, section 7, Entity and relation graph (audit P16) and Duplicates and versions (audit P17). Browse a workspace by entity and see the documents and relations linked to it; correct entity resolution (merge, split) and version families (split wrongly grouped documents, join undetected versions).

### FEAT-184: [ui] Source report view
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Behavior, and ingestion-pipeline-v2.md, Principles (nothing disappears silently). Per source: files not indexed and discarded chunks with the reason (FEAT-031), offline, disconnected and deactivated states (BUG-025, FEAT-156, FEAT-169), extraction quality and findability scores (FEAT-072, FEAT-073), what a disabled connector would cover (FEAT-160), and a 'reprocess' action.

### FEAT-185: [ui] System help panel
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 7 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/desktop-ui-v2.md, Rules inherited from the architecture decisions, and retrieval-pipeline-v2.md, step 1. A help panel over AnyContext's built-in documentation (IMP-017); the help is not a workspace in the tree and never mixes into workspace results.

### FEAT-186: [cli] Headless CLI over the core ports, with sign-in per terminal session
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 1, items 1 and 2. The actx CLI keeps the headless commands for automation (ask, sync, status, diagnostics, sudo) over the same core ports as the desktop app, asking for the password or biometrics once per terminal session. Acceptance: plan limits and policies are enforced by the governance engine in the core, so wrapping the CLI in a script, an API or an MCP server cannot unlock what the plan does not include.

### FEAT-187: [cli] actx sudo profile to install or switch the hardware profile
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 1, item 3. Installing or switching the hardware profile after installation is an administrative action (it downloads models): `actx sudo profile`, reusing crates/actx-installer/src/provision.rs, applying the profile's per-role bindings (IMP-055). Supersedes FEAT-001.

### FEAT-188: [release] Desktop installers for Windows, Linux and macOS with ONNX Runtime, Chromium and FFmpeg
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. Platform installers for the Tauri app carry ONNX Runtime, the embedded Chromium (ingestion-pipeline-v2.md, step 2) and FFmpeg as a separate library to comply with the LGPL (step 5), all updated with every release. Acceptance: Chromium never lags the release it ships with.

### FEAT-189: [release] macOS build (unsigned until there is revenue)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. The release builds and packages macOS like Linux and Windows (today release.yml builds only Linux x86_64 and Windows x86_64). Until the product has revenue the macOS build ships unsigned and not notarized and users allow it in System Settings (decided 2026-10-23); Developer ID signing and notarization come later as their own item. Supersedes FEAT-004.

### FEAT-190: [release] Code signing: Windows with Azure Artifact Signing, Linux with GPG
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. Windows binaries and installers are signed in CI with Azure Artifact Signing (Basic SKU: US$ 9.99 a month, 5,000 signatures; confirm eligibility on Microsoft's pages, including any business-history requirement) and Linux packages with a Levix GPG key (decided 2026-10-23).

### FEAT-192: [release] Local diagnostic bundle, /feedback and local crash reports, sent only with confirmation
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 4. A local diagnostic bundle (logs, versions, configuration, sensitive data masked) the user reviews; /feedback shows exactly what leaves the machine and sends only after confirmation; a crash writes a local report and offers to send it with confirmation. Nothing is sent automatically, not even in beta or release-candidate builds; the corporate policy can turn sending off. Supersedes FEAT-011 and FEAT-012.

### FEAT-193: [identity] Adopt the platform identity, policy and license crates
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 9 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topics 1, 2, 4 and 6, and docs/architecture/platform-shared-components.md. AnyContext adopts the levix-platform crates for accounts, root and sudo, session, biometrics, the key hierarchy and personal vault, policies and the governance engine, the Admin Hub client, the personal and corporate environments, plan license verification and Levix signature verification: per-account data directories, the first-run, sign-in and lock screens in the desktop app, sign-in once per terminal session in the CLI, workspace data keys as the platform's data keys, and AnyContext's policy schema (models and endpoints, sources, tools, quotas) registered with the engine, which enforces plan limits and policies below every surface (cli-and-release-v2.md, section 1). Acceptance (decided 2026-10-23): the machine-id key with a fixed salt (security/mod.rs:10-11, 44-50) and the clear lexical index are gone, together with FEAT-069; API keys are no longer stored in plain text in the api_keys table (storage/sqlite.rs:1470-1478). If the roadmap leaves this item out of the first release, the plain-text API keys become a high bug.

### FEAT-194: [release] macOS Developer ID signing and notarization
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, roadmap skeleton, 2026-10-23
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. Join the Apple Developer Program (about US$ 99 a year, D-U-N-S number for an organization), sign the macOS build with a Developer ID and notarize it, so users no longer have to allow the app in System Settings. Deferred until the product has revenue (decided 2026-10-23).

## Improvements

### IMP-015: [ingestion] Single source model: a sources table replaces the three source tables
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1, Single source model. A `sources` table in settings.db (id, workspace, kind local/web/builtin/connector:<name>, target, recursive, display_name, created_at, filters, account_ref, target_id) replaces workspace_folders, workspace_web_urls and workspace_sources, with a migration of existing sources. The unused columns of workspace_web_urls are dropped.

### IMP-016: [ingestion] Include and exclude filters in settings.db with .gitignore syntax
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1, Filters. Filters leave the code: global defaults in settings.db, filled with today's ignored folders and extensions, overridable per source. `.gitignore` syntax evaluated with the `ignore` crate; the `.gitignore` of indexed folders is honored by default with an option to turn it off. Only extensions some parser supports get in; the user can turn one off or map an unknown extension to the text parser.

### IMP-017: [ingestion] System help as a built-in source with a fixed index per version
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 1, System help. AnyContext's documentation stops being the Global workspace: it becomes a `builtin` source embedded in the binary, processed by the same pipeline, in a separate fixed index that ships with each version and needs no sync. It is never mixed into user workspace results. The `anycontext_help` tool belongs to the retrieval batch.

### IMP-018: [ingestion] Read each local file once per sync
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, Decisions. Local files are read at the origin, without copying, and only once: the same buffer feeds the hash, the parser and the enrichment (today there are up to three reads).

### IMP-019: [ingestion] Deterministic HTML to Markdown with a Readability fallback; Jina Reader removed
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 2, Web: rendering and conversion to Markdown. Selected DOM blocks become Markdown in Rust, keeping headings, tables, lists and code without rewriting text; a Readability-style heuristic selects content when the classifier model is not installed. Local .html/.htm use the same converter. The Jina Reader is removed entirely (code, configuration and JINA_API_KEY).

### IMP-020: [ingestion] Content-hash change detection and hash-based rename and move
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3, Decisions. mtime and size are only the fast filter; when they changed, the content hash decides, and a touch without a content change only updates the mtime cache. A vanished file and a new one with the same hash in the same source are a rename or move: only the chunks' path changes, no re-embedding. Replaces the name-and-size heuristic (ingestion/scanner.rs:316-366).

### IMP-021: [ingestion] Per-chunk diff keyed by the hash of the embedded text
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 3, Decisions. The chunk id becomes the hash of the embedded text plus the embedding target. On an edit the file is re-chunked, known chunks reuse their vectors, only new ones are embedded and the ones that stopped existing leave. An interrupted sync resumes where it stopped.

### IMP-022: [ingestion] Content classification by binary signature and text encoding detection
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 4, Decisions. Magic bytes decide the parser and the extension only breaks ties: ZIP containers are told apart by [Content_Types].xml or the mimetype file, OLE/CFB by their internal streams. Text has its encoding detected (UTF-8, UTF-16 with BOM, legacy encodings of any language) and CSV, JSON or Markdown are recognized by content.

### IMP-023: [ingestion] Chunk size in tokens, overlap only in running text
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Decisions. A single target of about 512 tokens for every parser, with a hard ceiling at the embedding model's input limit (today 1800 characters, PDF up to 8000). Overlap of about 10-15% only in running text (prose, PDF, Office, e-mail, web); code, tables and structured data are cut by structure without overlap.

### IMP-024: [ingestion] Table chunks repeat the header and use records for wide tables
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 5, Decisions. Each row chunk repeats the table header and carries its location (sheet and row range) for citation; tables wider than about 15 columns use a `column: value` record per row instead of a Markdown row.

### IMP-025: [ingestion] Quality gate keeps only safe, language-independent filters
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 6, Decisions. Only empty or too short, loose syntax, minified line and binary (control characters) filters stay. The byte entropy and lexical density filters go: they discard non-Latin text and tables or logs with repeated values. Each discard carries a reason for the source report (FEAT-031).

### IMP-026: [ingestion] Contextual header (title and section path) replaces the envelope
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7, Decisions. Metadata (source, location, parent, hashes, dates) lives in columns and is never embedded. The embedded header is only document title + section path, weighted lower than the body in the lexical index. The envelope (first-sentence summary + 7 keywords by frequency) and the `// Context:` breadcrumb are removed. Acceptance: the English and Portuguese stop-word list goes away (ingestion/enricher.rs:140).

### IMP-027: [ingestion] Embedding model independent of the chat model, target on each chunk
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 8, Decisions, and section 7, Decisions. The embedding model is defined by the model kit and changeable by the user, no longer derived from the chat model; each chunk records its embedding_target (provider, model, dimension). Continues the M1 work on branch feat/imp-004-embedding-model. Supersedes FEAT-005.

### IMP-028: [ingestion] Embedding never switches model silently; OpenAI and mock fallbacks removed
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 8, Decisions. If the chosen model fails, the files fail, the user is told and the next sync tries again. The fallback to OpenAI and the offline mock (deterministic pseudo-vectors, ingestion/orchestrator.rs:770 and 1007) end.

### IMP-029: [ingestion] Query and document prefixes per model from the catalog
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 8, Decisions. The catalog declares each model's query and document prefixes and the pipeline applies them at ingestion and at query time (e.g. gemini-embedding-2, e5). Supersedes IMP-014.

### IMP-030: [ingestion] Embedding batches sized by tokens and API limits, controlled parallelism
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 8, Decisions. Batches limited by tokens and by the provider's maximum, with controlled parallelism for remote providers and use of the available CPU/GPU locally. Today it is one call per file, unbounded, one at a time.

### IMP-031: [storage] Tantivy lexical index per workspace without chunk text
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 9, Decisions. Tantivy replaces the in-house BM25: on-disk, incremental, segment-based, with fields of different weights (header lower than body). One index per workspace; deleting a workspace deletes its directory. The index stores terms and ids only; the text comes from LanceDB. Acceptance: exact phrase and identifier queries (quoted passages, FEAT-087), which supersedes FEAT-017 with FEAT-087.

### IMP-032: [ingestion] Multilingual lexical tokenization without per-language tables
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 9, Decisions. Word segmentation by Unicode UAX #29 with dictionaries for languages without spaces (ICU4X), generic accent removal by NFKD without combining marks instead of the manual Latin-only table, identifier splitting (camelCase, snake_case) kept with the whole identifier, and no stemming.

### IMP-033: [storage] One LanceDB table per workspace and embedding target at the model's dimension
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 10, Decisions. Each workspace has its own LanceDB tables, one per embedding_target at the model's real dimension, instead of the single workspace_chunks table filtered by a column and padded to 1536. On a model switch the new table fills while the old one keeps answering and is dropped at the end. Supersedes IMP-004.

### IMP-034: [storage] Automatic compaction and schema migrations on open
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 2 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 10, Decisions. Compaction and pruning after syncs with many deletes or reindexes, so replaced data does not stay on disk in old Lance versions. Migrations leave the sync path: the schema is versioned and migrated once, when the store is opened. Supersedes IMP-013.

### IMP-035: [retrieval] History by token budget with a background summary of older turns
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input. Recent messages go whole and older ones become a conversation summary generated in the background, instead of the fixed 30-turn window.

### IMP-036: [storage] Chat history stored as an encrypted tree of messages
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 1, Question input, and step 10, History and evaluation. Each message has a parent_id and the chat records its active branch, so branches need no migration later. Summary, follow-up rewriting and the queue follow the active branch. History lives in the user's local database, encrypted.

### IMP-037: [retrieval] Two modes, Fast and Deep, with tools in both; ReAct mode removed
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 2, Routing. ReAct stops being a mode. The tools (help, status, source tools, table query, web search when allowed) are available in Fast (search first, at most about 2 tool calls) and in Deep. Supersedes FEAT-019.

### IMP-038: [retrieval] Automatic mode escalates from Fast to Deep on weak evidence
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 2, Routing. Automatic mode starts Fast and escalates to Deep when the best chunks have low reranker relevance or the LLM says the context is not enough, with a discreet indicator; the user can pin Fast or Deep. Acceptance: no keyword routing in any language; the PT/EN keyword classifier (actx-agent/src/router.rs:176-234) and the heuristic query classifier (actx-onnx/src/query_classifier.rs) are removed. Supersedes the query classifier part of IMP-006.

### IMP-039: [retrieval] Deep turns refine, search and evaluate, with turn and token ceilings
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 2, Routing. Each Deep turn refines or decomposes the question from what was found, searches, and evaluates what is missing. Default maximum of 3 turns (up to 5 only through a domain pack), stopping early when evidence is enough, plus a token ceiling; the default is tuned by the evaluation set.

### IMP-040: [retrieval] Pre-filtered dense and lexical retrievers over child chunks
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 4, Search. Dense and lexical retrievers run in parallel for each question or sub-question with filters inside the search, over child chunks, with the model's query prefix and the table of the embedding target (the old table answers during a model switch) and weighted lexical fields; 30 candidates per list, tuned by the evaluation set. Only the workspace is searched. A question embedding failure gives lexical-only search with a discreet warning in the answer (today only a log line).

### IMP-041: [retrieval] Single RRF over clean lists with a per-document cap, no score cut
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 5, Fusion. All lists (every Deep sub-question, every source, future retrievers) enter one RRF with k = 60 and equal weights, tuned per workspace by the evaluation set; each document takes at most a few reranker slots, the rest in reserve. Acceptance: the RRF minimum-score cut per preset and Deep's deduplication with score summing are removed.

### IMP-042: [retrieval] Reranker depth by time budget and profile
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 5, Fusion. Candidates sent to the reranker: Fast 30, Deep 50, adjusted by hardware profile (e.g. 20 on light profiles, up to 100 on strong ones or the Model Server) within a reranking time budget (about 1 second in Fast), tuned per workspace by where the right chunk usually sits.

### IMP-043: [retrieval] Cross-encoder as the only relevance judge, always on
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 6, Reranking. The cross-encoder from the hardware kit judges relevance in both modes, over the child chunk with its contextual header; summaries are reranked, SQL results are not. Acceptance: the /rerank command and the option to disable reranking are removed; Deep's chunk-by-chunk LLM yes/no is removed; candidates beyond the budget are discarded, not pushed to the end.

### IMP-044: [retrieval] Calibrated reranker relevance bands per model
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 6, Reranking. Reranker scores map to relevant, maybe and irrelevant bands with per-model thresholds in the catalog, refined per workspace by the evaluation. Irrelevant chunks never go to the LLM. The bands feed the nothing-found gate, the Fast-to-Deep escalation and selection. Supersedes IMP-007.

### IMP-045: [retrieval] Selection by relevance bands and the token budget
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 7, Selection. Chunks of the relevant band enter, then the best of the maybe band, up to a context token budget set by AnyContext from the chat model and the mode (typically 3 to 5 chunks with parents in Fast). Near-identical chunks deduplicated, current version only (previous ones in comparisons), no hard per-file limit. Acceptance: the fixed top-k (10/20/40), per-file caps (2/3/5) and character ceilings (20/40/60 thousand) are removed.

### IMP-046: [retrieval] Nothing-found and weak-evidence gate by reranker bands
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 7, Selection. No relevant chunk in automatic Fast escalates to Deep; nothing after Deep, or with Fast pinned, answers that nothing was found without calling the LLM, suggesting rephrasing, adding sources or @source; only maybe-band chunks give an answer with an explicit weak-evidence warning. Replaces today's gate on the RRF score.

### IMP-047: [retrieval] Context grouped by file, with ingestion locators, dates and versions
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 8, Context assembly. One final context per question: chunks of the same file stay together in file order, and the file groups are arranged in a U by their best relevance. Each evidence has a short citation id, title, source and path, locator, document date and version from ingestion and its type (chunk, summary, table result with SQL); text stays escaped. Acceptance: no version deduced from the path (ADR-0140); no rank attribute that shows the prompt position or mixed score scales. Supersedes ADJ-005.

### IMP-048: [retrieval] Prompt in layers with the workspace prompt, ordered for caching
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 8, Context assembly. One prompt in layers instead of one per path: AnyContext base rules (only from the context, cite ids, user's language, safety, tools), domain pack guidelines, the workspace system prompt (persona, tone, format, focus; changed only by owner and editors in a shared workspace, never overriding the base rules, used only in calls that produce the user's answer) and the mode complement. Stable layers go first, as a system message, for provider prompt caching. Supersedes IMP-010, IMP-011 and FEAT-009. The user layer (role, answer style and explicit memories, identity-and-governance-v2.md topic 7) sits below the workspace prompt.

### IMP-049: [retrieval] Context token budget computed from the model window
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 8, Context assembly. Model window = system prompt + history (with the summary of older parts) + evidence + reserve for the answer; evidence gets what is left, within the cost ceiling of selection. Today the ceiling is in characters, unrelated to the model window.

### IMP-050: [retrieval] Always stream answers with visible progress
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 3 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/retrieval-pipeline-v2.md, step 9, Generation and citations. Streaming in both modes, with visible progress (querying a table, Deep turn 2...). Web results, when allowed, are marked as web with their URL, separate from workspace sources; weak-evidence warnings and the enumerative count come at the start of the answer.

### IMP-051: [storage] Knowledge base port per workspace hiding the three stores
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 1, Storage port. A single port per workspace over LanceDB, the lexical index and the workspace database: write and remove a document with its whole tree (parents, children, vectors, terms, metadata); dense, lexical and filter search; chunks by id and a chunk's parent; table and form queries; statistics. It has a local implementation and, later, a remote one (FEAT-110). Acceptance: the core no longer uses NativeLanceStore directly (11 files today).

### IMP-052: [storage] Write journal applied idempotently across the three stores
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 1, Storage port. Each write of the knowledge base port goes through a journal and is applied idempotently in LanceDB, the lexical index and the workspace database; a failure in the middle is completed on the next sync. Solves today's uncoordinated writes even without a server.

### IMP-053: [storage] Split settings.db into user-local and workspace databases
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 1, Storage port. settings.db, which today mixes preferences, keys, history, sources, diff cache and summaries, is split into a user-local database (preferences, chat history, vault references, hardware profile; never leaves the machine) and a workspace database (sources, diff cache, entities, layout maps, summaries, version families; travels with the knowledge base), with a migration.

### IMP-054: [models] Model port with explicit roles and per-role bindings
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Roles: embedding, reranker, OCR, page parsing, form fields, image description, transcription (with VAD and speakers), entities and personal data, context and summary LLM, chat LLM. Each role has a binding: model (catalog id and version) + where it runs (local ONNX or endpoint). Acceptance: the ONNX roles get a common interface like LmProvider.

### IMP-055: [models] Hardware kits as sets of per-role bindings
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Each hardware profile becomes a set of bindings per role; an Enterprise / VPC kit points the heavy roles to the server. Acceptance: profile RAM and tiers recalibrated to the models that actually run. Supersedes ADJ-003.

### IMP-056: [governance] Admin Hub model policy checked at binding resolution
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Local only, corporate gateway only, hybrid and the allowed models are checked when a binding is resolved; a forbidden binding is refused with a clear reason, never swapped silently. Today the Admin Hub model policies are not wired to the core.

### IMP-057: [models] Rules for an unavailable remote model
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role. Embedding never switches models (the file waits for the next sync); the other ingestion roles are queued and retried and may fall back to local only with the same model and version; in chat the error is shown and switching LLM is the user's choice within the policy.

### IMP-058: [models] Single verified model catalog for core and installer
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 2, Model port, by role, and platform-shared-components.md, Verified catalog. One catalog module consumed by the core and the installer, replacing the duplicated catalogs in any-context-core-rs ingestion/model_manager.rs and actx-installer hardware.rs, guarded today only by catalog parity tests. It carries per-model prefixes, reranker band thresholds and fingerprint reference vectors. Supersedes IMP-005.

### IMP-059: [ingestion] Persistent prioritized job queue per workspace
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 5, Ingestion worker. A persistent queue per workspace with priorities: base indexing > enrichments > self-evaluation > maintenance. Jobs are idempotent and tied to the storage port journal (IMP-052), so an interruption resumes without duplicating or losing anything. Today each sync spawns an actx --sync-worker process (commands/engine.rs:574) with no queue.

### IMP-060: [ingestion] Single writer per workspace through a lease
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 5, Ingestion worker. One writer per workspace holds a lease; if the worker dies the lease expires and another takes over. Reading and searching are never blocked. Today nothing prevents two workers on the same workspace.

### IMP-061: [ingestion] Worker progress through events; cancel and pause as queue commands
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 4 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/deployment-architecture-v2.md, seam 5, Ingestion worker. Progress and status as events (start, item processed, error, end): native Tauri events locally, over the server connection in mode 3, replacing the SQLite status table. Cancel and pause are queue commands honored between jobs and inside long ones (pause points in a long transcription).

### IMP-062: [identity] Migration from the machine-id key on the first sign-in
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 2, Key hierarchy, item 7. On the first sign-in after the update, data encrypted with the old key derived from the machine id is re-encrypted with the new keys and the API keys move to the personal vault, without data loss.

### IMP-063: [release] Release build profile: LTO, private debug symbols, stripped binaries, unwind
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 2. [profile.release] moves to lto = "fat" and codegen-units = 1 (today lto = "off", codegen-units = 16); published binaries stay stripped while CI keeps split debug symbols privately to read crash reports; panic = "unwind" stays because retrieval/bm25.rs:336 and actx-onnx/src/runtime.rs:80 use catch_unwind. Acceptance: release build time and binary size measured before and after.

### IMP-064: [release] Updater behind a port; the core stops depending on actx_installer
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. The core reaches install paths, release lookup and update logging through a port implemented by an adapter, so actx_installer can be listed in architecture.forbidden_core_imports (today commands/engine.rs calls it directly). Supersedes IMP-002.

## Adjustments

### ADJ-006: [cli] Retire the TUI when the desktop app reaches parity
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 1, item 4. The ratatui TUI keeps working over the current core but receives no v2 features, and is removed once the desktop app reaches parity, with its documentation (tui.md) and manual tests; `actx` without arguments then shows the CLI help or opens the desktop app.

### ADJ-007: [docs] Mark the ADRs the v2 design replaces as superseded
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 5. Review the 73 ADRs in docs/adr (all already in English, all `accepted` today) against the v2 design and mark those it replaces as superseded with a pointer to the v2 document (e.g. 0116 path healing, 0129 and 0130 reranking, 0077 TUI); the valid ones stay.

### ADJ-008: [docs] Publish the license file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 5. The repository has no LICENSE file and the README tells readers to contact Levix Digital. Publish the license or the proprietary terms once the owner chooses them. Supersedes ADJ-002.

## Closed

### IMP-001: Review the Deep Search minimum score
- **Priority**: medium
- **Status**: duplicate
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Closed**: 2026-10-17, duplicate of BUG-006 (the threshold is above the maximum single-list RRF score)
- **Description**: Deep Search keeps results with a minimum score of 0.02, which looks low and may let irrelevant chunks into the combined answer.

### BUG-001: Data indexed before field encryption stays in plaintext
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: any_context_data_indexed_before_field_encryption_stays_in_plaintext
- **Closed**: 2026-10-17, plan any_context_data_indexed_before_field_encryption_stays_in_plaintext, PR #342
- **Description**: Chunks written to LanceDB before field-level encryption on every write (ADR-0131) remain unencrypted on disk until the workspace is re-indexed with actx --sync --force. Nothing tells the user, while the README states that indexed text is encrypted.

### BUG-002: Retired model ids in presets, onboarding and model menus
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: any_context_retired_model_ids_in_presets_onboarding_and_model_menus
- **Closed**: 2026-10-17, plan any_context_retired_model_ids_in_presets_onboarding_and_model_menus, PR #345
- **Description**: Every Claude id in the product is retired (claude-3-7-sonnet in /preset frontier, claude-3-5-sonnet in onboarding, claude-3-5-* in the models menu and /models), and so are gemini-1.5-pro, llama-3.3-70b-versatile (Groq, shut down 2026-08-16) and deepseek-chat/deepseek-reasoner (2026-07-24); o1 and o3-mini shut down on 2026-10-23 and gpt-4o-mini on 2027-01-14. Current Claude models also reject the temperature/top_p the Anthropic provider always sends, and Groq ids are routed to the wrong provider or sent with the groq/ prefix. Choosing these models fails on the first question.

### BUG-007: Retrieved text is not escaped in the XML prompt
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_retrieved_text_is_not_escaped_in_the_xml_prompt
- **Closed**: 2026-10-17, plan any_context_retrieved_text_is_not_escaped_in_the_xml_prompt, PR #343
- **Description**: `ContextFormatter::format_xml_chunks` writes chunk text and attribute values (file path, file name) into the XML prompt without escaping. Indexed content containing `</chunk>`, `</retrieved_context>` or quotes (this repository's own docs, crawled web pages) breaks the chunk boundaries the prompt relies on and opens a prompt-injection path.

### BUG-003: Changing a workspace chat model mixes embedding spaces on the next sync
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: any_context_embedding_target_changes_mix_vector_spaces_and_the_gemini_embedding_model_is_shut_down
- **Closed**: 2026-10-17, plan any_context_embedding_target_changes_mix_vector_spaces_and_the_gemini_embedding_model_is_shut_down
- **Description**: resolve_embedding_provider (ingestion/orchestrator.rs) derives the embedding model from the workspace chat model. After /model changes it, the next incremental sync embeds new or changed files with another provider/model while unchanged files keep their old vectors, and queries use the new model, so part of the workspace becomes unreachable by dense search. Needs the per-workspace embedding model record planned for Frente C, or a forced reindex when the embedding target changes.

### BUG-013: Gemini embedding model text-embedding-004 is shut down
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-17, 2026-10-17, BUG-002 discovery
- **Plan**: any_context_embedding_target_changes_mix_vector_spaces_and_the_gemini_embedding_model_is_shut_down
- **Closed**: 2026-10-17, plan any_context_embedding_target_changes_mix_vector_spaces_and_the_gemini_embedding_model_is_shut_down
- **Description**: Google shut down text-embedding-004 on 2026-01-14, so ingestion and query embedding for Gemini workspaces fail, and the dense search is skipped without a message (IMP-003). The successor is gemini-embedding-2. Changing the embedding model needs a reindex of the affected workspaces; plan it with BUG-003, FEAT-005 and IMP-004.

### BUG-004: Source round-robin puts diversity above relevance and breaks the U-shaped order
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_retrieval_order_follows_relevance
- **Closed**: 2026-10-17, PR #349 (plan any_context_retrieval_order_follows_relevance)
- **Description**: `search_single_async` returns chunks in the order produced by `apply_source_diversification` (round-robin across sources), and `ContextFormatter::format_xml_chunks` applies the U-shaped reordering to that order. The second-best chunk can land in the middle of the context while a weak chunk sits next to the user query. The round-robin also only reorders candidates that were already retrieved (dense + BM25 over the whole index), so it does not widen document coverage; its first pass gives every source one slot before any second chunk, so a low-scoring chunk from another source can displace a strong second chunk from the same source. Decision (2026-10-17): remove the round-robin and keep the per-source cap (`max_chunks_per_source`) applied in score order, in both `search_single_async` and `retrieve_hybrid_batch_async`; revisit the cap once hierarchical chunking (Frente D) and the evaluation (IMP-008) exist.

### BUG-005: Recency boost overrides relevance across sources
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_retrieval_order_follows_relevance
- **Closed**: 2026-10-17, PR #349 (plan any_context_retrieval_order_follows_relevance)
- **Description**: The recency boost in `search_single_async` adds up to 0.01 to the RRF score, while the best possible single-list RRF score is 1/61 ≈ 0.0164, so a recently modified file far down the rankings can outrank an older file ranked first. The intent was a tie-breaker between chunks of the same source, but all chunks of a file or web page share the same `last_modified`, and sync deletes the old chunks of a changed source, so the boost has no effect inside a source and only acts across sources. It also changes which candidates reach the cross-encoder.

### BUG-006: Deep Search minimum score discards single-list hits
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_retrieval_order_follows_relevance
- **Closed**: 2026-10-17, PR #349 (plan any_context_retrieval_order_follows_relevance)
- **Description**: Deep Search requests use `min_score: 0.02`, applied to RRF scores before re-ranking. A chunk found only by the dense search or only by BM25 scores at most 1/61 ≈ 0.0164, so it is discarded unless the recency boost (BUG-005) lifts it. Supersedes IMP-001.

### BUG-008: Scope and version attributes are guessed from path substrings
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_scope_and_version_attributes_are_guessed_from_path_substrings
- **Closed**: 2026-10-17, plan any_context_scope_and_version_attributes_are_guessed_from_path_substrings, PR #351
- **Description**: `ContextFormatter::infer_scope_and_version` labels chunks by substring matches on the path: `latest` becomes scope `test`, `author` becomes `auth`, `environment` becomes `config`, `rapid` becomes `api`, `dev2` becomes version `v2`. The prompt tells the model never to combine chunks with different scope or version, so wrong labels can block a correct answer.

### BUG-009: Deep Search synthesis bypasses the canonical context formatter
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_deep_search_synthesis_bypasses_the_canonical_context_formatter
- **Closed**: 2026-10-18, PR #353
- **Description**: The Deep Search `SUMMARY_PROMPT` places the original query before the chunks and lists them as plain `--- CHUNK n ---` blocks, without U-shaped reordering, XML isolation, scope/version attributes or a query anchor. It is the mode that gathers the most chunks from several sub-queries.

### BUG-010: Section and line metadata are lost at ingestion
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_section_and_line_metadata_are_lost_at_ingestion
- **Closed**: 2026-10-18, PR #355
- **Description**: When a document envelope exists, `embed_and_upsert_chunks` stores the document summary in place of the chunk's `header_path`, and start/end lines are not stored in LanceDB. The prompt header only shows the file name, and the Deep Search retriever passes `header_path`, `start_line` and `end_line` as `None`, although `SUMMARY_PROMPT` asks for `[path:lines]` citations. Related to the LanceDB schema changes planned in Frentes C and D (IMP-004).

### BUG-011: Chunk text sent to the model depends on the retrieval path
- **Severity**: low
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_chunk_text_and_deduplication_depend_on_the_retrieval_path
- **Closed**: 2026-10-18, plan any_context_chunk_text_and_deduplication_depend_on_the_retrieval_path
- **Description**: BM25 stores the enveloped text (`[Context: ... | Keywords: ...]` plus the chunk), while LanceDB stores the raw chunk. Chunks found only by BM25 reach the prompt with the envelope; chunks found by the dense search reach it without.

### BUG-016: Deep Search deduplication merges every chunk of a file
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-18, v0.45.0, BUG-011 discovery
- **Plan**: any_context_chunk_text_and_deduplication_depend_on_the_retrieval_path
- **Closed**: 2026-10-18, plan any_context_chunk_text_and_deduplication_depend_on_the_retrieval_path
- **Description**: retrieve_hybrid_batch_async keys its cross-query deduplication on content_hash, which is the SHA-256 of the whole file: every dense hit of one file collapses into the first chunk and the scores of the other chunks are added to it. BM25-only hits carry an empty hash and fall back to the chunk id, so they are not merged.

### BUG-012: Agent prompt names tools that do not exist
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: any_context_agent_prompt_names_tools_that_do_not_exist
- **Closed**: 2026-10-18, plan any_context_agent_prompt_names_tools_that_do_not_exist, PR #359
- **Description**: `config/AGENT.md` tells the model to call `add_web_source`, `list_web_sources`, `remove_web_source` and `search_db(search_session_memory=True)`, but only `search_db(query)` and `system_status` are registered. It also says the model must always call `search_db` first, while the deterministic path sends the retrieved chunks with no tools available.

### BUG-017: system_status and web_search never reach the model on the routed paths
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-18, v0.45.0, BUG-012 discovery
- **Plan**: any_context_system_status_and_web_search_never_reach_the_model_on_the_routed_paths
- **Closed**: 2026-10-18, PR #361 (status route); the web search half moved to FEAT-019, the raw-snippet tool is BUG-018
- **Description**: LanceDB always opens (NativeLanceStore::open creates the directory), so build_agent_sync always sets the deterministic retriever and Agent::run/stream send Fast and Auto-fast questions to the one-hop path and Deep and Auto-deep questions to Deep Search; neither offers tools. ReAct, the only path with search_db, system_status and web_search, runs only when LanceDB fails to open. Effects: /web-search on does not change answers, and status questions (MT-MEM-002) and live web search (MT-WEB-003) cannot be answered. Decide how status and web results reach the routed paths. Related to IMP-011.

### BUG-014: Failed embedding calls store pseudo-vectors next to real ones
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, 2026-10-17, BUG-003 discovery
- **Plan**: any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries
- **Closed**: 2026-10-18, PR #363 (plan any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries)
- **Description**: embed_and_upsert (ingestion/orchestrator.rs) replaces a failed provider.embed call with SHA-256 pseudo-vectors (generate_fallback_vectors) and writes them to the same table as real vectors, without telling the user or marking the file. A transient API error (rate limit, network) leaves those files unreachable by dense search until they change on disk. Decide between failing the file so the next sync retries it, or marking it for re-embedding; keep the ADR-110 no-key fallback. Related to IMP-003.

### IMP-003: Report query embedding failures instead of skipping dense search silently
- **Priority**: medium
- **Status**: done
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries
- **Closed**: 2026-10-18, PR #363 (plan any_context_embedding_failures_are_hidden_pseudo_vectors_at_ingestion_and_silent_bm25_only_queries)
- **Description**: NativeHybridPipeline::search_single_async discards the query embedding error (retrieval/pipeline.rs, '_ => None'), so a missing key or unsupported provider silently leaves only BM25. Keep the ADR-110 fallback but record the outcome (for example a last-status accessor like last_rerank_latency_ms) and surface it in the planned /embed status.

### BUG-019: Dense search ranks arbitrary chunks on the mock embedding target
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-18, 2026-10-18, v0.45.0, BUG-014/IMP-003 discovery
- **Plan**: any_context_dense_search_ranks_arbitrary_chunks_on_constant_vectors
- **Closed**: 2026-10-19, PR #365 (plan any_context_dense_search_ranks_arbitrary_chunks_on_constant_vectors)
- **Description**: Without an embedding key, resolve_embedding_provider falls back to MockLmProvider (target mock/mock-embed), whose embed returns the same constant vector (0.1 x 1536) for every text. Chunks indexed and queries embedded on that target are all at the same distance, so dense search returns an arbitrary candidate pool that RRF fuses with BM25: a rank-1 dense-only hit scores 1/61, above RRF_MIN_SCORE (0.005), so unrelated chunks can reach the prompt. Found while planning BUG-014/IMP-003; candidate fix: skip dense search on the mock target and report it as BM25-only.

### BUG-015: Deep Search reflection and rerank prompts inject raw chunk text
- **Severity**: medium
- **Status**: done
- **Found**: 2026-10-17, 2026-10-17, v0.45.0, BUG-009 discovery
- **Plan**: any_context_deep_search_reflection_and_rerank_prompts_inject_raw_chunk_text
- **Closed**: 2026-10-19, PR #367
- **Description**: REFLECT_PROMPT and RERANK_PROMPT (crates/actx-agent/src/deep_search.rs) insert chunk text, chunk previews and queries with plain string replacement, without neutralizing the reserved prompt tags of ADR-0136. RERANK_PROMPT wraps the text in <chunk> tags that the text itself can close. Same risk as BUG-007, on the intermediate Deep Search calls.

### BUG-018: web_search tool sends raw web snippets to the model
- **Severity**: medium
- **Status**: dropped
- **Found**: 2026-10-18, 2026-10-18, v0.45.0, BUG-017 discovery
- **Plan**: -
- **Closed**: 2026-10-19, Wrong premise (user decision, 2026-10-19): live web search authorized by the user is supplementary lookup and is not indexed; only web pages added to a workspace as sources must be indexed. Returning search results to the model is the intended behavior; the real gap, web search not reaching the routed paths, is FEAT-019.
- **Description**: The ReAct web_search tool (crates/actx-cli/src/engine.rs) returns the search provider's titles, URLs and snippets straight to the model, so the answer uses web content that was never indexed. This breaks the platform rule that web content, like any other, is indexed before inference. ReAct runs only when LanceDB fails to open (BUG-017), so it is rarely reached. Fix it together with the indexed web search feature.

### BUG-020: Router performance test fails intermittently in debug builds
- **Severity**: low
- **Status**: done
- **Found**: 2026-10-19, 2026-10-19, BUG-019 M1 test run
- **Plan**: -
- **Closed**: 2026-10-19, PR #369
- **Description**: test_router_sub_microsecond_performance (crates/actx-agent/tests/router_tests.rs) asserts an average under 50 µs per ChatModelRouter evaluation, measured with wall-clock time in an unoptimized build. Under parallel test load it fails intermittently (54.4 µs observed; 1 of 3 isolated runs failed), so cargo test --workspace can go red without a code change.

### BUG-021: Dense search never runs when the embedding dimension is not 1536
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-19, v0.45.0, IMP-004 discovery 2026-10-19
- **Plan**: -
- **Closed**: 2026-10-19, PR #371
- **Description**: Chunk vectors of a model whose dimension is not 1536 (seen with Ollama nomic-embed-text, 768) are zero-padded to the 1536 table on write, but the query vector is sent unpadded; LanceDB rejects it (no vector column matches the query dimension), the pipeline drops the error, and the workspace is answered by BM25 only while last_query_embedding() reports Dense.

### BUG-023: Default workspace retrieves chunks of every workspace
- **Severity**: high
- **Status**: done
- **Found**: 2026-10-19, IMP-004 plan review, 2026-10-19
- **Plan**: -
- **Closed**: 2026-10-19, PR #372
- **Description**: Retrieval treats the Default workspace as no workspace filter: dense search (retrieval/pipeline.rs:617, 660, 715), metadata search (pipeline.rs:770, 946) and BM25 (retrieval/bm25.rs:213) return chunks of all workspaces for a query in Default, and a request without a workspace falls back to Default (pipeline.rs:592). Expected: Default sees only its own files, plus the Global AnyContext documentation, like every other workspace. Decided by the user on 2026-10-19.

### FEAT-005: Embedding model independent of the chat model, with stale-workspace status
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-027
- **Description**: The embedding model is derived from the workspace chat model (see BUG-003). Make it a setting of its own, global by default, and record on each workspace the model it was indexed with. Changing it must warn the user strongly that every workspace needs a resync, mark each affected workspace as outdated in its status and guide the resync. A per-workspace embedding override was discussed and left out for now (main case: a sensitive workspace on a local model); recording the model per workspace keeps it possible later. Related to IMP-004.

### FEAT-006: Individual sources: single local files and single web pages
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-020
- **Description**: Allow adding a single file (not only folders) and a single web page (not only a crawled URL prefix) to a workspace, without indexing the same content twice when the file already sits inside a registered folder.

### IMP-004: Per-table vector dimension in LanceDB with assisted reindex
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: any_context_per_table_vector_dimension_in_lancedb_with_assisted_reindex
- **Closed**: 2026-10-23, Superseded by IMP-033
- **Description**: storage/lancedb.rs fixes DEFAULT_VECTOR_DIM = 1536 and ingestion writes every table with it, while Gemini (768), Ollama nomic-embed-text (768) and local ONNX embedders (384/768) produce other sizes. Store model and dimension per workspace/table, detect divergence and offer assisted reindex. First LanceDB schema change since C0: needs migration and rollback; combine with the Frente D parent_id change (roadmap 2.2 goal 4).

### IMP-012: Encrypt the BM25 index file at rest
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, BUG-001 planning, 2026-10-17
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-069
- **Description**: bm25_index.bin keeps chunk text in plaintext because lexical search needs the terms (ADR-0131 consequence), so the indexed text is not fully encrypted on disk. Options: encrypt the serialized file with the hardware-bound key and decrypt on load, or store only hashed terms.

### IMP-013: Compact and prune LanceDB after deletes and forced reindexes
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, BUG-001 implementation, 2026-10-17
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-034
- **Description**: Deletes and actx sync --force only write deletion markers and new versions, so replaced chunk data (including removed or changed documents) stays on disk in old Lance versions until a compaction and prune runs. Only the plaintext migration (ADR-0135) prunes today. Add periodic optimize (compact + prune) after syncs that delete rows.

### IMP-014: Asymmetric retrieval prefixes for gemini-embedding-2
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17, BUG-003/BUG-013 M1 evidence
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-029
- **Description**: Google's embeddings guide says gemini-embedding-2 rejects task_type and expects text prefixes instead: 'task: search result | query: {content}' for queries and 'title: {title} | text: {content}' (title: none) for documents. The Gemini adapter sends raw text for both today, which works but leaves retrieval quality on the table. Add the prefixes per role (query vs document) through the embedding port, and measure with IMP-008. Changing them changes the vector space, so it reuses the BUG-003 embedding target record (a new target key) to trigger the reindex.

### IMP-006: Replace labeled heuristics with real neural inference
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-052 and IMP-038
- **Description**: query_classifier.rs, scan sentinel, layout extractor and vision encoder run deterministic heuristics labeled as such. Document AI weights are already downloaded by the profiles; real inference is missing.

### IMP-007: No relevance cutoff on cross-encoder scores
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-044
- **Description**: The deterministic path always sends up to `top_k` (6) chunks, and `min_score` is applied before re-ranking on the RRF scale. The cross-encoder's calibrated scores are never used to drop weak chunks, and the Zero Hallucination Gate threshold (0.005) is tuned for RRF scores.

### IMP-008: No retrieval and answer quality evaluation
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-073 and FEAT-106
- **Description**: There is no reference question set or metric (recall@k, NDCG@5, citation faithfulness) to calibrate thresholds or show the effect of retrieval changes, including the planned hierarchical chunking (Frente D).

### IMP-009: Follow-up questions are retrieved without conversation context
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-081
- **Description**: The deterministic retriever searches with the current message only. A follow-up such as "and in v1?" is searched as written, without the subject of the previous turns.

### IMP-010: Deterministic RAG prompt is sent without a system message
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-048
- **Description**: The deterministic path places the system instructions inside the user message (`<system_instructions>`) after the session history, so the instructions are repeated per turn and cannot benefit from provider prompt caching.

### IMP-011: Review all prompts and organize them into skills
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-048
- **Description**: Prompts live in five places (`config/AGENT.md`, `actx-cli/src/prompt.rs`, `retrieval/prompt.rs`, the Deep Search prompts in `deep_search.rs`, `config/SYSTEM_KNOWLEDGE.md`) with overlapping and conflicting rules, for example four citation formats (`[chk_1]`, `[path:45-60]`, citation footers, file names). Goal: a minimal base agent prompt, with each function (deterministic RAG, agentic search, Deep Search, source management) moved into a skill loaded when needed, and every prompt reviewed for ambiguity and contradictions. There is no skill mechanism today.

### FEAT-009: Per-workspace instructions (system prompt)
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-048
- **Description**: Let the user write instructions for the model per workspace. They must sit below the grounding and safety rules, never override them, and have a size limit.

### FEAT-019: Live web search on the routed paths
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-18, 2026-10-18, v0.45.0, BUG-017 discovery
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-037
- **Description**: With /web-search on, the model may search the web to complement the indexed workspace content, deciding when a search is needed, on the one-hop (Fast, Auto-fast) and Deep Search (Deep, Auto-deep) paths. Today the web_search tool exists only on ReAct, which runs only when LanceDB fails to open, so /web-search on changes nothing and MT-WEB-003 fails (the web half of BUG-017). Search results are supplementary lookup, not workspace documentation: they are not indexed (unlike web pages added to a workspace as sources with /web, which are always indexed). Requirements: results are untrusted content and are neutralized in prompts like chunk text (BUG-015, actx_agent::prompt_safety); they reach the model and the answer marked as web sources, kept apart from workspace citations; with /web-search off nothing leaves the machine. Amend ADR-0145, which recorded that all web content is indexed before inference, to separate workspace web sources (indexed) from authorized live web search (not indexed). Open decisions: how the model decides to search on the routed paths (tool call, router signal or Deep Search step), how many results and whether to fetch full pages, latency budget.

### ADJ-005: Chunk rank attribute shows the prompt position
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-047
- **Description**: The `rank` attribute in `<chunk>` is the position after U-shaped reordering, not the relevance rank, and `score` mixes RRF and cross-encoder scales depending on whether re-ranking ran.

### FEAT-007: Document mode: work on one whole file
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-108
- **Description**: Let the user pick a file in the workspace and summarize or analyze it as a whole. Top-k retrieval cannot cover a whole document, so questions such as "summarize this file" are answered from a few chunks today.

### FEAT-015: Tools to list and read sources, folders and files
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-109
- **Description**: Agent tools and UI commands to list a workspace's sources, browse folders and files and read a file or a range of it, so the user does not need to leave AnyContext. Limited to registered sources and read-only. Full web browsing was discussed and left out; reading pages through the existing web reader covers it.

### FEAT-017: Exact text search over sources
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-087 and IMP-031
- **Description**: A grep-style search for exact identifiers and strings across a workspace's sources, as a complement to semantic search.

### FEAT-002: Local generative Gemma 4 (E2B/E4B ONNX)
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-115
- **Description**: The published family is E2B, E4B, 12B, 26B-A4B and 31B; the tiers in actx-onnx gemma.rs (Compact, 27b, 32b) do not exist. Official ONNX only for E2B/E4B, multi-graph and multi-shard (E2B q4 > 3.6 GB). Needs a multi-shard downloader, a generative loop (tokenization, KV-cache, sampling) and tier recalibration; today Planned with the fallback provider (ADR-130, C0 D14).

### FEAT-003: GPU execution providers (DirectML on Windows, CUDA on Linux)
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-116
- **Description**: ONNX inference runs on CPU only. Needs the matching ONNX Runtime package and care with DLL dependencies (C0 D1: static DirectML linking prevented actx.exe from starting).

### IMP-005: Single source for the Core and Installer model catalogs
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-058
- **Description**: Model catalogs are duplicated in any-context-core-rs ingestion/model_manager.rs and actx-installer hardware.rs, guarded only by catalog_parity_tests. Move them to one shared module consumed by both (roadmap 2.2 goal 7).

### ADJ-003: Recalibrate hardware profile RAM to what actually runs
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-055
- **Description**: Profile RAM and tier semantics are calibrated for local Gemma 4, which does not run locally. Recalibrate for embedding + reranker + heuristic Document AI; disk was already fixed in PR #327 (roadmap 2.2 goal 7).

### FEAT-013: Minimal user profile
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-146
- **Description**: A short profile (name, language, role, answer style) collected during onboarding, with the rest in settings, always available to the agent so answers fit the user. Automatic learning of user traits from conversations was discussed and left out for now (wrong inferences, preferences mixed with grounded facts, privacy).

### FEAT-014: Explicit user memories
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-147
- **Description**: Preferences the user asks the agent to keep ("remember that...", `/remember`), or that the agent proposes to keep and the user confirms, stored locally in a visible and editable list, scoped globally or per workspace. Nothing is saved silently.

### FEAT-018: AnyContext MCP server
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-148
- **Description**: Expose AnyContext workspaces through an MCP server so other agents and IDEs can query them. There is no API or MCP surface today. Discussion item.

### FEAT-008: Cloud drive connectors
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-164, FEAT-165 and FEAT-166
- **Description**: Sources from cloud drives (folders and individual files). No drive connector exists today. Discussion item.

### FEAT-010: Token usage and context window fill in the status bar
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-103 and FEAT-181
- **Description**: Show the tokens used per model and how full the context window is, including custom models outside the catalog. Discussion notes: providers report token usage in their responses; the context window size is not available from every provider, so custom models may need it entered by the user. Tokens sent over the network and tokens processed locally can be shown apart.

### FEAT-016: Open a cited chunk at its file and line
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-057, FEAT-101 and FEAT-177
- **Description**: Jump from a citation in an answer to the cited file and line. Depends on BUG-010.

### ADJ-001: Update the outdated .clinerules
- **Priority**: low
- **Status**: done
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Closed**: 2026-10-24, PR #374: .clinerules described the Python-era project and was removed (decided 2026-10-23)
- **Description**: .clinerules no longer matches the current project (Rust workspace, architecture-docs layout, role-based development cycle) and can mislead agents that read it.

### ADJ-004: Close out stale planning documents
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Obsolete: the subscription tiers plan and the CLI/TUI blueprint were removed; RFC-042 Deep Search is superseded by IMP-039 (decided 2026-10-23)
- **Description**: Decide the fate of implementation_plan_subscription_tiers.md (BillingManager tiers) and BLUEPRINT_PRIORITY_4_CLI_TUI.md at the repository root, verify the state of RFC-042 (retrieve_hybrid_batch, /fast and /deep already exist), and close milestone M8 Release of the released v0.45.0 C0 plan.

### FEAT-001: actx profile install command
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-187
- **Description**: Install or switch the hardware profile after installation, reusing crates/actx-installer/src/provision.rs (roadmap 2.2 goal 7).

### FEAT-004: macOS release binary
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-189
- **Description**: release.yml only builds Linux x86_64 and Windows x86_64.

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by IMP-064
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

### FEAT-011: Local diagnostic bundle and /feedback with confirmation
- **Priority**: high
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-192
- **Description**: A way to get concrete feedback on how the app behaves without breaking the no-telemetry promise of the README: a local diagnostic bundle (logs, versions, configuration, with sensitive data masked) that the user reviews and sends, and a `/feedback <text>` command that shows exactly what leaves the machine and only sends after confirmation. Nothing is sent by default; offline and privacy presets keep everything local.

### FEAT-012: Opt-in crash reporting for beta and release-candidate builds
- **Priority**: low
- **Status**: dropped
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by FEAT-192
- **Description**: Crash and error reports collected only in beta/RC builds, off by default, with a global kill switch (such as `DO_NOT_TRACK` or an enterprise policy) and always off under the offline and privacy presets. Discussion item: must not conflict with the air-gapped positioning.

### ADJ-002: Add a license file
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Closed**: 2026-10-23, Superseded by ADJ-008
- **Description**: The repository has no LICENSE file; the README tells readers to contact Levix Digital for usage and distribution terms. The license (or the proprietary terms) still has to be chosen and published.

### FEAT-120: [identity] Local accounts with password, several per machine, data separated per account
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, Personal accounts, and base decisions 1 and 2. Without Admin Hub, each person has a local account (display name, optional username or e-mail, password stored only as an Argon2id hash) with its own directory holding its local database and workspaces; nobody sees anyone else's workspaces or conversations. Removing an account deletes its data. A password change only rewraps the data key. Today there is no user at all (only an added_by_email column defaulting to 'user').

### FEAT-121: [identity] root administration account and sudo for administrative actions
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, Linux-style administration account. `root` is the fixed administration account, labeled 'Administrator (root)', with no default password and its own recovery key. Administrative actions (creating or removing accounts, downloading models, installation settings) ask for the root password for that action only (`actx sudo <command>` in the CLI, a prompt in the UI). root has no workspaces or chat and cannot read personal accounts' data.

### FEAT-122: [identity] First run: root password, personal account and recovery key
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, Personal accounts. One first-run screen sets the root password and creates the personal account (same password allowed, with a warning), shows the recovery key and asks for confirmation that it was saved, and offers biometrics when available. The display name and language of the account feed the user profile (FEAT-146).

### FEAT-123: [identity] Sign-in, fast user switching and session lock
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, Personal accounts. Sign-in screen with the machine's accounts (password or biometrics) and fast user switching. The session locks after inactivity (e.g. 30 minutes, a personal preference), on screen lock, on sleep and manually; CLI and TUI ask once per terminal session with the same locking. Increasing delay after wrong passwords.

### FEAT-124: [identity] Biometric unlock (Windows Hello, Touch ID)
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1 and topic 2, item 1. Biometrics unlock the session as a convenience: the OS vault releases the wrapped master key only upon user presence at each unlock, never to a service on its own, without weakening the password.

### FEAT-126: [identity] Corporate identity: company SSO and the Admin Hub directory
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, In the corporate environment. In the corporate environment, users come from the company's SSO and/or the Admin Hub's own directory (for customers without an identity provider); local-only accounts are forbidden by the policy. Administration power belongs to the Admin Hub; root, corporate admins and the Admin Hub never read users' data.

### FEAT-127: [identity] Account activation by single-use code, password rules and second factor
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, In the corporate environment. An Admin Hub directory user is activated on an enrolled device with login and a single-use activation code (short validity, e.g. 72 h, delivered through a separate channel), then creates their own password and gets the recovery key; the password never goes through the Admin Hub and keys are generated on the device. The policy can require length, complexity, periodic change and an authenticator-app second factor. A forgotten password needs a new activation code.

### FEAT-128: [identity] Corporate administration: sudo as 'ask IT' and a rotating break-glass root
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 1, In the corporate environment. In the corporate environment sudo becomes the authentication of a corporate admin or an approval request in the Admin Hub (optionally an ITSM ticket). root remains for on-site and offline emergencies with a unique per-device password generated and rotated by the Admin Hub (LAPS model), visible only to authorized administrators; every use is audited, reported in the next heartbeat and followed by rotation.

### FEAT-129: [identity] Key hierarchy: master key, user key pair and workspace data keys
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 2, Key hierarchy, items 1 to 3 and 6. A random master key per account wrapped by the password (Argon2id), the recovery key and biometrics; a user key pair whose private key is encrypted by the master key; a random data key per workspace wrapped by the owner's master key and each member's public key. Keys live only in memory while the session is unlocked. Acceptance (decided 2026-10-23): the machine-id key with a fixed salt (security/mod.rs:10-11, 44-50) and the clear lexical index are gone, together with FEAT-069; the plain-text API keys are gone with FEAT-130. If the roadmap leaves this item out of the first release, the plain-text API keys become a high bug.

### FEAT-130: [identity] Personal vault for API keys, file passwords, history and preferences
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 2, Key hierarchy, item 4. A vault encrypted from the master key holds API keys, file passwords (FEAT-035), conversation history, preferences, the user profile and memories. Acceptance: API keys are no longer stored in plain text in the api_keys table of settings.db (storage/sqlite.rs:1470-1478).

### FEAT-132: [identity] Organization recovery key (enterprise policy)
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 2, Key hierarchy, item 8. Optional by policy: each user's master key is also wrapped by an organization public key whose private part stays offline under the company's control, ideally split between people, never in the Admin Hub. Each use is audited and users are told the company has this recovery power.

### FEAT-137: [governance] Single versioned policy model with three precedence layers
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 4, Governance engine and policies, items 1 and 2. One versioned policy model by category (models and endpoints, data, sources, tools, accounts, quotas). Layers: product default < local policy set by root without Admin Hub < organization policy signed by the Admin Hub; the upper layer wins, and inside a workspace the owner can only tighten the rules. Today the core has no policy at all.

### FEAT-138: [governance] One governance engine for policies, plan licenses and quotas
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 4, Governance engine and policies, items 4 and 6. A single engine enforces model resolution, trust levels and pseudonymization, web search, connectors and OCR, the MCP server, accounts and session, token quotas and the plan licenses (connectors-v2.md, decisions 25 and 28), keeping license and policy distinct in the block message. Every block says which rule blocked; the user can view the policy in force, read-only. Acceptance: license and policy checks run in the core below every surface (desktop app, CLI, TUI, MCP server), so no interface or wrapper can bypass them (cli-and-release-v2.md, section 1).

### FEAT-139: [governance] Signed policies with the Admin Hub key pinned at enrollment
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 4, Governance engine and policies, items 3 and 5. The enrollment code carries the Admin Hub key fingerprint; the device confirms and records it and from then on accepts only policies signed by it (instead of a key embedded in the binary, which does not work with customer-hosted Admin Hubs). Offline, the last valid policy in an encrypted cache applies for a validity the company sets; once expired the corporate environment is suspended until it reconnects.

### FEAT-140: [governance] Local audit of blocked attempts, metadata-only to the Admin Hub
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 4, Governance engine and policies, item 7. Blocked attempts are logged locally and sent to the Admin Hub only as metadata (rule, when, device), never with content.

### FEAT-141: [governance] Admin Hub enrollment and channel on the client
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 5, Admin Hub integration, and topic 2, item 5. Client side of the Admin Hub: enrollment with a device identity (Ed25519 key in the OS vault, no access to data), an outbound-only WebSocket, heartbeat and kill-switch, carrying identities and activation, signed policies, workspace definitions, groups and curators, break-glass passwords, sudo requests, metadata-only audit and quotas. The Admin Hub itself belongs to the levix-admin-hub backlog.

### FEAT-144: [identity] Personal and corporate environments isolated in one binary
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 6, Personal and corporate environments, items 1, 2 and 5. A switcher between the personal environment (local root and family accounts, never governed or seen by the company) and a corporate environment created at enrollment (or by the company's own package) with its own data, keys, accounts, policies, root and license. No path between environments: no export, search, chat, key or license crosses.

### FEAT-145: [identity] Corporate environment lifecycle
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 5 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/identity-and-governance-v2.md, topic 6, items 3 and 4, and topic 1, In the corporate environment (exit). The policy can forbid the personal environment on company-owned computers, never on the employee's own. A revoked environment (kill-switch) is blocked; an orderly unenrollment deletes or keeps data per policy; changing companies removes the previous environment and creates the new one. The personal environment is never affected.

### FEAT-167: [governance] Plan license signed by Levix and verified offline on the client
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 6 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/connectors-v2.md, decisions 25 and 28. One signed license per plan (Free, Pro, Enterprise), verified offline with the shared verification code from levix-platform; one license per environment (identity-and-governance-v2.md, topic 6). License is not policy: the license says what the plan includes, the policy what the company allows, and the governance engine (FEAT-138) enforces both and says which one blocked.

### FEAT-191: [release] Updates, models and packs verified by the Levix signature
- **Priority**: medium
- **Status**: dropped
- **Found**: 2026-10-23, batch 8 of the v2 backlog rebuild
- **Plan**: -
- **Closed**: 2026-10-23, Moved to levix-platform
- **Description**: Design: docs/architecture/cli-and-release-v2.md, section 3. Every download (application updates, ONNX models, domain packs, extraction maps, Chromium) is verified against the Levix signature before use, also through the Admin Hub mirror (FEAT-143). Today the installer only checks that a downloaded binary is a valid PE/ELF file and models carry a SHA-256 in the catalog.
