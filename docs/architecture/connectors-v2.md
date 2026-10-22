# Connectors — target design (v2)

Review of the connectors. All decisions are made; what is commercial or about delivery
order is left to the roadmap.

## Current state

- There is no connector code. Sources are a local folder or file, a web site or page, and
  the system help (`builtin`). An old plan (`docs/plans/...chain_of_responsibility...`)
  foresaw a `CloudDriveSyncHandler`, never implemented.
- The `sources` table already reserves `kind = connector:<name>`, with a target and a
  single or recursive mode (`ingestion-pipeline-v2.md`, step 1).
- Already decided in other documents: permissions inherited from the origin (permission
  scope ID in the chunk, details in a separate SQLite database — `ingestion-pipeline-v2.md`,
  per-source access control); Admin Hub policy with allowed connectors and account domains
  (`identity-and-governance-v2.md`, topic 4); automatic sync and resource governor
  (`ingestion-pipeline-v2.md`, cross-cutting sections).

## Decisions

1. **A connector only acquires.** It is an adapter of acquisition (ingestion, step 2): it
   brings the external content, and from step 3 on it is the same pipeline as any file. No
   parsing, chunking or cleanup of its own, except origin-specific cleanup the pipeline
   does not cover. Single contract:
   - **list** the items of the target (single or recursive);
   - **read** the content as a stream (a single read);
   - **changes** since the last cursor;
   - **permissions** (permission scopes of the origin);
   - **metadata** (author, dates, link to open at the origin, and the connector's own
     metadata — decision 13).

   Each item has a **stable native ID** from the origin, which replaces the hash in rename
   and move detection.
2. **Indexed only, never a live query.** The connector syncs into the workspace; the
   question is never sent to the origin's API. This keeps retrieval quality (hybrid search,
   reranker, precise citation), does not send the question to third parties, and respects
   "no query crosses workspaces".
3. **Read-only, with the smallest possible scopes.** Never writes to the origin.
4. **OAuth 2.0 with PKCE through the system browser**, redirecting to the computer itself
   (`127.0.0.1`), **without going through any Levix server**.
   - The token lives in the user's **personal vault** (`identity-and-governance-v2.md`,
     topic 2): it belongs to the person, not to the workspace.
   - **Personal environment:** public OAuth app registered by Levix.
   - **Corporate environment:** app registered by **the company itself** (in its Microsoft
     Entra ID, in its Google Workspace), distributed through the Admin Hub policy.
   - **AnyContext Server (shared index in the company VPC):** company service account, with
     the per-reader permission filter mandatory.
   - Costs and audits required by vendors: see decision 20.
5. **Sharing a workspace without a server carries the target, never the credential.** The
   workspace definition (what is shared when there is no AnyContext Server) carries the
   connector and the target ID; each member signs in with their own account and indexes
   with their own access.
6. **Sync through the origin's change feed, not by scanning** (e.g. Google Drive
   `changes`, Microsoft Graph `delta`, Dropbox cursor, IMAP `CONDSTORE`).
   - **On the user's computer:** periodic polling with the cursor while AnyContext is open
     (webhooks cannot be received); recovery diff on opening; optional background service,
     as for local sources.
   - **On the AnyContext Server:** webhooks from the origin.
   - Honors API rate limits (exponential backoff).
   - **Metered network** (mobile data, metered connection): the resource governor holds
     connector sync, and **the user can release it** (per source or for now).
7. **Native cloud formats:** Google Docs, Sheets and Slides exported to DOCX, XLSX and PPTX
   and handed to the existing parsers; wiki pages (Confluence, Notion) converted from the
   block tree to deterministic Markdown; cloud shortcuts and links resolved and
   deduplicated as duplicates (ingestion, step 3).
8. **No copy of the original.** Only the extracted content is kept, in the encrypted index.
   The reader tab shows the indexed text and offers **"open at origin"** (native link).
   Streaming download, with the governor's disk reserve.
9. **Levix-only connectors**, in Rust, in an `actx-connectors` crate with one trait and one
   module per connector; no third-party plugins. Shared HTTP client with **corporate
   proxy** and **company certificates** (internal certificate authority, read from the OS
   certificate store and from the policy).
10. **Explicit target selection.** The user browses the origin's tree and picks what they
    want (a folder, which is recursive, a single file, or origin-specific targets: shared
    drive, SharePoint library, Gmail label, channel or playlist). **Everything the user
    selects and has access to is downloaded and indexed**; nothing outside the selection.
    Include and exclude filters (`.gitignore` syntax) also apply to remote paths.
11. **Admin Hub policy:** allowed connectors, **allowed account domains** (e.g. only
    `@company.com` accounts in the corporate environment) and a volume ceiling per source.
    A block always says which rule blocked it.
