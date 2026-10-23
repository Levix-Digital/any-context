---
plan_id: any_context_blueprint_v0348_path_healing_and_temporal_recency_v2
project: any-context
title: 'Blueprint v0.34.8: Smart Path Healing, Historical Temporal Rules & Lightweight Change Detection'
version: v0.34.8
status: superseded
author: Levi Silveira
created_at: '2026-10-10T12:18:43'
updated_at: '2026-10-10T12:42:22'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- path-healing
- temporal-recency
- http-304
- metadata-cascade
- rag
- change-detection
supersedes: any_context_blueprint_v0348_path_healing_and_temporal_recency
superseded_by: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_na_tui_auto_scroll_react_e_mensagens_profissionais_de_atualizacao
parent_plan_id: null
milestones:
- name: 'M1: Smart Path Healing'
  done: true
- name: 'M2: Temporal Cascade and Web HTTP 304'
  done: true
- name: 'M3: last_modified Propagation and Prompt'
  done: true
- name: 'M4: Change Detection and Source Health'
  done: true
- name: 'M5: Tests and Dual-Doc'
  done: true
status_log:
- at: '2026-10-10T12:37:58'
  note: 'Milestone ''M1: Smart Path Healing'': phonetic and structural healing implemented and tested'
- at: '2026-10-10T12:38:00'
  note: 'Milestone ''M2: Temporal Cascade and Web HTTP 304'': 5-tier cascade and RFC 7232 HTTP 304 recovered from the Python history'
- at: '2026-10-10T12:38:01'
  note: 'Milestone ''M3: last_modified Propagation and Prompt'': last_modified propagated through BM25, the RAG Pipeline and the LLM Prompt'
- at: '2026-10-10T12:38:03'
  note: 'Milestone ''M4: Change Detection and Source Health'': missing_folders, alert status and active changes in the TUI'
- at: '2026-10-10T12:38:04'
  note: 'Milestone ''M5: Tests and Dual-Doc'': 235 green tests, ADR-116 in TECDOC and Scenario 31 in MANUAL_TESTS'
- at: '2026-10-10T12:38:05'
  note: Release v0.34.8 finished and published successfully
- at: '2026-10-10T12:42:22'
  note: Superseded by anycontext_blueprint_v0349_harmonizacao_aurora_boreal_na_tui_auto_scroll_react_e_mensagens_profissionais_de_atualizacao
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Engineering Blueprint v0.34.8: Smart Path Healing, Recovery of the Historical Temporal Rules & Lightweight Change Detection

## 📌 Context & Recovery of the Development History (Python Legacy ➔ Rust)

After a deep investigation of the git history (`git log -S "Last-Modified"`, commits `2c85f68` and `9d77227`), we recovered exactly how the date, time and change verification rules worked in AnyContext:

### 1. How Date and Time Were Collected in Local Folders:
- Collected directly from the file system metadata (`fs::Metadata`):
  - `mtime` (`last_modified_date`): exact date and time of the last modification (`fs::metadata(path)?.modified()`).
  - `ctime` (`creation_date`): file creation date (`fs::metadata(path)?.created()`).
- In the current Rust: `mtime` was converted to RFC 3339, but it was **dropped** in the middle of the search pipeline (`HybridSearchResult`).

### 2. How Date and Time Were Collected on Web Sites (Without Downloading to Check):
- **Lightweight Header Check - RFC 7232**:
  - A conditional HTTP `HEAD` or `GET` request sending the headers:
    - `If-Modified-Since`: date of the last crawl (`cached_last_modified`)
    - `If-None-Match`: cached `ETag`
  - If the server answers **`304 Not Modified`**: AnyContext confirms in **< 50ms**, without downloading the HTML body, that the page is still 100% identical!
  - If the server answers `200 OK`, or on a `HEAD` request: the HTTP `Last-Modified` (or `Date`) header carries the server's official timestamp.
