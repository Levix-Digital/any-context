---
id: 85
title: Grounding Strategy vs Search Depth Mode Decoupling and Quint-Status Header
status: accepted
date: '2026-10-02'
version: v0.32.4
subsystems: [commands, tui, agent]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0085: Grounding Strategy vs Search Depth Mode Decoupling and Quint-Status Header

## Context

`auto|fast|deep` (retrieval depth) was conflated with `strict|hybrid|proactive` (how strictly the LLM must ground its answer), which confused users and mixed up two unrelated settings.

## Decision

Strictly separate `/mode` (grounding: `strict`, `hybrid`, `proactive`) from `/search` (search depth: `auto`, `fast`, `deep`). Provide independent interactive menus (`build_grounding_menu` and `build_search_menu`). Expand the TUI top header to five indicators: `WS`, `Model`, `Grounding`, `Search`, `Web`.

## Alternatives Considered

Not recorded.

## Consequences

- Each setting has its own command, menu and header indicator.
- Grounding rigor and retrieval depth can be tuned independently.
