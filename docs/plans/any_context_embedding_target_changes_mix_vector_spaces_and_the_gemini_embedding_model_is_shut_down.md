---
plan_id: any_context_embedding_target_changes_mix_vector_spaces_and_the_gemini_embedding_model_is_shut_down
project: any-context
title: Embedding target changes mix vector spaces and the Gemini embedding model is shut down
kind: bug
version: v0.45.1
status: in_progress
author: Levi Silveira
created_at: '2026-10-17T11:36:04'
updated_at: '2026-10-17T12:13:35'
session_id: null
tags:
- embeddings
- sync
- gemini
- storage
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-003
- BUG-013
milestones:
- name: M1 Evidence
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M2 Target and record
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M3 Reindex rule and sync
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M4 Query side and Gemini adapter
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M5 Documentation and manual tests
  done: true
  done_at: '2026-10-17'
  by: Levi Silveira
- name: M6 Review and pull request
  done: false
status_log:
- at: '2026-10-17T11:36:04'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-17T11:38:12'
  by: Levi Silveira
  status: planned
  note: Plan updated
- at: '2026-10-17T11:44:08'
  by: Levi Silveira
  status: active
  note: Approved by the user on 2026-10-17
- at: '2026-10-17T11:47:46'
  by: Levi Silveira
  note: 'Milestone ''M1 Evidence'' completed: H1 confirmed from docs, H2 refuted, IMP-014 captured'
- at: '2026-10-17T12:12:40'
  by: Levi Silveira
  note: Milestone 'M2 Target and record' completed
- at: '2026-10-17T12:13:30'
  by: Levi Silveira
  note: Milestone 'M3 Reindex rule and sync' completed
- at: '2026-10-17T12:13:33'
  by: Levi Silveira
  note: Milestone 'M4 Query side and Gemini adapter' completed
- at: '2026-10-17T12:13:35'
  by: Levi Silveira
  note: Milestone 'M5 Documentation and manual tests' completed
---

# Embedding target changes mix vector spaces and the Gemini embedding model is shut down

Bug plan for BUG-003 and BUG-013 (`dev-cycle-protocol` full path, bug plan: it adds a SQLite column and changes sync behavior). Scope agreed with the user on 2026-10-17:

- BUG-003 and BUG-013 together, because both need the same mechanism: know which embedding target built a workspace's vectors and reindex when it changes.
- When the target changes, the next sync **reindexes the whole workspace automatically** (no confirmation step, no `--force` needed), with a clear message.
- The mock fallback (no API key) is an embedding target of its own (`mock`), so adding a real key later triggers a reindex.
- Out of scope, left for Frente C: a separate embedding setting and outdated status (FEAT-005), a vector dimension per table with a LanceDB migration (IMP-004), and reporting query embedding failures (IMP-003). Found during discovery and captured as **BUG-014**: a failed embedding call stores pseudo-vectors next to real ones.

## Backlog items

### BUG-003: Changing a workspace chat model mixes embedding spaces on the next sync
- **Severity**: medium
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Description**: resolve_embedding_provider (ingestion/orchestrator.rs) derives the embedding model from the workspace chat model. After /model changes it, the next incremental sync embeds new or changed files with another provider/model while unchanged files keep their old vectors, and queries use the new model, so part of the workspace becomes unreachable by dense search. Needs the per-workspace embedding model record planned for Frente C, or a forced reindex when the embedding target changes.

### BUG-013: Gemini embedding model text-embedding-004 is shut down
- **Severity**: high
- **Found**: 2026-10-17, 2026-10-17, BUG-002 discovery
- **Description**: Google shut down text-embedding-004 on 2026-01-14, so ingestion and query embedding for Gemini workspaces fail, and the dense search is skipped without a message (IMP-003). The successor is gemini-embedding-2. Changing the embedding model needs a reindex of the affected workspaces; plan it with BUG-003, FEAT-005 and IMP-004.

## 1. Reproduction

Environment: v0.45.0 (`dev` at `b759ab2`), any OS, an OpenAI key and a Gemini key stored with `/keys`.

**BUG-003**
1. Create a workspace, add a folder with a few documents and sync (`/model gpt-4o-mini`, so the resolver picks OpenAI `text-embedding-3-small`).
2. `/model gemini-2.5-flash`.
3. Add one new document to the folder and sync again.
4. Ask a question whose answer is only in one of the **original** documents.
5. Expected: the dense search finds it. Actual: the query is embedded with the Gemini embedding model, while the original chunks still hold OpenAI vectors. The ranking of those chunks by dense distance is meaningless, and only BM25 can surface them. The new document holds vectors from a third state (see BUG-013), so the table now mixes spaces.

