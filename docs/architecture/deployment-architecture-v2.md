# Deployment architecture — target design (v2)

Cross-cutting document: the ingestion and retrieval pipelines describe **what** happens;
this document describes **where** each piece runs. It is decided before the backlog,
because it defines the seams both pipelines must respect.

Motivation: in the enterprise plan, the admin may want storage on a server in the
company's own VPC, heavy models on a GPU server, and a knowledge base indexed once and
queried by everyone.

## Principle

The core talks to **ports** (interfaces). Each port has a local implementation (default)
and a remote one. The seams are decided now; the remote modes are built later, as
enterprise plan items — a distributed system before having enterprise customers is
complexity with no return.

## Seams

1. Storage port
2. Model port, by role
3. Trust level of each endpoint
4. Embedding model identity (what produces the vector, not where it runs)
5. Ingestion worker as a separable role

### 1. Storage port

Today there is no interface: the core uses `NativeLanceStore` directly (9 files) and three
uncoordinated stores — LanceDB (vectors and text), the lexical index and `settings.db`
(which mixes preferences, keys, history, sources, diff cache and summaries).

- **A single "knowledge base" port per workspace**, hiding the three stores: write and
  remove a document with its whole tree (parents, children, vectors, terms, metadata);
  dense, lexical and filter search; chunks by id and the parent of a chunk; queries over
  tables and forms; statistics. **The port is responsible for consistency:** each write
  goes through a journal and is applied idempotently in the three places; a failure in the
  middle is completed on the next sync. It solves a problem that already exists today, even
  without a server.
- **`settings.db` is split in two:**
  - **user-local** (never leaves the machine): preferences, chat history, vault
    references, hardware profile;
  - **workspace** (travels with the knowledge base): sources, diff cache, entities, layout
    maps, summaries, version families.
- **No "LanceDB directly on S3/MinIO"**: Tantivy needs low-latency disk, consistency
  across the three stores becomes fragile over the network, and computation still happens
  on the client. Remote storage only through AnyContext Server (mode 3), which runs the
  three stores on local disk and exposes the port over an API.
- **Server security:** workspace data keys in the server's vault or key service; TLS
  communication with mutual authentication. The key hierarchy (user master key,
  per-workspace key, personal vault) is in `identity-and-governance-v2.md`.
- **Opening the original file** (browsing folders, opening files, viewing sites without
  leaving AnyContext) belongs to the Tauri 2 UI and is not decided here.

### 2. Model port, by role

Today only chat and embedding have an interface (`LmProvider` in `actx-lm`: OpenAI,
Anthropic, Gemini, Gemma, mock); the ONNX models have no common interface, and none can
run on a server. The Admin Hub model policies are not wired to the core.

- **Explicit roles:** embedding, reranker, OCR, page parsing, form fields, image
  description, transcription (with VAD and speaker identification), entities and personal
  data, context and summary LLM, chat LLM.
- **Binding per role:** model (catalog identifier and version) + **where it runs** (local
  ONNX or endpoint). Hardware kits become sets of bindings per role; an **"Enterprise /
  VPC"** kit points the heavy roles to the server.
- **Remote protocols (adapters):**
  - **Native vendor APIs:** OpenAI and Azure OpenAI, Anthropic (Messages) and **Google —
    Gemini API and Vertex AI** (`generateContent`, `embedContent`). The native API is
    preferred for each vendor, because the "OpenAI-compatible" layers they offer cover only
    part of the features (embedding task types, multimodal inputs, context caching).
    Vertex AI in the customer's own GCP project is Google's "corporate gateway" path.
  - **Generic OpenAI-compatible** for servers that use it (vLLM, Ollama, LM Studio), and
    the de facto formats for reranking (`/rerank`) and transcription
    (`/audio/transcriptions`).
  - **AnyContext Model Server** for roles with no standard (OCR, page parsing, fields,
    entities): the same ONNX runtime as the desktop packaged as a service, with the same
    model and the same result as locally.
- **Admin Hub policy checked at resolution** (local only, corporate gateway only, hybrid;
  allowed models): a forbidden binding is refused with a clear reason, never swapped
  silently.
- **Server unavailable:** embedding never switches models (the file waits for the next
  sync); the other ingestion roles are queued and retried, and may fall back to local
  **only with the same model and version**; in chat the error is shown and switching LLM is
  the user's choice, within what the policy allows.
- **Adapters for cloud services of the ONNX roles** (e.g. Google Document AI and
  Speech-to-Text, and the Microsoft and AWS equivalents), for customers who already pay for
  them. They follow the endpoint trust level (seam 3).

### 3. Trust level of each endpoint

Today this notion does not exist: any configured provider receives the full context.

- **Three levels:**
  - **local:** the machine itself;
  - **corporate:** the company perimeter — VPC, Model Server, AnyContext Server and AI
    services contracted **in the company's own account** (e.g. Vertex AI in its GCP
    project, Azure OpenAI in its tenant);
  - **external:** third-party public APIs.
- **Who defines it:** without Admin Hub, the user classifies endpoints (defaults:
  `localhost` local, public API external); with Admin Hub, the company policy defines it
  and **the user cannot raise the level** of an endpoint.
