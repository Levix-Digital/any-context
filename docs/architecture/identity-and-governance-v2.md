# Identity and governance — target design (v2)

Cross-cutting document on users, keys, roles and policies, with and without Admin Hub. It
is a foundation: it touches the `settings.db` split, the encryption key, the owner and
roles of each workspace, the individual chat and the vault of file passwords.

## Base decisions

1. **Local accounts with username and password in the version without Admin Hub**, with
   **several users per machine**. Each person has their own workspaces and conversations;
   nobody sees anyone else's. It serves the family computer and the small office without
   Admin Hub.
2. **The password releases the data key.** Today the key lives in the OS vault and anyone
   logged into that OS account can use it; with the AnyContext password, a stolen laptop
   or another person on the same OS account cannot read the data.
   - **Recovery key** generated when the account is created, for the user to keep (print
     or save) — a forgotten password without it means lost data.
   - **Biometric unlock** (Windows Hello, Touch ID) as a convenience, without weakening the
     password.
   - The password is stored only as a strong hash (Argon2id); it is never sent anywhere.
3. **A single identity and governance engine on the client, with two origins** — not two
   implementations (which tend to diverge):

   | | without Admin Hub | with Admin Hub |
   |---|---|---|
   | users | local accounts (username and password) | corporate identity (company SSO through the Admin Hub) |
   | policies | local defaults, adjustable by the owner | policies signed by the Admin Hub, pushed through the client API |
   | who enforces | the same engine on the client | the same engine on the client |

   User, owner, roles, policies, trust levels and vault exist once; the Admin Hub is an
   origin of identities and policies.
4. ~~Migration of the local account to the corporate one at enrollment~~ — **replaced** by
   isolated personal and corporate environments (topic 6).

## Topics

1. Local accounts and session
2. Key hierarchy
3. Roles and sharing
4. Governance engine and policies
5. Admin Hub integration
6. Personal and corporate environments

### 1. Local accounts and session

Today there is no user: everything lives in the OS account's `settings.db`; there are only
traces (an `added_by_email` column defaulting to `"user"`, identity injection into the
session — ADR-0082, FEAT-013 in the backlog).

**Linux-style administration account**

- **`root`** is the fixed administration account of the installation, clearly labeled in
  the UI ("Administrator (root)"). **No default password:** the password is mandatorily set
  at installation or first run. It gets its own recovery key.
