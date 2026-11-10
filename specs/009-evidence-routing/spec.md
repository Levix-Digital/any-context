# Feature Specification: Automatic mode routes by evidence, not by keywords

**Feature Branch**: `feat/009-evidence-routing`

**Created**: 2026-11-10

**Status**: Draft

**Input**: User description: "IMP-038: automatic mode starts Fast and escalates to Deep when the best chunks have low reranker relevance or the LLM says the context is not enough, with a discreet indicator; the user can pin Fast or Deep. No keyword routing in any language: the PT/EN keyword classifier and the heuristic query classifier are removed."

**Backlog**: IMP-038 (plan `imp_038_evidence_routing`, release v1.0.0 Personal). Design: `docs/architecture/retrieval-pipeline-v2.md`, step 2, Routing.

## Context

IMP-046 added the first evidence trigger: in automatic mode, a Fast route whose selection is empty or holds only maybe-band chunks escalates to Deep. The route itself is still chosen before any search by a keyword and length heuristic (`DeterministicClassifier` in `actx-agent/src/router.rs`, wrapped by the `actx-onnx` query classifier through `OnnxComplexityClassifierAdapter`):

- A question with "arquitetura", "compare", "end-to-end" or more than 25 words goes straight to Deep, even when one excerpt answers it; a short question in a third language never does.
- The intent (code, architecture, devops, data, security, document, general) is shown in the TUI and the one-shot output but drives nothing.
- The classifier's model weights (`bge-small-onnx`, 133 MB) are in the model catalog and in every hardware kit, are downloaded, and are never executed (ADR-0130).
- When Fast finds relevant excerpts that still do not answer the question, nothing escalates: the LLM answers from them.

Parts of the design that stay with their items:

| Part | Item that delivers it | Why |
|---|---|---|
| Live status questions recognized by status terms (`is_status_query`) | FEAT-082 | Status and help become agent tools there; until then the status route keeps its term list, which picks a data source, not a search depth. |
| Tools in Fast, ReAct removed as a mode | IMP-037 | IMP-038 changes how the route is chosen, not what each route can do. |
| Deep turn and token ceilings | IMP-039 | Deep keeps today's turns. |
| Composer selector in the desktop app | FEAT-172 to FEAT-179 | The CLI and TUI already pin with `/search fast`, `/search deep`, `/search auto`. |
| Escalation shown inside the streamed answer with progress | IMP-050 | The indicator stays where IMP-046 put it. |

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automatic mode always starts Fast (Priority: P1)

In automatic mode (the default), every question that is not a live status question starts with the quick search, whatever its words, length or language. Deep runs only when the evidence asks for it (User Stories 2 and 3) or when the user pins it.

**Why this priority**: It is the acceptance of the item: no routing by keywords in any language.

**Independent Test**: Ask in automatic mode questions that the old classifier sent to Deep ("compare the architecture of A and B end-to-end", a 30-word question, a question in Spanish) with a Fast retriever that finds a relevant chunk, and check that Fast answers with one LLM call and Deep does not run.

**Acceptance Scenarios**:

1. **Given** automatic mode and a Fast retriever with sufficient evidence, **When** the user asks any question that is not a status question, **Then** the route is Fast and Deep does not run.
2. **Given** automatic mode, **When** the route is shown, **Then** the indicator says automatic mode started with the quick search, and shows no domain or confidence.
3. **Given** automatic mode without a Fast retriever (no LanceDB at start), **When** the user asks, **Then** the agent behaves as today without one (the tool-calling path).

---

### User Story 2 - The LLM can say the excerpts are not enough (Priority: P1)

In automatic mode, when Fast found relevant excerpts but they do not hold the answer, the LLM answers with a fixed signal instead of an answer. AnyContext then discards that Fast attempt, shows the escalation indicator with the reason, and runs Deep.

**Why this priority**: It is the second trigger of the design; relevant excerpts on the topic often miss the specific fact.

**Independent Test**: With a Fast retriever with sufficient evidence, a mock LLM whose Fast answer is the signal and a Deep retriever, ask in automatic mode and check the escalation event, the Deep answer and that the signal never reaches the user or the history.

**Acceptance Scenarios**:

