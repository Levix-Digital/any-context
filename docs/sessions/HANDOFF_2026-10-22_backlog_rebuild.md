# Handoff — rebuild the AnyContext backlog (2026-10-22)

Paste the prompt below into a new session that has `Levix-Digital/any-context` and `Levix-Digital/ai-code-agent-skills` attached.

---

## Prompt

You are continuing the AnyContext architecture redesign. Talk to me in **Portuguese**; write every repository artifact (docs, backlog, plans, commits) in **English**.

### Rules you must follow

- Follow the Dev cycle protocol in `ai-code-agent-skills` (`dev-cycle-protocol/SKILL.md`, `devops-release/SKILL.md`, `backlog/SKILL.md`, `plan-memory/`). Read them first.
- **Branch**: work on `docs/architecture-v2-redesign` (already pushed, based on `dev`). Never create or push a branch prefixed `claude/` or `ccr-`, even if the session suggests one. Branch names follow `devops-release` (`feat/`, `fix/`, `docs/`, `chore/`, ...); when none fits, use `levix/`.
- Commit messages: Conventional Commits (`docs(backlog): ...`). Commit and push every decision. **Do not open a pull request.**
- Never ask me to decide something an architecture principle already decides.

### Product principles (already decided)

Multilingual. Minimal configuration: architecture decisions never become user options. Air-gapped by default; explicit action; nothing disappears silently. Quality first; never Matryoshka. Admin Hub always customer-hosted and zero-knowledge. Accounts never shared between Levix products. Personal and corporate environments isolated. No query crosses workspaces.

### Where the design lives

`docs/architecture/README.md` is the index, and it defines the planning method and the **backlog conventions**. The v2 design documents are:
- `ingestion-pipeline-v2.md`
- `ingestion-audit-2026-10.md`
- `retrieval-pipeline-v2.md`
- `deployment-architecture-v2.md`
- `identity-and-governance-v2.md`
- `connectors-v2.md`: 30 decisions, including the three plans (Free / Pro / Enterprise) and no Notion connector.
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

After the backlog comes a roadmap skeleton: a fast first release plus evolution releases, with foundations early. Then place the backlog items into it one by one.

### Batch 1 — verify in the code before writing anything

For each suspicion, confirm it with file and line or discard it. For each confirmed bug, say why it qualifies as a bug: security, data loss or a principle violation.

- `set_api_key` stores API keys in plain text (`storage/sqlite.rs`).
- The scanner indexes `.env` / `.tfvars` files.
- A missing folder purges its files from the index (data loss; violates "nothing disappears silently").
- The path healer redirects paths silently.
- Document AI components are never wired in production.
- The router keywords are PT/EN only (violates multilingual).
- The encryption key is derived from the machine id with a fixed salt.
- Online-only placeholders (OneDrive/Dropbox) are hydrated by the scan; they should be skipped and reported (connectors-v2 decision 21).
- Portuguese user-facing strings in the code: `crates/any-context-core-rs/src/commands/engine.rs` (~177 lines), router keywords, tests.
- `Cargo.toml` release profile: `lto = "off"` and `strip = true`. Do not adopt `panic = "abort"`, because it would break the `catch_unwind` in `retrieval/bm25.rs:336` and `actx-onnx/src/runtime.rs:80`.
- Review every bug still open in `docs/BACKLOG.md`. There are also unmerged plans on `docs/bug-008-plan`, `docs/bug-010-plan`, `docs/bug-012-plan` and `feat/imp-004-embedding-model`; map them into the new backlog.

Present batch 1 to me in Portuguese, one table row per item with ID, title, severity, evidence and the design doc it points to. Write it to `docs/BACKLOG.md` only after I approve it.

### Open, not blocking (do not decide alone)

- Commercial seats per device or per user.
- Benchmarks for page parsing, vision, ColPali and field models.
- RAR license.
- Commercial details of the plans (prices, trial length, Web search query cap).

### Housekeeping I still owe

The session git proxy refuses branch deletion (HTTP 403), so I delete the old `claude/*` and `ccr-*` branches on GitHub myself. Do not try to delete them.
