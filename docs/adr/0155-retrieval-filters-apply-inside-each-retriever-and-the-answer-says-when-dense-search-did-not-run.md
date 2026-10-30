---
id: 155
title: Retrieval filters apply inside each retriever, and the answer says when dense search did not run
status: accepted
date: '2026-10-30'
version: v1.0.0
subsystems: [retrieval, agent, cli, tui]
supersedes: []
superseded_by: []
amends: [146]
amended_by: []
---

# ADR-0155: Retrieval filters apply inside each retriever, and the answer says when dense search did not run

## Context

A search could only be narrowed to a workspace (plus the embedding target and the `Global` rule). FEAT-088 (`@source`, dates), IMP-047 (sources turned off) and FEAT-169 (deactivated sources) need narrower searches, and the v2 design (`docs/architecture/retrieval-pipeline-v2.md`, step 4) forbids filtering after the candidates are taken, because the narrow part is lost whenever the rest of the workspace outranks it. The dense search, the metadata lookups and BM25 ran one after the other. When the question could not be embedded, the answer came from BM25 with no word about it: the cause only reached the status report (ADR-0146). IMP-040 asks for all three; the parts that depend on open items stay with them (per-target tables IMP-033, child chunks FEAT-036, weighted lexical fields IMP-026 and FEAT-069, `Global` out of every search FEAT-082), as Levi decided on 2026-10-30 (`specs/003-prefiltered-retrievers/spec.md`).

## Decision

- **One filter, two renderings.** `HybridSearchRequest.filter` is a `RetrievalFilter`: source scopes (a path or URL, or anything under it with either separator), a modification date range (inclusive start, exclusive end, UTC RFC 3339) and content types. `lance_predicate()` renders it as a DataFusion expression joined to the dense search's `only_if` (LanceDB pre-filters before the limit) and to the file-name and date metadata lookups; `matches()` is the same rule for BM25, which skips non-matching documents before scoring. Values are SQL literals with `'` doubled; folder scopes use `file_path = 'x' OR starts_with(file_path, 'x/')`, which takes the prefix literally (LanceDB rejects a LIKE escape character other than the backslash, which is a Windows path separator). A bound that does not parse matches nothing. The default filter narrows nothing, so requests without one behave as before.
- **Retrievers in parallel.** The lexical search starts on a blocking thread before the question is embedded and is joined after the dense search. The pipeline keeps BM25 indexes as `Arc<BM25Index>`, so a search no longer copies the index.
- **30 candidates per list** in Fast and Deep (`retrieval::CANDIDATES_PER_LIST`; Deep took 40).
- **Dense outcome with the results.** `search_with_outcome` and `retrieve_hybrid_batch_with_outcome` return `RetrievalOutput { results, dense }` (for a batch, the first sub-query whose dense half did not run). `QueryEmbeddingOutcome::answer_notice()` turns it into one line, "Answered by keyword search only: <cause>. <fix>".
- **Notice in the answer.** `DeterministicRetrievalOutput` carries `notice`, `DeepSearchRetriever::last_notice` reports it for Deep (default `None`), and the agent sends `AgentEvent::Notice` once, before `Done`; a non-streamed answer ends with it. The TUI shows it as a system line under the answer, the one-shot CLI prints it after the answer. The session history keeps the answer only.

## Alternatives Considered

- **Filter the fused list**: rejected; it can leave zero results while matching chunks exist.
- **A retriever trait with pluggable implementations now**: not taken; two retrievers do not need it, and future ones (design step 4.8) can add it.
- **Notice only in `/status` (ADR-0146)**: kept, but not enough; the line is easy to miss.
- **Notice inside the answer text sent to the session history**: rejected; it would enter the next turn's context.

## Consequences

- FEAT-088, IMP-047 and FEAT-169 only need to fill the filter.
- Deep sub-questions take 30 candidates per list instead of 40 (benchmark results in `specs/003-prefiltered-retrievers/research.md`).
- Users see when an answer came from keyword search only, with the fix.
- BM25 document frequencies stay global to the BM25 file until the lexical engine moves (FEAT-069).
