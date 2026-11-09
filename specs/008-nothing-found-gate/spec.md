# Feature Specification: Nothing-found and weak-evidence gate by relevance bands

**Feature Branch**: `feat/008-nothing-found-gate`

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "IMP-046: no relevant chunk in automatic Fast escalates to Deep; nothing after Deep, or with Fast pinned, answers that nothing was found without calling the LLM, suggesting rephrasing, adding sources or @source; only maybe-band chunks give an answer with an explicit weak-evidence warning. Replaces today's gate on the RRF score."

**Backlog**: IMP-046 (plan `imp_046_nothing_found_gate`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 7, Selection, item 5, and step 2, Routing (automatic mode escalates on insufficient evidence).

## Context

After IMP-044 and IMP-045 every chunk that reaches the answer was judged by the cross-encoder: it is in the relevant band, in the maybe band, or a pinned file-name match (which has no band). Irrelevant chunks are already discarded. The answer path still ignores this:

- Fast answers "nothing found" only when no chunk scores at least 0.005 (`evaluate_zero_hallucination`), a cut chosen for RRF scores. After the reranker it only fires when the selection is empty.
- An answer made only of maybe-band chunks looks exactly like a well-grounded answer.
- Deep never stops before the synthesis: with no chunk at all it still calls the LLM to synthesize from nothing.
- In automatic mode a question routed to Fast that finds no relevant chunk stays in Fast.

Parts of the design that stay with their items:

| Part | Item that delivers it | Why |
|---|---|---|
| Automatic mode always starts Fast (no keyword routing), escalation when the LLM says the context is not enough | IMP-038 | IMP-046 adds the band trigger to the existing automatic route; removing the keyword classifier and the LLM trigger is IMP-038's acceptance. |
| `@source` in the question | FEAT-088 | `@source` does not exist yet; the nothing-found suggestions name it when it lands. |
| Warnings rendered in the streamed answer with visible progress | IMP-050 | IMP-046 sends the warning before the answer on today's surfaces. |
| Per-workspace refinement of the thresholds | IMP-008 | The gate reads the bands; their thresholds come from the catalog (ADR-0158). |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nothing relevant means an honest "nothing found", without the LLM (Priority: P1)

The user asks something the workspace does not hold, in Fast pinned (`/search fast` or `/fast`). No chunk survives the relevance cut, so AnyContext answers that nothing was found, without calling the LLM, and suggests what to do: rephrase the question, add the sources that hold the answer.

**Why this priority**: It is the core of the gate: no LLM call and no invented answer when there is no evidence.

**Independent Test**: With a retriever that returns no chunk, ask in Fast pinned and check the answer is the nothing-found message and the LLM was not called.

**Acceptance Scenarios**:

1. **Given** Fast pinned and an empty selection, **When** the user asks, **Then** the answer says nothing was found in the workspace, lists the workspace's monitored sources, suggests rephrasing and adding sources with `/folder` or `/web`, and the LLM is not called.
2. **Given** Deep (pinned or reached by escalation) and no chunk collected in any turn, **When** the turns end, **Then** the same nothing-found answer is given and the synthesis call is not made.
3. **Given** the nothing-found answer, **When** it is stored, **Then** the session history keeps the question and the message, as today.

---

### User Story 2 - Weak evidence is said up front (Priority: P1)

When every chunk that reaches the answer is in the maybe band, AnyContext still answers from them, but the answer starts with an explicit warning that no excerpt clearly answers the question, and the LLM is told the excerpts are only possibly related.

**Why this priority**: A maybe-only answer is the case where an invented or stretched answer is most likely; the user must know.

**Independent Test**: With a retriever whose chunks are all in the maybe band, ask in Fast pinned and in Deep and check the warning comes before the answer and the prompt carries the weak-evidence rule.

**Acceptance Scenarios**:

1. **Given** Fast pinned and only maybe-band chunks, **When** the user asks, **Then** the LLM is called once, the warning is sent before the first answer token, and the returned answer starts with it.
2. **Given** Deep and only maybe-band chunks after the last turn, **When** the synthesis runs, **Then** the same warning comes first.
3. **Given** at least one relevant-band chunk or one pinned file-name match, **When** the user asks, **Then** no warning is shown, even if the other chunks are maybe.
4. **Given** the warning, **When** the answer is stored, **Then** the session history keeps the answer without the warning (like the end-of-answer notices).

---

### User Story 3 - Automatic Fast escalates to Deep when nothing relevant is found (Priority: P1)

In automatic mode (`/search auto`, the default), a question routed to Fast that finds no relevant chunk (nothing at all, or only maybe-band chunks) is not answered from what Fast found: AnyContext moves to Deep, with a discreet indicator, and Deep then answers, or gives the nothing-found or weak-evidence answer of User Stories 1 and 2.

**Why this priority**: Deep refines and decomposes the question; a Fast miss is often a phrasing miss.

**Independent Test**: With a Fast retriever that finds nothing relevant and a Deep retriever that finds a relevant chunk, ask in automatic mode and check that the escalation event is sent and the answer comes from Deep.

**Acceptance Scenarios**:

1. **Given** automatic mode, a Fast route and an empty selection, **When** the user asks, **Then** an escalation indicator is sent and Deep runs.
2. **Given** automatic mode, a Fast route and only maybe-band chunks, **When** the user asks, **Then** Deep runs (the Fast LLM call is not made).
3. **Given** Fast pinned, **When** nothing relevant is found, **Then** there is no escalation (User Stories 1 and 2 apply).
4. **Given** automatic mode without a Deep engine (no LanceDB at start), **When** nothing relevant is found, **Then** Fast gives its own nothing-found or weak-evidence answer.
5. **Given** a live status question, **When** it is answered, **Then** the gate and the escalation do not apply (the status report is always context, ADR-0145).

---

### User Story 4 - Without bands, the answer is not judged (Priority: P2)

When the reranker could not run (the Lite profile until IMP-055, missing weights or runtime) or its model has no thresholds, chunks have no band. AnyContext cannot tell weak from strong evidence, so it answers normally from them (with today's "Answered without relevance ranking" notice) and does not escalate; only an empty selection gives the nothing-found answer.

**Why this priority**: The answer must keep working without the reranker, without warning on every question.

**Independent Test**: With chunks that carry no band, ask in automatic and Fast pinned mode and check there is no warning, no escalation and one LLM call.

**Acceptance Scenarios**:

1. **Given** chunks without a band, **When** the user asks, **Then** the answer has no weak-evidence warning and no escalation.
2. **Given** no chunk at all, **When** the user asks, **Then** the nothing-found path applies whatever the reranker state.

---

### User Story 5 - The benchmark shows the gate (Priority: P3)

The retrieval benchmark reports, for the delivered pass, how many negative questions would get the weak-evidence answer instead of nothing found, and how many positive questions would be answered with the warning.

**Why this priority**: It measures the gate on fixed data; per-workspace tuning is IMP-008.

**Independent Test**: Run the metrics on fixture results with maybe-only, relevant and empty deliveries and check the two rates.

**Acceptance Scenarios**:

1. **Given** a run, **When** the summary is written, **Then** it shows the weak-evidence rate of the negatives and of the positives, reported and not gated.

### Edge Cases

- Pinned file-name matches only (no unpinned candidate): sufficient evidence; the user named the file.
- Mixed band-less and maybe chunks cannot happen in one pass (bands are set for all scored chunks or none); a band-less chunk counts as not weak.
- Deep collects chunks over several turns: the evidence is judged on all collected chunks at the synthesis.
- An escalated Deep run reports its own notices (dense search, reranker); the Fast pass's notice is not repeated.
- The agent's `search_db` tool (ReAct path, removed by IMP-037) keeps returning the nothing-found message on an empty result, with the new wording.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The evidence of a retrieval MUST be classified from the bands of the delivered chunks: nothing (no chunk), weak (every chunk in the maybe band), sufficient (any other non-empty case: a relevant chunk, a pinned match, or chunks without a band).
- **FR-002**: The score cut of 0.005 of the old gate MUST be removed; the gate reads only the classification of FR-001.
- **FR-003**: With nothing, Fast (pinned, or automatic without a Deep engine) and Deep MUST answer with the nothing-found message without calling the LLM for the answer.
- **FR-004**: The nothing-found message MUST say nothing was found in the workspace, name its monitored folders and web sources, and suggest rephrasing and adding sources with `/folder` and `/web`.
- **FR-005**: With weak evidence, the answer MUST start with the weak-evidence warning, sent as its own event before the first answer token and kept out of the session history; the answer prompt MUST carry a rule that the excerpts are only possibly related.
- **FR-006**: In automatic mode, a Fast route whose evidence is nothing or weak MUST escalate to Deep when a Deep engine is configured, sending an escalation event first; Fast pinned and Deep pinned MUST NOT escalate.
- **FR-007**: The live status route MUST NOT be gated or escalated.
- **FR-008**: The benchmark MUST report the weak-evidence rate of negative and positive questions in the delivered pass, without gating it.

### Key Entities

- **Evidence**: nothing, weak or sufficient, derived from the delivered chunks' bands.
- **Weak-evidence warning**: a fixed line placed before the answer when the evidence is weak.
- **Escalation**: the move from Fast to Deep in automatic mode, with its reason.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In tests, no nothing-found answer makes an answer LLM call, in Fast or Deep.
- **SC-002**: In tests, every weak-only answer starts with the warning, and no answer with a relevant or pinned chunk has it.
- **SC-003**: In tests, automatic mode escalates on nothing and on weak evidence, and pinned modes never escalate.
- **SC-004**: The benchmark's gated metrics (MRR@10, HitRate@5, delivered hit rate) are unchanged: retrieval itself does not change.

## Assumptions

- The warning and the nothing-found message are fixed English text like today's notices; the answer language rule applies to LLM answers only.
- The automatic route keeps today's classifier until IMP-038; the band trigger is added to it.
- The band thresholds are the catalog's (ADR-0158).
