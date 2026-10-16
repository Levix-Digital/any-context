---
id: 85
title: Grounding Strategy vs Search Depth Mode Decoupling & Quint-Status Header
status: accepted
date: unknown
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0085: Grounding Strategy vs Search Depth Mode Decoupling & Quint-Status Header

- **Status**: Accepted & Implemented (`v0.32.4`).
- **Context**: `auto|fast|deep` (search retrieval depth) was conflated with `strict|hybrid|proactive` (LLM grounding verification rigor), creating user confusion and semantic corruption.
- **Decision**: Strictly isolate `/mode` (Grounding: `strict`, `hybrid`, `proactive`) from `/search` (Search Depth: `auto`, `fast`, `deep`). Provide independent interactive menus (`build_grounding_menu` vs `build_search_menu`). Expand TUI top header to quint-status telemetry (`WS`, `Model`, `Grounding`, `Search`, `Web`).

---
