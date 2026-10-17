# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-001: Data indexed before field encryption stays in plaintext
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: Chunks written to LanceDB before field-level encryption on every write (ADR-0131) remain unencrypted on disk until the workspace is re-indexed with actx --sync --force. Nothing tells the user, while the README states that indexed text is encrypted.

### BUG-002: Frontier preset selects an outdated model id
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: /preset frontier sets the model claude-3-7-sonnet, which may no longer be a valid model id for the Anthropic API; the preset could then fail on the first question.

### BUG-003: Source round-robin puts diversity above relevance and breaks the U-shaped order
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: `search_single_async` returns chunks in the order produced by `apply_source_diversification` (round-robin across sources), and `ContextFormatter::format_xml_chunks` applies the U-shaped reordering to that order. The second-best chunk can land in the middle of the context while a weak chunk sits next to the user query. The round-robin also only reorders candidates that were already retrieved (dense + BM25 over the whole index), so it does not widen document coverage; its first pass gives every source one slot before any second chunk, so a low-scoring chunk from another source can displace a strong second chunk from the same source. Decision (2026-10-17): remove the round-robin and keep the per-source cap (`max_chunks_per_source`) applied in score order, in both `search_single_async` and `retrieve_hybrid_batch_async`; revisit the cap once hierarchical chunking (Frente D) and the evaluation (IMP-004) exist.

### BUG-004: Recency boost overrides relevance across sources
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The recency boost in `search_single_async` adds up to 0.01 to the RRF score, while the best possible single-list RRF score is 1/61 ≈ 0.0164, so a recently modified file far down the rankings can outrank an older file ranked first. The intent was a tie-breaker between chunks of the same source, but all chunks of a file or web page share the same `last_modified`, and sync deletes the old chunks of a changed source, so the boost has no effect inside a source and only acts across sources. It also changes which candidates reach the cross-encoder.

### BUG-005: Deep Search minimum score discards single-list hits
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: Deep Search requests use `min_score: 0.02`, applied to RRF scores before re-ranking. A chunk found only by the dense search or only by BM25 scores at most 1/61 ≈ 0.0164, so it is discarded unless the recency boost (BUG-004) lifts it. Supersedes IMP-001.

### BUG-006: Retrieved text is not escaped in the XML prompt
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: `ContextFormatter::format_xml_chunks` writes chunk text and attribute values (file path, file name) into the XML prompt without escaping. Indexed content containing `</chunk>`, `</retrieved_context>` or quotes (this repository's own docs, crawled web pages) breaks the chunk boundaries the prompt relies on and opens a prompt-injection path.

### BUG-007: Scope and version attributes are guessed from path substrings
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: `ContextFormatter::infer_scope_and_version` labels chunks by substring matches on the path: `latest` becomes scope `test`, `author` becomes `auth`, `environment` becomes `config`, `rapid` becomes `api`, `dev2` becomes version `v2`. The prompt tells the model never to combine chunks with different scope or version, so wrong labels can block a correct answer.

### BUG-008: Deep Search synthesis bypasses the canonical context formatter
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The Deep Search `SUMMARY_PROMPT` places the original query before the chunks and lists them as plain `--- CHUNK n ---` blocks, without U-shaped reordering, XML isolation, scope/version attributes or a query anchor. It is the mode that gathers the most chunks from several sub-queries.

### BUG-009: Section and line metadata are lost at ingestion
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: When a document envelope exists, `embed_and_upsert_chunks` stores the document summary in place of the chunk's `header_path`, and start/end lines are not stored in LanceDB. The prompt header only shows the file name, and the Deep Search retriever passes `header_path`, `start_line` and `end_line` as `None`, although `SUMMARY_PROMPT` asks for `[path:lines]` citations. Related to the LanceDB schema changes planned in Frentes C and D.

### BUG-010: Chunk text sent to the model depends on the retrieval path
- **Severity**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: BM25 stores the enveloped text (`[Context: ... | Keywords: ...]` plus the chunk), while LanceDB stores the raw chunk. Chunks found only by BM25 reach the prompt with the envelope; chunks found by the dense search reach it without.

## Features

## Improvements

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

### IMP-003: No relevance cutoff on cross-encoder scores
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic path always sends up to `top_k` (6) chunks, and `min_score` is applied before re-ranking on the RRF scale. The cross-encoder's calibrated scores are never used to drop weak chunks, and the Zero Hallucination Gate threshold (0.005) is tuned for RRF scores.

### IMP-004: No retrieval and answer quality evaluation
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: There is no reference question set or metric (recall@k, NDCG@5, citation faithfulness) to calibrate thresholds or show the effect of retrieval changes, including the planned hierarchical chunking (Frente D).

### IMP-005: Follow-up questions are retrieved without conversation context
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic retriever searches with the current message only. A follow-up such as "and in v1?" is searched as written, without the subject of the previous turns.

### IMP-006: Deterministic RAG prompt is sent without a system message
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The deterministic path places the system instructions inside the user message (`<system_instructions>`) after the session history, so the instructions are repeated per turn and cannot benefit from provider prompt caching.

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

### ADJ-003: Chunk rank attribute shows the prompt position
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
- **Description**: Deep Search keeps results with a minimum score of 0.02, which looks low and may let irrelevant chunks into the combined answer.
- **Closed**: 2026-10-17, duplicate of BUG-005 (the threshold is above the maximum single-list RRF score)
