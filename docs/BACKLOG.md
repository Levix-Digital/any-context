# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-024: [ingestion] Secret files (.env, .tfvars, .key) are indexed in clear
- **Severity**: critical
- **Status**: open
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Plan**: -
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

### FEAT-001: actx profile install command
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Install or switch the hardware profile after installation, reusing crates/actx-installer/src/provision.rs (roadmap 2.2 goal 7).

### FEAT-002: Local generative Gemma 4 (E2B/E4B ONNX)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: The published family is E2B, E4B, 12B, 26B-A4B and 31B; the tiers in actx-onnx gemma.rs (Compact, 27b, 32b) do not exist. Official ONNX only for E2B/E4B, multi-graph and multi-shard (E2B q4 > 3.6 GB). Needs a multi-shard downloader, a generative loop (tokenization, KV-cache, sampling) and tier recalibration; today Planned with the fallback provider (ADR-130, C0 D14).

### FEAT-003: GPU execution providers (DirectML on Windows, CUDA on Linux)
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: ONNX inference runs on CPU only. Needs the matching ONNX Runtime package and care with DLL dependencies (C0 D1: static DirectML linking prevented actx.exe from starting).

### FEAT-004: macOS release binary
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: release.yml only builds Linux x86_64 and Windows x86_64.

### FEAT-007: Document mode: work on one whole file
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Let the user pick a file in the workspace and summarize or analyze it as a whole. Top-k retrieval cannot cover a whole document, so questions such as "summarize this file" are answered from a few chunks today.

### FEAT-008: Cloud drive connectors
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Sources from cloud drives (folders and individual files). No drive connector exists today. Discussion item.

### FEAT-009: Per-workspace instructions (system prompt)
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Let the user write instructions for the model per workspace. They must sit below the grounding and safety rules, never override them, and have a size limit.

### FEAT-010: Token usage and context window fill in the status bar
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Show the tokens used per model and how full the context window is, including custom models outside the catalog. Discussion notes: providers report token usage in their responses; the context window size is not available from every provider, so custom models may need it entered by the user. Tokens sent over the network and tokens processed locally can be shown apart.

### FEAT-011: Local diagnostic bundle and /feedback with confirmation
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: A way to get concrete feedback on how the app behaves without breaking the no-telemetry promise of the README: a local diagnostic bundle (logs, versions, configuration, with sensitive data masked) that the user reviews and sends, and a `/feedback <text>` command that shows exactly what leaves the machine and only sends after confirmation. Nothing is sent by default; offline and privacy presets keep everything local.

### FEAT-012: Opt-in crash reporting for beta and release-candidate builds
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Crash and error reports collected only in beta/RC builds, off by default, with a global kill switch (such as `DO_NOT_TRACK` or an enterprise policy) and always off under the offline and privacy presets. Discussion item: must not conflict with the air-gapped positioning.

### FEAT-013: Minimal user profile
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: A short profile (name, language, role, answer style) collected during onboarding, with the rest in settings, always available to the agent so answers fit the user. Automatic learning of user traits from conversations was discussed and left out for now (wrong inferences, preferences mixed with grounded facts, privacy).

### FEAT-014: Explicit user memories
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Preferences the user asks the agent to keep ("remember that...", `/remember`), or that the agent proposes to keep and the user confirms, stored locally in a visible and editable list, scoped globally or per workspace. Nothing is saved silently.

### FEAT-015: Tools to list and read sources, folders and files
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Agent tools and UI commands to list a workspace's sources, browse folders and files and read a file or a range of it, so the user does not need to leave AnyContext. Limited to registered sources and read-only. Full web browsing was discussed and left out; reading pages through the existing web reader covers it.

### FEAT-016: Open a cited chunk at its file and line
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Jump from a citation in an answer to the cited file and line. Depends on BUG-010.

### FEAT-017: Exact text search over sources
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: A grep-style search for exact identifiers and strings across a workspace's sources, as a complement to semantic search.

### FEAT-018: AnyContext MCP server
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Expose AnyContext workspaces through an MCP server so other agents and IDEs can query them. There is no API or MCP surface today. Discussion item.

