# Feature Specification: Development retrieval benchmark on a fixed corpus

**Feature Branch**: `feat/001-retrieval-benchmark`

**Created**: 2026-10-25

**Status**: Draft

**Input**: User description: "IMP-065: a development-only retrieval benchmark on a fixed, versioned corpus (starting with the fictitious CMR) and a hand-labeled question file (expected documents and passages, plus negative questions that must get the nothing-found answer); a runner over the real retrieval pipeline that reports HitRate@1/3/5/10, MRR@10, NDCG@10, the nothing-found rate and retrieval latency (mean and p95); results saved as JSON in the repository and compared with a previous run, so each retrieval PR shows its effect. Runs locally and in CI when the retrieval code changes, with a regression threshold. Out of scope: user-facing commands, synthetic question generation (FEAT-073) and answer evaluation (FEAT-102)."

**Backlog**: IMP-065 (plan `imp_065_retrieval_benchmark`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 10, History and evaluation.

## Context

The R1 retrieval rework (IMP-040 to IMP-046) and the R1 foundations that change what is retrieved (token-sized chunks IMP-023, the contextual header IMP-026, the embedding model IMP-027 and FEAT-067, per-target tables IMP-033) set defaults that the v2 design says are tuned by measurement. The per-workspace evaluation set (FEAT-073, FEAT-106) only arrives in v1.1.0, so today none of these changes can show its effect on retrieval quality. This feature gives developers that measurement first.

Decided with Levi on 2026-10-25:

- Order: this benchmark first, then FEAT-067 together with IMP-027 and IMP-029; that later plan picks the local embedding model by this benchmark and turns the dense retriever on in CI. IMP-040 comes after both.
- Until FEAT-067 there is no local embedding model and CI has no provider keys: CI measures the lexical retriever plus the local cross-encoder reranker, without the dense retriever. Locally, the developer can include the dense retriever with the embedding provider they have configured.
- Corpus of about 30 to 40 fictitious documents in Portuguese and English, with 60 to 80 hand-labeled questions, about 15 of them negative.
- Regression rule: the run fails when MRR@10 or HitRate@5 drops more than 2 points (0.02) below the committed baseline. Latency is reported, never gated.

## User Scenarios & Testing *(mandatory)*

The users of this feature are AnyContext developers (and the coding agents working for them), not end users.

### User Story 1 - Measure retrieval quality on a fixed corpus (Priority: P1)

A developer runs the benchmark on their machine and gets, for the current code, the retrieval quality on a known corpus with known right answers: HitRate@1, @3, @5 and @10, MRR@10, NDCG@10, the nothing-found rate on the negative questions and the retrieval latency (mean and p95), overall and per language.

**Why this priority**: Without the measurement nothing else in this feature has value; it is the baseline that every R1 retrieval and ingestion change is compared with.

**Independent Test**: On a clean checkout, run the benchmark once and check that it indexes the corpus from scratch, answers every question and prints every metric, with a per-question detail that names the expected and the retrieved documents.

**Acceptance Scenarios**:

1. **Given** a clean checkout with the corpus and the question file, **When** the developer runs the benchmark, **Then** it builds a fresh index of the corpus in a temporary location (never the developer's own workspaces), runs every question through the real retrieval pipeline and reports all the metrics above.
2. **Given** a question whose expected document is retrieved at position 3, **When** the run finishes, **Then** that question counts as a miss for HitRate@1, a hit for HitRate@3, @5 and @10, and contributes 1/3 to MRR@10.
3. **Given** a negative question (its answer is not in the corpus), **When** the pipeline returns no result above its own cut, **Then** it counts as nothing-found; negative questions never count for HitRate, MRR or NDCG.
4. **Given** the developer has an embedding provider configured, **When** they ask the benchmark to include the dense retriever, **Then** the run says which retrievers and which models it used, and results with and without dense are never compared with each other.

---

### User Story 2 - See the effect of a change against the previous run (Priority: P1)

A developer changing retrieval or ingestion code sees how each metric moved compared with the committed baseline, and can save the new results as the baseline when the change is accepted.

**Why this priority**: The point of the benchmark is that "each retrieval PR shows its effect"; a number alone does not say whether a change helped.

**Independent Test**: Run the benchmark, change one retrieval default (for example the candidate pool size), run again and check that the comparison shows the delta of every metric and lists the questions that changed from hit to miss and from miss to hit.

**Acceptance Scenarios**:

1. **Given** a committed baseline result file, **When** the benchmark runs, **Then** it reports, per metric, the baseline value, the new value and the delta, plus the questions that changed between hit and miss.
2. **Given** a run whose configuration (retrievers, models, corpus version or question file version) differs from the baseline's, **When** the comparison runs, **Then** it says the runs are not comparable instead of reporting deltas.
3. **Given** an accepted change, **When** the developer asks to update the baseline, **Then** the new result file replaces the committed one, and the diff of that file shows the effect in the pull request.

---

### User Story 3 - CI blocks a retrieval regression (Priority: P2)

When a pull request changes retrieval, ingestion or storage code, CI runs the benchmark and fails if quality dropped beyond the threshold, so a regression cannot be merged unnoticed.

**Why this priority**: The local run already gives the measurement; CI makes it a rule. It depends on stories 1 and 2.

**Independent Test**: Open a pull request that deliberately worsens retrieval (for example disabling the reranker) and check that the CI job fails and names the metric and the drop; open one that changes only documentation and check that the job does not run.

**Acceptance Scenarios**:

1. **Given** a pull request that changes retrieval, ingestion or storage code, the corpus or the question file, **When** CI runs, **Then** the benchmark job runs and publishes the comparison in the job summary.
2. **Given** a run where MRR@10 or HitRate@5 is more than 0.02 below the baseline, **When** CI evaluates it, **Then** the job fails with the metric, the baseline value, the new value and the questions that regressed.
3. **Given** a run where latency got worse but quality did not, **When** CI evaluates it, **Then** the job passes and the latency change is only reported.
4. **Given** a pull request that changes none of the watched paths, **When** CI runs, **Then** the benchmark job is skipped.

---

### User Story 4 - Grow the corpus and the questions safely (Priority: P3)

A developer adds documents or questions (for example new formats in R2, or a case found while fixing a bug) and the benchmark checks that the labels still point to real documents and passages.

**Why this priority**: The corpus starts small; it must be easy to extend without silently breaking the labels. Not needed for the first baseline.

**Independent Test**: Add a question that points to a document that does not exist and check that the benchmark refuses to run and names the question.

**Acceptance Scenarios**:

1. **Given** a question whose expected document or passage is not in the corpus, **When** the benchmark starts, **Then** it stops before indexing and names the question and the missing reference.
2. **Given** a change to the corpus or to the question file, **When** the benchmark runs, **Then** the result records the new versions and the comparison with an older baseline reports "not comparable" until the baseline is updated.

---

### Edge Cases

- A question with more than one relevant document (graded relevance): NDCG@10 uses the grades; HitRate and MRR use the first relevant document found.
- The pipeline returns fewer than 10 results: the metrics are computed on what was returned, never padded.
- Two results come from the same document (several chunks): a document is counted once, at its best position.
- A passage label: a result counts as a passage hit only when the retrieved chunk contains the expected passage text; document-level metrics are the gate, passage-level metrics are reported.
- The reranker model cannot be downloaded or loaded: the run fails with a clear message and is never reported as a quality regression; it never silently falls back to a run without the reranker.
- The embedding provider is missing or fails during a run that asked for dense: the run fails and says so; it never switches model or provider silently (as IMP-028 requires for the product).
- No baseline file exists yet (first run): the run reports the metrics and offers to save them as the first baseline; CI fails asking for the baseline to be committed.
- Machine speed varies (CI runners): latency is never part of the pass/fail rule.
- Non-deterministic order on equal scores: the run must give the same quality metrics when repeated on the same machine and code.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The repository MUST contain a fixed, versioned benchmark corpus of fictitious documents (no real personal or customer data), in Portuguese and English, in the formats the current ingestion supports (at least Markdown, plain text and PDF), including the existing fictitious CMR form.
- **FR-002**: The corpus MUST include hard distractors: documents that share vocabulary with the right answer but differ in version, negation, module or date, so that ranking quality, not only keyword presence, is measured.
- **FR-003**: The repository MUST contain a hand-labeled question file in which each question has an id, its language, the question text, the expected documents with a relevance grade, optionally an expected passage, and a category (factual, version or date, form field, cross-language, negative).
- **FR-004**: Negative questions MUST be marked as such and have no expected documents.
- **FR-005**: The benchmark MUST validate the question file against the corpus before running and stop, naming each offending question, when a label points to a missing document or passage.
- **FR-006**: The benchmark MUST index the corpus from scratch with the product's real ingestion into an isolated temporary location, and MUST NOT read or write the developer's own workspaces, settings or keys, apart from an embedding provider the developer explicitly chose for a local dense run.
- **FR-007**: The benchmark MUST run every question through the real retrieval pipeline with its current defaults, so that a change to those defaults shows up in the results.
- **FR-008**: The benchmark MUST report, overall and per language and category: HitRate@1, @3, @5 and @10, MRR@10, NDCG@10 (document level), passage HitRate@5, the nothing-found rate on negative questions, the share of positive questions that wrongly got nothing-found, and the retrieval latency mean and p95.
- **FR-009**: The benchmark MUST write its results as a JSON file containing the metrics, the per-question results (expected documents, retrieved documents with positions and scores), and the run configuration: corpus version, question file version, retrievers used, embedding and reranker models, and the code revision.
- **FR-010**: The benchmark MUST compare a run with a baseline result file committed in the repository and report, per metric, baseline value, new value and delta, plus the questions that changed between hit and miss.
- **FR-011**: The comparison MUST refuse to report deltas, saying the runs are not comparable, when the configuration differs (corpus version, question file version, retrievers or models).
- **FR-012**: The benchmark MUST fail when MRR@10 or HitRate@5 is more than 0.02 below the baseline of the same configuration; latency MUST never make it fail.
- **FR-013**: The developer MUST be able to save a run as the new baseline with one explicit action; the benchmark MUST never update the baseline on its own.
- **FR-014**: CI MUST run the benchmark, without the dense retriever until FEAT-067, on pull requests and pushes that change retrieval, ingestion or storage code, the corpus or the question file; it MUST publish the comparison in the job summary and keep the result JSON as a downloadable artifact.
- **FR-015**: CI MUST NOT need any provider key or network service other than downloading the verified reranker model, which MUST be cached between runs.
- **FR-016**: The benchmark MUST NOT add any end-user command, setting or behavior; it is development tooling only and is not shipped in the product.
- **FR-017**: Repeated runs on the same code and configuration MUST give identical quality metrics.

### Key Entities

- **Benchmark corpus**: the set of fictitious documents, with a version that changes whenever a document is added, removed or edited.
- **Question**: id, language, text, category, expected documents with relevance grades, optional expected passage, negative flag.
- **Question file**: the list of questions, with its own version.
- **Run configuration**: corpus version, question file version, retrievers on (lexical, dense, reranker), models used, code revision.
- **Run result**: the run configuration, metrics overall and per language and category, per-question results, latency figures.
- **Baseline**: the run result committed in the repository for a given configuration, against which new runs are compared.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer gets the full metric report with one command on a clean checkout, and the run (indexing plus all questions, without dense) finishes in under 5 minutes on a CI runner, excluding the first model download.
- **SC-002**: Two consecutive runs on the same code and machine give identical quality metrics (zero difference).
- **SC-003**: A deliberate regression (reranker off, or the candidate pool cut to 5) is detected: the run fails and names the metric and the regressed questions.
- **SC-004**: Every R1 pull request that changes retrieval, ingestion or storage shows the benchmark comparison in its CI summary, starting with the first pull request after this feature is merged.
- **SC-005**: The corpus has at least 30 documents and the question file at least 60 questions, at least 15 of them negative and at least 20 in each language, all of which pass label validation.
- **SC-006**: No product binary, command or setting changes because of this feature (the benchmark code is not part of the shipped build).

## Assumptions

- "Nothing-found" for the current pipeline means it returned no result after its own cut (today the RRF score floor). The current pipeline has no nothing-found gate yet (IMP-046), so the baseline nothing-found rate is expected to be low; it is reported but not gated until IMP-046 changes it.
- Document-level metrics are the gate because the chunking changes of R1 (IMP-023, FEAT-036) will move passage boundaries; passage metrics are reported only.
- The fictitious documents and questions are written by the agent and reviewed by Levi before the first baseline is committed.
- The reranker used is the product's default verified cross-encoder (mmarco today), downloaded with the existing verified download; CI already provisions the ONNX Runtime library.
- The dense retriever in a local run uses whatever embedding provider the developer configured (Gemini as the reference); those results are kept apart from the CI baseline and are not committed as the gate.
- Synthetic question generation (FEAT-073), answer evaluation (FEAT-102), per-workspace evaluation (FEAT-106) and any user-facing command are out of scope.
- The later FEAT-067 plan adds a second CI configuration (with local dense) and its own baseline; this feature only has to make that possible, not do it.
