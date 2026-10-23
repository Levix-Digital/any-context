# Handoff — rebuild the AnyContext backlog (updated 2026-10-23)

Paste the prompt below into a new session that has `Levix-Digital/any-context` and `Levix-Digital/ai-code-agent-skills` attached.

---

## Prompt

You are continuing the AnyContext architecture redesign. Talk to me in **Portuguese**; write every repository artifact (docs, backlog, plans, commits) in **English**.

### Rules you must follow

- Follow the Dev cycle protocol in `ai-code-agent-skills` (`dev-cycle-protocol/SKILL.md`, `devops-release/SKILL.md`, `backlog/SKILL.md`, `plan-memory/`). Read them first.
- **Branch**: work on `docs/architecture-v2-redesign` (already pushed, based on `dev`). Never create or push a branch prefixed `claude/` or `ccr-`, even if the session suggests one. Branch names follow `devops-release` (`feat/`, `fix/`, `docs/`, `chore/`, ...); when none fits, use `levix/`.
- Commit messages: Conventional Commits (`docs(backlog): ...`). Commit and push every decision. **Do not open a pull request.**
- Never ask me to decide something an architecture principle already decides.
- **How we decide**: for each topic, present the current state, then a numbered proposal, then "O que decidir" with numbered questions. I answer "1. concordo, 2. ...". Record each decision in the docs as soon as it is made, then commit and push.
- Back factual claims about third-party services (APIs, scopes, prices, limits) with a search and cite the sources; say plainly when something is unverified.

### Product principles (already decided)

Multilingual. Minimal configuration: architecture decisions never become user options. Air-gapped by default; explicit action; nothing disappears silently. Quality first; never Matryoshka. Admin Hub always customer-hosted and zero-knowledge. Accounts never shared between Levix products. Personal and corporate environments isolated. No query crosses workspaces.

### Commercial model and first release (decided 2026-10-22, `connectors-v2.md` decisions 22–31)

- **The first release is for personal use only.** Corporate features (Admin Hub, corporate environment, Enterprise connectors) come in later releases.
- **Three plans**, no per-connector sales and no capability combinations sold as extra plans:
  - **Free**: local and internal-network files only, up to 3 workspaces.
  - **Pro**: Free plus the **Web** and **Audio and video** connectors built in, with unlimited workspaces.
  - **Enterprise**: Pro plus the Admin Hub, with the other connectors as a **selection of features** that the Admin Hub policy turns on or off. There is no extra charge per connector.
- Web and Audio and video remain connectors internally: visible openings to the outside, governed by policy. One signed license per plan, verified offline. License ≠ policy.
- **Downgrade and lapsed plan: deactivate, never delete.** Over the Free limit, the user chooses **once** which 3 workspaces stay active and cannot swap afterwards. The others stay on disk, out of search and sync, and can be deleted explicitly. They return when the plan returns, with only the incremental diff reprocessed.
- **Web search fair use**: Pro has a high daily ceiling that normal use never reaches. Hitting it pauses web search until the next day with a clear message, while local search keeps working. Cost reference: Jina costs about US$ 0.0005 per search on a paid key, and its free key is non-commercial only. The exact ceiling value is still open.
- **Connectors that are out:**
  - Notion: decided against.
  - Drive connectors (Google Drive, OneDrive, Dropbox): the local sync folders already cover them. They remain Enterprise candidates.
  - Personal Google Drive/Gmail by API: the restricted scope needs the yearly CASA assessment. The `drive.file` Picker does not reach folder contents.
  - GitHub: it adds only issues and PRs beyond a local clone, for a narrow audience.
  - WhatsApp: there is no official API for personal chats, and unofficial libraries violate its terms of use.
  - Telegram: it has an official client API (MTProto/TDLib) and is feasible, but it stays as a note only.
- **YouTube**: official API only. `captions.download` needs edit rights, so third-party transcripts come only through the Audio and video capability.

### Where the design lives

