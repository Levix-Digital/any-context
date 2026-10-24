# Architecture v2 — documents and method

Full review of AnyContext (October 2026): code, documentation and backlog.

## Documents

| document | content |
|---|---|
| `ingestion-pipeline-v2.md` | ingestion pipeline — target design |
| `ingestion-audit-2026-10.md` | ingestion audit ("Any Context"), proposals and decisions |
| `retrieval-pipeline-v2.md` | retrieval pipeline — current state and target design |
| `deployment-architecture-v2.md` | where each piece runs: ports, trust levels, workspace model, Admin Hub, deployment modes |
| `identity-and-governance-v2.md` | users, keys, roles and policies, with and without Admin Hub |
| `connectors-v2.md` | connectors: acquisition contract, authentication, sync, per-connector metadata, plans and licensing, policy |
| `platform-shared-components.md` | components shared across Levix products (Admin Hub, identity, licensing, LSIE) |
| `desktop-ui-v2.md` | desktop UI layout reference (Tauri 2) |
| `cli-and-release-v2.md` | command line, build profile, installers, updates, diagnostics and feedback |

## Language

Everything in the repository is written in English: code, documentation, backlog and
plans. (Several documents above are still being translated.)

## Planning method

1. **Complete backlog** of AnyContext at its ideal, without cutting by priority.
2. **Release roadmap:** a first release to reach the market as fast as possible, and the
   following ones as the natural evolution of the product.
3. **Prioritization:** backlog items are placed into the roadmap releases, one by one.

Cautions:

- **Foundations come early**, even when invisible (storage port with consistency, model
  port by role, `settings.db` split, parent–child hierarchy): the first release has little
  functionality on top of an already correct base.
- **Every backlog item points to the design section** it came from.

## Backlog conventions

1. **Same file and format** (`docs/BACKLOG.md`, maintained with `plan-memory/backlog.py`),
   so `validate` and `promote` keep working.
2. **Fresh start without losing history:** the items that predate the v2 redesign move to
   `Closed` as superseded, each naming the new item that absorbed it; new items continue the
   numbering (ids are never reused).
3. **Design link:** the first line of each description is
   `Design: docs/architecture/<doc>.md, <section or decision>.`
4. **Area prefix in the title:** `[ingestion]`, `[retrieval]`, `[storage]`, `[models]`,
   `[identity]`, `[governance]`, `[connectors]`, `[ui]`, `[cli]`, `[release]`, `[docs]`.
5. **Granularity:** one item is one deliverable that can become a plan on its own (a few
   days to two weeks); a large design becomes several items.
6. **Bugs** only for defects in the current code worth fixing before the redesign
   (security, data loss, violation of a principle), each confirmed in the code first; when
   the whole component is replaced, the defect becomes an acceptance criterion of the
   redesign item.
7. **Provisional priority:** every item enters as `medium`; bugs carry their real
   severity. Priorities are set in the prioritization step.
8. **Written in batches**, one per design document, each committed and reviewed before the
   next: current-code bugs; ingestion (with the audit); retrieval; deployment; identity and
   governance; connectors; desktop UI; CLI and release; platform and Admin Hub.
