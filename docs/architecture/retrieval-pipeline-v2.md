# Retrieval pipeline — target design (v2)

Living document of the retrieval pipeline review, following the same method as
`ingestion-pipeline-v2.md`. The "Current state" section records how the code works today
(surveyed on 2026-10-20); the target design was filled in step by step after discussion.

## Current state

1. **Question routing.** A classifier based on fixed keywords in Portuguese and English
   ("arquitetura", "compare", "onde está"…), plus the length of the question, decides
   between three paths:
   - **Fast (1-hop):** one search and a single LLM call.
   - **ReAct:** the LLM calls tools in a loop, up to 10 turns.
   - **Deep Search:** decomposes the question into up to 4 sub-questions, searches,
     reflects on what is missing, searches again and synthesizes.

   Status questions ("has the sync finished?") go to their own path. The user can force the
   mode with `/fast` and `/deep`.
2. **Query preprocessing.**
   - Detects file names and dates in the question (ISO and numeric formats, and English
     month names).
   - Cited files enter with a fixed maximum score (1.0). Dates become filters on path
     fragments (`/2025/03/`), also with a fixed score (0.95).
   - The question is used as is: it is not rewritten with the conversation context
     (IMP-009).
3. **Dense search.** The question is embedded with the workspace's registered model and
   searched in LanceDB, in the current workspace and in `Global`. If embedding fails, search
   continues with BM25 only and the failure is logged.
4. **Lexical search:** BM25 in the same workspace.
5. **RRF fusion** of the two lists (k = 60). Candidates that came only from BM25 have their
   text fetched from LanceDB.
6. **Minimum score:** a cut on the RRF score, which varies by preset (Turbo, Balanced,
   Deep).
7. **Reranking:** a local multilingual cross-encoder via ONNX reorders up to 30 candidates.
   No relevance cut afterwards (IMP-007).
8. **Diversification and budget.**
   - At most 2, 3 or 5 chunks per file, depending on the preset.
   - Top-k of 10, 20 or 40.
   - Ceiling of 20, 40 or 60 thousand characters of context (in characters, not tokens).
9. **"Zero context" gate.** If no chunk passes a minimum score, it answers "nothing found"
   without calling the LLM.
10. **Prompt assembly.** Each chunk in an XML tag with origin, version and location;
    U-shaped order (most relevant at the ends); retrieved text escaped.
11. **Generation.**
    - **Fast:** conversation history + prompt in a single call.
    - **ReAct and Deep Search:** their own prompts, with the same context formatter.
12. **Answer and history:** the answer and the original question, without the XML, go to
    the session history in SQLite.

### Items inherited from ingestion

- Aggregate questions over large tables ("sum of March sales"): store the table in a
  queryable format and answer by query (ingestion, step 5).
- Use of "parent" chunks (small-to-big) in context assembly (ingestion, step 5).
- **No query crosses workspaces.** Today every question searches the current workspace and
  `Global` and mixes the results; AnyContext's documentation becomes the system help,
  consulted only for questions about AnyContext itself, through its own route or tool
  (ingestion, step 1).
- One chat per workspace, individual to each user even in a shared workspace
  (`deployment-architecture-v2.md`).
- Version families: prefer the current version by default and use the whole family for
  questions about changes between versions (ingestion, step 3).
- Document entities and metadata as filters and facets; search without masking and
  reversible pseudonymization of the context sent to an external endpoint, with the real
  values restored locally in the answer (ingestion, step 7; trust levels in
  `deployment-architecture-v2.md`).
- "Confidential" ceiling per workspace and web search treated as an exit to the external
  level (`deployment-architecture-v2.md`, seam 3).
- Document and folder summaries for overview questions and for choosing which documents to
  search (ingestion, step 7).
- Answer guidelines from domain packs (ingestion, cross-cutting section H3).
- Forms of the same layout model as a queryable table (ingestion, step 6).
- Evaluation set per workspace made of the self-evaluation synthetic questions (ingestion,
  cross-cutting section).