- **Metadata Freshness Cascade** when the page was processed:
  1. **Tier 1 (High Confidence - HTML Meta Tags & Schema.org)**:
     - `<meta property="article:modified_time" content="...">`
     - `<meta name="dcterms.modified" content="...">`
     - `<meta name="dc.date.modified" content="...">`
     - `<meta property="article:published_time" content="...">`
     - `<meta name="dcterms.issued" content="...">`
     - Schema.org JSON-LD: `"dateModified": "..."` and `"datePublished": "..."`
     - `<time datetime="...">`
  2. **Tier 2 (Footer Text Patterns)**:
     - `Date modified: YYYY-MM-DD`, `Last modified: YYYY-MM-DD`, `Last updated: YYYY-MM-DD`, `Page details YYYY-MM-DD`.
  3. **Tier 3 (Date Pattern in the URL)**:
     - `/YYYY/MM/DD/` or `/YYYY/MM/`.
  4. **Tier 4 (XML Sitemap `lastmod`)**:
     - The `<lastmod>` field of `sitemap.xml`.
  5. **Tier 5 (HTTP Header)**:
     - `Last-Modified` from the HTTP header (RFC 2822 / 7231).
  6. **Tier 6 (Fallback)**:
     - Current crawl timestamp (`Utc::now()`). *(In Rust it was fixed at `0.0` / 1970!)*

### 3. How Chunks Were Formatted for the LLM to Apply the Recency Rule:
- In the original `search_tools.py`:
  ```text
  --- [Document Chunk 1 | Source: file.docx | Workspace: Default | Last Modified: 2026-10-10 12:00:00 | Type: Local Document] ---
  Path: ...
  Content: ...
  ```
- In the current Rust (`search_db`), the string was generated as:
  `• [{}] (Score: {:.2}, Source: {}, Type: {}):\n{}` — **with no date or time at all**!
  That is why the LLM could not apply the most-recent-information rule.

---

## 🏛️ Complete Solution Architecture for Release `v0.34.8`

```mermaid
flowchart TD
    subgraph PathHealing["1. Smart Path Healing"]
        A["Configured Folder (SQLite)"] --> B{"Path::exists()?"}
        B -- Yes --> C["Valid Path"]
        B -- No --> D["try_heal_path()"]
        D --> E["Remove/Add 'Documentos/'"]
        E --> F["Normalize Slashes & Drive Root"]
        F --> G{"Healed Path Exists?"}
        G -- Yes --> H["Auto-Update SQLite & Notify TUI"]
        G -- No --> I["Mark Source as Offline/Unreachable"]
    end

    subgraph TemporalExtraction["2. Native Temporal Extraction & Cascade"]
        J["Local File"] --> K["fs::Metadata: mtime + ctime -> RFC 3339"]
        L["Web Site"] --> M["HTTP HEAD / Conditional GET 304"]
        M --> N{"HTTP 304 Not Modified?"}
        N -- Yes --> O["✔ Up to date without downloading the body!"]
        N -- No --> P["Cascade: Meta Tags -> JSON-LD -> Footer -> URL -> Sitemap -> Header -> Utc::now()"]
    end

    subgraph RAGProvenance["3. RAG Provenance & Recency Rule"]
        K --> Q["LanceDB VectorRecord (last_modified)"]
        P --> Q
        Q --> R["HybridSearchResult (last_modified)"]
        R --> S["search_db Tool: Header with | Modified: YYYY-MM-DDTHH:MM:SSZ"]
        S --> T["LLM applies the Universal Temporal Recency Rule with absolute precision"]
    end

    subgraph ChangeDetection["4. Active Change Detection"]
        U["TUI Heartbeat / Workspace Switch / /sync"] --> V{"Folders and URLs valid?"}
        V -- No --> W["Header/Footer: ⚠️ Folder/URL unreachable"]
        V -- Yes --> X["Fast Diff: local mtime vs file_stat_cache & HTTP HEAD 304"]
        X -- Changes --> Y["Header/Footer: ⚡ Changes detected (run /sync)"]
        X -- Identical --> Z["Header/Footer: ✔ Up to date"]
        AA["/sync Execution"] --> AB["Purge moved/deleted files from LanceDB and BM25"]
    end
```

---

## 📋 Implementation Milestones (v0.34.8)