- **What the level decides:**
  - reversible pseudonymization (ingestion, step 7) only applies to **external**;
  - Admin Hub policies become rules over levels: "local only" → local; "corporate gateway
    only" → local + corporate; "hybrid" → all, within the allowed models.
- **Ceiling per workspace:** a workspace marked **confidential** (e.g. medical records,
  sealed cases) never sends anything to an external endpoint, even if the general policy
  allows it.
- **Other data exits count the same:** web search (the question text sent to a public
  search engine) gets the external treatment — pseudonymization and the workspace ceiling;
  applications that consume AnyContext through the MCP server (Cursor, Claude Desktop) are
  external consumers, governed by the policy.

### 4. Embedding model identity

`embedding_target` is stored in each chunk and there is one table per workspace and target
(ingestion, steps 7 and 10). This seam defines when two targets are the same.

- **Identity is what produces the vector, not where it runs:** model and weights version
  (catalog hash), dimension, **precision** (fp32, fp16, int8) and preprocessing (tokenizer
  version, query and document prefixes). The same model, with the same precision, local or
  on the server, is the same target — no reindexing.
- **Fingerprint test:** the catalog ships a fixed set of texts and the reference vectors of
  each model. When a binding is configured, and periodically, AnyContext embeds these
  texts and compares: similarity above a high threshold (e.g. 0.999) → same target; below →
  another target, with the usual reindexing rule.
- The test also catches silent changes (a provider that changes the model without
  changing the name, a server updated with another version or precision) and decides cases
  of different precision (e.g. fp16 on the GPU versus fp32 on the desktop).

### 5. Ingestion worker

Today each sync spawns an `actx --sync-worker` process (or a thread, if the process fails
to start); progress, status and cancellation go through an SQLite table; there is no
queue, and nothing prevents two workers on the same workspace.

1. **Persistent job queue per workspace**, by priority:
   1. **base indexing** (text, lexical index, embedding) — what makes content searchable;
   2. **enrichments** (contextual retrieval, summaries, image description, transcription
      and speakers);
   3. **self-evaluation** (probes, automatic reprocessing);
   4. **maintenance** (compaction, web recrawl, fingerprint tests).
2. **Idempotent jobs**, tied to the storage port journal (seam 1): an interruption resumes
   without duplicating or losing anything.
3. **A single writer per workspace**, through a lease; if the worker dies, the lease
   expires and another takes over. Reading and searching are never blocked.
4. **The worker is a role:** a local process in modes 1 and 2 (with a shared workspace
   definition, each member runs their own); in mode 3 it runs on the AnyContext Server, and
   clients only enqueue requests and query. The optional background service (ingestion,
   automatic sync) is this same worker as a system service.
5. **Progress and status through events** (start, item processed, error, end): native
   events in Tauri; over the server connection in mode 3.
6. **Cancel and pause are queue commands**, honored between jobs and inside long ones
   (e.g. a 3-hour transcription has pause points).

## Workspace model

- **Each user creates their own workspaces and is the sole owner.** They can share with
  other users, assigning roles.
- **The admin can create organization workspaces**, available to the whole company, that
  nobody but the admin edits.
- **One chat per workspace, never mixed.**
- **No query crosses workspaces.** Separating contexts as the user likes is the purpose of
  the workspace. AnyContext's documentation stops being the `Global` workspace and becomes
  the system help, outside the workspaces.
- **Sharing, with the same roles in every mode:** **owner**, **editor** (manages sources)
  and **reader** (query only). The organization workspace has the admin as owner and
  everyone else as readers.
  - **Desktop without a server (mode 1): the workspace definition is shared** — sources,
    filters, domain pack and prompt, never the indexed content. Each member indexes the
    same sources locally, which must be reachable by all (e.g. file server, SharePoint).
  - **With AnyContext Server (mode 3): the index is shared** — the workspace lives on the
    AnyContext Server and members query the same index.
- **Chat belongs to each user**, even in a shared workspace: each has their own history,
  and nobody reads anyone else's chat.

## Admin Hub and AnyContext Server

- **Admin Hub (control plane):** fleet management, enrollment, policies, ITSM/SCIM and
  kill-switch. It is **zero-knowledge** (never sees content) and is **always hosted by the
  customer** (VPC or on-premises) — **never a Levix SaaS**. (The initial Admin Hub
  specifications list "Levix SaaS" as a hosting option; that is corrected here.)
- **AnyContext Server (data plane, mode 3):** also hosted in the company VPC; enrolled and
  governed by the Admin Hub like a device.
- **Perimeter rule:** content (documents, chunks, vectors, prompts) **never leaves the
  company perimeter** — in mode 1, it does not even leave the machine. Mode 1 remains the
  default.

## Deployment modes

| mode | description | when |
|---|---|---|
| 1. Desktop | everything local; default | now |
| 2. Desktop + model server | local index, heavy models in the VPC; only the model port is remote | first enterprise step |
| 3. Client + AnyContext Server | index, ingestion and models on the server; the machine is a client; shared knowledge | full enterprise plan, with the users module and per-source access control (audit P23) |
