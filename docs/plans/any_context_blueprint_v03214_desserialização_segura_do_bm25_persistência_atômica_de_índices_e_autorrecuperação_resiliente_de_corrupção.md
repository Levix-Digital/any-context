---
plan_id: any_context_blueprint_v03214_desserialização_segura_do_bm25_persistência_atômica_de_índices_e_autorrecuperação_resiliente_de_corrupção
project: any-context
title: 'Blueprint v0.32.14: Safe BM25 Deserialization, Atomic Index Persistence and Resilient Self-Healing from Corruption'
version: v0.32.14
status: superseded
author: Levi Silveira
created_at: '2026-10-06T12:51:14'
updated_at: '2026-10-09T11:25:02'
session_id: 19321f47-165d-446a-8872-6d03b214fa63
tags: []
supersedes: any_context_blueprint_v03213_clean_screen_startup_and_long_term_memory_preservation
superseded_by: any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
parent_plan_id: null
milestones:
- name: 'M1: Bounded, Panic-Safe Deserialization in the Rust Core'
  done: false
- name: 'M2: Atomic Persistence of Binary Indexes'
  done: false
- name: 'M3: Resilient Pipeline and Quarantine of Corrupted Indexes'
  done: false
- name: 'M4: Local Sanitization and Automated Tests'
  done: false
- name: 'M5: Dual-Doc and Cumulative Manual Tests'
  done: false
- name: 'M6: Production Release v0.32.14 and CI Watch'
  done: false
status_log:
- at: '2026-10-09T11:25:02'
  note: Superseded by any_context_blueprint_v0330_paridade_integral_de_ingestao_e_recuperacao_de_indices_rust_core_e_purga_do_codigo_legado_python
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.14: Safe BM25 Deserialization, Atomic Index Persistence and Resilient Self-Healing from Corruption

## 1. Overview & Context of the Critical Failure
While running queries in the `IKEAShipments` workspace with the `search_db` tool, the application suffered a catastrophic crash (process abort / panic):
```
memory allocation of 7939688266103746149 bytes failed
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Forensic Root Cause Analysis
1. **The Allocation Number**:
   `7939688266103746149` in hexadecimal is `0x6e2f706968736e65`, which corresponds exactly to the ASCII bytes `'enship/n'` (a fragment of `citizenship/news` from indexed government URLs).
2. **Race Condition on Write**:
   The `bm25_index.bin` file (more than 32,000 documents and 85 MB) was being written directly through `std::fs::File::create(path)`, non-atomically.
   When the background sync worker and the TUI/search_db wrote concurrently to the same file, the bytes of two different documents overlapped at offset `5976850`.
3. **Unbounded Deserialization (`bincode`)**:
   When reading through `bincode::deserialize_from(reader)` in `BM25Index::load_from_file`:
   - By default, `bincode` imposes no limit when reading collection/string sizes.
   - On reaching the 8 corrupted bytes `b'enship/n'`, it read them as the size of a `workspace` string, attempting `Vec::with_capacity(7939688266103746149)`.
   - Rust's global allocator panicked at once and aborted the process, bringing down the TUI and any worker, with no chance of being caught by `try/catch` blocks in Python or `Result` in Rust.

---

## 2. Architecture Decisions (ADR-096)

1. **Defensive, Bounded Deserialization (`bincode::Options`)**:
   - Configure the `bincode` deserializer with a strict allocation limit: `bincode::DefaultOptions::new().with_limit(100 * 1024 * 1024)`.
   - If any field requires an allocation above the limit (or an absurd one such as 7.9 exabytes), `bincode` aborts immediately with a canonical `Err(bincode::ErrorKind::SizeLimit)` error, without trying to allocate heap memory.
   - Wrap the operation in `std::panic::catch_unwind` as a second-layer protection against unexpected panics.

2. **Atomic Persistence through a Temporary File and Rename (`atomic_save`)**:
   - Always write new BM25 indexes to a unique temporary file (`<path>.tmp.<pid>.<timestamp>`), flush/sync to disk and replace atomically through a rename (`std::fs::rename` / replace).
   - Removes any chance of readers accessing half-written files or concurrent writers corrupting binary streams.

3. **Resilient Self-Healing of Corrupted Indexes (*Self-Healing Recovery*)**:
   - In `NativeHybridPipeline::get_or_load_bm25` and `HybridRetrieverEngine::load_bm25_from_file`: if reading fails because of corruption or a limit error, the application:
     1. Logs a detailed warning.
     2. Moves/quarantines the faulty binary file (`.corrupt.<timestamp>`).
     3. Instantiates a clean BM25 index and, when there are intact records in LanceDB (`workspace_chunks`), rebuilds the lexical index resiliently.
     4. Writes the new valid index atomically.

---

## 3. Execution Milestones

- **M1: Bounded, Panic-Safe Deserialization in the Rust Core (`bm25.rs`)**:
  Implement strict `bincode` limits and a `catch_unwind` barrier in `BM25Index::load_from_file`.
- **M2: Atomic Persistence of Binary Indexes (`bm25.rs`)**:
  Implement writing to a temporary file with an atomic rename in `save_to_file`.
- **M3: Resilient Pipeline and Quarantine of Corrupted Indexes (`pipeline.rs` & `hybrid.rs`)**:
  Integrate corruption detection, quarantine and safe initialization/recovery without crashing.
- **M4: Local Database Sanitization and Automated Tests**:
  Quarantine the current corrupted file in `%LOCALAPPDATA%`, add automated Rust tests covering anomalous allocations and concurrent roundtrip tests.
- **M5: Dual-Doc Pattern & Cumulative Manual Tests**:
  Update `README.md`, `TECDOC.md` with ADR-096 and add Scenario 15 to `tests/MANUAL_TESTS.md`.
- **M6: Production Release v0.32.14 & Uninterrupted CI Follow-up**:
  Version bump to `0.32.14`, commit, tag, push and CI monitoring until the binaries are publicly available.
