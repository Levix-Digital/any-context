# Feature Specification: Local multilingual embedding model, independent of the chat model

**Feature Branch**: `feat/002-local-embedding`

**Created**: 2026-10-29

**Status**: Draft

**Input**: User description: "FEAT-067: local multilingual ONNX embedding models per profile as the default; cloud providers only by explicit choice. IMP-027: the embedding model is independent of the chat model, defined by the model kit and changeable by the user, and each chunk records its embedding target. IMP-028: embedding never switches model silently; the OpenAI and mock fallbacks end. IMP-029: the catalog declares each model's query and document prefixes and the pipeline applies them at ingestion and at query time. The default model of each profile is picked by the development retrieval benchmark (IMP-065), and the dense retriever is turned on in the CI benchmark."

**Backlog**: FEAT-067, IMP-027, IMP-028, IMP-029 (plan `feat_002_local_embedding`, release v1.0.0 Personal). Design: `docs/architecture/ingestion-pipeline-v2.md`, section 8, Embedding, and section 7, Decisions.

## Context

Today the embedding model is derived from the workspace chat model (ADR-0132, ADR-0138): a Gemini chat model gives `gemini-embedding-2`, Ollama gives `nomic-embed-text`, anything else gives OpenAI `text-embedding-3-small`. With no API key, ingestion silently falls back to OpenAI and then to a mock provider whose vectors are all the same, so dense search is quietly turned off (BUG-019). An AnyContext installed without keys therefore has no semantic search at all, and switching the chat model reindexes the whole workspace. The v2 design makes AnyContext air-gapped by default: a local, multilingual embedding model that runs on ONNX Runtime, chosen by the hardware profile, with cloud embedding only when the user asks for it.

Decided with Levi on 2026-10-29:

- One plan for FEAT-067, IMP-027, IMP-028 and IMP-029: the local model, the separate setting, the end of silent fallbacks and the prefixes change the same code and the same vector-space identity.
- Candidates measured by the benchmark: `multilingual-e5-small`, `multilingual-e5-base` and `BGE-M3` (all MIT). `Qwen3-Embedding-0.6B` is left out (too heavy for CI and for the smaller profiles).
- One vector table per embedding target without zero-padding (design section 10, IMP-033) is out of scope: the current table and its padding to 1536 stay; the change of tables comes with the retrieval rework (IMP-040).
- The M1 work of the old IMP-004 branch (commit `4042b89`: application-wide embedding setting, per-chunk model record, no fallback) is reused as a reference and ported onto the current code, not merged.

## Clarifications

### Session 2026-10-29

- Q: A local model is missing when a sync needs it: download it or fail? → A: Download it on first need through the verified download, from the configured model source (catalog upstream or an internal mirror); files already in the models directory are used offline after the SHA-256 check. Organization-approved models and the Admin Hub mirror stay in v2.0.0 (IMP-056, FEAT-143) and plug into the model source and the allowed-models check added here (FR-018, FR-019).
- Q: `Lite` profile without a cloud embedding choice: lexical only or a local model? → A: Download `multilingual-e5-small` as for any profile; there is no lexical-only mode, because retrieval without semantic search is not RAG.
- Q: Workspaces already indexed with OpenAI or Gemini: keep that model or move to the local default? → A: The upgrade adopts the existing cloud model as the setting (no surprise reindex); mock-indexed workspaces are reindexed with the local model.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Semantic search works out of the box, without any API key (Priority: P1)

A user installs AnyContext with a local profile, adds a folder and asks a question in Portuguese about a document written in English. Without configuring any provider or key, the answer is found by meaning, not only by shared words, and nothing leaves the machine to build the index or embed the question.

**Why this priority**: It is the product promise of R1 (local, private, multilingual) and the reason the other stories exist. Today this scenario has no dense search at all.

**Independent Test**: On a machine with no provider keys, install a local profile, index the benchmark corpus and check that the dense retriever is on, that cross-language questions find their documents, and that no network request is made during sync or query apart from the one-time verified model download.

