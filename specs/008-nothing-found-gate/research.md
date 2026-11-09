# Research: Nothing-found and weak-evidence gate by relevance bands

## Decision 1: weak means every delivered chunk is in the maybe band

- **Decision**: the evidence is weak only when the selection is not empty and every chunk carries the maybe band. One relevant chunk, one pinned file-name match or chunks without a band make it sufficient.
- **Rationale**: the design's wording is "only chunks in the maybe band". Pinned matches are the user naming the file. Band-less chunks come from a pass the reranker did not judge; calling them weak would put a warning on every Lite answer until IMP-055.
- **Alternatives**: a minimum share of relevant chunks (rejected: a single relevant chunk answers most questions); treating band-less chunks as weak (rejected above).

## Decision 2: automatic Fast escalates on nothing and on weak evidence

- **Decision**: in automatic mode a Fast route escalates to Deep when the selection is empty or weak; the Fast LLM call is not made.
- **Rationale**: the design says "no relevant chunk in automatic Fast escalates to Deep", which covers both. Deep refines and decomposes the question, which is what a Fast miss usually needs.
- **Cost**: an escalation costs the Deep calls (decomposition, at most two reflections and the synthesis, or none for the synthesis when Deep also finds nothing). It replaces one Fast call that would have answered from weak or no evidence.
- **Alternatives**: escalate only on nothing (rejected: maybe-only answers are the ones Deep improves); always answer weak in Fast (rejected: contradicts the design).

## Decision 3: the warning is a fixed line before the answer, not part of the history

- **Decision**: `AgentEvent::Caveat` before the first answer token; `run` prefixes it; the session history keeps the answer only, like the end-of-answer notices (ADR-0155). The prompt also carries a rule so the LLM says what the excerpts do not answer.
- **Rationale**: the design puts weak-evidence warnings at the start of the answer (step 9, item 5); a fixed line does not depend on the LLM. Keeping it out of the history avoids the LLM repeating it in later turns.

## Decision 4: the nothing-found message suggests `/folder` and `/web`; `@source` waits for FEAT-088

- **Decision**: rephrase, and add the sources with `/folder <path>` or `/web <url>`; the web search suggestion of today's message goes away (web search is not a source and is off by default in strict mode).
- **Rationale**: suggesting `@source` before it exists would mislead. FEAT-088's description gets the suggestion.

## Decision 5: retrieval itself does not change

- **Decision**: the gate runs after the selection; the pipeline output and the benchmark's gated metrics stay the same. The benchmark only adds reported rates.