### FEAT-019: Live web search on the routed paths
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-18, 2026-10-18, v0.45.0, BUG-017 discovery
- **Plan**: -
- **Description**: With /web-search on, the model may search the web to complement the indexed workspace content, deciding when a search is needed, on the one-hop (Fast, Auto-fast) and Deep Search (Deep, Auto-deep) paths. Today the web_search tool exists only on ReAct, which runs only when LanceDB fails to open, so /web-search on changes nothing and MT-WEB-003 fails (the web half of BUG-017). Search results are supplementary lookup, not workspace documentation: they are not indexed (unlike web pages added to a workspace as sources with /web, which are always indexed). Requirements: results are untrusted content and are neutralized in prompts like chunk text (BUG-015, actx_agent::prompt_safety); they reach the model and the answer marked as web sources, kept apart from workspace citations; with /web-search off nothing leaves the machine. Amend ADR-0145, which recorded that all web content is indexed before inference, to separate workspace web sources (indexed) from authorized live web search (not indexed). Open decisions: how the model decides to search on the routed paths (tool call, router signal or Deep Search step), how many results and whether to fetch full pages, latency budget.

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

## Improvements

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

### IMP-005: Single source for the Core and Installer model catalogs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Model catalogs are duplicated in any-context-core-rs ingestion/model_manager.rs and actx-installer hardware.rs, guarded only by catalog_parity_tests. Move them to one shared module consumed by both (roadmap 2.2 goal 7).

### IMP-006: Replace labeled heuristics with real neural inference
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: query_classifier.rs, scan sentinel, layout extractor and vision encoder run deterministic heuristics labeled as such. Document AI weights are already downloaded by the profiles; real inference is missing.

### IMP-007: No relevance cutoff on cross-encoder scores
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic path always sends up to `top_k` (6) chunks, and `min_score` is applied before re-ranking on the RRF scale. The cross-encoder's calibrated scores are never used to drop weak chunks, and the Zero Hallucination Gate threshold (0.005) is tuned for RRF scores.

### IMP-008: No retrieval and answer quality evaluation
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: There is no reference question set or metric (recall@k, NDCG@5, citation faithfulness) to calibrate thresholds or show the effect of retrieval changes, including the planned hierarchical chunking (Frente D).

### IMP-009: Follow-up questions are retrieved without conversation context
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic retriever searches with the current message only. A follow-up such as "and in v1?" is searched as written, without the subject of the previous turns.

### IMP-010: Deterministic RAG prompt is sent without a system message
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic path places the system instructions inside the user message (`<system_instructions>`) after the session history, so the instructions are repeated per turn and cannot benefit from provider prompt caching.

### IMP-011: Review all prompts and organize them into skills
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Prompts live in five places (`config/AGENT.md`, `actx-cli/src/prompt.rs`, `retrieval/prompt.rs`, the Deep Search prompts in `deep_search.rs`, `config/SYSTEM_KNOWLEDGE.md`) with overlapping and conflicting rules, for example four citation formats (`[chk_1]`, `[path:45-60]`, citation footers, file names). Goal: a minimal base agent prompt, with each function (deterministic RAG, agentic search, Deep Search, source management) moved into a skill loaded when needed, and every prompt reviewed for ambiguity and contradictions. There is no skill mechanism today.

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
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 9, Decisions. Tantivy replaces the in-house BM25: on-disk, incremental, segment-based, with fields of different weights (header lower than body). One index per workspace; deleting a workspace deletes its directory. The index stores terms and ids only; the text comes from LanceDB.

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

## Adjustments

### ADJ-001: Update the outdated .clinerules
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: .clinerules no longer matches the current project (Rust workspace, architecture-docs layout, role-based development cycle) and can mislead agents that read it.

### ADJ-002: Add a license file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The repository has no LICENSE file; the README tells readers to contact Levix Digital for usage and distribution terms. The license (or the proprietary terms) still has to be chosen and published.

### ADJ-003: Recalibrate hardware profile RAM to what actually runs
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Profile RAM and tier semantics are calibrated for local Gemma 4, which does not run locally. Recalibrate for embedding + reranker + heuristic Document AI; disk was already fixed in PR #327 (roadmap 2.2 goal 7).

### ADJ-004: Close out stale planning documents
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: Decide the fate of implementation_plan_subscription_tiers.md (BillingManager tiers) and BLUEPRINT_PRIORITY_4_CLI_TUI.md at the repository root, verify the state of RFC-042 (retrieve_hybrid_batch, /fast and /deep already exist), and close milestone M8 Release of the released v0.45.0 C0 plan.

### ADJ-005: Chunk rank attribute shows the prompt position
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The `rank` attribute in `<chunk>` is the position after U-shaped reordering, not the relevance rank, and `score` mixes RRF and cross-encoder scales depending on whether re-ranking ran.

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