**Acceptance Scenarios**:

1. **Given** a fresh installation with a local profile and no API keys, **When** the user syncs a folder, **Then** every chunk is embedded by the profile's local model and the sync reports which model it used.
2. **Given** an indexed workspace, **When** the user asks a question, **Then** the question is embedded by the same local model, the dense retriever takes part in the search and the result is fused with the lexical one as today.
3. **Given** a profile whose local embedding model is not installed yet, **When** a sync needs it, **Then** it is downloaded with the existing verified download (pinned revision and SHA-256) from the configured model source (the catalog's upstream by default, or the internal mirror the installation is configured with), with progress shown; if that is not possible (offline, mirror unreachable, checksum mismatch), the sync stops before indexing and says which model is missing, where it would be downloaded from and how to install it by hand.
4. **Given** the `Lite` profile, **When** the user syncs without having chosen a cloud embedding model, **Then** the smallest local model (`multilingual-e5-small`) is downloaded and used as for any other profile: there is no lexical-only mode, because retrieval without semantic search is not RAG.
5. **Given** the model files were placed in the models directory beforehand (by IT or by the installer), **When** a sync needs the model, **Then** the files are used after their SHA-256 is checked, with no network access.
6. **Given** an installation configured with an internal mirror, **When** a model is downloaded, **Then** only the mirror is contacted (same file paths, same SHA-256 check), never the public upstream.

---

### User Story 2 - The embedding model is a setting of its own (Priority: P1)

A user switches the chat model (for example from a local model to Gemini for one hard question) and the index is untouched. Separately, the user can see which embedding model is in use and change it, choosing another local model of the catalog or, explicitly, a cloud model; that change is announced as a reindex before it happens.

**Why this priority**: Without it, every chat model switch reindexes the workspace and a cloud chat model silently moves the index to the cloud.

**Independent Test**: Index a workspace, change the chat model, sync again and check that no chunk is re-embedded; then change the embedding model and check that the command warns about the reindex, and that the next sync reindexes the workspace and records the new model on every chunk.

**Acceptance Scenarios**:

1. **Given** an indexed workspace, **When** the user changes the chat model, **Then** the embedding model, the stored vectors and the next sync are unaffected.
2. **Given** the CLI or TUI, **When** the user asks for the embedding model, **Then** they see the model in use, its provider (local or cloud), its dimension, and the other models of the catalog they can pick.
3. **Given** the user picks another embedding model, **When** they confirm, **Then** they are told that every workspace indexed with the previous model will be reindexed on its next sync, and each affected workspace shows that a reindex is pending.
4. **Given** a cloud embedding model, **When** the user picks it, **Then** the choice says that document text will be sent to that provider; it never becomes the embedding model as a side effect of a chat model choice.
5. **Given** a workspace being reindexed after a model change, **When** the user asks a question before the reindex finishes, **Then** dense search only compares the question with chunks of the same model (the old model keeps answering, as today), never with vectors of another model.

---

### User Story 3 - Embedding never switches model silently (Priority: P1)

When the chosen embedding model cannot run (missing key, provider error, model file missing or corrupt), the affected files fail with a clear message instead of being embedded by another model or by fake vectors; the next sync tries them again.

**Why this priority**: A silent switch mixes vector spaces or fills the index with meaningless vectors, and the user never learns that semantic search is broken (BUG-019 was one such case).

**Independent Test**: Choose a cloud embedding model without its key and sync: the sync stops before embedding with a message naming the model, the cause and the fix; no chunk with another model is written; adding the key and syncing again indexes the files.

**Acceptance Scenarios**:

1. **Given** a chosen model that cannot be built (missing key, missing model file), **When** a sync starts, **Then** it stops before embedding, names the model, the cause and the fix, and leaves the existing index as it was.
2. **Given** a provider that fails for some files during a sync, **When** the sync ends, **Then** those files are reported as failed and are retried on the next sync; files already embedded keep their vectors.
3. **Given** the query side cannot embed the question, **When** the user asks, **Then** the answer uses lexical search only, as today, and the failure is logged; a query is never embedded by a different model than the one recorded for the workspace.
4. **Given** the product build, **When** any embedding model is resolved, **Then** the mock provider is never chosen; it remains only for tests that ask for it explicitly.

---

### User Story 4 - Each model gets its own query and document prefixes (Priority: P2)

Models that expect instruction prefixes (for example `query: ` and `passage: ` for e5, task prefixes for `gemini-embedding-2`) receive them, taken from the catalog, both at ingestion and at query time, so retrieval quality is not left on the table.

**Why this priority**: It changes quality, not availability; it must land together with the first local model because e5 is trained with prefixes and changing them later forces another reindex.

**Independent Test**: For each catalog model, check that the text sent at ingestion carries the document prefix and the text sent for a question carries the query prefix; change the prefixes of a model and check that its workspaces are marked for reindex.

**Acceptance Scenarios**:

1. **Given** a catalog model with a document prefix, **When** a chunk is embedded, **Then** the prefix is added to the embedded text only (the stored chunk text and the lexical index are unchanged).
2. **Given** a catalog model with a query prefix, **When** a question is embedded, **Then** the prefix is added to the question text sent to the model.
3. **Given** a model without prefixes (for example BGE-M3 or OpenAI), **When** text is embedded, **Then** it is sent as is.
4. **Given** a change of a model's prefixes in the catalog, **When** the next sync runs, **Then** the workspaces embedded with the previous prefixes are reindexed, because their vectors belong to another space.

---

### User Story 5 - The benchmark picks the default of each profile and gates dense retrieval in CI (Priority: P2)

A developer runs the retrieval benchmark (IMP-065) with each candidate local model, compares them with the lexical-only baseline and with each other, and the default of each profile is set from those numbers. CI then runs the benchmark with the dense retriever on, with the default local model, and fails on a regression.

**Why this priority**: It turns the model choice into a measured decision and keeps dense retrieval from regressing unnoticed; it depends on stories 1 and 4.

**Independent Test**: Run the benchmark with each candidate and check that each run reports its configuration and metrics; check that the CI job runs with the dense retriever on and fails when the dense retriever is deliberately broken.

**Acceptance Scenarios**:

1. **Given** a candidate local model, **When** the developer runs the benchmark with it, **Then** the run uses the product's real local embedding path (no API key, no network beyond the verified download) and records the model and its prefixes in the run configuration.
2. **Given** the runs of all candidates, **When** the defaults are chosen, **Then** the comparison (quality, cross-language quality, indexing time, model size and memory) is recorded in the plan's research notes, and each local profile names its default model in the catalog.
3. **Given** a pull request that touches retrieval, ingestion, storage or the ONNX code, **When** CI runs, **Then** the benchmark runs with lexical, dense (the default local model) and the reranker, against its own committed baseline, with the same 0.02 rule as today.
4. **Given** the dense configuration, **When** its results are compared with the lexical-only baseline, **Then** they are reported as not comparable, never mixed.

---

### Edge Cases

- A workspace indexed before this feature with a cloud model (OpenAI or Gemini) and a working key: the upgrade adopts that model as the embedding setting, so nothing is reindexed by surprise; the user can switch to the local default when they want.
- A workspace indexed before this feature with the mock target (no key at the time): its vectors carry no meaning, so it is reindexed with the profile's local model on its next sync, and the user is told why.
- The previously recorded cloud model has lost its key: the sync stops with the missing-key message (story 3), it does not move to the local model by itself.
- Text longer than the model's input limit (512 tokens for e5): the input is truncated to the model limit and the number of truncated chunks is reported in the sync summary; chunk sizing by tokens is IMP-023.
- The ONNX Runtime library cannot be loaded on the machine: the local model is reported as unavailable with the cause, as for a missing model file (story 3).
- A machine with a GPU: the local model may run on the accelerator the existing device selection picks; results must stay comparable with the CPU (same model, same space).
- A file with the right name but the wrong SHA-256 in the models directory or on the mirror: never loaded; the sync stops and names the file and the expected checksum.
- The `Global` (system help) workspace: embedded with the same embedding model as user workspaces and kept out of dense search while its vectors belong to another model, as today.
- Two embedding models with the same dimension: the identity of the vector space is the whole target (provider, model, dimension and prefixes), never the dimension alone.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The model catalog MUST contain, for each embedding model: id, display name, runtime (local ONNX or a cloud provider), license, output dimension, maximum input length, pooling (mean or CLS) and normalization, query prefix, document prefix and, for local models, the files with pinned URL, size and SHA-256.
- **FR-002**: The catalog MUST contain at least `multilingual-e5-small`, `multilingual-e5-base` and `BGE-M3` as local models, plus the cloud models already supported (`text-embedding-3-small`, `gemini-embedding-2`, `nomic-embed-text` via Ollama).
- **FR-003**: Every profile MUST name a default local embedding model, and that model MUST be part of the profile's kit (counted in its disk size and installed with it); `Lite` uses `multilingual-e5-small`.
- **FR-004**: A local embedding provider MUST run the catalog's local models on ONNX Runtime through the existing ONNX layer: tokenize, run the model, pool as the catalog says, L2-normalize, and return one vector per input in input order, in batches.
- **FR-005**: The embedding model MUST be an application-wide setting stored in `settings.db`, independent of the chat model: the chat model, the `/model` command and the CLI model flag MUST NOT change it.
- **FR-006**: When the setting is absent, the embedding model MUST be the default of the active profile; on upgrade, an installation whose workspaces were indexed with a real (non-mock) model MUST adopt that model as the setting, so the upgrade does not reindex by itself.
- **FR-007**: The user MUST be able to see the embedding model in use and change it from the CLI and the TUI; a change MUST warn that affected workspaces will be reindexed, and choosing a cloud model MUST say that document text is sent to that provider.
- **FR-008**: The identity of a vector space (the embedding target) MUST include the provider, the model, the dimension and the prefix scheme; a change of any of them MUST trigger the existing reindex rule (ADR-0138) for the affected workspaces.
- **FR-009**: Every stored chunk MUST record the embedding target that produced its vector, and dense search MUST only compare a question vector with chunks of the same target.
- **FR-010**: Ingestion MUST NOT fall back to another model or to the mock provider: when the chosen model cannot be built, the sync MUST stop before embedding with a message that names the model, the cause and the fix; when it fails for some files, those files MUST be reported as failed and retried on the next sync.
- **FR-011**: The mock provider MUST NOT be selectable by the product's resolution; it MUST remain available only to tests that request it explicitly.
- **FR-012**: The pipeline MUST apply the catalog's document prefix to the text it embeds at ingestion and the query prefix to the question it embeds at query time; the stored chunk text and the lexical index MUST NOT contain the prefixes.
- **FR-013**: There MUST be no lexical-only indexing mode: when no embedding model can be made available, the sync MUST stop before indexing with a message naming the model, the source it would be downloaded from and how to install it by hand.
- **FR-014**: Local model files MUST be obtained only through the existing verified download (pinned revision, SHA-256 check) or from files already present in the models directory that pass the same check; a model that fails verification MUST NOT be loaded.
- **FR-018**: The model source MUST be configurable per installation (a mirror base URL in `settings.db`, overridable by an environment variable): when set, every model download goes to the mirror with the catalog's file paths and checksums, and never to the public upstream.
- **FR-019**: The resolution MUST ask a single "is this model allowed?" check before building any embedding model; in this release it allows every catalog model, and the organization policy (IMP-056, v2.0.0) plugs into it later. A refused model MUST stop the sync with the reason, never be swapped.
- **FR-015**: The development benchmark MUST accept a local embedding model as its dense configuration and run it through the product's real embedding path, with no API key.
- **FR-016**: CI MUST run the benchmark with the dense retriever on, using the default local model of the reference profile, against a committed baseline of that configuration, with the existing regression rule; the lexical-only configuration MAY keep running as a second configuration.
- **FR-017**: The comparison of the candidate models (quality overall, per language and cross-language, indexing time, model size, peak memory) MUST be recorded with the plan, and the profile defaults MUST follow it.

### Key Entities

- **Embedding model (catalog entry)**: id, runtime, license, dimension, input limit, pooling, normalization, query prefix, document prefix, files (local) or provider (cloud), profiles it is the default for.
- **Embedding target**: the identity of a vector space: provider, model, dimension, prefix scheme; stored as a stable key.
- **Embedding setting**: the application-wide choice of embedding model in `settings.db`, separate from the chat model.
- **Workspace embedding record**: the target that built a workspace's vectors (exists today, ADR-0138), used for the reindex decision.
- **Chunk embedding record**: the target stored on each chunk, used to keep dense search inside one vector space.
- **Model source**: where model files come from: the catalog's upstream (default), an internal mirror base URL, or files already in the models directory; the checksums are the catalog's in every case.
- **Benchmark dense configuration**: a run configuration with the lexical retriever, the dense retriever with a named local model, and the reranker, with its own baseline.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a machine with no provider keys, a local profile indexes the benchmark corpus with the dense retriever on, and no network request other than the one-time verified model download happens during sync and query.
- **SC-002**: With the chosen default local model, the benchmark's cross-language MRR@10 is clearly above the lexical-only baseline (0.42 today), and MRR@10 and HitRate@5 overall are not below the lexical-only baseline by more than the 0.02 rule.
- **SC-003**: Changing the chat model triggers zero re-embedded chunks on the next sync.
- **SC-004**: In every failure case of story 3, zero chunks are written with a model other than the chosen one, and the message names the model, the cause and the fix.
- **SC-005**: Every pull request that changes retrieval, ingestion, storage or the ONNX code shows the dense benchmark comparison in its CI summary, and a deliberately broken dense retriever (for example prefixes swapped) fails the job.
- **SC-006**: Indexing the benchmark corpus with the reference profile's default model takes under 5 minutes on a CI runner, excluding the first model download.

## Assumptions

- Candidate files are the INT8 ONNX exports with a pinned revision on Hugging Face (about 118 MB for e5-small, 278 MB for e5-base, 568 MB for BGE-M3); the FP32 exports are measured only if INT8 loses quality in the benchmark.
- Proposed defaults before measurement: e5-small for `Lite` and `SmartBalanced`, e5-base for `AirGappedPower`, BGE-M3 for `WorkstationPro` and `SovereignUltra`. A bigger model becomes a default only when it beats the smaller one by more than 0.01 in MRR@10 or HitRate@5 (overall or cross-language); otherwise the smaller one is kept.
- BGE-M3 is used for dense vectors only; its sparse output is FEAT-068, out of scope. Its input is capped at the same length as the other candidates for now; longer chunks come with IMP-023.
- The current single LanceDB table and its zero-padding to 1536 dimensions stay (per-target tables are IMP-033, with IMP-040); the chunk-level target record is what keeps vector spaces apart until then.
- Embedding batches stay as today (one call per file); token-sized batches and parallelism are IMP-030.
- The mock provider stays in the code for tests only; the benchmark's lexical-only configuration keeps using its "no dense" mode, not mock vectors.
- Organization governance is out of scope here and stays in v2.0.0: the allowed-models policy (IMP-056) and the Admin Hub mirror (FEAT-143) reuse the check of FR-019 and the model source of FR-018. Models chosen by an organization outside the catalog come with IMP-056.
- The installed ONNX Runtime and the existing device selection (CPU or accelerator) are reused; no new native dependency is added.
