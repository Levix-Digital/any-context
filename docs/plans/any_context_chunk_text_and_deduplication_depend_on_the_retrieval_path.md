---
plan_id: any_context_chunk_text_and_deduplication_depend_on_the_retrieval_path
project: any-context
title: Chunk text and deduplication depend on the retrieval path
kind: bug
version: v0.45.1
status: completed
author: Levi Silveira
created_at: '2026-10-18T10:49:11'
updated_at: '2026-10-18T12:23:01'
session_id: null
tags:
- retrieval
- bm25
- deep-search
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-011
- BUG-016
milestones:
- name: M1 Fix and tests
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
- name: M2 Docs
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
- name: M3 Review and PR
  done: true
  done_at: '2026-10-18'
  by: Levi Silveira
status_log:
- at: '2026-10-18T10:49:11'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-18T10:51:55'
  by: Levi Silveira
  status: active
  note: Plan updated
- at: '2026-10-18T11:29:15'
  by: Levi Silveira
  status: in_progress
  note: Approved by the user on 2026-10-18
- at: '2026-10-18T12:01:22'
  by: Levi Silveira
  note: 'Milestone ''M1 Fix and tests'' completed: strip_envelope, lookup_stored_rows, BM25-only branch, chunk-id dedup key, 6 tests; cargo test --workspace green (423 passed)'
- at: '2026-10-18T12:01:26'
  by: Levi Silveira
  note: 'Milestone ''M2 Docs'' completed: ADR-0143, amended_by on ADR-0108 and ADR-0142, retrieval and ingestion pages; docs, readme, manual test and design checks clean'
- at: '2026-10-18T12:23:01'
  by: Levi Silveira
  note: 'Milestone ''M3 Review and PR'' completed: PR #357 merged into dev'
- at: '2026-10-18T12:23:01'
  by: Levi Silveira
  status: completed
  note: All milestones completed
---

# Chunk text and deduplication depend on the retrieval path

Bug plan for BUG-011 and BUG-016 (`dev-cycle-protocol` full path, chosen by the user on 2026-10-18: it changes what the re-ranker and the model receive and records a decision with alternatives). Both are fixed together, in one pull request into `dev`, because they touch the same lines of `retrieval/pipeline.rs` and fixing BUG-011 alone (filling the content hash of BM25-only hits) would make BUG-016 worse. Target: patch release v0.45.1.

## Backlog items

### BUG-011: Chunk text sent to the model depends on the retrieval path
- **Severity**: low
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Description**: BM25 stores the enveloped text (`[Context: ... | Keywords: ...]` plus the chunk), while LanceDB stores the raw chunk. Chunks found only by BM25 reach the prompt with the envelope; chunks found by the dense search reach it without.

### BUG-016: Deep Search deduplication merges every chunk of a file
- **Severity**: medium
- **Found**: 2026-10-18, v0.45.0, BUG-011 discovery
- **Description**: `retrieve_hybrid_batch_async` keys its cross-query deduplication on content_hash, which is the SHA-256 of the whole file: every dense hit of one file collapses into the first chunk and the scores of the other chunks are added to it. BM25-only hits carry an empty hash and fall back to the chunk id, so they are not merged.

## 1. Reproduction

No log is involved: chunk text and deduplication keys are built in memory and not logged. Both defects are reproduced by reading the code on `dev` at 5fb2544 and by the regression tests of section 5.

**BUG-011.** Workspace with `guide.md`, enriched at sync with a document envelope. The question "kestrel retries" matches a chunk lexically but not semantically (or the embedding key is missing, so every hit is BM25-only). The chunk reaches the cross-encoder and the `<chunk>` envelope as:

```
[Context: Document 'guide.md': Kestrel is a gateway... | Keywords: kestrel, timeouts, retries, gateway, ...]
---
kestrel timeouts are retried three times
```

The same chunk found by the dense search reaches them as `kestrel timeouts are retried three times`. The summary also repeats in every BM25-only chunk of the same file, consuming the density budget.

**BUG-016.** `/search deep` on a question whose sub-queries find chunks `a_0`, `a_1` and `a_2` of `alpha.md` by dense search. The batch returns only `a_0`, with the sum of the three scores; `a_1` and `a_2` never reach the synthesis. Chunks of the same file found only by BM25 are not merged.

## 2. Evidence and root cause

Confirmed facts (code read on `dev` at 5fb2544):

