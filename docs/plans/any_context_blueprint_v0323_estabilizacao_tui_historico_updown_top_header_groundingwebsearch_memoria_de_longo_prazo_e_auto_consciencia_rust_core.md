---
plan_id: any_context_blueprint_v0323_estabilizacao_tui_historico_updown_top_header_groundingwebsearch_memoria_de_longo_prazo_e_auto_consciencia_rust_core
project: any-context
title: 'Blueprint v0.32.3: TUI Stabilization (Up/Down History, Grounding/WebSearch Top Header, Long-Term Memory and Rust Core Self-Awareness)'
version: v0.32.3
status: superseded
author: Levi Silveira
created_at: '2026-10-02T09:43:05'
updated_at: '2026-10-02T10:27:58'
session_id: 0a07fe95-3c95-480f-904a-97aa13635137
tags:
- tui
- prompt-history
- top-header
- grounding
- web-search
- long-term-memory
- sqlite-session-store
- rag
- actx-cli
- v0.32.3
supersedes: any_context_blueprint_prioridade_4_interface_cli_tui_100_nativa_em_rust_actx_cli_actx_tui
superseded_by: any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
parent_plan_id: null
milestones:
- name: 'M1: Windows Binary & Real Auto-Update'
  done: true
- name: 'M2: Up/Down Prompt History per Workspace'
  done: true
- name: 'M3: Reactive Top Header with Grounding and WebSearch'
  done: true
- name: 'M4: Session Memory and Rust Core Self-Awareness'
  done: true
- name: 'M5: Automated Tests and Dual-Doc'
  done: true
status_log:
- at: '2026-10-02T10:14:14'
  note: 'Milestone ''M1: Windows Binary & Real Auto-Update'': release v0.32.3 resolved and built into bin/actx.exe'
- at: '2026-10-02T10:14:16'
  note: 'Milestone ''M2: Up/Down Prompt History per Workspace'': Up/Down readline isolated per workspace implemented'
- at: '2026-10-02T10:14:17'
  note: 'Milestone ''M3: Reactive Top Header with Grounding and WebSearch'': WS/Model/Mode/Web quad-status implemented and reactive'
- at: '2026-10-02T10:14:18'
  note: 'Milestone ''M4: Session Memory and Rust Core Self-Awareness'': SQLite session store connected and self-awareness prompt active'
- at: '2026-10-02T10:14:19'
  note: 'Milestone ''M5: Automated Tests and Dual-Doc'': 581 tests 100% green, TECDOC ADR-080..083, README and Scenario 5'
- at: '2026-10-02T10:14:27'
  note: Release v0.32.3 deployed to main and tag pushed
- at: '2026-10-02T10:27:58'
  note: Superseded by any_context_blueprint_v0324_desacoplamento_universal_do_motor_de_comandos_e_separacao_grounding_mode_vs_search_mode
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# Blueprint v0.32.3: TUI Stabilization, Up/Down History, Top Header and Rust Core Memory

## 1. Goal & Architecture Decisions (ADRs)
- ADR-080: Prompt History Isolated per Workspace through a Ring Buffer and the Up/Down Keys, reserving PageUp/PageDown and the Mouse for Scrolling.
- ADR-081: Reactive Top Header with a Quad-Status (Workspace, Model, Grounding Mode, Web Search).
- ADR-082: Continuous Session, Self-Awareness and Context Tool Calling in actx-agent with SqliteSessionStore and a Canonical System Prompt.
- ADR-083: Canonical Releases Repository Levix-Digital/any-context-releases and Real Update in actx-cli.

## 2. Interface Parity
- TUI / CLI: complete Top Header, prompt history navigation, restoration of visual messages, /mode and /web-search commands updated in real time.
- Core: SqliteSessionStore active with session_id ws_{workspace}, native RAG through search_db, prompt with self-awareness.

## 3. Affected Files
- crates/actx-cli/src/tui/app.rs
- crates/actx-cli/src/tui/events.rs
- crates/actx-cli/src/tui/ui.rs
- crates/actx-cli/src/engine.rs
- crates/actx-cli/src/cli/oneshot.rs
- crates/actx-cli/src/commands/dispatcher.rs
- Cargo.toml / pyproject.toml / version.txt

## 4. Dual-Doc & Manual Tests
- UserDoc: README.md with the Top Header and Up/Down navigation.
- TecDoc: TECDOC.md with ADR-080 to ADR-083 and a session persistence diagram.
- Manual Tests: cumulative Scenario 5 in tests/MANUAL_TESTS.md.
