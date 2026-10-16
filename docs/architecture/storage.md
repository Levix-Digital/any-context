---
title: Storage and Encryption
summary: LanceDB vector store, SQLite settings and session database, path healing and encryption at rest.
code: [crates/any-context-core-rs/src/storage, crates/any-context-core-rs/src/security]
---

# Storage and Encryption

LanceDB vector store, SQLite settings and session database, path healing and encryption at rest.

## Responsibilities

- Own the two local datastores: the LanceDB vector database (chunks and embeddings) and the SQLite settings database (`settings.db`), which is the single source of truth for workspaces, settings, credentials, sync state and conversation memory.
- Resolve the canonical data, log and model directories per operating system, with overrides for tests and custom installs.
- Make the synchronous storage API safe to call from inside and outside Tokio runtimes.
- Heal folder paths that moved slightly (path healing).
- Decrypt chunk text encrypted at rest with the hardware-bound key.

The BM25 index file lives in the LanceDB directory but its format belongs to [retrieval](retrieval.md).

## Structure

| Module | Role |
|---|---|
| `storage/lancedb.rs` | `NativeLanceStore`, `VectorRecord`, `ScoredVectorResult`, `get_default_lancedb_path`. |
| `storage/sqlite.rs` | `NativeConfigDb` and record types, `get_default_settings_db_path`, `get_default_logs_dir`, `get_default_models_dir`. |
| `storage/path_healer.rs` | `try_heal_path`. |
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

- One table, `workspace_chunks` (`DEFAULT_TABLE_NAME`), with 1536-dimension vectors (`DEFAULT_VECTOR_DIM`) and the columns `id`, `vector`, `text`, `file_name`, `file_path`, `workspace`, `last_modified`, `content_type`, `document_summary`, `keywords`, `content_hash`.
- Every workspace shares the table and is separated by the `workspace` column; the virtual `Global` workspace holds the system documentation and is included in every workspace filter.
- `upsert_records` appends a record batch (`table.add`); callers delete the old chunks of a file before re-adding it.
- `search_vector_async` (similarity search with an optional workspace filter and extra filter expression), `search_metadata_async` (filter-only queries such as file name or date clauses), `count_records`, `delete_by_workspace`, `delete_by_file` and `delete_by_id`.
- The store owns an internal Tokio runtime. Synchronous methods use `safe_block_on`, which runs the future on a scoped thread when the caller is already inside a runtime. `Drop` destroys the internal runtime on a separate thread in that case, so a store can be dropped from async code without a panic.

### SQLite settings database

`NativeConfigDb::open` creates the parent directory, sets a 30 s busy timeout and WAL pragmas, creates missing tables, adds missing columns to older databases, ensures the `Default` workspace exists and migrates legacy `paths_json` folder lists into `workspace_folders`. The connection is guarded by a mutex.

| Table | Content |
|---|---|
| `workspaces` | Name, description and per-workspace settings: `grounding_mode` (default `strict`), `search_mode` (default `auto`), `model` (default `gpt-4o-mini`), `web_search_enabled`. |
| `workspace_folders`, `workspace_web_urls` | Monitored folders and web sources per workspace. |
| `workspace_sources` | Display names for sources (`display_name`, `canonical_slug`), resolved without touching LanceDB. |
| `workspace_sync_status` | Live sync telemetry: syncing flag, worker PID, current/total items, stage, item name, progress bar, error. |
| `workspace_files_stat_cache` | Modification time, size and hash of every indexed file, used for differential sync. |
| `workspace_sync_ledger` | Files deleted, added and modified by the last sync. |
| `semantic_envelopes` | Cached document summaries and keywords by content hash. |
| `file_metadata` | File hashes and status per workspace. |
| `app_settings` | Key-value settings: `active_workspace`, `active_model`, `default_model`, `default_provider`, `active_profile`, `rerank_mode`, `reranker_model`, `enable_vision_llm`, `onboarding_completed`, `global_knowledge_version`. |
| `api_keys` | Provider API keys saved with `/keys` (`provider`, `api_key`). |

`get_api_key` reads the provider's environment variable first (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`, `GROQ_API_KEY`, `OPENROUTER_API_KEY`, `MISTRAL_API_KEY`) and then the `api_keys` table when it exists. `ensure_tables` does not create `api_keys`, so on a database created from scratch `set_api_key` fails silently and a key saved with `/keys` only lives in the process environment until the application exits; databases that already have the table (created by older versions) persist keys normally.

Settings are strictly per workspace: there is no global fallback for workspace settings.

### Path healing

`try_heal_path` tries, in order: normalizing separators; removing a redundant `Documentos/` or `Documents/`; adding `Documentos/` under `My Drive`; removing `OneDrive/`; and searching for the leaf folder in the parent or the drive root. `NativeConfigDb::get_workspace_folders` stores a healed path back in `workspace_folders`, and the scanner uses healing before reporting a folder as missing.

### Encryption at rest

`NativeSecurityEngine` derives a 256-bit AES-GCM key with PBKDF2-HMAC-SHA256 (100,000 rounds) from a machine identifier (`ACTX_MACHINE_ID`, or the Windows MachineGuid, Linux `/etc/machine-id` or macOS IOPlatformUUID, falling back to the host name) combined with a fixed domain salt and pepper. Encrypted values are base64 strings with the `enc::` prefix; `decrypt_text` returns plain values unchanged. The parameters match the earlier Python implementation, so data encrypted by older versions stays readable on the same machine. Today the retrieval pipeline decrypts `enc::` texts when it reads them; the ingestion pipeline writes chunk text unencrypted.

## Interfaces

- **Used by**: [ingestion](ingestion.md) (writes chunks, stat cache, telemetry), [retrieval](retrieval.md) (vector and metadata search, decryption), the [command engine](commands.md) (workspace, source and settings management), the [CLI](cli.md) and [TUI](tui.md) (settings, telemetry polling), and the [agent](agent.md) (session memory in the same database).
- **Exports**: `NativeLanceStore`, `NativeConfigDb`, `VectorRecord`, `try_heal_path`, `NativeSecurityEngine` and the path helpers from `any_context_core_rs::storage`.
- **Environment**: `ACTX_CONTEXT_DB`, `ACTX_SETTINGS_DB`, `ACTX_MODELS_DIR`, `ACTX_TEST_MODE`, `ACTX_MACHINE_ID`.

## Decisions

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
- [ADR-0116](../adr/0116-path-healing-temporal-metadata-change-detection.md): path healing.
- [ADR-0120](../adr/0120-source-display-names-shared-sources-removal.md): `workspace_sources` display names.