1. **Two texts per chunk.** `embed_and_upsert_chunks` (`crates/any-context-core-rs/src/ingestion/orchestrator.rs:609-624`) builds `text_to_embed` with `apply_envelope_to_chunk` (`ingestion/enricher.rs:111-118`) and gives it to `bm25.add_chunk` and to the embedding call; the LanceDB record stores `text: c.text.clone()` (`:657`), the raw chunk.
2. **The BM25 text reaches the prompt.** In `search_single_async` (`retrieval/pipeline.rs:686-712`) a dense hit takes `d.text` from LanceDB; a BM25-only hit takes `doc.text` from `BM25Index::get_doc_by_id`, that is the enveloped text. The text then feeds `apply_neural_rerank`, the density budget and `ContextFormatter`.
3. **The raw row is already read.** Step 10, `lookup_provenance` (`pipeline.rs:806-829`), reads the LanceDB row of every BM25-only hit (`id IN (...)`, matched on chunk id and normalized path, ADR-0142) and keeps only `header_path`, `start_line` and `end_line`, dropping `text` and `content_hash`.
4. **BM25 needs the envelope for matching.** `rebuild_postings` re-tokenizes `DocRecord::text` after every removal (`bm25.rs:163-184`), so the indexed text must stay the enveloped one; the keywords and summary are part of contextual retrieval (ADR-0106).
5. **The content hash is per file.** `content_hash` is `compute_file_sha256` of the whole file (`orchestrator.rs:463`), stored unchanged in every chunk record of that file (`:665`); for web pages, the hash of the page text.
6. **Batch dedup key.** `retrieve_hybrid_batch_async` (`pipeline.rs:878-885`) keys `dedup_map` on `content_hash` when not empty, else `chunk_id`, else `file_path::text`. Dense hits carry the file hash, so every chunk of one file maps to one key: the first stays, the later ones only add their score and sub-query labels. BM25-only hits carry `String::new()` (`:710`) and are keyed by chunk id. Step 6 of the same function already maps results back by `chunk_id` (`:935-948`).
7. **Only Deep Search is affected by BUG-016.** `retrieve_hybrid_batch_async` is called only by `PipelineBatchRetriever` (`crates/actx-cli/src/engine.rs:701`); the fast path and `search_db` use `search_single_async`.
8. **Chunk ids.** Ids are `<first 12 hex of the file hash>_<index>` (`orchestrator.rs:601-608`), so files with identical content share chunk ids and texts; keying on chunk id still merges those true duplicates.

Root causes: BUG-011, the BM25 index doubles as a text store for hits the dense search did not return, and it stores the text built for matching, not the text meant for the model. BUG-016, the batch deduplication was written assuming `content_hash` identifies a chunk (ADR-0108 "content-hash deduplication"), while ingestion stores the hash of the source file.

## 3. Fix design (`software-design`)

Hexagonal style; everything stays inside the core retrieval adapter (`retrieval/pipeline.rs`) and the ingestion module that owns the envelope format. No port, public type or stored format changes.

### BUG-011: BM25-only hits take their text from LanceDB

- `lookup_provenance` becomes `lookup_stored_rows(ids, workspace, table) -> HashMap<(String, String), ScoredVectorResult>`: the same single `id IN (...)` metadata query, keyed by chunk id and normalized file path, returning the whole row instead of only the provenance.
- The BM25-only branch of step 11 takes `text`, `content_hash` and the provenance from that row. `content_type`, `file_name`, `file_path`, `workspace` and `last_modified` keep coming from the BM25 record, as today. Encrypted texts (`enc::`) are decrypted by the existing step after the branch.
- **Fallback** when the row is missing or the lookup fails (BM25 file out of step with LanceDB, lookup error): `NativeContextualEnricher::strip_envelope(text: &str) -> &str`, an associated function next to `apply_envelope_to_chunk` in `ingestion/enricher.rs`, so the envelope format has one owner. It removes a leading envelope matched by an anchored regex built from the exact format, `(?s)\A\[Context: .*? \| Keywords: [\p{L}, ]*\]\n---\n` (keywords are only letters, commas and spaces; `general` when empty), compiled once with `OnceLock`; any other text is returned unchanged. A test ties it to `apply_envelope_to_chunk` (round trip).
- BM25 keeps indexing the enveloped text (fact 4), so lexical matching on keywords and summary does not change.

### BUG-016: deduplicate by chunk id

- The `dedup_map` key becomes `chunk_id`, falling back to `file_path::text` when the id is empty; `content_hash` is no longer used as a key. This matches step 6, which already maps by `chunk_id`, and still merges files with identical content (fact 8).
- Score accumulation, `matched_subqueries`, the per-source cap and the density budget are unchanged; the cap (`max_chunks_per_source`) now actually sees several chunks per file, as designed.

### Error strategy

No new error path: a lookup error keeps today's behavior (hits without provenance) and now also the stripped BM25 text; `strip_envelope` is total.

### Deliberately left out

- Storing the raw text in the BM25 file next to the indexed text: changes the positional bincode format (quarantine and reindex, as ADR-0142 explains) for a value LanceDB already holds.
- A per-chunk content hash in LanceDB: not needed once the key is the chunk id; `content_hash` keeps meaning "hash of the source" for incremental sync (`orchestrator.rs:759`).
- IMP-012 (BM25 file encryption), IMP-003 (query embedding failures), BUG-014 (pseudo-vectors).

