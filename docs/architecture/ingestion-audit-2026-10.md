# Ingestion audit — "Any Context" (October 2026)

Audit of the ingestion pipeline (target design in `ingestion-pipeline-v2.md`) against the
latest techniques on the market, with one goal: **any file type must be able to serve as
context for any professional** — IT, hospitals, public agencies, universities, accounting,
law, engineering.

Sections 1–4 are **proposals** (P1, P2…). Accepted ones go into the pipeline document; the
decisions are in section 5.

They respect the principles already decided: multilingual, explicit action, air-gapped by
default (local ONNX models, per hardware profile) and nothing disappears silently.

---

## 1. Harness² (Google Research)

[Harness²](https://github.com/google-research/harness2) (Apache-2.0) **is not an ingestion
framework**: it is a "recursive agent harnessing" framework — it optimizes the system
around a model (prompt, guardrails, memory, tools, skills…) without touching the weights.
It does not deal with parsing, chunking or indexing.

Three of its ideas transfer well to ingestion:

| Harness² idea | application to ingestion |
|---|---|
| **Propose–Probe–Compose without ground truth**: proposes variations, tests them on the task itself, composes from the contrast between runs | **H1 — ingestion self-evaluation** (below) |
| **Experience reflector**: keeps proven lessons for future tasks | **H2 — strategy memory per source** |
| **Editable skills and components** per domain | **H3 — domain packs** |

---

## 2. Format coverage by audience

The v2 design covers web, HTML, text, code, structured, tabular, PDF, Office, e-mail,
e-book and image. Formats central to several audiences are missing:

| audience | missing formats |
|---|---|
| everyone | archives (ZIP, 7z, TAR), mailboxes (MBOX, PST), audio and video, calendar (`.ics`), contacts (`.vcf`), MHTML, XPS |
| law, public sector | PDF with digital signature, annotations and forms; DOCX with comments and tracked changes; hearing recordings |
| hospitals | DICOM (metadata, structured reports, embedded PDF), HL7 v2, FHIR, CDA |
| universities | LaTeX, BibTeX, Jupyter (`.ipynb`), formulas in PDF, lecture recordings |
| engineering | DXF, IFC (BIM), STEP/IGES header, drawing sheets in PDF (title block), GIS (GeoJSON, KML/KMZ, Shapefile, GeoPackage) |
| accounting | XBRL; fixed-layout delimited record files (any country) |
| general office | Visio (`.vsdx`), Apple iWork (Pages, Numbers, Keynote) |

Out of reach without proprietary software: native DWG, OneNote, DRM-protected files.

---

## 3. Proposals

### Coverage

- **P1 — Containers and archives.** ZIP, 7z, TAR/GZ, MBOX and PST are opened recursively;
  each inner item becomes a child document with provenance (`case.zip > attachment 3.pdf`)
  and goes through the normal pipeline. Depth and uncompressed-size limits (zip bomb
  protection).
- **P2 — Audio and video.** Local multilingual transcription via ONNX (Whisper family, MIT),
  with timestamps as the chunk locator; optional speaker separation per profile. Video:
  audio track + key frames described by the vision model. Hearings, consultations,
  lectures, meetings.
- **P3 — Academic.** LaTeX by section, BibTeX as records, Jupyter by cell (code + output),
  PDF formulas recognized as LaTeX (via P8).
- **P4 — Engineering and GIS.** DXF (texts, layers, title block), IFC (entities and
  properties), STEP/IGES header, drawing sheets in PDF (vector text + title block), GIS
  (feature attributes and spatial metadata).
- **P5 — Health.** DICOM (tags, structured reports, embedded PDF; pixels through the
  optional vision model), HL7 v2, FHIR and CDA with chunkers that understand the structure
  of each.
- **P6 — Full Office.** DOCX: comments, tracked changes, footnotes, headers and footers.
  PPTX: speaker notes. XLSX: formulas, merged cells, named ranges, hidden sheets. Plus
  Visio, iWork, MHTML, XPS, `.ics` and `.vcf`.
- **P7 — PDF beyond text.** Outline/bookmarks as the section path, annotations, form fields
  (key–value), files attached to the PDF (through P1) and digital signature metadata
  (signer, date, validity) — essential in legal cases and public acts.

### Parsing quality

- **P8 — Document parsing model (small VLM, ONNX).** A single model that reads the page
  image and returns Markdown with reading order, tables, formulas and code:
  [Granite-Docling-258M](https://infoq.com/news/2025/10/granite-docling-ibm) (Apache-2.0,
  ONNX conversion exists) or [PaddleOCR-VL](https://arxiv.org/pdf/2510.14528) (0.9B, 109
  languages; license to be confirmed). Replaces the sum of OCR + layout model + 2D
  heuristic on the profiles that can afford it; light profiles keep the OCR + layout
  cascade. The choice requires a benchmark (e.g. OmniDocBench).
- **P9 — Broken text layer detection.** Digital PDFs whose extracted text is garbage (fonts
  without a Unicode mapping, CID) are treated as scanned and go to OCR/P8.
  Language-independent detection: proportion of replacement characters, private-use
  characters and improbable sequences, checked against an OCR sample.
- **P10 — Visual page retrieval.** For visually rich documents (charts, drawing sheets,
  forms), multi-vector embeddings of the page image, in the line of
  [ColPali](https://arxiv.org/abs/2407.01449v6), as an additional index on high profiles.
  Licenses vary by model (some are Gemma). Depends on the retrieval pipeline.
- **P11 — Tables as queryable data.** Every table (spreadsheet, CSV, PDF or DOCX table) is
  also stored in a queryable format, besides the chunks, for aggregate questions. Already
  noted as a retrieval dependency; here it extends to tables inside documents.

### Enrichment

- **P12 — Language-independent document metadata.** Document language, document date
  extracted from the content (not the mtime), type, title and authors; plus **zero-shot
  entity extraction** with types configurable per workspace (e.g. "case number",
  "patient", "sheet number", "tax identifier") via multilingual
  [GLiNER](https://pypi.org/project/GLiNER/) (Apache-2.0, exportable to ONNX). It becomes a
  filter and facet in retrieval.
- **P13 — Personal and sensitive data.** The same model tags chunks with personal data
  categories; the workspace policy decides: tag only, mask in the answer, or do not index.
  Hospitals and public sector (LGPD, GDPR, HIPAA — no rule tied to one country).
- **P14 — Hierarchical summaries.** Document and folder summaries generated by the
  profile's local LLM, in the background, stored as their **own searchable nodes** (not
  prefixed to chunks, like the removed envelope). They answer "what is this contract
  about?" and overview questions.
- **P15 — Late chunking.** With the long-context local embedding model (e.g. BGE-M3, 8192
  tokens), the document is embedded whole and each chunk receives the vector with document
  context, at no LLM cost — the "cheap contextual retrieval" for light profiles.
  [Reported gain is modest](https://arxiv.org/html/2409.04701v3) (≈ 3% nDCG@10). Effect on
  per-chunk diff: an edit re-embeds the window around it, not only the edited chunk.
- **P16 — Entity and relation graph.** Relations between documents (parties ↔ cases,
  components ↔ drawing sheets, patients ↔ exams). Expensive; evaluate only for high
  profiles and after P12.
- **P17 — Versions and near-duplicates.** MinHash/SimHash across documents: groups versions
  (`contract_v3_final_FINAL.docx`), marks the most recent one and stores identical content
  only once. Avoids answers that mix versions.

### Storage and efficiency

- **P18 — Compact vectors.** Matryoshka truncation and int8/binary quantization with
  reranking by the full vectors: an index several times smaller and faster, per profile.
- **P19 — Learned sparse vectors.** BGE-M3 sparse vectors as a complement to BM25, with
  better multilingual behavior. Optional.

### Operation

- **P20 — Automatic sync of the chosen sources.** File system watching on the sources the
  user selected (still explicit action) and scheduled recrawl for web sources. Today sync
  only runs when requested.
- **P21 — Resource governance.** Background ingestion at low priority, pausing on low
  battery or heavy use, GPU use and parallelism per profile.
- **P22 — Precise locators.** Page + region (bounding box) in PDF, timestamp in audio, cell
  in a spreadsheet, line in code — to open the original at the exact point (FEAT-016).
  Trust for law and health.
- **P23 — Per-source access control.** Who can see each source; depends on the future
  users module. Only record it for now.

### Inspired by Harness²

- **H1 — Ingestion self-evaluation (propose–probe–compose).** For each source, the local
  LLM generates synthetic questions from a sample of chunks and retrieval is executed: the
  hit rate becomes an **ingestion quality score** per file and per source. A low score
  triggers alternative strategies (OCR instead of the text layer, another chunk size, web
  rendering) and the best one stays. Poorly ingested files appear in the report.
- **H2 — Strategy memory per source.** The winning strategy (parser, OCR, rendering, chunk
  size) is remembered per source and file type and reused — it generalizes the web
  rendering calibration already decided.
- **H3 — Domain packs.** Per-area settings (health, legal, engineering, accounting,
  academic, IT): P12 entity types, enabled parsers, filter metadata, P13 sensitive data
  policy. Chosen per workspace, **orthogonal to the hardware profiles** and independent of
  language.

---

## 4. Suggested priority

| priority | proposals | why |
|---|---|---|
| 1 — base | P8, P9, P1, P6, P7, P12, P13, P17, P22, H1 | extraction quality and trust for every audience |
| 2 — specific audiences | P2, P3, P4, P5, P14, P15, P18, P20, H2, H3 | coverage per area and efficiency |
| 3 — evaluate later | P10, P16, P19, P21, P23 | high cost, depend on retrieval or the users module |

---

## 5. Decisions

| proposal | decision | where it was recorded |
|---|---|---|
| P8 | accepted: digital text is the truth; page parsing model for scanned pages on the profiles that can afford it; benchmark before fixing it | `ingestion-pipeline-v2.md`, step 6 |
| P9 | accepted: third page state "broken text", with verification against OCR when in doubt | `ingestion-pipeline-v2.md`, step 4 |
| P1 | accepted, with PST/OST at level 1, RAR read-only (license to be confirmed) and a vault for file passwords | `ingestion-pipeline-v2.md`, step 4 |
| P6, P7 | accepted; tracked changes with current text + changes as their own items; `.ics`, `.vcf`, MHTML at level 1, Visio, XPS and iWork at level 2 | `ingestion-pipeline-v2.md`, step 5 |
| Forms | accepted: LSIE (DocMapper) as the engine, automatic phase 0 per profile, reviewable layout maps, a single Rust crate for AnyContext and DocMapper, local integration with DocMapper, test corpus started with the fictitious CMR | `ingestion-pipeline-v2.md`, step 6 |
| P12, P13 | accepted: metadata and entities configurable per workspace; personal data masked by default with a cloud LLM and only tagged with a local LLM; secrets never indexed | `ingestion-pipeline-v2.md`, step 7 |
| P17, P22 | accepted, respecting the parent–child hierarchy: an exact duplicate stores the tree once; children never merged across different parents; version families at the document root; precise locator on the child and extent on the parent | `ingestion-pipeline-v2.md`, steps 3 and 7 |
| H1, H2 | accepted: deterministic signals on every profile, probing with synthetic questions on profiles with a local LLM, automatic reprocessing already in the first version, strategy memory per source, questions as the retrieval evaluation set | `ingestion-pipeline-v2.md`, cross-cutting section |
| H3 | accepted: 11 packs (Personal as default + legal, health, engineering, accounting, logistics, HR, academic, IT, public sector, real estate); learned layout maps, catalog only as an accelerator | `ingestion-pipeline-v2.md`, cross-cutting section |
| P13 (revision) | the default masking with a cloud LLM was replaced by unmasked search + reversible pseudonymization only of what goes to the cloud; "do not index" only for secrets or explicit choice | `ingestion-pipeline-v2.md`, step 7 |
| P3, P4, P5 | accepted: DWG refused with a suggestion to export (commercial license to evaluate), DICOM medical image off by default, SPSS and Stata later | `ingestion-pipeline-v2.md`, step 5 |
| P2 | accepted: Whisper transcription via ONNX, existing captions first, speakers on in the legal pack, FFmpeg in the installer for video frames | `ingestion-pipeline-v2.md`, step 5 |
| P14, P15 | accepted: document and folder summaries as their own nodes by the local LLM; late chunking on light profiles with long-context embedding, enabled only with a measured gain | `ingestion-pipeline-v2.md`, step 7 |
| P20 | accepted: watching only the chosen sources, fallback scan, diff on opening, daily web recrawl, sync only with AnyContext open by default and an optional background service | `ingestion-pipeline-v2.md`, cross-cutting section |
| P18 | accepted with quality first: no compaction by default, only when needed and without measurable loss; automatic and internal switch-off, without notifying the user; Matryoshka truncation never used | `ingestion-pipeline-v2.md`, step 10 |
| P10 | accepted: only visually rich pages, page as an extra list in search, image as evidence for multimodal chat (never to an external endpoint with pseudonymization active), per profile, full vectors | `ingestion-pipeline-v2.md`, step 7 |
| P16 | accepted: base graph without LLM (entities, documents, structural relations, correctable resolution), typed relations by the local LLM with citable evidence and pack types, used as a list, tool and enumerative source, one graph per workspace | `ingestion-pipeline-v2.md`, step 7 |
| P19 | accepted: only when the embedding model already produces sparse vectors (e.g. BGE-M3), stored in the lexical index, extra list in RRF enabled only with a measured gain | `ingestion-pipeline-v2.md`, step 9 |
| P21 | accepted: single governor with global priority (question first), adaptation to battery, load, temperature and network, memory budget, disk reserve, only "pause indexing" for the user, windows through the enterprise policy | `ingestion-pipeline-v2.md`, cross-cutting section |
| P23 | accepted: permission scope ID in the chunk, details in a separate SQLite database (scopes per workspace, users and groups on the server); linking through SSO and SCIM, denying what does not link; pre-filter in every retriever; aggregates per scope and no shared cache; permission change without reprocessing; unknown denies by default | `ingestion-pipeline-v2.md`, cross-cutting section |
