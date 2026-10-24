---
plan_id: any_context_ingestion_secret_files_env_tfvars_key_are_indexed_in_clear
project: any-context
title: '[ingestion] Secret files (.env, .tfvars, .key) are indexed in clear'
kind: bug
version: v1.0.0
status: active
author: Levi Silveira
created_at: '2026-10-24T14:19:38'
updated_at: '2026-10-24T14:23:19'
session_id: null
tags:
- ingestion
- security
- secrets
- r1
supersedes: null
superseded_by: null
parent_plan_id: null
backlog_items:
- BUG-024
milestones:
- name: M1 Secret detector
  done: false
- name: M2 Redaction in the pipeline
  done: false
- name: M3 Repair of existing indexes
  done: false
- name: M4 Documentation and manual tests
  done: false
- name: M5 Review and PR
  done: false
status_log:
- at: '2026-10-24T14:19:38'
  by: Levi Silveira
  status: draft
  note: Plan created
- at: '2026-10-24T14:23:19'
  by: Levi Silveira
  status: active
  note: Plan updated
---

# [ingestion] Secret files (.env, .tfvars, .key) are indexed in clear

Bug plan for BUG-024 (critical), the first item of release v1.0.0 Personal (roadmap v2, R1 bugs, group A). Discovery and planning follow `dev-cycle-protocol` Phases 1 and 2.

## Backlog items

### BUG-024: [ingestion] Secret files (.env, .tfvars, .key) are indexed in clear
- **Severity**: critical
- **Found**: 2026-10-23, batch 1 of the v2 backlog rebuild
- **Description**: Design: docs/architecture/ingestion-pipeline-v2.md, section 7 "Personal data, sensitive data and secrets (audit P13)". The scanner accepts `.env` and `.env.*` (ingestion/scanner.rs:102-106) and the `env` and `tfvars` extensions (scanner.rs:186, 192); `key` is accepted as Keynote (scanner.rs:146), but a PEM `server.key` falls to the plain-text chunker (ingestion/router.rs:157-160). Their secrets are indexed in clear and can reach a cloud embedding model or LLM. Expected: secret values are detected and never indexed (`[SECRET REDACTED]`), with no configurable exception, while the rest of the file stays searchable.

## 1. Triage

- **Expected**: `docs/architecture/ingestion-pipeline-v2.md`, section 7, "Personal data, sensitive data and secrets": secrets (API keys, tokens, passwords, private keys) are detected by known patterns and entropy, and the value is never indexed, with no configurable exception; it becomes `[SECRET REDACTED]` and the rest of the file stays useful.
- **Actual**: no redaction exists anywhere in the code (no `redact` or secret detector in `crates/`). Every accepted file reaches the chunkers as is.
- **Verdict**: bug. The expected behavior is defined by the approved v2 design, and the current behavior leaks credentials to the index on disk, to the embedding provider and to the chat model.
- **Severity**: critical confirmed. A secret that reached a cloud provider cannot be taken back; the user has to rotate it.
- **Suspected code paths** (all confirmed in section 3):
  - `ingestion/scanner.rs` `is_supported_file`: accepts `.env`, `.env.*`, `env`, `tfvars`, `key`, `ini`, `cfg`, `conf`, `properties`, `yaml`, `json`, `toml` and every code file, all of which commonly hold secrets.
  - `ingestion/router.rs` `chunk_file_native` / `chunk_text_native` / `chunk_bytes_native`: the single entry point of file and web text into chunks; no filtering.
  - `ingestion/orchestrator.rs:603-609`: the contextual envelope reads the **raw file again** (`std::fs::read_to_string`) and its summary and keywords are prefixed to every chunk that is embedded, so a fix limited to the chunk text would still leak through the envelope.
  - `ingestion/orchestrator.rs:911` `index_web_page`: crawled page text goes through `chunk_text_native` too.
  - `ingestion/model_router.rs` `process_document_ai_candidates`: sends chunk text to the LLM (Document AI); it receives chunks from the router, so it is covered once the router redacts.

## 2. Reproduction

On `dev` 968dc89, a workspace with one folder that holds:

```
# .env
DATABASE_URL=postgres://app:<password>@db.internal:5432/app
OPENAI_API_KEY=sk-proj-<48 random characters>
STRIPE_KEY=sk_live_<random characters>
```

and a PEM `server.key` (`-----BEGIN PRIVATE KEY----- ... -----END PRIVATE KEY-----`).

