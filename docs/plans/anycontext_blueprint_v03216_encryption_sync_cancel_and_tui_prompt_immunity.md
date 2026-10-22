---
plan_id: anycontext_blueprint_v03216_encryption_sync_cancel_and_tui_prompt_immunity
project: any-context
title: 'Blueprint v0.32.16: Transparent Vector Decryption, Sync Cancellation, Session Directory Navigation & TUI Prompt Immunity'
version: null
status: completed
author: Levi Silveira
created_at: '2026-10-08T09:17:35'
updated_at: '2026-10-08T09:17:35'
session_id: null
tags: []
supersedes: null
superseded_by: null
parent_plan_id: null
milestones: []
status_log:
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  status: completed
  note: Plan file had no entry in the plan-memory database; metadata added during migration
---

# Blueprint v0.32.16: Transparent Vector Decryption, Sync Cancellation, Session Directory Navigation & TUI Prompt Immunity

## 1. Context & Root Cause
In version `v0.32.15`, after tests on several operating systems (Windows and Linux), three engineering and usability bottlenecks were identified:
1. **Encrypted Text on Linux with `/inspect --full` (`enc::...`)**:
   - AnyContext implements AES-GCM-256 encryption at rest with PBKDF2-HMAC-SHA256 bound to the hardware signature (`SecurityEngine` in Python).
   - When documents were synced on Linux through the Python worker, the `text`, `document_summary` and `keywords` fields were persisted in LanceDB as `enc::<base64_payload>`.
   - In the Rust Core, `execute_inspect` and the vector reader read this raw data, showing undecipherable encrypted strings to the user and threatening to pollute the context given to the LLM in RAG queries.
2. **No Way to Cancel Running Syncs (`/sync cancel`)**:
   - While syncing large directories (e.g. 268,865 files), the background worker monopolized I/O and CPU with no option in the TUI or CLI to interrupt it immediately.
   - The `/folder .` command inside the TUI referred to the process launch directory with no way to navigate internally.
3. **BM25 Warning Leaking into the Prompt Widget and I/O Concurrency**:
   - Warning messages printed with `eprintln!` in `pipeline.rs` leaked straight to `stderr`. In Ratatui (*Alternate Screen* mode), this overwrote characters directly on the cursor line of the `Prompt` widget.
   - During concurrent reindexing, `BM25Index::load_from_file` tried to read the file while the worker was writing it or before the atomic flush, resulting in `failed to fill whole buffer`.

---

## 2. Architecture Decisions & Solution (ADR-100)

### 2.1 NativeSecurityEngine in the Rust Core (`crates/any-context-core-rs/src/security/mod.rs`)
- **Hardware Derivation Identical to Python**:
  - Linux: `/etc/machine-id` or `/var/lib/dbus/machine-id` (`lin_mid_*`).
  - Windows: registry `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` (`win_guid_*`).
  - macOS: `IOPlatformUUID` (`mac_uuid_*`).
  - Override for tests through the `ACTX_MACHINE_ID` environment variable.
- **KDF & Cipher**: PBKDF2-HMAC-SHA256 (100,000 rounds) with `DOMAIN_SALT` and `INTERNAL_PEPPER`, producing a 256-bit key for `Aes256Gcm`.
- **Transparent Decryption at Runtime**:
  - `NativeLanceStore::extract_scored_results` automatically decrypts `text`, `document_summary` and `keywords` for every vector and metadata query.
  - `/inspect` (with or without `--full`) decrypts the content in the sampling and the taxonomy.

### 2.2 Sync Cancellation & Directory Navigation
- **`/sync cancel` (and `/cancel`, `/sync --cancel`)**:
  - Looks up the worker PID stored in SQLite (`workspace_sync_status.pid`).
  - Kills the process tree (`taskkill /PID /F /T` on Windows and `kill -9` on Unix).
  - Updates SQLite to `is_syncing = 0`, `stage = "cancelled"`, `progress_bar = "[cancelled]"`.
  - Adds a "Cancel Running Sync" button to the TUI menu (`[F1]` or `/sync`).
- **`/pwd` and `/cd <path>`**:
  - `/pwd`: shows the process's current working directory.
  - `/cd <path>`: changes the working directory at runtime (`std::env::set_current_dir`).
  - `/folder .`: prints an explicit note `(resolved from current working directory)`.

### 2.3 TUI Prompt Immunity and Concurrent BM25 Resilience
- **No more `eprintln!`**: corruption or quarantine warnings are written to a dedicated file (`bm25_quarantine.log`) without leaking bytes into the Ratatui screen buffer.
- **0-Byte Tolerance**: `BM25Index::load_from_file` detects empty files (`meta.len() == 0`) and initializes a new index without raising an error.
- **Retry with Backoff on Windows**: `save_to_file` and `load_from_file` use exponential retry loops with sleep, tolerating momentary locks from concurrent reads.

---

## 3. Execution Milestones
- **M1: NativeSecurityEngine & LanceDB Decryption**: cryptographic module in Rust with Python/Rust interoperability tests 100% green.
- **M2: Sync Cancellation & `/cd`/`/pwd` Commands**: `/sync cancel` (with `taskkill`/`kill`), `/cd`, `/pwd`, and a TUI menu option.
- **M3: BM25 Hardening and TUI Immunity**: removal of `eprintln!`, 0-byte tolerance and concurrent retry backoff.
- **M4: Automated Test Suite**: 131 Core unit tests, 18 command tests (`command_tests.rs`), 14 CLI tests (`cli_tests.rs`) and Python tests 100% green.
- **M5: Dual-Doc & Scenario 17 in MANUAL_TESTS.md**: complete documentation in `README.md`, `TECDOC.md` (ADR-100) and cumulative, non-truncated Scenario 17.
- **M6: Gate 8 Verification & Release v0.32.16**: validation through `verify_gate.py`, tag `v0.32.16`, push and CI watch of the 12 release binaries.