The same mix happens with no key at sync time (mock pseudo-vectors, reported as `text-embedding-3-small`) followed by `/keys openai <key>`, and with any change to `ACTX_EMBEDDING_MODEL` or `ACTX_MODEL`.

**BUG-013**
1. `/model gemini-2.5-flash` with a Gemini key, then sync a folder.
2. Every `batchEmbedContents` call to `models/text-embedding-004` fails (model shut down on 2026-01-14), and `embed_and_upsert` silently stores SHA-256 pseudo-vectors (`orchestrator.rs:597-605`).
3. Each query embedding fails too, and the dense search is skipped without a message (`retrieval/pipeline.rs`, IMP-003).

Logs: the sync and query paths discard the provider error (`Err(_) =>` in both), so no log line carries the API error today. Step 1 of section 8 calls the endpoint directly and quotes the real error response in the pull request.

## 2. Evidence and root cause

### Confirmed facts

- **One resolver, no record.** `resolve_embedding_provider(db, model_override, workspace)` (`crates/any-context-core-rs/src/ingestion/orchestrator.rs:783`) picks a model in this order: override, `ACTX_EMBEDDING_MODEL`, `ACTX_MODEL`, the workspace model (`get_workspace_model`, the **chat** model), the default model, then `text-embedding-3-small`. The provider comes from the model-name prefix: `mock*` → mock; `gemini*` with a key → Gemini `text-embedding-004`; `ollama*`/`local*` → Ollama `nomic-embed-text`; otherwise OpenAI `text-embedding-3-small` with a key, otherwise **mock while still reporting `text-embedding-3-small`**. The sync (`orchestrator.rs:214`) and the queries (`crates/actx-cli/src/engine.rs:141`, through `set_embedding`, ADR-0132) both call it. They agree at any given moment, but nothing stores which target built the vectors that are already in LanceDB.
- **Incremental sync re-embeds only the diff.** Steps 6 to 11 of `NativeSyncOrchestrator::sync` re-embed only new, modified and renamed files, so a target change leaves the unchanged files in the old space.
- **`/model`, `/preset` and onboarding change the chat model** (`commands/engine.rs:1349,1450,1476` → `set_workspace_model`), which silently changes the embedding target of the next sync and of every query.
- **Fixed dimension.** Every table is written at `DEFAULT_VECTOR_DIM = 1536` (`storage/lancedb.rs`), and `build_record_batch` pads shorter vectors with zeros (`lancedb.rs:357-362`). Vector search uses the LanceDB default distance (L2; no `distance_type` is set).
- **Gemini successor.** The Gemini API models page lists the stable id `gemini-embedding-2`, with an output size from 128 to 3072 and recommended sizes 768, 1536 and 3072. The output size is set with `EmbedContentConfig.output_dimensionality` (the top-level field is deprecated), and smaller sizes are truncations of the full vector. Source: ai.google.dev/gemini-api/docs/models/gemini-embedding-2 and ai.google.dev/api/embeddings, read on 2026-10-17.
- `GeminiProvider::embed` (`crates/actx-lm/src/providers/gemini.rs:333`) sends `batchEmbedContents` with only `model` and `content`, so it would get the model's default size (3072), which doesn't fit the 1536 table.

### Hypotheses to confirm before the fix is final

- **H1 (confirmed from the documentation, 2026-10-17)**: the embeddings guide's REST example for `gemini-embedding-2` sets `output_dimensionality` next to `content` in the request body, and its `batchEmbedContents` example sends `requests[]` with `model` and `content` per item. The adapter therefore sends `outputDimensionality: 1536` in each request item. Calling the real API needs a key, and the agent doesn't take one from `settings.db`, so the live call is MT-SYNC-007 on the release candidate.
- **H2 (refuted, 2026-10-17)**: the embeddings guide says gemini-embedding-2 "auto-normalizes truncated dimensions (e.g., 768, 1536)" (manual normalization is only for `gemini-embedding-001`). The adapter doesn't normalize.
- **Also found**: gemini-embedding-2 rejects `task_type` and expects text prefixes for asymmetric retrieval. The fix sends raw text, as today. Adding the prefixes is a quality change and is captured as **IMP-014**.

## 3. Fix design (`software-design`)

Hexagonal style: the embedding target and the reindex rule are domain knowledge in the core (`ingestion`); the record is a storage concern (`storage/sqlite.rs`); the Gemini wire format stays in the `actx-lm` adapter. No new crate, no new dependency.