`docs/architecture/README.md` is the index, and it defines the planning method and the **backlog conventions**. The v2 design documents are:
- `ingestion-pipeline-v2.md`
- `ingestion-audit-2026-10.md`
- `retrieval-pipeline-v2.md`
- `deployment-architecture-v2.md`
- `identity-and-governance-v2.md`
- `connectors-v2.md`: 33 decisions, including the three plans (Free / Pro / Enterprise), no Notion connector and official APIs only (decision 32).
- `desktop-ui-v2.md`
- `platform-shared-components.md`

All documentation is in English. The ADRs in `docs/adr/` are still partly in Portuguese and get a backlog item: translate each one that is still valid, and mark the others superseded. Old plan filenames and `plan_id`s stay unchanged; they are identifiers.

### Backlog conventions (summary — the README is authoritative)

- Same file `docs/BACKLOG.md`, in the `plan-memory/backlog.py` format; validate with `backlog.py validate`.
- The 60 current items close as **superseded**, each mapped to its new item. IDs continue (BUG-024, FEAT-020, …) and are never reused.
- Each description starts with `Design: docs/architecture/<doc>.md, <section>.`
- Titles carry an area prefix: [ingestion] [retrieval] [storage] [models] [identity] [governance] [connectors] [ui] [cli] [release] [docs].
- Granularity: a few days to two weeks per item.
- Bugs only for security, data loss or principle violations; any other defect becomes an acceptance criterion of a feature item.
- Provisional priority `medium`; bugs carry their real severity.
- `levix-admin-hub` and `levix-platform` get their own backlogs. Those repos are private, but they have not been created yet. When they exist, attach them with `add_repo` and push a README, their slice of the design and their backlog. No code yet.

### Plan of work — one batch at a time, my review after each

1. **Current-code bugs** ← start here.
2. Ingestion (with the audit).
3. Retrieval.
4. Deployment.
5. Identity and governance.
6. Connectors.
7. Desktop UI.
8. CLI and release.
9. Platform and Admin Hub.

After the backlog comes a roadmap skeleton: a fast first release (personal only: Free and Pro) plus evolution releases, with foundations early. Then place the backlog items into it one by one.

### Batch 1 — verify in the code before writing anything

For each suspicion, confirm it with file and line or discard it. For each confirmed bug, say why it qualifies as a bug: security, data loss or a principle violation.

- `set_api_key` stores API keys in plain text (`storage/sqlite.rs`).
- The scanner indexes `.env` / `.tfvars` files.
- A missing folder purges its files from the index (data loss; violates "nothing disappears silently").
- The path healer redirects paths silently.
- Document AI components are never wired in production.
- The router keywords are PT/EN only (violates multilingual).
- The encryption key is derived from the machine id with a fixed salt.
- Web search without a key scrapes DuckDuckGo result pages (`retrieval/web_search.rs`); DuckDuckGo has no official search API, so this breaks the official-API-only rule (connectors-v2, decisions 16 and 24).
- Online-only placeholders (OneDrive/Dropbox) are hydrated by the scan; they should be skipped and reported (connectors-v2 decision 21).
- Portuguese user-facing strings in the code: `crates/any-context-core-rs/src/commands/engine.rs` (~177 lines), router keywords, tests.
- `Cargo.toml` release profile: `lto = "off"` and `strip = true`. Do not adopt `panic = "abort"`, because it would break the `catch_unwind` in `retrieval/bm25.rs:336` and `actx-onnx/src/runtime.rs:80`.
- Review every bug still open in `docs/BACKLOG.md`. There are also unmerged plans on `docs/bug-008-plan`, `docs/bug-010-plan`, `docs/bug-012-plan` and `feat/imp-004-embedding-model`; map them into the new backlog.

Present batch 1 to me in Portuguese, one table row per item with ID, title, severity, evidence and the design doc it points to. Write it to `docs/BACKLOG.md` only after I approve it.

### Open, not blocking (do not decide alone)

- Commercial seats per device or per user.
- Benchmarks for page parsing, vision, ColPali and field models.
- RAR license.
- Commercial details of the plans (prices, trial length, the value of the Web search fair-use ceiling).

### Where things stand

