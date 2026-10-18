---
id: 131
title: Field-level encryption on every LanceDB write
status: accepted
date: '2026-10-16'
version: null
subsystems: [storage, ingestion]
supersedes: []
superseded_by: []
amends: [100]
amended_by: [135, 142]
---

# ADR-0131: Field-level encryption on every LanceDB write

## Context

The design of encryption at rest (field-level AES-GCM-256 of `text`, `document_summary` and `keywords` with a hardware-bound key, `enc::` payloads) came from the Python version. ADR-0100 ported `NativeSecurityEngine` to Rust so encrypted payloads could be read, but the native write path never encrypted: `NativeLanceStore::upsert_records` stored chunk text, summaries and keywords in plaintext. Every chunk indexed by the Rust versions was readable directly from the LanceDB files.

## Decision

- `NativeLanceStore::upsert_records` encrypts `text`, `document_summary` and `keywords` with `NativeSecurityEngine::encrypt_text` for every record, whatever the caller (sync orchestrator, web pages, system knowledge bootstrap).
- `NativeLanceStore::extract_scored_results` keeps decrypting those fields on read, so every reader gets plaintext. Values that are already plaintext (older data) pass through unchanged.
- Ids, vectors, `file_name`, `file_path`, `workspace`, `last_modified`, `content_type` and `content_hash` stay in clear text so filters keep working in LanceDB.
- `/inspect <filter>` no longer filters with `text LIKE` in LanceDB (ciphertext cannot match); it scans a bounded sample and matches the decrypted fields in memory.

## Alternatives Considered

- **Encrypt in each caller**: rejected; one write path in the store guarantees no caller can forget it.
- **An opt-in setting**: rejected; the original design had encryption on by default and the user never chose to turn it off.
- **Migrating existing plaintext rows automatically**: not done here, because rows must be rewritten with their vectors; re-indexing (`actx sync --force`) rewrites them encrypted.

## Consequences

- New and re-indexed chunks are encrypted on disk; data indexed before this change stays plaintext until its files are re-indexed.
- Data is bound to the machine identifier: copying the LanceDB folder to another machine (or changing `ACTX_MACHINE_ID`) makes the protected fields unreadable.
- SQL filters on `text`, `document_summary` or `keywords` no longer match; filters must use the clear-text metadata columns.
- The BM25 index file still stores chunk text in plaintext because lexical search needs the terms; encrypting it is separate work.
- Verified by the `upsert` test in `storage/lancedb.rs`: a plaintext `text LIKE` query finds nothing while reads return the decrypted text.