## Target design

### Principle: minimal configuration

Architecture decisions do not become user options, not even advanced ones. Reranker, chunk
size, overlap, top-K, thresholds, RRF weights and number of candidates are decided by
AnyContext and tuned automatically by the evaluation. The user configures only what is
theirs: sources and include filters, chat model, Fast or Deep, domain pack, workspace
privacy policies, crawl ceiling of a site.

### Overview (happy path)

User question → Fast or Deep → (Deep: question refinement) → dense + lexical search → RRF →
selection of the best chunks → U order + XML → inference model. The steps below detail this
path and add the steps the code already has and the items inherited from ingestion.

1. Question input
2. Routing
3. Question understanding
4. Search
5. Fusion
6. Reranking
7. Selection
8. Context assembly
9. Generation and citations
10. History and evaluation

### 1. Question input

Today the question is searched as typed (the history goes to the LLM, not to the search —
IMP-009); the history is a 30-turn window; AnyContext's documentation is mixed into every
search; status questions are detected by fixed PT/EN keywords ("what is the status of case
123?" falls into AnyContext's status route).

- **Follow-up rewriting, for search only.** A question that depends on the conversation
  ("and the deadline of the second one?") is rewritten as a complete, standalone question
  before searching; the original is what stays in the history and what the answering LLM
  sees. Who rewrites: the profile's local LLM; without it, the chat LLM, respecting the
  trust level and pseudonymization. Without a local LLM and with an external chat, the
  LLM-free alternative is to search with the user's previous question added to the current
  one. Works in any language.
- **History by token budget:** recent messages whole; older ones become a conversation
  summary, generated in the background.
- **Help and status are tools, not keyword routes:** `anycontext_help` (AnyContext
  documentation) and `workspace_status` (state of sync and sources), called by the LLM in
  any language; the UI also offers a help panel and source status in the tree.
- **Source tools** (decided 2026-10-23): the LLM can list the workspace's sources, browse
  their folders and files, and read a file or a range of it, in both modes. Read-only and
  limited to the registered sources of the workspace; no general file system or web
  browsing.
- **No temporary chat attachment.** The context is what is in the workspace — a single
  concept. **Dragging a file into the chat is a shortcut to add it to the workspace as a
  single file**, with a short confirmation (explicit action), visible indexing progress
  and, for large files, the ingestion time estimate. The file stays available in the next
  conversations.
- **`@source` in the question** restricts the search to that source, always inside the
  workspace (detailed in step 3).
- **Question queue.** While the agent answers, the user can send new questions, which are
  queued:
  - processed **in order, one at a time** — each may depend on the previous answer, and
    follow-up rewriting uses that answer;
  - the queue is visible in the chat; questions not yet started can be edited, removed or
    reordered; there is "stop the current one and send now";
  - the queue belongs to the workspace chat and survives closing the app (like local
    drafts);
  - each question counts toward the policy token quotas (Admin Hub);
  - stopping the current question does not clear the queue.
- **Stop and edit.** Stopping (button or `Esc`) cancels search, Deep turns and streaming;
  the question returns to the composer with the original text to edit and resend; the
  interrupted question and the partial answer leave the history.
- **Conversation branches.** Editing an already answered question creates a **new branch**
  from it, preserving the previous one; the edited message shows "‹ 2/3 ›" to switch
  between versions. Conversation summary, follow-up rewriting and the queue follow the
  active branch. **Foundation:** history is stored as a tree from now on (each message with
  `parent_id`, the chat with the active branch), even if the branches UI arrives in a later
  release — it avoids migrating existing chats.

### 2. Routing

Today there are three modes (Fast, ReAct, Deep Search); in automatic mode, a classifier
based on fixed PT/EN keywords and the question length decides; tools exist only in ReAct.

- **Two modes: Fast and Deep.** ReAct stops being a mode; the **tools** (help, status,
  table query, web search when allowed) are available in both.
  - **Fast:** search comes first; the LLM answers and only calls a tool if needed, with a
    small limit (e.g. up to 2 calls).
  - **Deep:** refinement and search turns (below).
- **Automatic: starts Fast and escalates to Deep** when the evidence is insufficient (best
  chunks with low reranker relevance, or the LLM saying the context is not enough), with a
  discreet indicator. No keywords, in any language. The user can pin Fast or Deep in the
  composer selector.
- **Deep:** each turn (1) refines or decomposes the question based on what was already
  found, (2) searches and (3) evaluates what is missing — refining without seeing what was
  found is refining blind.
  - **Default maximum of 3 turns**, stopping early when the evidence is enough (most stop
    at the 1st or 2nd); a **token ceiling** besides the turn ceiling.
  - The maximum can go up to 5 **through the domain pack** (e.g. legal, academic), never as
    a user option (minimal configuration principle).
  - The default is tuned with data: the evaluation set measures the gain of each turn; if
    the 3rd brings no gain, the default drops to 2.

### 3. Question understanding

Today file names (regex with fixed extensions) enter with a fixed score of 1.0 and dates
(regex with English-only months) become filters on path fragments with a fixed score of
0.95; there is no filter by type, entity or version, nor a distinction between question
types.

**Question planner**, in the same LLM call as the follow-up rewriting (step 1), which
returns a structured search:

1. **Search text:** the standalone question; in Deep, also the sub-questions.
2. **Filters:** source (`@file`, a cited folder — always inside the workspace); **dates by
   the document date** (ingestion, step 7), in any language; document type; entities (with
   tolerance to different spellings); version (current by default, all of them in
   questions about changes); connector metadata (`connectors-v2.md`, decision 13).
3. **Question type**, which decides the path:
   - point question → search over chunks;
   - overview → document and folder summaries;
   - aggregate over a table or form → table query;
   - comparison → version family, usually in Deep;
   - **enumerative** ("which", "all the", "list") → does not use top-k: entity and
     metadata filters or SQL over tables and forms (exhaustive by nature); without
     structure, a sweep in Deep, verifying document by document in batches over a broad set
     of candidate documents. **The answer says how many documents were checked** ("I checked
     212 contracts; 14 match"). Large sweeps use the estimate and confirmation of long jobs;
   - help or status → step 1 tools;
   - **whole document** (decided 2026-10-23): an overview or analysis question with
     `@file` ("summarize this contract") reads the whole document — its parent sections
     in document order — instead of top-k chunks; a document larger than the token budget
     is summarized in parts and the parts are combined. Top-k cannot cover a whole
     document.
4. **Exact text** (decided 2026-10-23): a passage in quotes in the question is searched as
   an exact phrase or identifier in the lexical index (Tantivy phrase query), as a
   restricting filter; it complements semantic search for identifiers, codes and literal
   strings.

**Filter rules:**

- **No invented scores:** filters restrict or boost the search.
- **Explicit restricts, inferred boosts:** what the user said (`@file`, "from March 2025")
  restricts; what the planner deduced only gives preference.
- **A filter that empties the result is relaxed**, with a warning in the answer ("I found
  no contracts from March 2025; these are the closest").
- With an external chat, the question goes through pseudonymization before the planner.
  LLM-free alternative, with fewer features: `@source`, dates by a multilingual parser and
  entities through the entity index.

### 4. Search

Today: exhaustive dense search in the current workspace and in `Global`, BM25 with the
question "expanded" by dates, 50/100/150 candidates per list; the only real filter is the
workspace; failure to embed the question leaves only BM25 (logged, no warning); there is no
table query.

1. **Retrievers in parallel, with filters inside the search (pre-filter)** — dense and
   lexical together, for each question or Deep sub-question; filtering after taking the
   candidates can leave zero.
2. **What is searched:** the **child chunks** (parents enter at selection, step 7); the
   document and folder **summaries** only for overview questions. Dense search uses the
   model's query prefix and the table of the embedding target (during a model switch, the
   old table answers until the new one is complete); lexical search uses weighted fields
   (header weighted lower than the body).
3. **30 candidates per list**, tuned by the evaluation set (goes up if 50 improves the hit
   rate measurably).
4. **Table and form query:** for aggregate questions, the LLM generates **read-only SQL**
   over the schema of the workspace tables (columns, types, sample values), executed by the
   SQL engine LanceDB already ships (DataFusion). Safeguards: read-only and only that
   workspace's tables; time and row limits; **the query and the rows used appear in the
   citation**; failure or ambiguity falls back to normal search, with a warning.
5. **Question embedding failure:** lexical-only search, with a **discreet warning** in the
   answer.
6. **Search stratified by source.** In a workspace with several sources, search runs **per
   source, in parallel**, with a smaller list per source — each source secures its slots,
   and an answer spread across several places reaches the fusion. **Parallelism is
   dynamic**: it depends on the workspace size and the free capacity of the machine (or the
   server, in mode 3) at that moment. With a single source or few documents, search is
   single.
7. **Two stages in large workspaces:** above a certain number of documents, the most
   relevant **documents** are found first (document summaries and aggregation of chunks per
   document), then the **chunks inside them** — breadth before depth.
8. **Future retrievers enter as extra lists** (visual page search, sparse vectors — audit
   P10 and P19), without changing the rest of the pipeline.

### 5. Fusion

Today: RRF (k = 60) of the dense and lexical lists with equal weights; the dense list
comes with the file and date results injected with an invented score; a cut by the
minimum RRF score (0.012/0.008/0.005) — arbitrary, because the RRF score measures position
in the lists, not relevance; in Deep, sub-questions are joined by deduplication with score
summing.

1. **RRF with k = 60 over clean lists**, and **all lists in a single fusion**: in Deep,
   each sub-question produces two lists and all enter the same RRF (a chunk found by several
   sub-questions rises naturally); future retrievers enter the same way.
2. **Equal weights by default**, tuned per workspace by the evaluation set (e.g. lexical in
   a code workspace, dense in a legal texts one), automatically and internally.
3. **No cut by the RRF score:** the relevance decision moves to the reranker (step 6), and
   so does the "nothing found" gate (step 7).
4. **Per-document limit at the reranker input:** when choosing candidates, each document
   takes at most a few slots (e.g. 3) and the rest stays in reserve — a long document does
   not "hijack" the candidates. (This differs from the final selection, step 7, which is
   free.) The lists of the source-stratified search all enter the same RRF; the reranker
   then decides the real relevance (in RRF, 1st place of an irrelevant source is worth the
   same as that of an excellent one).
5. **How many go to the reranker — by time budget, not a fixed number:**
   - **Fast: 30**; **Deep: 50** (Deep fusion joins several sub-questions and the user
     already accepts waiting longer);
   - adjusted by the hardware profile (e.g. 20 in Fast on light profiles; up to 100 in
     Deep on strong profiles or on the Model Server with GPU), with a reranking time budget
     (e.g. up to about 1 second in Fast);
   - the evaluation measures at which fusion position the right chunk usually is and tunes
     the number per workspace (it drops if it is almost always among the first 20; it
     rises, within the budget, if it often appears after the 30th).

### 6. Reranking

Today: ONNX multilingual cross-encoder (`mmarco-reranker` by default;
`bge-reranker-v2-m3` INT8), chosen or disabled by the user with `/rerank`; it reorders up
to 30, the rest go to the end without evaluation; no relevance cut (IMP-007); Deep asks the
LLM for a chunk-by-chunk "yes/no" (one call per chunk), duplicating the cross-encoder.

1. **The cross-encoder is the only relevance judge, in both modes.** The chunk-by-chunk
   LLM judgment in Deep goes away; the Deep LLM evaluates the set ("what is still
   missing?").
2. **Model comes from the hardware kit** through the model port (small on light profiles,
   `bge-reranker-v2-m3` on medium and strong ones, Model Server in enterprise mode).
   **Always on — an architecture decision, with no user option;** the `/rerank` command
   goes away.
3. **Input:** the text of the child chunk with the contextual header (the same embedded
   text), truncated at the model's useful limit. Summaries (overview) go through the
   reranker; SQL results do not (they are exact).
4. **Score calibrated into three bands** — relevant, maybe, irrelevant —, with per-model
   thresholds in the catalog, refined per workspace by the evaluation. They feed the
   "nothing found" gate (step 7), the escalation from Fast to Deep (step 2) and discarding:
   irrelevant chunks do not go to the LLM, even if they are the "best" ones (solves
   IMP-007).
5. **Candidates beyond the budget are discarded**, not pushed to the end without
   evaluation.

### 7. Selection

Today: at most 2/3/5 chunks per file, top-k 10/20/40, up to 20/40/60 thousand characters
(unrelated to the model window); parents are not used; the "nothing found" gate uses the
RRF score.

1. **By relevance and token budget, not a fixed number.** The chunks in the reranker's
   **relevant** band enter and, if there is room left, the best of the **maybe** band, up
   to a context token budget defined by AnyContext according to the chat model (window and
   cost) and the mode (smaller in Fast, larger in Deep). In Fast, this typically gives
   **3 to 5 chunks**, already expanded with the parent.
2. **Small-to-big:** each selected child is replaced by its **parent** (section or page)
   when it fits the budget; several children of the same parent → the parent enters once;
   a parent too large → the child with its immediate neighbors. The citation points to the
   child; the parent is the context.
3. **Diversity:** near-identical chunks deduplicated; **current version** (previous ones
   only in comparison questions); **no hard per-file limit** in the final selection —
   breadth is secured earlier, by the stratified search and the per-document limit at the
   reranker input.
4. **Other evidence enters too:** SQL results (with the query and the rows) and summaries,
   for overview questions.
5. **"Nothing found" gate, by the reranker bands:**
   - no relevant chunk in automatic Fast → **escalates to Deep**;
   - still nothing after Deep, or with Fast pinned → answers that nothing was found,
     **without calling the LLM**, suggesting rephrasing, adding sources or using `@source`;
   - only chunks in the maybe band → answers with an **explicit weak-evidence warning**.
6. **Coverage in Deep:** each turn's evaluation sees which documents already appeared and
   can ask "are there others?"; the next search **excludes the documents already found**.
7. **Measuring coverage:** the evaluation set includes questions whose answers are spread
   across several documents and measures how many of the right documents reach the
   context — this is what calibrates stratification, the per-document limit and the two
   stages in each workspace.

### 8. Context assembly

Today: each chunk in an XML tag with id, origin and a "version" deduced from the path
(ADR-0140); U order after selection; retrieved text escaped (ADR-0136); one system prompt
per path (ADR-0144); no pseudonymization nor context sizing by the model window.

A **single final context per question**, assembled once and sent in a single call; the
selected chunks may come from different source files of the workspace.

1. **Order:** select first, then order. **Chunks from the same file stay together and in
   the order they appear in the file**; the per-file groups are arranged in a U by the best
   relevance of each. (The "lost in the middle" effect is smaller in current models and
   with few chunks; the U stays because it costs nothing and helps smaller and local
   models.)
2. **XML envelope per evidence:** short id for citation, document title, source and path,
   **locator** (page, section, lines, cells, minute), **document date and version coming
   from ingestion** (not deduced from the path), **type** (chunk, summary, table result
   with the SQL query). Text escaping stays.
3. **Pseudonymization applied here** when the LLM endpoint is external; the mapping table
   stays local and holds for the whole conversation (the `[PERSON_1]` of one question is
   the same in the next).
4. **Prompt assembled in layers** (instead of "one prompt per path"):
   1. **AnyContext base rules** (fixed): answer only from the context and say when it is
      not enough; cite by the ids; answer in the user's language; safety; tool use;
   2. **domain pack guidelines**;
   3. **workspace system prompt** (FEAT-009): persona, tone, format, focus;
   4. **user layer** (decided 2026-10-23): the user's role and answer style and their
      explicit memories (`identity-and-governance-v2.md`, topic 7), below the workspace
      prompt and within the token budget;
   5. **mode complement** (answer in Fast, synthesize in Deep).

   The workspace prompt and the user layer enter **only in the calls that produce the user's answer** (Fast
   answer, Deep synthesis), never in internal tasks (planner, evaluation, summaries, chunk
   context, synthetic questions). **Base rules take precedence**: the workspace prompt
   shapes tone, format and focus, but does not turn off "only from the context" or "cite
   the sources". In a shared workspace, only the owner and editors change the prompt.
5. **Computed token budget:** model window = system prompt + history (with the summary of
   older parts) + evidence + reserve for the answer; evidence gets what is left, within the
   cost ceiling of step 7.
6. **Prompt caching:** the stable layers (1 to 3) go at the start of the prompt, to take
   advantage of the providers' caches.

### 9. Generation and citations

Today: in Fast, one call with history and prompt; streaming not always used; the LLM is
instructed to cite, but nobody checks whether the cited id exists or whether the chunk
supports the sentence, and the citation is just text.

1. **Always streaming**, in both modes, with visible progress ("querying the CMR table…",
   "Deep turn 2…").
2. **Structured, checked citations:** the LLM cites by evidence ids (`[2]`); at the end,
   AnyContext checks each id and removes those that do not exist in the context; each
   citation becomes a link to the locator (page and region, cell, minute), opening the
   original at the exact point in the Tauri UI.
3. **Support verification by the local reranker:** each cited sentence is compared with
   the cited chunk; a weak citation is marked discreetly (e.g. a dotted underline and the
   hint "the cited chunk does not directly confirm this sentence").
4. **Pseudonymization undone during streaming:** the markers (`[PERSON_1]`) are replaced by
   the real values before appearing on screen, including when they arrive split between two
   streaming pieces. The user never sees a marker.
5. **Special evidence:** SQL numbers show the query and the rows used (expandable detail);
   web search results (when allowed) marked as **web**, with URL, separated from the
   workspace sources; weak-evidence warning and the enumerative sweep count at the start of
   the answer.
6. **Language:** always the user's, even with documents in other languages.

### 10. History and evaluation

Today the history stores the question and the final answer (without XML or intermediate
steps — ADR-0088), per workspace, in SQLite; citations are not stored; there is no
retrieval evaluation (IMP-008) nor user feedback.

1. **"Receipt" for each answer:** citations with locator and **file version** (hash) at the
   time of the answer; mode used (Fast, Deep, Fast that escalated); tools called; tokens
   consumed (they feed the FEAT-010 status bar and the Admin Hub quotas, which receives only
   numbers). When a conversation is reopened, citations work; if the file changed
   afterwards, the citation shows **"the document changed since this answer"**.
2. **History** as a tree (branches), with a summary of older parts, in the user's local
   database, encrypted.
3. **User feedback:** 👍/👎 per answer, with an optional reason ("wrong information",
   "wrong citation", "missing information"). It stays on the machine; in enterprise mode,
   the Admin Hub receives at most counts, never content.
4. **Self-tuning loop** — where the internal adjustments of the previous steps live. Each
   question records quality signals locally (reranker bands, escalation to Deep, document
   coverage, proportion of weak citations, 👍/👎). With them and the evaluation set,
   AnyContext tunes **per workspace**, without the user seeing it: RRF weights, reranker
   depth, band thresholds, Deep turns, stratification and compaction.
5. **Regression evaluation:** the ingestion synthetic question set is enriched with the
   questions that received 👍/👎 and runs after every change that may worsen quality (model
   switch, change in the ingestion of a source, AnyContext update); a regression reverts
   the automatic adjustment that caused it. In development, the test corpus (starting with
   the fictitious CMR) runs on every code change.
6. **Retention:** the user can delete conversations; when deleting a workspace, AnyContext
   asks what to do with its conversations; in enterprise mode, the policy can set a
   retention period.
