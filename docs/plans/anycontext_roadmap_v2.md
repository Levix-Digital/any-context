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
6. **Delivery:** installers for Windows, macOS and Linux; Windows signed with Azure
   Artifact Signing and Linux with GPG; macOS unsigned and not notarized until there is
   revenue (decided 2026-10-23).

**Items (placed 2026-10-23, 95):**

| area | items |
|---|---|
| bugs | BUG-022, BUG-024, BUG-025, BUG-026, BUG-027, BUG-028, BUG-029 |
| storage | IMP-031, IMP-033, IMP-034, IMP-036, IMP-051, IMP-052, IMP-053, FEAT-069 |
| models | IMP-027, IMP-028, IMP-029, IMP-030, IMP-054, IMP-055, IMP-057, IMP-058, FEAT-067 |
| ingestion base | IMP-015, IMP-016, IMP-017, IMP-020, IMP-021, IMP-023, IMP-025, IMP-026, IMP-032, FEAT-020, FEAT-031, FEAT-036, FEAT-057 |
| ingestion worker | IMP-059, IMP-060, IMP-061, FEAT-050 |
| Pro: Web | IMP-019, FEAT-021, FEAT-022, FEAT-023, FEAT-149, FEAT-160, FEAT-161, FEAT-170 |
| Pro: Audio and video | FEAT-047 |
| retrieval | IMP-035, IMP-037, IMP-038, IMP-039, IMP-040, IMP-041, IMP-042, IMP-043, IMP-044, IMP-045, IMP-046, IMP-047, IMP-048, IMP-049, IMP-050, FEAT-081, FEAT-082, FEAT-085, FEAT-088, FEAT-098, FEAT-101 |
| identity and plan | FEAT-193, IMP-062, FEAT-168, FEAT-169 |
| desktop UI | FEAT-172, FEAT-173, FEAT-174, FEAT-175, FEAT-176, FEAT-177, FEAT-178, FEAT-179, FEAT-184, FEAT-185 |
| CLI and release | FEAT-186, FEAT-188, FEAT-189, FEAT-190, FEAT-192, IMP-063, IMP-064, ADJ-006, ADJ-007, ADJ-008 |

Needed from `levix-platform`: FEAT-001 (personal account only), FEAT-003, FEAT-004, FEAT-009,
FEAT-010, FEAT-013, FEAT-019, FEAT-020, FEAT-021.

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