1. **Given** automatic mode, a Deep engine and sufficient Fast evidence, **When** the Fast answer starts with the signal, **Then** no Fast token is shown, an escalation event with the reason "the quick search excerpts do not answer the question" is sent, and Deep answers.
2. **Given** automatic mode and a Fast answer that does not start with the signal, **When** it streams, **Then** the tokens reach the user as today (held back only until they cannot be the signal).
3. **Given** Fast pinned, or automatic mode without a Deep engine, **When** the Fast prompt is built, **Then** it carries no signal rule, and the LLM says in its own words what the excerpts do not answer.
4. **Given** the signal appears later in an answer, **When** the answer is shown or stored, **Then** the signal text is removed and there is no escalation.
5. **Given** an escalated question, **When** the history is stored, **Then** it keeps the question and the Deep answer only, not the Fast attempt.

---

### User Story 3 - The band trigger keeps working (Priority: P1)

The IMP-046 escalation (nothing found or maybe-band-only evidence in automatic Fast) is unchanged; it is now the first of the two triggers.

**Independent Test**: The IMP-046 agent tests pass unchanged.

**Acceptance Scenarios**:

1. **Given** automatic mode and an empty or weak Fast selection, **When** the user asks, **Then** Deep runs without a Fast LLM call, as in IMP-046.

---

### User Story 4 - Pinned modes are respected (Priority: P1)

`/search fast` and `/search deep` pin the mode: Fast never escalates and Deep always runs, for every question that is not a status question.

**Acceptance Scenarios**:

1. **Given** Deep pinned, **When** the user asks a short question, **Then** Deep runs.
2. **Given** Fast pinned, **When** the evidence is nothing, weak or not enough, **Then** there is no escalation.
3. **Given** any mode and a live status question, **When** it is asked, **Then** the status route answers, as today.

---

### User Story 5 - Unused classifier weights are not downloaded (Priority: P2)

The query classifier weights are removed from the model catalog and from the hardware kits, so a new installation or a kit download no longer fetches 133 MB that are never executed. A file already on disk is left alone and ignored.

**Acceptance Scenarios**:

1. **Given** any hardware kit, **When** its models are listed or downloaded, **Then** the query classifier weights are not among them.
2. **Given** the model menu, **When** it is opened, **Then** there is no query classifier entry.

### Edge Cases

- The LLM writes the signal with surrounding spaces or a line break: leading whitespace is ignored when matching.
- A short answer shorter than the signal ("Yes."): it is shown when the stream ends.
- The stream fails after the signal: the error is reported as today and Deep does not run.
- Weak evidence never reaches the signal rule: in automatic mode it escalates before the Fast call.
- An escalation through the signal costs the Fast call plus the Deep run; the reason is shown so the user can pin Fast.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The route MUST be chosen without reading the question's words, except for the live status terms kept until FEAT-082: automatic and Fast pinned start Fast, Deep pinned starts Deep.
- **FR-002**: The keyword and length classifier, the query intent, the complexity and confidence values, and the heuristic query classifier with its adapter MUST be removed.
- **FR-003**: In automatic mode with a Deep engine and sufficient Fast evidence, the Fast prompt MUST carry a rule asking the LLM to reply only with a fixed signal when the excerpts do not contain the answer.
- **FR-004**: A Fast answer that starts with the signal MUST escalate to Deep with an escalation event, and MUST NOT show any Fast token or store the Fast attempt.
- **FR-005**: The signal text MUST never reach the user or the session history.
- **FR-006**: Fast pinned MUST NOT escalate and MUST NOT carry the signal rule; Deep pinned MUST always run Deep.
- **FR-007**: The routing indicator MUST show the route (quick search, deep search or live status) and whether it was pinned or automatic, without domain or confidence.
- **FR-008**: The query classifier weights MUST be removed from the model catalog, the hardware kits, the installer list and the model menu.

### Key Entities

- **Route**: Fast, Deep or Status, and whether the user pinned it.
- **Not-enough signal**: the fixed reply the LLM gives in automatic Fast when the excerpts do not hold the answer.
- **Escalation**: the move from Fast to Deep, with one of three reasons (nothing found, weak evidence, not enough).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No code path chooses Fast or Deep from the question's words; the classifier sources and their tests are gone.
- **SC-002**: In tests, automatic mode with sufficient evidence answers in Fast for questions the old classifier sent to Deep.
- **SC-003**: In tests, the not-enough signal escalates in automatic mode, never in Fast pinned, and never reaches the user or the history.
- **SC-004**: Hardware kit downloads are 133 MB smaller.
- **SC-005**: The retrieval benchmark is unchanged: retrieval itself does not change.

## Assumptions

- The signal is a fixed ASCII token that no ordinary answer starts with; models that ignore the rule just answer, as today.
- The escalation reasons are fixed English text like today's notices.
- Existing users who downloaded the weights keep the file until they delete it; no cleanup command is added.
