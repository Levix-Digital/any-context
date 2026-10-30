---
title: Storage and Encryption
summary: LanceDB vector store, SQLite settings and session database and encryption at rest.
code: [crates/any-context-core-rs/src/storage, crates/any-context-core-rs/src/security]
---

# Storage and Encryption

LanceDB vector store, SQLite settings and session database and encryption at rest.

## Responsibilities

- Own the two local datastores: the LanceDB vector database (chunks and embeddings) and the SQLite settings database (`settings.db`), which is the single source of truth for workspaces, settings, credentials, sync state and conversation memory.
- Resolve the canonical data, log and model directories per operating system, with overrides for tests and custom installs.
- Make the synchronous storage API safe to call from inside and outside Tokio runtimes.
- Encrypt chunk text at rest with the hardware-bound key, migrate rows stored in plaintext by older versions, and decrypt on read.

The BM25 index file lives in the LanceDB directory but its format belongs to [retrieval](retrieval.md).

## Structure

| Module | Role |
|---|---|
| `storage/lancedb.rs` | `NativeLanceStore`, `VectorRecord`, `ScoredVectorResult`, `get_default_lancedb_path`. |
| `storage/sqlite.rs` | `NativeConfigDb` and record types, `get_default_settings_db_path`, `get_default_logs_dir`, `get_default_models_dir`. |
| `security/mod.rs` | `NativeSecurityEngine`: AES-GCM-256 with a PBKDF2-derived machine key. |

Conversation memory is stored in the same `settings.db` by `SqliteSessionStore` in `crates/actx-agent` (see [Agent Runtime](agent.md)).

## Behavior

### Locations

| Data | Windows | Linux | Override |
|---|---|---|---|
| LanceDB | `%LOCALAPPDATA%\AnyContext\data\context_db\lancedb` | `$XDG_DATA_HOME/any-context/...` or `~/.local/share/any-context/data/context_db/lancedb` | `ACTX_CONTEXT_DB` |
| Settings database | `%LOCALAPPDATA%\AnyContext\config\settings.db` | `~/.local/share/any-context/config/settings.db` | `ACTX_SETTINGS_DB` |
| Logs | `%LOCALAPPDATA%\AnyContext\logs` | `~/.local/share/any-context/logs` | none |
| Models | `%LOCALAPPDATA%\AnyContext\models` | `~/.local/share/any-context/models` | `ACTX_MODELS_DIR` |

On macOS the LanceDB default is `~/Library/Application Support/AnyContext/data/context_db/lancedb`. The settings path resolver also accepts an existing database under the platform data directory or `~/.anycontext/config/` before falling back to the default. When `ACTX_TEST_MODE` is set, every location moves to an `actx_test_sandbox` folder in the system temporary directory, so tests never touch user data.

### LanceDB vector store

- One table, `workspace_chunks` (`DEFAULT_TABLE_NAME`), with 1536-dimension vectors (`DEFAULT_VECTOR_DIM`) and the columns `id`, `vector`, `text`, `file_name`, `file_path`, `workspace`, `last_modified`, `content_type`, `document_summary`, `keywords`, `content_hash`, `header_path`, `start_line`, `end_line`, `embedding_target`. `document_summary` and `keywords` come from the document envelope; `header_path` is the chunk's section, symbol, page, sheet or row locator; `start_line`/`end_line` (UInt32) are file line numbers, set only for chunkers that emit them (see [ingestion](ingestion.md)); `embedding_target` is the key of the vector space that produced the row's vector (`provider/model@dim[+pN]`, ADR-0154).
- The provenance columns (`header_path`, `start_line`, `end_line`, `embedding_target`) are nullable and were added after the first schema. `ensure_provenance_columns` adds the missing ones to an older table as all-null columns (a schema-only commit, nothing rewritten or re-embedded); it runs at the start of every sync, on every write (`get_or_create_table`) and before `encrypt_plaintext_rows`. Old rows keep null provenance until their file is re-indexed. Reads use the columns only when present, and versions without them keep reading and appending (their rows get nulls).
- Every workspace shares the table and is separated by the `workspace` column; the virtual `Global` workspace holds the system documentation and is included in every workspace filter.
- `upsert_records` appends a record batch (`table.add`) and encrypts `text`, `document_summary`, `keywords` and `header_path` first (see Encryption at rest); callers delete the old chunks of a file before re-adding it. A vector shorter than the table dimension (Ollama `nomic-embed-text`, 768) is zero-padded, and `search_vector_async` pads a shorter query vector the same way, so scores equal those of the short space; a longer vector is rejected on write and on search instead of being truncated (BUG-021). Dense search filters on `embedding_target_filter(key)` (`embedding_target = key OR embedding_target IS NULL`, the null for rows written before the column), so vectors of two models never meet in one search even though they share the table. Per-target tables are planned in IMP-033.
- `encrypt_plaintext_rows` encrypts the rows older versions stored in plaintext and removes the old dataset files that held them (see Encryption at rest).
- `delete_by_file` deletes a path and everything under it (folder prefix); `delete_by_exact_path` deletes one exact path, which web pages use because one URL can be a prefix of another.
- `search_vector_async` (similarity search with an optional workspace filter and extra filter expression), `search_metadata_async` (filter-only queries such as file name or date clauses), `count_records`, `delete_by_workspace`, `delete_by_file` and `delete_by_id`.
- The store owns an internal Tokio runtime. Synchronous methods use `safe_block_on`, which runs the future on a scoped thread when the caller is already inside a runtime. `Drop` destroys the internal runtime on a separate thread in that case, so a store can be dropped from async code without a panic.

