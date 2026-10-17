# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-002: Frontier preset selects an outdated model id
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: /preset frontier sets the model claude-3-7-sonnet, which may no longer be a valid model id for the Anthropic API; the preset could then fail on the first question.

### BUG-003: Changing a workspace chat model mixes embedding spaces on the next sync
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: resolve_embedding_provider (ingestion/orchestrator.rs) derives the embedding model from the workspace chat model. After /model changes it, the next incremental sync embeds new or changed files with another provider/model while unchanged files keep their old vectors, and queries use the new model, so part of the workspace becomes unreachable by dense search. Needs the per-workspace embedding model record planned for Frente C, or a forced reindex when the embedding target changes.

### BUG-004: Source round-robin puts diversity above relevance and breaks the U-shaped order
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: `search_single_async` returns chunks in the order produced by `apply_source_diversification` (round-robin across sources), and `ContextFormatter::format_xml_chunks` applies the U-shaped reordering to that order. The second-best chunk can land in the middle of the context while a weak chunk sits next to the user query. The round-robin also only reorders candidates that were already retrieved (dense + BM25 over the whole index), so it does not widen document coverage; its first pass gives every source one slot before any second chunk, so a low-scoring chunk from another source can displace a strong second chunk from the same source. Decision (2026-10-17): remove the round-robin and keep the per-source cap (`max_chunks_per_source`) applied in score order, in both `search_single_async` and `retrieve_hybrid_batch_async`; revisit the cap once hierarchical chunking (Frente D) and the evaluation (IMP-008) exist.

### BUG-005: Recency boost overrides relevance across sources
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The recency boost in `search_single_async` adds up to 0.01 to the RRF score, while the best possible single-list RRF score is 1/61 ≈ 0.0164, so a recently modified file far down the rankings can outrank an older file ranked first. The intent was a tie-breaker between chunks of the same source, but all chunks of a file or web page share the same `last_modified`, and sync deletes the old chunks of a changed source, so the boost has no effect inside a source and only acts across sources. It also changes which candidates reach the cross-encoder.

### BUG-006: Deep Search minimum score discards single-list hits
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: Deep Search requests use `min_score: 0.02`, applied to RRF scores before re-ranking. A chunk found only by the dense search or only by BM25 scores at most 1/61 ≈ 0.0164, so it is discarded unless the recency boost (BUG-005) lifts it. Supersedes IMP-001.

### BUG-007: Retrieved text is not escaped in the XML prompt
- **Severity**: high
- **Status**: in_progress
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: any_context_retrieved_text_is_not_escaped_in_the_xml_prompt
- **Description**: `ContextFormatter::format_xml_chunks` writes chunk text and attribute values (file path, file name) into the XML prompt without escaping. Indexed content containing `</chunk>`, `</retrieved_context>` or quotes (this repository's own docs, crawled web pages) breaks the chunk boundaries the prompt relies on and opens a prompt-injection path.

### BUG-008: Scope and version attributes are guessed from path substrings
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: `ContextFormatter::infer_scope_and_version` labels chunks by substring matches on the path: `latest` becomes scope `test`, `author` becomes `auth`, `environment` becomes `config`, `rapid` becomes `api`, `dev2` becomes version `v2`. The prompt tells the model never to combine chunks with different scope or version, so wrong labels can block a correct answer.

### BUG-009: Deep Search synthesis bypasses the canonical context formatter
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: The Deep Search `SUMMARY_PROMPT` places the original query before the chunks and lists them as plain `--- CHUNK n ---` blocks, without U-shaped reordering, XML isolation, scope/version attributes or a query anchor. It is the mode that gathers the most chunks from several sub-queries.

### BUG-010: Section and line metadata are lost at ingestion
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: When a document envelope exists, `embed_and_upsert_chunks` stores the document summary in place of the chunk's `header_path`, and start/end lines are not stored in LanceDB. The prompt header only shows the file name, and the Deep Search retriever passes `header_path`, `start_line` and `end_line` as `None`, although `SUMMARY_PROMPT` asks for `[path:lines]` citations. Related to the LanceDB schema changes planned in Frentes C and D (IMP-004).

### BUG-011: Chunk text sent to the model depends on the retrieval path
- **Severity**: low
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, RAG best-practices review
- **Plan**: -
- **Description**: BM25 stores the enveloped text (`[Context: ... | Keywords: ...]` plus the chunk), while LanceDB stores the raw chunk. Chunks found only by BM25 reach the prompt with the envelope; chunks found by the dense search reach it without.

### BUG-012: Agent prompt names tools that do not exist
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: `config/AGENT.md` tells the model to call `add_web_source`, `list_web_sources`, `remove_web_source` and `search_db(search_session_memory=True)`, but only `search_db(query)` and `system_status` are registered. It also says the model must always call `search_db` first, while the deterministic path sends the retrieved chunks with no tools available.

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

### FEAT-005: Embedding model independent of the chat model, with stale-workspace status
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: The embedding model is derived from the workspace chat model (see BUG-003). Make it a setting of its own, global by default, and record on each workspace the model it was indexed with. Changing it must warn the user strongly that every workspace needs a resync, mark each affected workspace as outdated in its status and guide the resync. A per-workspace embedding override was discussed and left out for now (main case: a sensitive workspace on a local model); recording the model per workspace keeps it possible later. Related to IMP-004.

### FEAT-006: Individual sources: single local files and single web pages
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, v0.45.0, product ideas review
- **Plan**: -
- **Description**: Allow adding a single file (not only folders) and a single web page (not only a crawled URL prefix) to a workspace, without indexing the same content twice when the file already sits inside a registered folder.

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

## Improvements

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

### IMP-003: Report query embedding failures instead of skipping dense search silently
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: NativeHybridPipeline::search_single_async discards the query embedding error (retrieval/pipeline.rs, '_ => None'), so a missing key or unsupported provider silently leaves only BM25. Keep the ADR-110 fallback but record the outcome (for example a last-status accessor like last_rerank_latency_ms) and surface it in the planned /embed status.

### IMP-004: Per-table vector dimension in LanceDB with assisted reindex
- **Priority**: high
- **Status**: open
- **Found**: 2026-10-17, 2026-10-17 triage of the post-v0.45.0 roadmap
- **Plan**: -
- **Description**: storage/lancedb.rs fixes DEFAULT_VECTOR_DIM = 1536 and ingestion writes every table with it, while Gemini (768), Ollama nomic-embed-text (768) and local ONNX embedders (384/768) produce other sizes. Store model and dimension per workspace/table, detect divergence and offer assisted reindex. First LanceDB schema change since C0: needs migration and rollback; combine with the Frente D parent_id change (roadmap 2.2 goal 4).

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

### IMP-012: Encrypt the BM25 index file at rest
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, BUG-001 planning, 2026-10-17
- **Plan**: -
- **Description**: bm25_index.bin keeps chunk text in plaintext because lexical search needs the terms (ADR-0131 consequence), so the indexed text is not fully encrypted on disk. Options: encrypt the serialized file with the hardware-bound key and decrypt on load, or store only hashed terms.

### IMP-013: Compact and prune LanceDB after deletes and forced reindexes
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-17, BUG-001 implementation, 2026-10-17
- **Plan**: -
- **Description**: Deletes and actx sync --force only write deletion markers and new versions, so replaced chunk data (including removed or changed documents) stays on disk in old Lance versions until a compaction and prune runs. Only the plaintext migration (ADR-0135) prunes today. Add periodic optimize (compact + prune) after syncs that delete rows.

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
