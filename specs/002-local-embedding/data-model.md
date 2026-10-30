# Data Model: Local multilingual embedding model (spec 002)

## EmbeddingModelSpec (catalog, static)

| Field | Type | Notes |
|---|---|---|
| `id` | string | Catalog id, e.g. `multilingual-e5-small`, `gemini-embedding-2` |
| `name` | string | Display name |
| `runtime` | `Local` \| `Cloud { provider }` | `provider` is `openai`, `gemini` or `ollama` |
| `license` | string | `MIT` for the three local candidates |
| `dimension` | u32 | 384, 768, 1024, 1536... |
| `max_tokens` | u32 | Input truncation limit (512 for the local candidates) |
| `pooling` | `Mean` \| `Cls` | Local models only |
| `normalize` | bool | L2 normalization (true for all local candidates) |
| `query_prefix` | string | Empty when none |
| `document_prefix` | string | Empty when none |
| `prefix_rev` | u32 | Bumped whenever a prefix changes; 0 = no prefixes |
| `files` | `[ModelFile]` | Local models only; pinned URL, size, SHA-256 (existing type) |

Validation: local entries have files, pooling and a non-zero dimension; ids are unique; every local profile's default exists and is `Local`.

## EmbeddingTarget (vector-space identity)

| Field | Type | Notes |
|---|---|---|
| `provider` | string | `local`, `openai`, `gemini`, `ollama`; `mock` only in tests |
| `model` | string | Model id |
| `dimension` | u32 | |
| `prefix_rev` | u32 | |

Key: `provider/model@dimension` plus `+p<prefix_rev>` when `prefix_rev > 0`. Legacy keys `provider/model` (before this feature) parse into a target with the dimension the old resolver produced (1536 for OpenAI and Gemini, 768 for `nomic-embed-text`) and `prefix_rev = 0`; they never equal a new key, except where adopted by the upgrade (see Migration).

## Settings (`settings.db`, `settings` table)

| Key | Value | Notes |
|---|---|---|
| `embedding_model` | catalog id | Absent = active profile default; set by `/embedding` or by the upgrade adoption |
| `embedding_migration_v1` | timestamp | Marks the one-time upgrade as done |

Existing per-workspace record (`workspaces.embedding_target`, ADR-0138) is kept and now holds new-format keys after each sync.

## Chunk (LanceDB `workspace_chunks`)

New nullable column `embedding_target` (Utf8). Written on every upsert with the key of the target that produced the vector; `NULL` only for rows written before the upgrade until the backfill runs, and for `Global` placeholders. Dense search filters on it.

## State transitions

```text
setting absent ──(profile default resolved)──▶ local model in use
setting = X ──(/embedding Y, confirmed)──▶ setting = Y; workspaces recorded with X show "reindex pending"
workspace recorded X, setting Y ──(next sync)──▶ purge + re-embed with Y; record Y
model unavailable ──(sync)──▶ sync stops before embedding (EmbeddingUnavailable); index unchanged
```

## Migration (one time, on open)

1. If `embedding_migration_v1` is set: skip.
2. Collect the recorded workspace targets; ignore `mock/*` and empty.
3. If any remain: set `embedding_model` to the most frequent one (ties: most recently synced workspace) and rewrite those records to the new key format (same space, no reindex).
4. Backfill `embedding_target` on chunks from their workspace record (rows of mock workspaces stay `NULL` and are reindexed on next sync).
5. Set `embedding_migration_v1`.
