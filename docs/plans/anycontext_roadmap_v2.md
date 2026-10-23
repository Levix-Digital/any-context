---
plan_id: anycontext_roadmap_v2
project: any-context
title: AnyContext roadmap (v2)
version: v1.0.0
status: active
author: Levi Silveira
created_at: '2026-10-23T20:45:30'
updated_at: '2026-10-23T20:45:30'
session_id: null
tags:
- roadmap
- v2
supersedes: anycontext_roadmap_pos_v0450_proximas_frentes
superseded_by: null
parent_plan_id: null
milestones:
- name: R1 v1.0.0 Personal
  done: false
- name: R2 v1.x Personal evolution
  done: false
- name: R3 v2.0.0 Enterprise foundations
  done: false
- name: R4 v2.x Enterprise shared index
  done: false
- name: Later
  done: false
status_log:
- at: '2026-10-23T20:45:30'
  by: Levi Silveira
  status: active
  note: Plan created
---

# AnyContext roadmap (v2)

Release roadmap of the v2 redesign, built after the complete backlog (`docs/BACKLOG.md`),
following the planning method of `docs/architecture/README.md`: a first release that
reaches the market as fast as possible on an already correct base, then the natural
evolution of the product. Backlog items are placed into the releases one by one, with the
owner's review; this skeleton fixes the releases and their goals.

Decided on 2026-10-23.

## Principles that shape it

- **The first release is for personal use only** (Free and Pro); corporate features come
  later (`connectors-v2.md`, decisions 22 to 31).
- **Foundations come early**, even when invisible, so the first release has little
  functionality on top of a correct base.
- **Bugs first:** the open bugs are fixed before the new work of a release.
- **Pro must ship with its connectors:** Web and Audio and video (`connectors-v2.md`,
  decision 28).
- The key hierarchy ships in R1, so the plain-text API keys never become a bug
  (`identity-and-governance-v2.md`, topic 2).

## Releases

| release | version | name | goal |
|---|---|---|---|
| R1 | v1.0.0 | Personal | reach the market with Free and Pro on a correct base |
| R2 | v1.x | Personal evolution | format coverage and extraction quality for the professional audiences |
| R3 | v2.0.0 | Enterprise foundations | corporate environment, Admin Hub, Enterprise connectors; deployment modes 1 and 2 |
| R4 | v2.x | Enterprise shared index | AnyContext Server (mode 3) and per-source access control |
| Later | — | Advanced | high-cost items, or items enabled only by a gain measured by the evaluation |

### R1 — v1.0.0 Personal

An MVP in features on a complete foundation.

1. **Bugs:** the open current-code bugs.
2. **Foundations:** knowledge base port with journal; `settings.db` split; single source
   model; LanceDB tables per workspace and embedding target; Tantivy per workspace; model
   port by role and hardware kits as bindings; embedding independent of the chat model,
   local and multilingual; per-chunk diff by hash; hierarchical, token-sized chunks; job
   queue, lease and worker events; chat history as a tree.
3. **Security and plan:** adoption of the platform crates with the key hierarchy, the
   personal vault and **one personal account** (password and recovery key); the plan
   license verified offline; Free and Pro limits and downgrade; Levix signature
   verification of downloads.
4. **Product:** the Tauri desktop app (explorer, chat, reader, inspector), from the first
   release; Fast and Deep with tools; the reranker as the relevance judge, selection and
   the nothing-found gate; checked citations; the source report.
5. **Pro:** the Web connector (sites, pages and web search with the fair-use ceiling and
   the embedded Chromium) and the Audio and video connector (local transcription).
6. **Delivery:** installers for Windows, macOS and Linux; code signing and macOS
   notarization, depending on the owner's purchase of the certificates.

### R2 — v1.x Personal evolution

Full Office, PDF and e-mail; archives; ONNX OCR and Document AI; forms with LSIE;
ingestion self-evaluation; domain packs; automatic sync; summaries and the profile's local
LLM; several accounts per machine with `root` and `sudo`; biometrics.

### R3 — v2.0.0 Enterprise foundations

Personal and corporate environments; Admin Hub (`levix-admin-hub`) and the platform's
corporate identity, policies and governance engine; endpoint trust levels and the
confidential ceiling; Enterprise connectors (Google Drive, OneDrive and SharePoint,
Dropbox, IMAP, YouTube, Obsidian); the Model Server (mode 2).

### R4 — v2.x Enterprise shared index

AnyContext Server (mode 3), groups and organization workspaces, per-source access control.

### Later

Entity graph, visual page retrieval, late chunking, learned sparse vectors, compact
vectors, approximate vector index, niche formats.

## Versioning

- The version files of `dev` carry `1.0.0` once R1 is bumped; release candidates are tagged
  on `dev` as `v1.0.0-rc.N` (`devops-release`), installed and accepted by the owner; the
  final `v1.0.0` tag is created when the owner merges `dev` into `main`.

## Next step

Place the backlog items into the releases one by one, release by release, for the owner's
review; each item's placement is recorded in this plan.
