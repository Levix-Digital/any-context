# Analysis: spec 002 consistency (speckit-analyze)

Date: 2026-10-29. Inputs: spec.md, plan.md, tasks.md, research.md, data-model.md, contracts/embedding-cli.md, constitution 1.5.0.

## Requirement coverage

| Requirement | Tasks |
|---|---|
| FR-001 catalog fields | T003, T004 |
| FR-002 catalog entries | T003, T004, T030 |
| FR-003 profile defaults in the kit | T006, T034 |
| FR-004 local provider | T009 to T012 |
| FR-005 setting independent of chat | T008, T024 |
| FR-006 default and upgrade adoption | T015, T020 |
| FR-007 show and change | T021 to T023, T026 |
| FR-008 identity includes dimension and prefixes | T005, T029 |
| FR-009 per-chunk target, filtered search | T007, T017 |
| FR-010 no fallback, failures retried | T015, T017, T018 |
| FR-011 mock only in tests | T015, T018 |
| FR-012 prefixes both sides, not stored | T027 to T029 |
| FR-013 no lexical-only mode | T014 |
| FR-014 verified download only | T004, T013 |
| FR-015 benchmark local dense | T031 |
| FR-016 dense CI job | T032, T035, T037 |
| FR-017 comparison recorded, defaults follow | T033, T034 |
| FR-018 model source (mirror, pre-placed files) | T014a |
| FR-019 allowed-models seam | T014b |

All 19 requirements and SC-001 to SC-006 are covered (SC-003 T024, SC-004 T018, SC-005 T036, SC-006 T035).

## Findings

| Id | Severity | Finding | Resolution |
|---|---|---|---|
| A1 | Resolved | Clarifications answered by Levi on 2026-10-29: download on first need from the configured source; `Lite` downloads e5-small (no lexical-only mode); upgrade adopts existing cloud indexes. Corporate governance seam added (FR-018, FR-019) | Spec, research R9b, tasks T014a and T014b updated |
| A2 | Low | Rollback to the previous version reindexes workspaces whose new-format key it does not parse | Accepted; stated in the plan's rollback section and release notes |
| A3 | Low | Gemini indexes reindex once because of the new prefixes | Accepted (quality gain, IMP-014 note); release notes |
| A4 | Low | Reference-vector test needs model files; skipped locally when absent | Required in CI through `ACTX_REQUIRE_EMBEDDER_TESTS=1` (T011, T037) |

No constitution violations; no duplicated or conflicting requirements; terminology consistent (embedding target, embedding setting, catalog).