1. `actx sync` (any embedding target; `mock` is enough to see the index).
2. `actx search "OPENAI_API_KEY"` (or `/search` in the TUI): the chunk comes back with the full key; the `server.key` chunk comes back with the whole private key.
3. With a cloud embedding target, the same text was sent to the provider in the embedding request; with a cloud chat model, the retrieved chunk is sent in the prompt.

The regression test of section 6 reproduces steps 1 and 2 in a sandbox (`IngestionRouter` on the `.env` and PEM samples, and a full `NativeSyncOrchestrator::run` with the `mock` target). It is written first in M2 and must fail on `dev` before the fix is applied; until then the reproduction rests on the code reading of section 3.

## 3. Evidence and root cause

Confirmed in the code:

- `scanner.rs:101-106` returns `true` for `.env` and `.env.*`; `scanner.rs:181-192` accepts `env`, `ini`, `cfg`, `conf`, `properties`, `tf`, `tfvars`, `hcl`; `scanner.rs:146` accepts `key` (meant for Keynote).
- `router.rs:145-162`: `key` is not an Office, PDF, image or tabular extension, so `chunk_file_native` reads it with `read_to_string` and sends it to `chunk_text_native`, which falls to `TextChunker`. A PEM file is valid UTF-8, so it is chunked as text. A real Keynote package is a ZIP, so `read_to_string` fails and the file is reported as an error (no leak, but no indexing either; out of scope here).
- No stage between reading a file and writing a chunk inspects its content for credentials: the router, the quality gate (`model_router.triage_chunks`), the enricher and `embed_and_upsert_chunks` pass the text through.
- `orchestrator.rs:603-609`: `doc_sample` is the raw file; `enricher.extract_envelope` builds a summary from its first sentences and keywords by frequency, and `apply_envelope_to_chunk` prefixes them to every chunk before embedding and storage. Secrets in the first lines of a `.env` file are therefore copied into **every** chunk of that file.
- Chunks are encrypted at rest (`storage/lancedb.rs:457-466`), so the disk copy is protected by the machine key, but the plaintext still reaches the embedding provider, the BM25 index in memory, search results and the chat model.

**Root cause**: the pipeline has no secret detection stage; the design's "never indexed" rule was not implemented.

## 4. Fix design (`software-design`)

### Where it lives

A new pure core module, `crates/any-context-core-rs/src/ingestion/secrets.rs` (domain logic, no I/O, no adapter imports), with one entry point:

```rust
pub const SECRET_PLACEHOLDER: &str = "[SECRET REDACTED]";

pub struct Redaction {
    pub text: String,       // input with every secret value replaced by SECRET_PLACEHOLDER
    pub secrets_found: usize,
}

pub fn redact_secrets(text: &str) -> Redaction;
```

It is called at the **two places where content enters the index**, so every format and every caller is covered without touching the chunkers:

1. **`IngestionRouter`** (`ingestion/router.rs`), the single entry point of file and web text into chunks:
   - text formats (`chunk_text_native`, and therefore `chunk_file_native`'s text branch, `chunk_bytes_native`'s text branch and `index_web_page`): the **whole content is redacted before chunking**, so a secret that spans lines (a PEM block) is never split across two chunks and line numbers are preserved (a redacted value keeps the line structure; see rules).
   - extracted formats (PDF, Office, spreadsheets, images with OCR): their text only exists after extraction inside the chunker, so **each returned chunk's `text` and `header_path` are redacted** after chunking.
2. **The envelope sample** (`orchestrator.rs:603-609`): `doc_sample` is redacted before `extract_envelope`. The envelope itself goes away in v2 (section 7, "The current envelope goes away", a later R1 item), but while it exists it must not copy secrets into every chunk.

`SyncResult` gains `secrets_redacted: usize` (count of values replaced in the pass), shown by the CLI and TUI sync summary next to the other counters ("3 secret values redacted"), so the user sees that it happened. File paths are not listed in the summary; they go to the verbose log only.

### Detection rules (patterns and entropy, as in the market's secret scanners)

Applied in this order; each match replaces only the **value**, keeping the key name, quotes and line breaks, so the rest of the file stays searchable (`OPENAI_API_KEY=[SECRET REDACTED]`).