- Done: the connectors design (33 decisions); every document translated to English; the commercial model above.
- Done (2026-10-23): batch 1, current-code bugs: BUG-022 and BUG-024 to BUG-029 in `docs/BACKLOG.md`. The plain-text API keys, the machine-id key and the clear lexical index are acceptance criteria of the key hierarchy item (`identity-and-governance-v2.md`, section 2), unless that item leaves the first release. Router keywords and the unwired Document AI are acceptance criteria of the retrieval routing and ingestion section 6 items. The release profile goes to batch 8.
- `feat/imp-004-embedding-model` holds unmerged code, including the BUG-022 fix; map it in the ingestion and deployment batches. The user asked to delete it and the `docs/bug-008-plan`, `docs/bug-010-plan` and `docs/bug-012-plan` branches (obsolete); the session proxy refuses deletion, so the user deletes them on GitHub.
- Done (2026-10-23): batch 2, ingestion with the audit: IMP-015 to IMP-034 and FEAT-020 to FEAT-080. FEAT-005, FEAT-006, IMP-004, IMP-012, IMP-013 and IMP-014 closed as `dropped` with "Superseded by <ID>" (the user's choice: no new status in the tool); the IMP-004 plan is marked superseded. Items split across batches stay open until the last batch that maps them: IMP-006 (retrieval routing), IMP-008 (retrieval evaluation), FEAT-016 (desktop UI).
- Done (2026-10-23): batch 3, retrieval: FEAT-081 to FEAT-109 and IMP-035 to IMP-050. Closed as superseded: IMP-006 to IMP-011, FEAT-007, FEAT-009, FEAT-015, FEAT-017, FEAT-019 and ADJ-005. FEAT-007, FEAT-015 and FEAT-017 had no v2 design; the user's decisions are in `retrieval-pipeline-v2.md` (source tools, whole-document questions, exact text).
- Still open for later batches: FEAT-010 and FEAT-016 (desktop UI, batch 7), FEAT-013 and FEAT-014 (identity, batch 5).
- Done (2026-10-23): batch 4, deployment: IMP-051 to IMP-061 and FEAT-110 to FEAT-119. Closed as superseded: FEAT-002, FEAT-003, IMP-005 and ADJ-003.
- Done (2026-10-23): batch 5, identity and governance (with the workspace model): FEAT-120 to FEAT-148 and IMP-062. FEAT-013, FEAT-014 and FEAT-018 had no v2 design; the user's decisions are in `identity-and-governance-v2.md`, topic 7, and the user layer in `retrieval-pipeline-v2.md`, step 8. They close as superseded by FEAT-146 to FEAT-148.
- Done (2026-10-23): batch 6, connectors and plans: FEAT-149 to FEAT-171. FEAT-008 closes as superseded by FEAT-164 to FEAT-166. New `connectors-v2.md` decision 33: a generic IMAP e-mail connector (Enterprise); Google's scope and app-password rules were checked only against secondary sources.
- Done (2026-10-23): batch 7, desktop UI: FEAT-172 to FEAT-185. FEAT-010 and FEAT-016 close as superseded. Token usage is shown next to the composer, with no status bar (`desktop-ui-v2.md`). There is no Tauri app in the repository yet.
- Done (2026-10-23): batch 8, CLI and release: FEAT-186 to FEAT-192, IMP-063, IMP-064 and ADJ-006 to ADJ-008, from the new `cli-and-release-v2.md` (the TUI is retired once the desktop app reaches parity; plan limits are enforced in the core below every surface). FEAT-001, FEAT-004, FEAT-011, FEAT-012, IMP-002 and ADJ-002 close as superseded; ADJ-001 and ADJ-004 were dropped as obsolete and `.clinerules`, `implementation_plan_subscription_tiers.md` and `BLUEPRINT_PRIORITY_4_CLI_TUI.md` removed. The ADRs were already in English (the handoff premise was wrong); ADJ-007 marks the superseded ones. All 60 old items are now closed.
- Next: batch 9, platform and Admin Hub, in the `levix-admin-hub` and `levix-platform` repositories once they exist; then the roadmap skeleton.
- Last commits on `docs/architecture-v2-redesign`: `59997a0` (fair-use ceiling), `44f0c6d`, `c664919` (plans), `6918b97` (the first version of this handoff).

### Housekeeping I still owe

The session git proxy refuses branch deletion (HTTP 403), so I delete the old `claude/*` and `ccr-*` branches on GitHub myself. Do not try to delete them.