- **`sudo`:** day to day the personal account is used; administrative actions (creating or
  removing accounts, downloading models, installation settings) ask for the `root`
  password just for that action (`actx sudo <command>` in the CLI; "Administrative action:
  enter the administrator password" in the UI).
- **`root` has no workspaces or chat** and **does not read personal accounts' data** — each
  account has its own keys.

**Personal accounts**

- **First run:** one screen sets the `root` password and creates the personal account
  (display name, optional username or e-mail, password), already recommended and filled
  in; it allows the same password for both, warning that different passwords protect
  better. It shows the **recovery key** and asks for confirmation that it was saved; it
  offers biometrics if available.
- **Data separated per account:** its own directory with the local database (preferences,
  history, vault) and the workspaces. Removing an account deletes its data.
- **Sign-in screen** with the machine's accounts, password or biometrics, fast user
  switching.
- **Session:** unlocked while the app is in use; locks after inactivity (e.g. 30 min, a
  personal preference), when the screen locks or the system sleeps, and manually. CLI and
  TUI ask for the password (or biometrics) once per terminal session, with the same
  locking.
- **Increasing delay** between attempts after wrong passwords.
- **Password change** only rewraps the data key — instant, no re-encryption.

**Sync with the app closed — sealed inbox** (no device-bound key, rejected as risky)

- The background service receives only the user's **public key**: it can encrypt, never
  decrypt, and does not read anything already indexed.
- With the app closed it does the heavy work — detecting changed files, downloading pages,
  extraction, OCR, transcription, chunking and embedding **with local models** — and writes
  the result to an **inbox encrypted with the public key**.
- On sign-in, the unlocked app merges the inbox into the index (fast: only writing).
- **No new risk:** the service only uses what is already in clear on disk (the user's
  files). Limitation: cloud API keys stay in the locked vault, so with cloud embedding the
  embedding waits for sign-in.
- Sync modes: **default** with the session unlocked (and a recovery diff on sign-in);
  **optional** sealed inbox with the app closed; **mode 3** on the AnyContext Server.

**In the corporate environment (with Admin Hub — see topic 6)**

- **Administration power belongs to the Admin Hub:** the corporate environment has no
  everyday `root` (the personal environment's `root` is unaffected); creating accounts,
  classifying endpoints, installing models outside the catalog and changing the
  installation become governed by the Admin Hub.
- **Identity origins:** **SSO** through the company's provider and/or the **Admin Hub's own
  directory** (for customers without an identity provider — offices, clinics, smaller
  municipalities). Local-only accounts are forbidden by the policy.
- **Activation of an Admin Hub directory user:** the admin registers them (name, login,
  roles); the Admin Hub generates a **single-use activation code** with a short validity
  (e.g. 72 h), delivered through a separate channel; on first use on an enrolled device,
  the user enters login and code, **creates their own password** and receives the recovery
  key. **The password never goes through the Admin Hub**; keys are generated on the device.
  The policy can require length and complexity, periodic change and a **second factor**
  (authenticator app).
- **Forgotten password:** the admin issues a new activation code; without the recovery key,
  the old data becomes unreadable (organization recovery key option: topic 2).
- **`sudo` becomes "ask IT":** authentication of a corporate admin, or an approval request
  in the Admin Hub (which can become a ServiceNow/Jira ticket).
- **Break-glass:** `root` still exists for on-site and offline emergencies, with a **unique
  password per device, generated and rotated by the Admin Hub** (LAPS model), visible only
  to authorized administrators with access logging; every use is audited, reported in the
  next heartbeat, and the password is rotated afterwards.
- **Golden rule:** neither `root`, nor the corporate admin, nor the Admin Hub read users'
  data — they govern the installation, not the content.
- **Exit:** a revoked corporate environment (kill-switch) is blocked; when unenrolled in an
  orderly way (e.g. device returned to IT), the policy decides whether its data is deleted
  or kept. The personal environment is never affected.

### 2. Key hierarchy

Today there is a single key, derived from the machine id with a salt fixed in the code,
that protects only LanceDB text fields; the lexical index is in clear; and **API keys are
stored in plain text** in the `api_keys` table of `settings.db`.

1. **User master key** — random, one per account, never stored in clear. Wrapped by: the
   **password** (Argon2id), the **recovery key** and **biometrics** as a convenience (the
   OS vault releases it only upon user presence at each unlock — never to a service on its
   own).
2. **User key pair:** the public key seals the inbox for sync with the app closed; the
   private key is encrypted by the master key.
3. **Data key of each workspace** — random; encrypts everything in the workspace (storage
   layer of LanceDB and of the lexical index, workspace database, summaries). Wrapped by the
   owner's master key and, when shared, by **each member's public key** (accounts on the
   same machine; in mode 3, the server keeps it in its own vault). **A member leaving
   rotates the key:** new data uses the new key; old data is re-encrypted in the
   background.
4. **Personal vault**, encrypted from the master key: API keys, file passwords,
   conversation history, preferences — fixes the plain-text API keys.
5. **Device identity** (Ed25519 key from the Admin Hub enrollment) in the OS vault: it
   identifies the machine, it gives no access to data.
6. **Keys only in memory** while the session is unlocked; erased on lock.
7. **Migration:** on the first sign-in after the update, data encrypted with the old key
   (derived from the machine id) is re-encrypted with the new keys, and API keys move to
   the vault.
8. **Organization recovery key** (enterprise policy option): each user's master key is
   also wrapped by an organization public key; **the private part stays offline, under the
   company's control, ideally split between two or more people** — not in the Admin Hub,
   which stays zero-knowledge day to day. Each use is audited and the user is told that the
   company has this recovery power.

### 3. Roles and sharing

Already decided (`deployment-architecture-v2.md`): owner, editor and reader roles;
organization workspaces with the admin as owner and read-only for everyone else; individual
chat; without a server the workspace definition is shared, with AnyContext Server the
index is shared.

1. **Permissions:**

   | action | owner | editor | reader |
   |---|---|---|---|
   | ask, open cited sources, 👍/👎 | ✓ | ✓ | ✓ |
   | add and remove sources, filters, reprocess | ✓ | ✓ | — |
   | edit the workspace prompt and domain pack | ✓ | ✓ | — |
   | mark confidential, privacy policies | ✓ | — | — |
   | share, change roles | ✓ | — | — |
   | delete the workspace | ✓ | — | — |

   Only the owner shares; an editor does not pass access on.
2. **Groups and curators:** in enterprise mode, sharing with **groups** (from SSO or the
   Admin Hub directory); in the organization workspace, the admin appoints **curators**
   (editors from a department) without giving up ownership.
3. **Workspace definition** (what is shared without a server): name, sources, filters,
   domain pack and prompt — the assembly instructions, **never the indexed content**
   (chunks, vectors, index), which stays encrypted on the machine of whoever indexed it.
   Each member indexes the same sources on their own machine (the sources must be
   reachable by all, e.g. network folder, site).
   - **Without Admin Hub:** the definition is exported to a file (e.g.
     `cases-2025.actx-workspace`) and imported by the colleague — no documents, chunks,
     vectors, API keys or passwords.
   - **With Admin Hub:** the definition is distributed by the Admin Hub (metadata only; it
     stays zero-knowledge).
4. **Confidential applies to every member:** the "never send to an external endpoint"
   ceiling applies to whoever asks in the workspace.
5. **A member leaving:** the workspace key is rotated (topic 2); their conversations stay
   with them (personal data), but citations stop opening; in enterprise mode, the policy
   can require deleting them.
6. **Permissions inherited from the source** (AnyContext Server and connectors): in a
   shared index, a reader never receives chunks of documents they cannot open at the
   origin (e.g. SharePoint or file server permissions). Implementation:
   `ingestion-pipeline-v2.md`, per-source access control section (audit P23).

### 4. Governance engine and policies

Today the core has no policy at all. The Admin Hub specification already defines a rich
policy (model mode and allowed models, token quota, OCR, web search with scope and
domains, MCP server, connectors and account domains), sent through `UPDATE_POLICIES` and
kept in an encrypted cache valid for 72 h — none of it is wired to the client. The
specification embeds the Admin Hub public key in the binary, which does not work with each
customer hosting their own Admin Hub.

1. **A single, versioned policy model on the client**, by category:
   - models and endpoints (trust levels, allowed models);
   - data (pseudonymization, confidential workspaces, retention, organization recovery
     key);
   - sources (allowed types, connectors and web domains, limits);
   - tools (web search, MCP server);
   - accounts (password rules, second factor, session time, background sync);
   - quotas (tokens).
2. **Three precedence layers:** product default < local policy (set by `root` without
   Admin Hub) < organization policy (signed by the Admin Hub). **The upper layer wins;
   inside a workspace, the owner can only tighten the rules, never loosen them** (they can
   mark it confidential; they cannot enable web search forbidden by the company).
3. **Signed policies, with the Admin Hub key pinned at enrollment:** the enrollment code
   carries the key fingerprint, the device confirms and records it, and from then on only
   accepts policies signed by it (fixes the key embedded in the binary).
4. **A single engine enforces everything:** model resolution, trust levels and
   pseudonymization, web search, connectors and OCR, MCP server, accounts and session,
   quotas — and connector licenses (`connectors-v2.md`, decision 25), kept distinct from
   policy in the block message.
5. **Offline:** the last valid policy in an encrypted cache applies; the **validity** (72 h
   in the specification) is **configurable by the company**; once expired, the device is
   suspended until it reconnects.
6. **Transparency:** a block says which rule blocked it ("web search disabled by the
   organization policy"); the user can view the policy in force, read-only.
7. **Audit:** blocked attempts are logged locally and sent to the Admin Hub only as
   metadata (rule, when, device), never with content.

### 5. Admin Hub integration

The initial Admin Hub specifications (enrollment, outbound-only WebSocket, heartbeat,
kill-switch, ITSM, SCIM, offline validity) fit, with these adjustments:

1. **Installable package and offline license.** Without a Levix SaaS, the Admin Hub ships as
   a package (containers for a single server; Helm for Kubernetes). Seat control uses a
   **license signed by Levix** (seats, validity), verified **offline** by the Admin Hub.
2. **Admin Hub in Rust**, sharing crates with the core: the policy model, signature
   verification and the workspace definition format must be identical on both sides (the
   specification planned Python/FastAPI; AnyContext removed Python — ADR-0106).
3. **Source telemetry at a level set by the policy:** counts only; **anonymized
   identifiers** (a hash that says "it is the same source" without revealing the name) —
   **default**; or full paths, when the company needs to audit and informs users. (The
   specification sent full paths, which can be personal data, e.g.
   `...\Cases\Divorce John Doe`.)
4. **Internal mirror:** the Admin Hub distributes AnyContext updates, ONNX models, domain
   packs and Chromium to devices without internet (air-gapped); IT downloads once, verified
   by the Levix signature, and controls when each version reaches the fleet.
5. **Two installation profiles, same code:** **compact** (one container with an embedded
   database — offices, clinics) and **scalable** (Postgres, Redis, replicas — large
   companies).
6. **What goes through the Admin Hub channel:** identities (SSO, own directory, SCIM) and
   activation; signed policies; workspace definitions and organization workspaces; groups
   and curators; break-glass password (LAPS style); `sudo` requests and ITSM tickets;
   metadata-only audit; quotas; update mirror; AnyContext Server enrollment.

Commercial decision for the roadmap: seats per device (as in the specification) or per
user (common in the market); the architecture supports both.

### 6. Personal and corporate environments

Replaces base decision 4 ("migration of the local account to the corporate one"): a
company's enrollment must not affect the family accounts on a remote-work machine.

1. **Two isolated environments in the same binary**, in the spirit of Android's "work
   profile", with a switcher ("Personal | Company X"):
   - **personal:** local `root` and family accounts; the company never sees or governs
     anything here (no telemetry, policy or kill-switch);
   - **corporate:** created at enrollment, with **its own data, keys, accounts, policies
     and `root`** (the Admin Hub break-glass); only the company governs it.

   The company may distribute its own installation package through IT; the result is the
   same isolated corporate environment.
2. **No path between environments.** There is no export from personal to corporate: whoever
   wants the same sources adds them in the corporate environment, where they are indexed
   under the policy (and work content that was outside governance should indeed be
   recreated). If real user demand appears, alternatives are evaluated later. Export and
   import of the workspace definition (topic 3) applies **within** each environment; the
   corporate policy can accept only definitions coming from the organization itself
   (signed by the Admin Hub).
3. **Company computer:** the policy can forbid the personal environment on machines that
   belong to the company; on the employee's own laptop, it cannot.
4. **Leaving and changing companies** affect only the corporate environment (data deleted
   or blocked according to the policy); the personal one stays as it was. Changing
   companies removes the previous company's environment and creates the new one at the new
   enrollment.
5. **No data crosses between environments** — no search, no chat, no key (the rule "no
   query crosses workspaces", one level up). Licenses are separate too: a personal
   connector license does not apply to the corporate environment, and vice versa.