1. **Private key blocks**: `-----BEGIN [A-Z ]*PRIVATE KEY-----` up to the matching `END` line (RSA, EC, DSA, OPENSSH, PGP PRIVATE KEY BLOCK, encrypted PKCS#8). The whole block becomes one placeholder line; the lines it covered are kept as empty lines so later line numbers do not move. Public keys and certificates are not secrets and are kept.
2. **Known token formats** (provider prefixes with their fixed shapes): AWS access key id (`AKIA`/`ASIA` + 16), AWS secret in an `aws_secret_access_key` assignment, GitHub (`ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`), GitLab (`glpat-`), Slack (`xox[abprs]-`, webhook URLs), Stripe (`sk_live_`, `sk_test_`, `rk_live_`, `rk_test_`), Google API key (`AIza` + 35), OpenAI (`sk-`, `sk-proj-`), Anthropic (`sk-ant-`), Hugging Face (`hf_`), npm (`npm_`), SendGrid (`SG.`), Twilio (`SK` + 32 hex), JWT (`eyJ….eyJ….…`), Azure storage `AccountKey=` and `SharedAccessSignature=`.
3. **Credentials in URLs and connection strings**: the password part of `scheme://user:password@host` and `Password=` / `Pwd=` in ADO.NET-style strings.
4. **Secret-named assignments**: a key whose name contains `password`, `passwd`, `pwd`, `secret`, `token`, `api_key`/`apikey`/`api-key`, `access_key`, `private_key`, `client_secret`, `credential`, `auth` (as a word), `dsn`, `connection_string` (case-insensitive, `_`, `-` and `.` equivalent), in any of the forms `KEY=value`, `KEY: value`, `"key": "value"`, `key = "value"`, `export KEY=value`, `<key>value</key>` (env, YAML, JSON, TOML, tfvars/HCL, INI, properties, XML and code). Any non-empty value is redacted, except references that hold no secret: `${VAR}`, `$VAR`, `%VAR%`, `{{ … }}`, `var.x`, `process.env.X`, `os.environ[...]`, `env("X")`, booleans and numbers shorter than 6 digits.
5. **High-entropy values in assignments**: in a `KEY=value` / `key: value` assignment with any key name, a value of 20 or more characters from the base64, base64url or hex alphabets with Shannon entropy at or above 3.5 bits per character (hex) or 4.0 (base64) is redacted. Entropy is **only** applied to assignment values, never to free text, so commit hashes in prose, checksums in `go.sum`/lock files and long identifiers in code are not touched.

**No configuration**: there is no allow list, no setting and no flag to turn it off (design: "with no configurable exception"). A false positive costs one value in the index; a false negative costs a leaked credential.

### Deliberately left out

- Personal and sensitive data tagging and pseudonymization (FEAT-060 and the pseudonymization items): different rules, a model, and "nothing is masked in search".
- A "do not index" user choice per file (design: only by explicit user choice; a later item).
- Keynote packages being reported as errors (ZIP read as text): not a leak; the formats items of R2 handle Keynote.
- Removing the envelope: a later R1 item (contextual header). This plan only stops it from leaking.
- Redacting chat history or text the user types: not indexed content.

## 5. Affected surfaces and files

- Core: `ingestion/secrets.rs` (new), `ingestion/mod.rs` (module and re-export), `ingestion/router.rs` (redaction around chunking), `ingestion/orchestrator.rs` (envelope sample, `SyncResult.secrets_redacted`, the repair of section 8).
- Storage: `storage/lancedb.rs` and `storage/sqlite.rs` only through existing calls (`delete_by_exact_path`, stat cache delete) used by the repair.
- Surfaces (`architecture.surfaces`): `cli` and `tui` show the new counter in the sync summary (`actx-cli` sync output and TUI footer/summary). No command, flag, file format or public API changes.
- Same defect elsewhere: none found. `actx-cli/src/prompt.rs` reads `AGENT.md`-style instruction files the user writes for the agent; they are not indexed and are out of scope.

## 6. Tests (`quality-assurance`)

Automated (`cargo test --workspace`):

- **Unit, `ingestion/secrets.rs`**: one test per rule (PEM block with line count preserved; each provider prefix; URL password; each assignment syntax; entropy threshold on both sides: a 40-character random base64 value is redacted, a 40-character hex `go.sum`-style hash in free text is not); references (`${DB_PASSWORD}`, `process.env.API_KEY`) kept; key names and the rest of the line kept; idempotence (`redact(redact(x)) == redact(x)`); a text with no secret is returned unchanged with `secrets_found == 0`.
- **Unit, `ingestion/router.rs`**: a `.env`, a `terraform.tfvars`, a `server.key` (PEM), a `config.yaml` and a `settings.json` produce chunks with no secret value and with the key names still present; a PDF-like extracted chunk path (via `chunk_bytes_native` on a small text-layer PDF fixture) is redacted after extraction.
- **Regression, `tests/orchestrator_tests.rs`** `test_secrets_are_never_indexed`: a sandbox workspace with the `.env` and `server.key` samples of section 2 and one normal Markdown file; after `run` with the `mock` target, no stored chunk text, document summary or keyword list (decrypted) contains any of the secret values, the key names are searchable through BM25, and `secrets_redacted` equals the number of planted values. Written before the fix, it must fail on `dev`.
- **Repair**, `tests/orchestrator_tests.rs` `test_secrets_indexed_before_the_fix_are_reindexed`: rows written with a secret (direct `upsert_records`, as an older version would have) are found by the repair, their file is reindexed redacted on the next sync, and the second run of the repair is a no-op.

**Test fixtures**: GitHub push protection rejects commits that contain realistic tokens, so fixtures never hold a complete token literal: tests build each sample at run time from its parts (prefix + generated random body), and no fixture file holds a real-looking key.

Manual (`tests/MANUAL_TESTS.md`): new **MT-SYNC-009 "Secrets are redacted and the rest of the file stays searchable"**: index a folder with a `.env` and a PEM key, search the key name and the value, check the summary counter, and check that an older index with secrets is repaired after the upgrade. Validate with `manual_tests.py validate`.

## 7. Documentation (`architecture-docs`, `user-docs`)

- `docs/architecture/ingestion.md`: a "Secret redaction" section (where it runs, the rules, no configuration, the repair).
- `docs/architecture/chunkers.md`: one line that chunkers receive redacted text and extracted chunks are redacted after extraction.
- New ADR `0149-secret-values-are-redacted-before-chunking-with-no-exception.md`: decision (redaction at the router plus the envelope sample, patterns plus entropy restricted to assignments, no allow list), alternatives (skip secret files entirely: loses the useful rest of `.env`/config files and misses secrets in code and documents; redact per chunk only: splits PEM blocks and misses the envelope; entropy on all text: false positives on hashes), consequences. Then `build_index.py` and `check_docs.py`.
- `README.md` (`user-docs`): one sentence in the sync/sources section that secret values (API keys, tokens, passwords, private keys) are never indexed and appear as `[SECRET REDACTED]`; `check_readme.py`.

## 8. Release, migration and rollback (`devops-release`)

- **Version impact**: patch-level behavior fix inside the v1.0.0 line; no version bump in this PR (R1 is bumped once at the release candidate).
- **Data already affected**: indexes built by earlier versions hold secrets (encrypted at rest, but returned in search and sent to models). A one-time, idempotent repair runs at the start of a sync, next to the existing `encrypt_plaintext_rows` migration:
  1. scan the workspace's stored chunks (decrypted `text`, `document_summary`, `keywords`) with the same detector;
  2. for each file with a hit, delete its chunks (`delete_by_exact_path`), drop it from BM25 and from the stat cache, so the normal diff reindexes it redacted in the same pass;
  3. compact the table so old dataset files that kept the rows are removed (as the encryption migration does);
  4. report it once: "N files indexed by an earlier version contained secrets; they were reindexed with the secrets redacted. If your embedding or chat model is a cloud service, rotate those credentials." with the file list in the verbose log.
  Only affected files are re-embedded (no full reindex cost). A scan that finds nothing costs one read of the workspace's rows once; a marker in `settings.db` (`secret_redaction_version = 1` per workspace) skips it afterwards and lets a future rule change trigger it again.
- **Rollback**: revert the PR on `dev`. Rolling back would index secrets again on the next change of each file, so it is only for a broken build; already redacted chunks stay redacted. The repair is additive (it only deletes rows that are rebuilt in the same pass) and older versions keep reading the index.
- **Observability**: the `secrets_redacted` counter in the sync result and summary, the repair message, and verbose log lines with the file path and the rule name (never the value). Tested by the regression and repair tests.
- **Security**: the detector never logs, prints or stores a matched value; tests assert the value is absent from the result, the summary and the logs captured in verbose mode.

## 9. Execution steps and milestones

1. **M1 Secret detector**: `ingestion/secrets.rs` with the rules of section 4 and its unit tests.
2. **M2 Redaction in the pipeline**: router (before chunking for text, after extraction for the other formats), the envelope sample, `SyncResult.secrets_redacted` and the CLI/TUI summary; router unit tests and the regression test `test_secrets_are_never_indexed`.
3. **M3 Repair of existing indexes**: the one-time scan, reindex and compaction with its marker and message; `test_secrets_indexed_before_the_fix_are_reindexed`.
4. **M4 Documentation and manual tests**: `ingestion.md`, `chunkers.md`, ADR-0149, README, MT-SYNC-009; `check_docs.py`, `check_readme.py`, `manual_tests.py validate`.
5. **M5 Review and PR**: `software-design` checklist and `check_design.py`, PR `fix/bug-024-secret-redaction` into `dev`, protocol review, CI green.