### ADR

New ADR-0143 "BM25-only hits take their text from LanceDB; batch deduplication by chunk id", amending ADR-0108 (content-hash deduplication) and ADR-0142 (the BM25-only lookup now returns the whole row). Alternatives recorded: store the raw text in `DocRecord` (format change, reindex), strip the envelope only (works but keeps two text sources and fails on texts the regex does not recognize), stop enveloping BM25 text (loses contextual lexical matching), a per-chunk hash column (schema change for no gain).

## 4. Affected surfaces and files

- `crates/any-context-core-rs/src/retrieval/pipeline.rs`: `lookup_stored_rows`, BM25-only branch of `search_single_async`, dedup key of `retrieve_hybrid_batch_async`, tests.
- `crates/any-context-core-rs/src/ingestion/enricher.rs`: `strip_envelope`, tests.
- Surfaces `cli` and `tui`: no command, flag, setting or file format changes. Answers no longer contain document envelopes in chunks found by BM25 only, and Deep Search can return several chunks of the same file. No stored data changes.
- Same defect elsewhere: none found; `get_doc_by_id` has no other caller, and no other code keys results on `content_hash`.

## 5. Tests (`quality-assurance`)

Automated (`cargo test --workspace`):

- **Regression BUG-011** (`pipeline.rs`) `test_bm25_only_hits_take_raw_text_from_lancedb`: BM25 holds the enveloped text of `g_0`, LanceDB the raw text and a content hash; the BM25-only result has the raw text (no `[Context:`) and the hash. Fails before the fix.
- `test_bm25_only_hits_without_a_row_drop_the_envelope`: a BM25-only hit with no LanceDB row returns the text without the envelope.
- **Regression BUG-016** (`pipeline.rs`) `test_batch_keeps_every_chunk_of_a_file`: three LanceDB rows of one file with the same `content_hash` and a supplied `query_vector`; two sub-queries; every chunk id is in the batch result (up to `max_chunks_per_source`) and none carries another chunk's score. Fails before the fix (one result).
- `test_batch_merges_the_same_chunk_across_sub_queries`: one chunk found by two sub-queries is returned once with both labels and the summed score.
- `enricher.rs`: `strip_envelope` round trip with `apply_envelope_to_chunk` (with keywords and with `general`), text without envelope unchanged, a summary containing `]`, `|` and `---` lines.
- The existing provenance, ordering and min-score tests stay green.

Manual (`tests/MANUAL_TESTS.md`): no change. Both behaviors are fully covered by automated tests and are not visible as a distinct user step; MT-ANS-002 (deep research) and MT-ANS-007 (citations) keep exercising the paths.

Acceptance criteria: a chunk has the same text whichever list found it; Deep Search keeps distinct chunks of one file; the same chunk across sub-queries is still merged.

## 6. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/retrieval.md`: step 7 (BM25-only hits take text, content hash and provenance from their LanceDB row, the envelope is stripped when no row is found; BM25 indexes the enveloped text) and the batch paragraph (deduplicated by chunk id); ADR-0143 in Decisions.
- `docs/architecture/ingestion.md`: the envelope is indexed by BM25 and embedded, not stored as LanceDB text; `strip_envelope`.
- ADR-0143 (new); `amended_by` on ADR-0108 and ADR-0142; `build_index.py` and `check_docs.py` with zero errors.
- README: no change expected; `check_readme.py` still run.

## 7. Release, migration and rollback (`devops-release`)

- **Version**: patch, v0.45.1, with the other fixes already in `dev`.
- **Data already affected**: none to repair; the fix applies at query time to existing indexes, with no reindex.
- **Rollback**: revert the pull request (or the release through a pull request on `main`) and ship the next patch.
- **Observability**: none added; chunk text is not logged (privacy). Covered by the tests above.
- **Security**: no change; encrypted LanceDB texts are decrypted by the existing step.

## 8. Execution steps and milestones

1. **M1 Fix and tests**: on `fix/chunk-text-retrieval-path` (from `dev`): `strip_envelope`, `lookup_stored_rows`, BM25-only branch, dedup key, regression and unit tests; `cargo test --workspace` green.
2. **M2 Docs**: ADR-0143, `amended_by` on ADR-0108 and ADR-0142, retrieval and ingestion pages; `check_docs.py`, `check_readme.py` and `manual_tests.py validate` with zero errors.
3. **M3 Review and PR**: code review, `software-design` checklist and `check_design.py`; PR into `dev`, CI green; close BUG-011 and BUG-016 when merged.
4. Release with v0.45.1 when the user wants it.
