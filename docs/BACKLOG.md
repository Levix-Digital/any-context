# Backlog

Work captured for later planning: bugs, features, improvements and adjustments. Capturing an item takes no discovery; when the user decides to work on items, they are promoted into a plan (`plan-memory/backlog.py promote`), where the discovery and the planning happen. Open bugs come before new work.

Each item: `### <ID>: <title>` with the fields `Severity` (bugs: critical, high, medium, low) or `Priority` (others: high, medium, low), `Status` (open, planned, in_progress), `Found`, `Plan` and `Description`. Closed items (done, dropped, duplicate) move to `Closed` with a `Closed` note and keep their ids, which are never reused. Edit this file with `plan-memory/backlog.py` or by hand, and check it with `backlog.py validate`.

## Bugs

### BUG-001: Data indexed before field encryption stays in plaintext
- **Severity**: high
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: Chunks written to LanceDB before field-level encryption on every write (ADR-0131) remain unencrypted on disk until the workspace is re-indexed with actx --sync --force. Nothing tells the user, while the README states that indexed text is encrypted.

### BUG-002: Frontier preset selects an outdated model id
- **Severity**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: /preset frontier sets the model claude-3-7-sonnet, which may no longer be a valid model id for the Anthropic API; the preset could then fail on the first question.

## Features

## Improvements

### IMP-001: Review the Deep Search minimum score
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: Deep Search keeps results with a minimum score of 0.02, which looks low and may let irrelevant chunks into the combined answer.

### IMP-002: Remove the core dependency on actx_installer
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The core crate (any-context-core-rs) calls actx_installer directly from commands/engine.rs (install paths, latest release lookup, update logging), so actx_installer cannot be listed in architecture.forbidden_core_imports. In the hexagonal style the core should reach this through a port implemented by an adapter.

## Adjustments

### ADJ-001: Update the outdated .clinerules
- **Priority**: low
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: .clinerules no longer matches the current project (Rust workspace, architecture-docs layout, role-based development cycle) and can mislead agents that read it.

### ADJ-002: Add a license file
- **Priority**: medium
- **Status**: open
- **Found**: 2026-10-16, v0.45.0, PR #333
- **Plan**: -
- **Description**: The repository has no LICENSE file; the README tells readers to contact Levix Digital for usage and distribution terms. The license (or the proprietary terms) still has to be chosen and published.

## Closed