### Milestone 1: Smart Path Healing (`crates/any-context-core-rs/src/storage/path_healer.rs`)
1. Implement `try_heal_path(p: &Path) -> Option<PathBuf>`:
   - Tests variations: removing/adding `Documentos/`, `Documents/`, `OneDrive/`, normalizing `/` and `\` slashes.
   - Tests whether the corresponding subfolder exists on the volume.
2. Integrate into `NativeConfigDb::get_workspace_folders`:
   - Self-heals and updates the record in the `workspace_folders` table if the stored path is corrupted but the folder exists.
3. Integrate into the `/folder` and `/sync` commands:
   - Tells the user: `🔄 Auto-resolved folder path to 'G:/My Drive/Levix Digital/AnyContext'`.

### Milestone 2: Recovery of the Temporal Cascade and HTTP 304 for the Web (`crates/any-context-core-rs/src/ingestion/crawler.rs`)
1. **Support for HTTP Conditional GET / HEAD (RFC 7232)**:
   - When checking web sources, pass `If-Modified-Since` and `If-None-Match`.
   - If the server answers `304 Not Modified`, mark the page as unchanged instantly without downloading the HTML.
2. **Implementation of the Metadata Freshness Cascade (Native Rust)**:
   - Extracts the `<meta property="article:modified_time">`, `<meta name="dcterms.modified">`, `<meta name="dc.date.modified">` tags.
   - Extracts schema.org JSON-LD (`"dateModified": "..."`, `"datePublished": "..."`).
   - Extracts footer regexes (`Date modified:`, `Last modified:`, `Updated:`).
   - Extracts the URL regex (`/YYYY/MM/DD/`).
   - Extracts the HTTP `Last-Modified` header.
   - Falls back to `Utc::now()` (never `0.0`!).
3. **In the Orchestrator (`orchestrator.rs`)**:
   - Pass the real timestamp returned by the crawler to `embed_and_upsert_chunks`.

### Milestone 3: `last_modified` Propagation in the RAG Pipeline and Injection into the LLM Prompt
1. **BM25 (`crates/any-context-core-rs/src/retrieval/bm25.rs`)**:
   - Add `pub last_modified: Option<String>` to `DocRecord`.
2. **RAG Pipeline (`crates/any-context-core-rs/src/retrieval/pipeline.rs`)**:
   - Add `pub last_modified: Option<String>` to `HybridSearchResult`.
   - Propagate it from `d.last_modified` (LanceDB) and `doc.last_modified` (BM25).
   - Apply a soft recency boost in reranking to break ties in favor of newer versions.
3. **Header Injection in the `search_db` Tool (`crates/actx-cli/src/engine.rs`)**:
   - Format the header handed to the LLM with an explicit timestamp:
     ```rust
     let mod_str = match &r.last_modified {
         Some(lm) if !lm.is_empty() => format!(" | Modified: {}", lm),
         _ => String::new(),
     };
     out.push(format!(
         "• [{}] (Score: {:.2}, Source: {}, Type: {}{}):\n{}",
         r.file_name, r.score, r.file_path, type_str, mod_str, r.text
     ));
     ```

### Milestone 4: Active Change Detection, Source Health and Purge
1. **Source Health Diagnosis (`WorkspaceHealth`)**:
   - Checks whether every configured local folder and web portal is reachable.
   - If any source fails: Header/Footer shows `⚠️ [Sync: 1 source unreachable]`.
2. **Fast Local and Web Diff**:
   - Folders: compare `(mtime, size)` with `file_stat_cache`.
   - Web: lightweight `ETag` / `Last-Modified` check through HTTP HEAD / 304.
   - If changes are detected: shows `⚡ [Sync: Changes detected - run /sync]`.
3. **Purge Guarantee in `/sync`**:
   - Purges moved/deleted files from LanceDB (`delete_by_file`) and BM25 (`remove_by_file`).
   - Detailed log: `✔ Purged X moved/deleted file(s). Indexed Y new file(s). Workspace 100% synchronized.`

### Milestone 5: Unit Tests, E2E and Dual-Doc
1. Test of `try_heal_path` with real Google Drive paths and slash separators.
2. Test of the web metadata cascade and the HTTP 304 response.
3. Test of `last_modified` propagation in `search_db`.
4. Update of `TECDOC.md` (ADR-112) and `MANUAL_TESTS.md` (Scenario 27).
