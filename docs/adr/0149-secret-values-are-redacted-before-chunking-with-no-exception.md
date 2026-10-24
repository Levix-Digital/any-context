---
id: 149
title: Secret values are redacted before chunking, with no exception
status: accepted
date: '2026-10-24'
version: v1.0.0
subsystems: [ingestion, chunkers]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0149: Secret values are redacted before chunking, with no exception

## Context

The design of the v2 ingestion pipeline (`docs/architecture/ingestion-pipeline-v2.md`, section 7, "Personal data, sensitive data and secrets") says that secrets (API keys, tokens, passwords, private keys) are detected by known patterns and entropy and that their value is never indexed, with no configurable exception: it becomes `[SECRET REDACTED]` and the rest of the file stays useful. The code had no such stage (BUG-024, critical):

- The scanner accepts `.env`, `.env.*`, `env`, `tfvars`, `ini`, `cfg`, `conf`, `properties`, YAML, JSON, TOML and every code file, and `key` (meant for Keynote), so a PEM `server.key` is read as plain text.
- `IngestionRouter` passed every text to the chunkers as is, and nothing after it (quality gate, Document AI, enricher, embedding) looked at credentials.
- The document envelope read the raw file again and prefixed its summary and keywords to every chunk, so a secret in the first lines of a `.env` file was copied into every chunk of that file.
- Chunks are encrypted at rest (ADR-0131), but the plaintext still went to the embedding provider, the BM25 index, search results and the chat model.

## Decision

- **Detector**: `crates/any-context-core-rs/src/ingestion/secrets.rs`, a pure core module with `redact_secrets(text) -> Redaction { text, secrets_found }` and `SECRET_PLACEHOLDER` (`[SECRET REDACTED]`). Rules, in order:
  1. private key blocks (`-----BEGIN … PRIVATE KEY-----` to its END line, or to the end of a truncated text) become one placeholder line, and the lines they covered stay as empty lines; public keys and certificates are kept;
  2. provider tokens with a fixed prefix and shape (AWS access key ids, GitHub, GitLab, Slack and Slack webhooks, Stripe, Google API keys, Anthropic, OpenAI, Hugging Face, npm, SendGrid, Twilio, JWT);
  3. the password of `scheme://user:password@host`, and connection-string fields (`;Password=`, `;Pwd=`, `AccountKey=`, `SharedAccessSignature=`, `SharedAccessKey=`);
  4. the value of an assignment (`KEY=value`, `key: value`, `"key": "value"`, `key := value`, `<key>value</key>`) whose key name contains a secret word (`password`, `secret`, `token`, `api_key`, `client_secret`, `private_key`, `credential`, `auth`, `dsn`, …), unless the value is a reference (`${VAR}`, `$VAR`, `{{ … }}`, `var.x`, `process.env.X`, `os.environ[…]`, `<placeholder>`), a flag, a short number, or code (a call, a type name, a variable);
  5. the value of any assignment that is 20 or more characters of the base64 or hex alphabets with letters and digits and Shannon entropy of at least 4.0 bits per character (3.5 for hex). Entropy is never scored on free text, on `key:value` without a space (`h1:` checksums), on `sha*-` integrity strings, or on keys that name a checksum or an id.
  Only the value is replaced; key names, quotes and line breaks stay. There is no setting, flag or allow list.
- **Where it runs**: in `IngestionRouter`, the single entry point of file and web text into chunks. `chunk_text_redacted`, `chunk_file_redacted` and `chunk_bytes_redacted` return `RedactedChunks { chunks, secrets_redacted }`; text formats are redacted as a whole before chunking, and extracted formats (PDF, Office, workbooks, images) are redacted chunk by chunk (text and `header_path`) after extraction. `chunk_text_native`, `chunk_file_native` and `chunk_bytes_native` call them, so every caller gets redacted chunks. The orchestrator also redacts the raw file it reads for the document envelope.
- **Reporting**: `SyncResult::secrets_redacted` counts the values replaced in the files of the pass; the CLI prints `🔐 N secret value(s) redacted` after a sync, and verbose syncs name each file (never the value).
- **Repair of older indexes**: at the start of a sync, a workspace not yet scanned under `SECRET_REDACTION_VERSION` (`1`, recorded in `app_settings` as `secret_redaction_version:<workspace>`) has every stored chunk text, summary, keyword list and section path checked with the detector, one Arrow batch at a time (`NativeLanceStore::for_each_row`). Each file with a hit loses its rows, removed from the disk by compaction and pruning (`NativeLanceStore::purge_paths_from_disk`), and its stat cache entry, so the diff of the same pass reindexes it redacted. `SyncResult::secrets_repaired_files` counts them, and the CLI tells the user to rotate those credentials when a cloud model was used. Bumping the version runs the repair again after a rule change.

## Alternatives Considered

- **Skip secret files entirely** (stop indexing `.env`, `tfvars`, `key`): rejected. It loses the useful rest of configuration files, misses secrets in code, Markdown and documents, and the design asks for the rest of the file to stay searchable.
- **Redact each chunk after chunking, for every format**: rejected for text formats. A private key block split across two chunks escapes the block rule, and the envelope sample would still leak; kept only for extracted formats, whose text exists only inside their chunker.
- **Entropy on all text**: rejected. Commit hashes, checksums in lock files and long identifiers would be redacted and become unsearchable.
- **A user allow list or an off switch**: rejected by the design: a false positive costs one value in the index, a false negative a leaked credential.
- **Reindex the whole workspace once after the upgrade**: rejected. Only files with a stored secret need it, and a full reindex re-embeds everything with a cloud model's cost.

## Consequences

- No secret value matched by the rules reaches the index, the embedding provider, the BM25 index, search results or the chat model; key names stay searchable, so a search for `OPENAI_API_KEY` still finds the file.
- Secrets that do not match any rule (a plain password in free text with no key name) are still indexed; new rules are added to `secrets.rs` with a version bump.
- False positives remove a value from the index, never the file.
- A secret sent to a cloud provider by an earlier version cannot be taken back: the repair message asks the user to rotate it.
- Web pages are redacted on the same path; their values are not counted in `secrets_redacted`, which counts files.
- Test fixtures never hold a complete token literal (GitHub push protection rejects them): tests build samples at run time.
- Verified by the unit tests of `secrets.rs` and `router.rs`, the regression test `test_secrets_are_never_indexed` and the repair test `test_secrets_indexed_before_the_fix_are_reindexed` in `crates/any-context-core-rs/tests/orchestrator_tests.rs`, and manual test MT-SYNC-009.