### 3.1 `EmbeddingTarget` value

In `crates/any-context-core-rs/src/ingestion/orchestrator.rs` (or a small `ingestion/embedding.rs` if the orchestrator grows too much):

```rust
/// The provider and model that produced a workspace's vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTarget {
    pub provider: String, // "openai", "gemini", "ollama", "mock"
    pub model: String,    // the id sent to the provider ("mock-embed" for mock)
}

impl EmbeddingTarget {
    /// Stable text form stored in SQLite, e.g. "openai/text-embedding-3-small".
    pub fn key(&self) -> String;
    pub fn parse(key: &str) -> Option<Self>;
}
```

`resolve_embedding_provider` returns `(Option<Arc<dyn LmProvider>>, EmbeddingTarget)` instead of `(…, String)`. Its order of precedence doesn't change (the separate embedding setting is FEAT-005). Two corrections:

- Gemini resolves to `gemini/gemini-embedding-2` (BUG-013).
- The no-key fallback resolves to `mock/mock-embed`, the same target as an explicit `mock*` model, instead of reporting `text-embedding-3-small`.

The callers that only need the model id (`set_embedding`, `embed_and_upsert`) read `target.model`.

### 3.2 Record per workspace (storage)

An additive column on `workspaces`: `embedding_target TEXT` (NULL by default), added with the same `check_column` + `ALTER TABLE` pattern as the other columns (`sqlite.rs:259-293`). It's also added to the `CREATE TABLE` statement for new databases. Two methods on `NativeConfigDb`:

```rust
pub fn get_workspace_embedding_target(&self, workspace: &str) -> Result<Option<String>>;
pub fn set_workspace_embedding_target(&self, workspace: &str, target: Option<&str>) -> Result<()>;
```

A record per workspace, not a global setting, because each workspace has its own chat model today and keeps its own vectors in the shared table.

### 3.3 Reindex rule in the sync

A pure function, unit-tested without I/O:

```rust
pub enum ReindexDecision {
    Keep,                                                        // recorded == current
    Adopt,                                                       // nothing indexed yet: record current
    Reindex { from: String, to: EmbeddingTarget },               // purge and re-embed everything
}

pub fn decide_reindex(recorded: Option<&str>, legacy: Option<EmbeddingTarget>,
                      current: &EmbeddingTarget, has_indexed_data: bool) -> ReindexDecision;
```

- **Recorded and equal** → `Keep`.
- **Recorded and different** → `Reindex`.
- **No record, and the workspace has no indexed files** (empty stat cache) → `Adopt`.
- **No record, but indexed files exist (a workspace from before this fix)**: the recorded target is the one the pre-fix resolver would give for the same inputs (`legacy_target`). For the same inputs that's identical to `current`, except that Gemini was `gemini/text-embedding-004` and the no-key fallback was mock. If `legacy == current` → `Adopt` (no needless reindex for OpenAI or Ollama users). Otherwise → `Reindex`. This is how existing Gemini workspaces, whose vectors are text-embedding-004 or pseudo-vectors, get repaired by their first sync after the upgrade.

In `NativeSyncOrchestrator::sync`, right after step 4:

- `Reindex` turns the pass into a **full workspace** force pass (purge vectors, BM25 and the stat cache of the whole workspace, as `options.force` does today). It does that even when the sync was scoped to one folder (`target_folder`), because a scoped pass can't make the workspace consistent. Verbose output and `SyncResult` say so.
- The record is written (`set_workspace_embedding_target`) only **after** the pass completes without cancellation. A cancelled or failed reindex leaves the old record, so the next sync tries again. Each pass purges before embedding, so a half-finished reindex never survives as a mix that looks finished.
- `SyncResult` gets `embedding_reindex: Option<String>`. The message ("Embedding model changed (A → B): the whole workspace is reindexed.") is written to the sync status detail and the verbose sync output. The sync runs in a background worker, so the user-facing warning comes in advance: `pending_reindex_notice(db, workspace)` lets `/model` and `/sync` add "Embedding model changes (A → B): the next sync reindexes the whole workspace." (implementation note: chosen over a TUI status-line change, which only shows the progress bar).

### 3.4 Queries between the change and the next sync

The CLI's `resolve_embedding_provider` call for queries (`crates/actx-cli/src/engine.rs:141`) prefers the **recorded** target when its provider can be built (key present; mock and Ollama always can), and falls back to the current resolution otherwise. A `/model` change therefore doesn't break dense search before the reindex runs. One exception: a recorded `gemini/text-embedding-004` can't serve queries anymore, so it falls back to the current target (the next sync reindexes anyway). The core exposes this as `resolve_query_embedding_provider(db, workspace)`, so the CLI stays an adapter.

