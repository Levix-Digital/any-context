# Roadmap

The releases of the product in delivery order, from the MVP onwards. Each release has a goal and the backlog items (`docs/BACKLOG.md`) it delivers; the roadmap is the only place that says which item goes into which release. Discovery starts from the open items of the current release (the first one not released), after the bugs that come first.

Each release: `## vX.Y.Z: <name>` with the fields `Status` (planned, in_progress, released), an optional `Target`, `Released` (the date, once released), `Goal` and `Items` (backlog ids separated by commas, `-` when empty). Edit this file with `plan-memory/roadmap.py` or by hand, and check it with `roadmap.py validate`.

The rationale of these releases (principles, the goal of each release, the items grouped by area, what is needed from `levix-platform` and the versioning) is in the roadmap v2 decided on 2026-10-23: [`docs/plans/anycontext_roadmap_v2.md`](plans/anycontext_roadmap_v2.md). Its releases map to versions here: R1 is v1.0.0, R2 (v1.x) is v1.1.0, R3 is v2.0.0, R4 (v2.x) is v2.1.0 and Later is v3.0.0, a placeholder for unscheduled items.

## v1.0.0: Personal
- **Status**: in_progress
- **Goal**: Reach the market with Free and Pro on a correct base: an MVP in features on a complete foundation
- **Items**: ADJ-006, ADJ-007, ADJ-008, BUG-022, BUG-024, BUG-025, BUG-026, BUG-027, BUG-028, BUG-029, FEAT-020, FEAT-021, FEAT-022, FEAT-023, FEAT-031, FEAT-036, FEAT-047, FEAT-050, FEAT-057, FEAT-067, FEAT-069, FEAT-081, FEAT-082, FEAT-085, FEAT-088, FEAT-098, FEAT-101, FEAT-149, FEAT-160, FEAT-161, FEAT-168, FEAT-169, FEAT-170, FEAT-172, FEAT-173, FEAT-174, FEAT-175, FEAT-176, FEAT-177, FEAT-178, FEAT-179, FEAT-184, FEAT-185, FEAT-186, FEAT-188, FEAT-189, FEAT-190, FEAT-192, FEAT-193, FEAT-195, IMP-015, IMP-016, IMP-017, IMP-019, IMP-020, IMP-021, IMP-023, IMP-025, IMP-026, IMP-027, IMP-028, IMP-029, IMP-030, IMP-031, IMP-032, IMP-033, IMP-034, IMP-035, IMP-036, IMP-037, IMP-038, IMP-039, IMP-040, IMP-041, IMP-042, IMP-043, IMP-044, IMP-045, IMP-046, IMP-047, IMP-048, IMP-049, IMP-050, IMP-051, IMP-052, IMP-053, IMP-054, IMP-055, IMP-057, IMP-058, IMP-059, IMP-060, IMP-061, IMP-062, IMP-063, IMP-064, IMP-065

## v1.1.0: Personal evolution
- **Status**: planned
- **Target**: v1.x, internal order decided when R1 is close
- **Goal**: Format coverage and extraction quality for the professional audiences
- **Items**: FEAT-024, FEAT-025, FEAT-026, FEAT-027, FEAT-028, FEAT-029, FEAT-030, FEAT-032, FEAT-033, FEAT-035, FEAT-037, FEAT-038, FEAT-039, FEAT-040, FEAT-041, FEAT-043, FEAT-044, FEAT-045, FEAT-046, FEAT-048, FEAT-049, FEAT-051, FEAT-052, FEAT-053, FEAT-054, FEAT-055, FEAT-056, FEAT-058, FEAT-059, FEAT-060, FEAT-061, FEAT-063, FEAT-072, FEAT-073, FEAT-074, FEAT-075, FEAT-076, FEAT-077, FEAT-078, FEAT-079, FEAT-083, FEAT-084, FEAT-086, FEAT-087, FEAT-089, FEAT-090, FEAT-091, FEAT-092, FEAT-093, FEAT-094, FEAT-099, FEAT-100, FEAT-102, FEAT-103, FEAT-104, FEAT-105, FEAT-106, FEAT-107, FEAT-108, FEAT-109, FEAT-112, FEAT-115, FEAT-116, FEAT-125, FEAT-131, FEAT-133, FEAT-135, FEAT-146, FEAT-147, FEAT-148, FEAT-180, FEAT-181, FEAT-182, FEAT-183, FEAT-187, IMP-018, IMP-022, IMP-024

## v2.0.0: Enterprise foundations
- **Status**: planned
- **Goal**: Corporate environment, Admin Hub and Enterprise connectors; deployment modes 1 and 2
- **Items**: FEAT-111, FEAT-113, FEAT-114, FEAT-117, FEAT-118, FEAT-119, FEAT-134, FEAT-136, FEAT-142, FEAT-143, FEAT-150, FEAT-151, FEAT-152, FEAT-153, FEAT-154, FEAT-155, FEAT-156, FEAT-157, FEAT-158, FEAT-159, FEAT-162, FEAT-163, FEAT-164, FEAT-165, FEAT-166, FEAT-171, IMP-056

## v2.1.0: Enterprise shared index
- **Status**: planned
- **Target**: v2.x
- **Goal**: AnyContext Server (mode 3) and per-source access control
- **Items**: FEAT-080, FEAT-110

## v3.0.0: Later
- **Status**: planned
- **Target**: unscheduled
- **Goal**: High-cost items, or items enabled only by a gain measured by the evaluation
- **Items**: FEAT-034, FEAT-042, FEAT-062, FEAT-064, FEAT-065, FEAT-066, FEAT-068, FEAT-070, FEAT-071, FEAT-095, FEAT-096, FEAT-097, FEAT-194
