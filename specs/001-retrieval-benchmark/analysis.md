# Specification Analysis: Development retrieval benchmark

Consistency check of spec.md, plan.md and tasks.md (`/speckit-analyze`), 2026-10-25.

## Coverage

| Requirement | Tasks | Covered |
|---|---|---|
| FR-001 corpus pt/en, md/txt/pdf, CMR | T021, T022, T023 | yes |
| FR-002 hard distractors | T021, T022 | yes |
| FR-003, FR-004 question file, negatives | T006, T024 | yes |
| FR-005 validation before running | T019, T020 | yes |
| FR-006 isolated index, no user data | T010, T011 | yes |
| FR-007 real pipeline and defaults | T003 to T005, T011 | yes |
| FR-008 metrics overall, per language and category | T008, T009 | yes |
| FR-009 result JSON with configuration | T013 | yes |
| FR-010, FR-011 comparison, comparability | T016, T017 | yes |
| FR-012 0.02 gate, latency never gates | T016, T017 | yes |
| FR-013 explicit baseline update | T018 | yes |
| FR-014 CI on watched paths, summary, artifact | T027 | yes |
| FR-015 no keys in CI, cached reranker | T027 | yes |
| FR-016 no end-user change | plan structure (separate crate); `release.yml` builds only `actx-cli`, `actx-installer`, `any-context-core-rs` | yes |
| FR-017 deterministic metrics | T015, T026, T027 | yes |
| SC-001 under 5 minutes in CI | T029 | yes |
| SC-002 identical repeated runs | T015 | yes |
| SC-003 deliberate regression detected | T028 | yes |
| SC-004 every retrieval PR shows the comparison | T027 | yes |
| SC-005 corpus and question counts | T024 | yes |
| SC-006 product unchanged | T005 (no behavior change), release workflow unchanged | yes |

## Findings

| Id | Severity | Finding | Resolution |
|---|---|---|---|
| A1 | medium | The spec's "real retrieval pipeline with its current defaults" cannot report @10 metrics on the product's Fast request (6 results). | Resolved in research R2: ranking pass with `top_k` 10 and no density cut feeds the @k metrics and the gate; the delivered pass keeps the product's request for `delivered_hit_rate`, nothing-found and latency. FR-008 now names the delivered hit rate. |
| A2 | low | The spec says passage metrics are "reported only"; the plan's gate uses document metrics only. | Consistent. |
| A3 | low | Phase 5 (US4, P3) runs before Phase 6 although it has the lowest priority. | Intentional: validation must exist before the corpus is labeled; noted in tasks.md. |
| A4 | low | `Global` documentation is searched by the product but excluded by the benchmark. | Recorded in research R5 as a deliberate difference; IMP-017 replaces `Global`. |
| A5 | info | The CI workflow is not a required check at first. | Levi decides when it becomes required (plan, CI section). |

No critical or high findings; no constitution violations. Ready for the plan approval gate.