### 3.5 Gemini adapter (BUG-013)

`GeminiProvider::embed` sends the output size per request item in the shape confirmed by H1, set from a constant `GEMINI_EMBEDDING_DIM = 1536` (equal to `DEFAULT_VECTOR_DIM`, asserted in a test). The adapter owns the size because it's part of the wire request. The core only passes the model id.

### 3.6 Deliberately not done

- No new user setting, `/embed` command or outdated badge (FEAT-005).
- No per-table dimension or LanceDB schema change. Ollama `nomic-embed-text` (768) keeps being zero-padded, as today (IMP-004).
- No change to the per-call fallback that stores pseudo-vectors when an embedding call fails (BUG-014).
- No confirmation prompt before the automatic reindex (the user's decision on 2026-10-17).

### 3.7 ADR

ADR-0138 "Record the embedding target per workspace and reindex on change": options were (a) record + automatic reindex (chosen), (b) record + block until `--force`, (c) pin the recorded model. It also records the legacy inference rule and the 1536 output size for `gemini-embedding-2`. It supplements ADR-0132.

## 4. Affected surfaces and files

| File | Change |
|---|---|
| `crates/any-context-core-rs/src/ingestion/orchestrator.rs` (and `ingestion/mod.rs` exports) | `EmbeddingTarget`, resolver return type, Gemini and mock targets, `decide_reindex`, `legacy_target`, `resolve_query_embedding_provider`, sync step 4b, record after success, `SyncResult.embedding_reindex` |
| `crates/any-context-core-rs/src/storage/sqlite.rs` | `embedding_target` column (create + additive migration), get/set methods |
| `crates/actx-lm/src/providers/gemini.rs` | output size per request, `GEMINI_EMBEDDING_DIM` |
| `crates/actx-cli/src/engine.rs` | query embedding through `resolve_query_embedding_provider` |
| `crates/any-context-core-rs/src/commands/engine.rs` | `/model` and `/sync` show `pending_reindex_notice` (`/preset` is left alone: its models stay on the OpenAI embedding target, and the BUG-002 branch rewrites those lines) |
| `crates/any-context-core-rs/src/retrieval/pipeline.rs` | doc comment of `set_embedding` only |

Surfaces: the sync output (CLI and TUI) gains one message. No command, flag, configuration key or file format changes. The `settings.db` schema gains one nullable column.

Other places with the same defect: none besides the per-call fallback (BUG-014). Before the fix, a `grep` for `text-embedding-004` and `text-embedding-3-small` across the repository confirms that no other hard-coded embedding id exists.

**Conflict note**: the uncommitted BUG-002 work on `fix/retired-model-ids` touches `crates/actx-cli/src/engine.rs`, `actx-lm/src/providers/gemini.rs`, `storage/sqlite.rs` and the TUI files. BUG-003 starts from `dev` **after the BUG-002 pull request is merged**, or rebases onto it, to avoid conflicting edits.

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`, native runner and CI):

1. **Regression BUG-003** (`crates/any-context-core-rs/tests/orchestrator_tests.rs`): sync a temp folder with the mock target, record `mock/mock-embed`, then simulate a target change (record a different target, or set `ACTX_EMBEDDING_MODEL` to another mock-backed id). Sync again without changing files, and assert that every chunk of the workspace was re-embedded (all files indexed again, `embedding_reindex` set, new record stored). It fails before the fix: zero files indexed.
2. **Regression BUG-013** (unit, `orchestrator.rs`): with a Gemini key stored, the resolver returns `gemini/gemini-embedding-2`, and `legacy_target` for the same inputs returns `gemini/text-embedding-004`, so `decide_reindex(None, legacy, current, true)` is `Reindex`.
3. `decide_reindex` table test: Keep / Adopt (empty workspace) / Adopt (legacy == current, OpenAI) / Reindex (recorded differs) / Reindex (legacy differs).
4. No-key fallback resolves to `mock/mock-embed`, and adding an OpenAI key changes the target.
5. Scoped sync (`target_folder`) with a pending reindex purges and re-embeds the whole workspace.
6. Cancelled reindex: the record is written only on the completed paths of `run` (early exit and end of pass), not on the cancellation return. Implementation note: there is no deterministic hook to cancel a pass mid-run from a test, so this is verified by review and by MT-SYNC-003 rather than by an automated test.
7. `sqlite.rs`: the column is added to an old database without it (open a database created with the pre-fix `CREATE TABLE`), get/set round trip, NULL by default.
8. `gemini.rs`: request body serialization contains the model and the output size for every item (pure helper, no network); `GEMINI_EMBEDDING_DIM == DEFAULT_VECTOR_DIM` (asserted in the core, which depends on `actx-lm`).
9. Query side: `resolve_query_embedding_provider` prefers the recorded target, and falls back for a dead target or a missing key.

Manual (`tests/MANUAL_TESTS.md`, section "Sources and Sync"), edited in place:

- **New MT-SYNC-006: Changing the model reindexes the workspace once.** Sync with an OpenAI model, `/model` to a Gemini model, sync. Expected: the "embedding model changed" message, every file re-embedded, a question about an old document answered from it. A second sync reports 0 changes.
- **New MT-SYNC-007: A workspace indexed before the upgrade with a Gemini model is repaired.** With a v0.45.0 Gemini workspace, install the candidate and sync. Expected: one automatic reindex, then dense results (a question answered from a passage with no shared keywords). This is the live-API evidence for H1.

Acceptance criteria: the tests above pass on the native runner and in CI; MT-SYNC-006 and MT-SYNC-007 pass on the release candidate; no second reindex happens on a sync after a completed one.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/ingestion.md`, "Embedding and writing": the target record, the reindex rule, the legacy inference rule, `gemini-embedding-2` at 1536 dimensions, the mock target. Steps of the sync pass: add step 4b.
- `docs/architecture/retrieval.md`, dense search: queries prefer the recorded target.
- `docs/architecture/storage.md`: the `workspaces.embedding_target` column.
- `docs/architecture/lm-providers.md`: Gemini embedding model and output size.
- New ADR-0138 (section 3.7); `build_index.py` and `check_docs.py` with zero errors.
- README: a short line in the troubleshooting or models section: "Changing the chat model can change the embedding model; the next sync reindexes the workspace automatically, which costs one embedding pass." Run `check_readme.py`.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch `v0.45.1`, in the same release as BUG-002 (planned for v0.45.1). Release notes: "Gemini workspaces and workspaces whose chat model changed are reindexed automatically on their next sync (one-time embedding cost)."
- **Data already affected**: repaired automatically. Gemini workspaces from before the fix are reindexed by the legacy rule. Workspaces already mixed by a past `/model` change cannot be detected (there's no record of the old target). The release notes tell users to run a forced sync once (`/sync --force`, or `actx sync --force`) if they changed models on a workspace before v0.45.1. This is a known limitation, listed in the notes.
- **Migration**: one nullable column added with `ALTER TABLE`; older binaries ignore it. No LanceDB change.
- **Rollback**: revert the pull request on `main` and release the next patch. A v0.45.0 binary reading a database with the column works unchanged. Gemini workspaces rolled back go back to failing embeddings (the pre-fix state).
- **Observability**: the reindex decision and its reason are written to the sync's verbose output and `SyncResult` (covered by test 1), and the Gemini error body is kept in `LmError::ProviderError`. Surfacing the per-call embedding error stays with IMP-003 and BUG-014.
- **CI**: no workflow change.
- **Security**: no secrets involved. The Gemini key keeps going in the query string, as today (out of scope).

## 8. Execution steps and milestones

1. **M1 Evidence**: confirm H1 and H2 from the official documentation (done: H1 confirmed, H2 refuted, IMP-014 captured); `grep` for other hard-coded embedding ids (done: only `resolve_embedding_provider`, the retrieval default `DEFAULT_QUERY_EMBEDDING_MODEL` and the token-limit table, which already falls back to 2048 tokens for any `gemini*` id).
2. **M2 Target and record**: `EmbeddingTarget`, resolver changes (Gemini, mock), SQLite column and methods, tests 4, 7.
3. **M3 Reindex rule and sync**: `decide_reindex`, `legacy_target`, sync step 4b, record after success, `SyncResult.embedding_reindex`, tests 1, 2, 3, 5, 6.
4. **M4 Query side and Gemini adapter**: `resolve_query_embedding_provider`, CLI wiring, Gemini output size, CLI/TUI message, tests 8, 9.
5. **M5 Documentation and manual tests**: architecture pages, ADR-0138, README line, MT-SYNC-006/007, `check_docs.py`, `check_readme.py`, `manual_tests.py validate`.
6. **M6 Review and pull request**: code review, `software-design` checklist, `check_design.py`, pull request into `dev` listing BUG-003, BUG-013 and the captured BUG-014, CI green.