### SQLite settings database

`NativeConfigDb::open` creates the parent directory, sets a 30 s busy timeout and WAL pragmas, creates missing tables, adds missing columns to older databases, ensures the `Default` workspace exists and migrates legacy `paths_json` folder lists into `workspace_folders`. The connection is guarded by a mutex.

| Table | Content |
|---|---|
| `workspaces` | Name, description and per-workspace settings: `grounding_mode` (default `strict`), `search_mode` (default `auto`), `model` (default `gpt-4o-mini`), `web_search_enabled`, `embedding_target` (`provider/model` that built the workspace's vectors, NULL until the first completed sync; see [ingestion](ingestion.md)). |
| `workspace_folders`, `workspace_web_urls` | Monitored folders and web sources per workspace. |
| `workspace_sources` | Display names for sources (`display_name`, `canonical_slug`), resolved without touching LanceDB. |
| `workspace_sync_status` | Live sync telemetry: syncing flag, worker PID, current/total items, stage, item name, progress bar, error. |
| `workspace_files_stat_cache` | Modification time, size and hash of every indexed file, used for differential sync. |
| `workspace_sync_ledger` | Files deleted, added and modified by the last sync. |
| `semantic_envelopes` | Cached document summaries and keywords by content hash. |
| `file_metadata` | File hashes and status per workspace. |
| `app_settings` | Key-value settings: `active_workspace`, `active_model`, `default_model`, `default_provider`, `active_profile`, `rerank_mode`, `reranker_model`, `enable_vision_llm`, `onboarding_completed`, `global_knowledge_version`. |
| `api_keys` | Provider API keys saved with `/keys` (`provider`, `api_key`). |

`get_api_key` reads the provider's environment variable first (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`, `GROQ_API_KEY`, `OPENROUTER_API_KEY`, `MISTRAL_API_KEY`) and then the `api_keys` table. `set_api_key` upserts the key by provider; `/keys` also exports it to the process environment and reports when it could not be saved.

Settings are strictly per workspace: there is no global fallback for workspace settings.

### Workspace folders

`NativeConfigDb::get_workspace_folders` returns the saved folder paths as the user registered them; a path that no longer exists is never rewritten or redirected ([ADR-0150](../adr/0150-offline-sources-keep-their-index-and-folders-are-never-redir.md)). The scanner reports it offline and its index is kept.

### Encryption at rest

`NativeSecurityEngine` derives a 256-bit AES-GCM key with PBKDF2-HMAC-SHA256 (100,000 rounds) from a machine identifier (`ACTX_MACHINE_ID`, or the Windows MachineGuid, Linux `/etc/machine-id` or macOS IOPlatformUUID, falling back to the host name) combined with a fixed domain salt and pepper. Encrypted values are base64 strings with the `enc::` prefix; `decrypt_text` returns plain values unchanged. The parameters match the earlier Python implementation, so data encrypted by older versions stays readable on the same machine.

Every LanceDB write encrypts `text`, `document_summary`, `keywords` and `header_path` (`upsert_records`), and every read decrypts them (`extract_scored_results`), so callers always see plaintext. Ids, vectors, names, paths, workspace, dates, content type, content hash and line numbers stay in clear text for filtering; SQL filters on the encrypted columns therefore never match, and `/inspect <filter>` matches decrypted text in memory instead. The data is bound to the machine identifier, and the BM25 index file (see [retrieval](retrieval.md)) still keeps chunk text in plaintext for lexical search.

Rows written before every write was encrypted are migrated automatically by `encrypt_plaintext_rows`, which runs at startup (TUI background task and CLI startup) and at the start of every sync:

1. Add the provenance columns if the table lacks them, then count the rows where `text`, `document_summary`, `keywords` or `header_path` is non-empty and does not start with `enc::`; with none, stop (the steady state costs one count).
2. Read them with their vectors in batches of 1,000 and rewrite them encrypted with `merge_insert` keyed on `(id, workspace, file_path)`, update only (chunk ids repeat across paths and workspaces). Nothing is re-embedded. A batch that encrypts nothing stops with an error.
3. Compact the table with `materialize_deletions_threshold = 0.0` and prune every version except the current one (`delete_unverified = false`): updated rows only get deletion markers and older versions keep the old files, so this is what removes the plaintext from disk.

Pruning drops Lance time travel to earlier versions. Regular deletes and forced re-indexes do not compact or prune, so their old versions stay on disk until the next migration.

## Interfaces

- **Used by**: [ingestion](ingestion.md) (writes chunks, stat cache, telemetry), [retrieval](retrieval.md) (vector and metadata search, decryption), the [command engine](commands.md) (workspace, source and settings management), the [CLI](cli.md) and [TUI](tui.md) (settings, telemetry polling), and the [agent](agent.md) (session memory in the same database).
- **Exports**: `NativeLanceStore`, `NativeConfigDb`, `VectorRecord`, `NativeSecurityEngine` and the path helpers from `any_context_core_rs::storage`.
- **Environment**: `ACTX_CONTEXT_DB`, `ACTX_SETTINGS_DB`, `ACTX_MODELS_DIR`, `ACTX_TEST_MODE`, `ACTX_MACHINE_ID`.

## Decisions

- [ADR-0078](../adr/0078-relational-workspace-folders.md): `workspace_folders` is the only source of folder paths; legacy `paths_json` is migrated once.
- [ADR-0082](../adr/0082-sqlite-session-memory-identity-injection.md): per-workspace session memory in SQLite.
- [ADR-0086](../adr/0086-non-blocking-lancedb-safe-runtime-drop.md): `safe_block_on` and safe runtime destruction.
- [ADR-0088](../adr/0088-conversational-session-boundary.md): only user prompts and final answers are persisted.
- [ADR-0090](../adr/0090-grounding-parity-streaming-single-lancedb-path.md): single canonical LanceDB directory; grounding mode per workspace.
- [ADR-0091](../adr/0091-hermetic-workspace-isolation-atomic-exe-swap.md): relational workspace columns, migrations, no global fallbacks.
- [ADR-0094](../adr/0094-test-sandbox-option-hinting-global-self-knowledge.md): `ACTX_TEST_MODE` sandbox and the `Global` workspace.
- [ADR-0097](../adr/0097-sqlite-sync-telemetry-agent-background-awareness.md): `workspace_sync_status` telemetry table.
- [ADR-0098](../adr/0098-lancestore-count-records-parameter-fix.md): `count_records` parameter order.
- [ADR-0100](../adr/0100-native-encryption-sync-cancel-tui-isolation.md): native Rust security engine compatible with the Python parameters.
- [ADR-0112](../adr/0112-ux-stabilization-deep-search-multiline-prompt.md): `search_mode` column and `global_knowledge_version` setting.
- [ADR-0116](../adr/0116-path-healing-temporal-metadata-change-detection.md): path healing, removed by ADR-0150.
- [ADR-0120](../adr/0120-source-display-names-shared-sources-removal.md): `workspace_sources` display names.
- [ADR-0131](../adr/0131-field-level-encryption-on-every-lancedb-write.md): field-level encryption on every LanceDB write.
- [ADR-0135](../adr/0135-automatic-encryption-of-lancedb-rows-stored-in-plaintext.md): automatic encryption of rows stored in plaintext, with compaction and pruning.
- [ADR-0138](../adr/0138-record-the-embedding-target-per-workspace-and-reindex-on-change.md): `workspaces.embedding_target` column.
- [ADR-0150](../adr/0150-offline-sources-keep-their-index-and-folders-are-never-redir.md): saved folder paths are never redirected; path healing removed.
- [ADR-0142](../adr/0142-chunk-provenance-columns-in-lancedb-added-in-place.md): nullable `header_path` (encrypted), `start_line` and `end_line` columns, added in place without reindex.