12. **Schema:** `sources` gains `account_ref` (reference to the credential in the vault)
    and `target_id` (native ID of the target, besides the displayed path). The change
    cursor lives in the workspace sync state.
13. **Each connector's own metadata.** Each connector declares a **typed metadata schema**
    (e.g. YouTube: `view_count`, `like_count`, `published_at`; Jira: `status`, `assignee`;
    e-mail: `from`, `labels`), usable as a filter by retrieval (question understanding) the
    same way as the metadata of a domain pack.
14. **Live metadata kept apart from content.** What changes without the content changing
    (views, likes, status, labels) lives only in filterable columns, **outside the embedded
    text and the chunk header**; a change updates only those columns, without re-embedding
    — the same logic as permissions (ID in the chunk, details elsewhere).
15. **High-volume, noisy sources** (e.g. comments): the most relevant set is indexed (e.g.
    the most liked comments and their replies), each item as its own chunk with author and
    relevance in the header; the source report says how much was left out ("100 of 10,000
    comments") and the ceiling is configurable by the user, like the crawl ceiling. Comment
    authors are third-party personal data: reversible pseudonymization of what goes to an
    external endpoint applies (ingestion, step 7).
16. **YouTube only through the official API.** Metadata and comments of any public video;
    transcript through the official caption only when the user owns the channel (the API
    only hands captions to whoever can edit the video). A third-party video is transcribed
    by the Video connector (capability, decision 22) from a file the user legitimately
    has, locally — never through an unofficial client (InnerTube), impersonation of another
    client, or a third-party transcription service.
17. **Obsidian:** the vault is a local folder; the Markdown parser ignores `.obsidian/` by
    default, reads YAML frontmatter as metadata and turns `[[wikilinks]]` into graph
    relations (ingestion, step 7, P16).
18. **Source states when something fails:**
    - item **deleted at the origin** → leaves the index;
    - **no technical access** (expired or revoked token, changed password, network down) →
      source **disconnected** ("reconnect account"); nothing is deleted and the indexed
      content remains searchable;
    - **access to an item revoked** for the user → the item leaves the index and appears in
      the source report with the reason.
19. **Extraction maps** (for origins without a stable API): a declarative selector file,
    distributed through the verified catalog like the domain packs, **signed by Levix and
    reviewed by a person** before publishing — never generated and published automatically.
20. **Full Google Drive, only through the official API**, with the read scope over the
    whole Drive (no shortcuts such as restricting to the file picker). If Google's security
    assessment (CASA) does not fit early for the personal environment, the connector comes
    when it fits; the corporate environment uses the company's internal app, which is
    exempt from the assessment.
21. **Online-only files in local folders** (OneDrive, Google Drive, Dropbox — Cloud Files on
    Windows, File Provider on macOS): the local scanner detects them and **does not force
    the download**; the source report warns ("120 files are only in the cloud — use the
    connector or mark 'always keep on this device'"). Files already on disk are indexed
    normally. Today the scan downloads everything without the user asking: **current bug**.

## Product model: everything is a connector

22. **The base is search over local and internal-network files**, with no external call at
    all. Everything else is a **connector plugged explicitly** by the user — "air-gapped by
    default" becomes literal: each connector is an opening to the outside, visible and
    governable. The system help is not a connector. Two internal kinds, one name for the
    user:
    - **source connector:** brings content from outside through the contract of decision 1
      (web sites, Google Drive, OneDrive and SharePoint, YouTube, e-mail, Obsidian…);
    - **capability connector:** unlocks processing of local files (e.g. transcription of
      local videos and audio).

    Without the connector, what it would cover appears in the source report ("12 videos not
    indexed — enable the Video connector"); nothing disappears silently.
23. **A paid connector delivers more than the base.** E.g. an Obsidian vault is a Markdown
    folder and the base indexes its text; the Obsidian connector adds the wikilinks in the
    graph, frontmatter as metadata, embedded notes, aliases and canvas (decision 17).
24. **Web search at question time is also a connector**, being the main data exit to the
    outside.
25. **License per connector**, with a **trial period** per connector, in the signed license
    verified offline (`levix-platform`). License ≠ policy: the license says what was paid
    for, the policy says what the company allows; the same engine enforces both and the
    block message says which one blocked.
26. **Expired license: sources deactivated, never deleted.** The indexes of that
    connector's sources are kept but left out of search, and sync stops. On renewal, the
    source comes back and the incremental diff reprocesses only what changed at the origin.
    The user can explicitly delete deactivated indexes to free disk.
27. **In the corporate environment**, the organization license unlocks the connectors, and
    the Admin Hub policy can still block each one.

## For the roadmap

- **Commercial packaging** (which connectors are free, which are paid, prices, trial
  length).
- Which connectors exist and in which order: decided in the backlog and the roadmap.
