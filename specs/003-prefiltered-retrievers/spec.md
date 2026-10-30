# Feature Specification: Pre-filtered dense and lexical retrievers

**Feature Branch**: `feat/003-prefiltered-retrievers`

**Created**: 2026-10-30

**Status**: Draft

**Input**: User description: "IMP-040: dense and lexical retrievers run in parallel for each question or sub-question with filters inside the search, over child chunks, with the model's query prefix and the table of the embedding target (the old table answers during a model switch) and weighted lexical fields; 30 candidates per list, tuned by the evaluation set. Only the workspace is searched. A question embedding failure gives lexical-only search with a discreet warning in the answer (today only a log line)."

**Backlog**: IMP-040 (plan `imp_040_prefiltered_retrievers`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 4, Search.

## Context

Today one function does the whole search of a question (`NativeHybridPipeline::search_single_async`): it runs the file-name and date metadata lookups, then embeds the question and runs the dense search, then runs BM25, one after the other. The only filters are the workspace, the embedding target of the question (ADR-0154) and the exclusion of `Global` from dense search (BUG-019). Both retrievers already apply them before taking their candidates: LanceDB receives them as a pre-filter and BM25 skips other workspaces while scoring. There is no way to ask for anything narrower (a source, a date range, a content type), so FEAT-088 (`@source`, dates) and IMP-047 (sources turned off) would have to filter after the candidates are taken, which the design forbids because it can leave zero. Fast takes 30 candidates per list and each Deep sub-question 40. When the question cannot be embedded, search goes on with BM25 only and the cause appears only in the status report and the log; the answer says nothing.

Several parts of the IMP-040 description depend on items that are still open, and stay with them:

| Part of IMP-040 | Item that delivers it | Why |
|---|---|---|
| The table of the embedding target, the old table answering during a model switch | IMP-033 | There is still one table per installation; the target filter already keeps vector spaces apart. |
| Search over child chunks | FEAT-036 | There are no parent chunks yet: every stored chunk is what will become a child. |
| Weighted lexical fields (header lower than the body) | IMP-026, FEAT-069 | The header does not exist as a separate field yet (the envelope is still indexed), and the lexical engine moves to Tantivy. |
| Only the workspace is searched (no `Global`) | FEAT-082 | `Global` holds AnyContext's own help; it leaves every search when help becomes its own tool. |
| `@source` and multilingual dates | FEAT-088 | They produce the filters; this feature makes the retrievers apply them. |

Decided with Levi on 2026-10-30: the core of IMP-040 now; the parts above stay with the items that own them. Plan approved the same day.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A narrowed search never comes back empty because of the narrowing (Priority: P1)

A question is restricted to part of the workspace (one source, a date range, a content type). The candidates of both retrievers come only from that part, and as long as the part holds matching chunks, they are found, however many better matches exist elsewhere in the workspace.

**Why this priority**: It is the mechanism FEAT-088 (`@source`, dates), IMP-047 (sources off) and FEAT-169 (deactivated sources) build on. Filtering after the candidates are taken loses the narrow part whenever the rest of the workspace outranks it.

**Independent Test**: Index a workspace where one small source holds a few weak matches and a large source holds many strong ones; search with a filter on the small source and check that both lists return only, and all of, the small source's matches up to the list size.

**Acceptance Scenarios**:

1. **Given** a workspace with sources A and B, **When** a question is searched with a filter on source A, **Then** every candidate of the dense and of the lexical list belongs to A, and A's best matches are present even when B has more than 30 better ones.
2. **Given** a filter on a modification date range, **When** the question is searched, **Then** only chunks of files modified inside the range are candidates.
3. **Given** a filter on content types, **When** the question is searched, **Then** only chunks of those types are candidates.
4. **Given** a filter that matches no chunk, **When** the question is searched, **Then** both lists are empty: nothing from outside the filter is returned to fill them.
5. **Given** a request without filters, **When** the question is searched, **Then** the results are the same as today (workspace, embedding target and `Global` rules unchanged).

---

### User Story 2 - The two retrievers run at the same time (Priority: P2)

For each question, and for each Deep sub-question, the dense and the lexical retriever start together, so the search takes about as long as the slower of the two, not their sum. Each one returns at most 30 candidates, the same for Fast and Deep.

**Why this priority**: The design's latency budget (step 4) and the Deep fan-out depend on it, but answers are already correct without it.

**Independent Test**: Run the development retrieval benchmark (IMP-065) and compare retrieval latency (mean and p95) with the baseline; check in a unit test that the lexical search does not wait for the question embedding.

**Acceptance Scenarios**:

1. **Given** a question, **When** it is searched, **Then** the lexical search starts without waiting for the question embedding or the dense search.
2. **Given** a Deep turn with several sub-questions, **When** they are searched, **Then** every sub-question runs both retrievers, and the sub-questions run concurrently as today.
3. **Given** the product defaults, **When** a Fast or Deep request is built, **Then** each list takes 30 candidates; the benchmark records 30 against 50 so the value can move with data.

---

### User Story 3 - The answer says when semantic search was off (Priority: P2)

When the question cannot be embedded (model missing, files not installed, provider error, vector search error), the answer still comes from the lexical search, and it ends with one discreet line saying that it was found by words only, why, and what to do. A user never has to open the status report to learn that half of the search did not run.

**Why this priority**: Silent degradation was BUG-019's lesson; the status line (IMP-003) is easy to miss.

**Independent Test**: Configure a workspace whose embedding model cannot run, ask a question in the TUI, the one-shot CLI and Deep mode, and check the warning line on each answer; ask with a working model and check there is no line.

**Acceptance Scenarios**:

1. **Given** a workspace whose embedding model cannot run, **When** the user asks a question, **Then** the answer is produced from the lexical results and ends with a single warning line naming the cause and the fix (for example "Answered by keyword search only: the embedding model multilingual-e5-base is not installed. Run `actx embedding status`.").
2. **Given** a query embedding that fails at query time (provider error or timeout), **When** the user asks, **Then** the same warning appears with the error in short form.
3. **Given** the nothing-found answer and a failed embedding, **When** the user asks, **Then** the nothing-found answer also carries the warning, because the missing dense list may be why nothing was found.
4. **Given** a working embedding model, **When** the user asks, **Then** no warning is shown.
5. **Given** a Deep turn where some sub-questions embedded and others failed, **When** the answer is shown, **Then** the warning appears once.

---

### Edge Cases

- A filter value with quotes or SQL wildcards (`'`, `%`, `_`) is matched literally and cannot change the filter expression.
- A source scope given as a folder matches files under it, not files whose path only starts with the same characters (`docs` does not match `docs-old`).
- A date range with only a start or only an end is open on the other side; a range whose start is after its end matches nothing.
- Chunks without a modification date never match a date filter.
- A lexical index entry whose stored row is gone (index out of step) is dropped as today, not shown with text from another workspace.
- Old BM25 files keep working: the filter reads only fields the lexical index already stores (workspace, file path, content type, modification date).
- With `Global` still searched (until FEAT-082), a source, date or type filter applies to `Global` rows too, so a narrowed question does not pull in help pages.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each retrieval request MUST carry a filter with the workspace (required) and, optionally, source scopes (folders, files or web roots), a modification date range and content types.
- **FR-002**: The dense retriever MUST apply the filter, the question's embedding target and the `Global` rule inside the vector search, before the candidate limit.
- **FR-003**: The lexical retriever MUST apply the same filter before ranking, so its candidate limit counts only chunks that pass it.
- **FR-004**: Both retrievers MUST treat filter values as data: no value can widen or break the filter expression.
- **FR-005**: A filter that matches no chunk MUST give empty lists, and the pipeline MUST NOT fill them from outside the filter.
- **FR-006**: For each question and each Deep sub-question, the lexical search MUST start without waiting for the question embedding or the dense search.
- **FR-007**: Fast and Deep requests MUST take 30 candidates per list by default, defined once in the product's request builders.
- **FR-008**: The dense retriever MUST keep embedding the question with the model's query prefix (IMP-029) and searching only its embedding target (ADR-0154).
- **FR-009**: When the dense half does not run (no provider, model unavailable, embedding failed, vector search failed), the search MUST still return the lexical results and MUST report the cause to the caller with the results.
- **FR-010**: The TUI, the one-shot CLI and Deep mode MUST show that cause as one warning line at the end of the answer, including the nothing-found answer, once per answer, and never when the dense half ran.
- **FR-011**: The status report MUST keep its dense search line (IMP-003).
- **FR-012**: The development retrieval benchmark MUST show no regression beyond its threshold, and its results with the new defaults MUST be saved as the baseline.

### Key Entities

- **Retrieval filter**: the workspace and optional source scopes, date range and content types of one request; turned into a LanceDB predicate and a lexical predicate.
- **Retriever list**: the ordered candidates one retriever returns for one question, at most 30.
- **Dense outcome**: how the dense half of a search went (ran, no provider, unavailable, failed, search failed); returned with the results and turned into the warning line.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a filter on a source holding 3 matching chunks in a workspace where another source holds 100 better ones, both lists return the 3 chunks (today's code without a filter returns none of them in the top 30).
- **SC-002**: Benchmark MRR@10 and HitRate@5 do not drop beyond the regression threshold with the dense retriever on and with lexical only.
- **SC-003**: Benchmark retrieval latency p95 does not grow; with the dense retriever on, the mean drops by at least the lexical search time.
- **SC-004**: 100% of answers given while the embedding model cannot run carry the warning line, and 0% of answers with a working model do.

## Assumptions

- Source scopes are matched on the stored file path (local files) or URL (web pages) under the workspace; the user-facing syntax that produces them is FEAT-088's.
- The lexical index already stores the workspace, file path, content type and modification date of each chunk, so no reindex is needed.
- File-name and date mentions keep today's fixed-score metadata lists until FEAT-088 replaces them with filters and IMP-041 with clean lists.
- Lexical statistics (document frequencies) stay global to the BM25 file until the lexical engine moves (FEAT-069).
