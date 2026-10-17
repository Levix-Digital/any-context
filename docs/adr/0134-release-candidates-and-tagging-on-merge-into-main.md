---
id: 134
title: Release candidates and tagging on merge into main
status: accepted
date: '2026-10-16'
version: null
subsystems: [ci-release]
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0134: Release candidates and tagging on merge into main

## Context

Tests ran only on pull requests into `main` and pushes to `dev`, so changes reached `dev` without a test run on their pull request. Releases were published only when someone pushed a `v*` tag by hand, and every published release became the latest one offered to users: there was no way to hand a build to the user for acceptance testing without also releasing it. A tag pushed by a workflow with `GITHUB_TOKEN` does not trigger other workflows, so tagging from CI alone cannot start the release workflow.

## Decision

- `e2e-tests.yml` runs `cargo test --workspace` on pull requests into `dev` and into `main`, on pushes to `dev` and manually; concurrency is grouped per pull request number (or commit), cancelling superseded runs.
- A tag `vX.Y.Z-rc.N` pushed from `dev` is a release candidate: `release.yml` publishes it with `prerelease: true` and `make_latest: false`, so `/releases/latest` and `actx --check-update` never offer it. The user installs it with `actx-installer --version=<tag>` and runs `tests/MANUAL_TESTS.md` against it.
- The user's merge into `main` is the release gate. `tag-on-merge.yml` runs on pushes to `main`, reads `version.txt`, creates the annotated tag `vX.Y.Z` when it does not exist and, in the same run, calls `release.yml` as a reusable workflow (`workflow_call` with input `tag`, `secrets: inherit`) to build and publish the final release.
- `release.yml` takes the tag from `inputs.tag` or the pushed ref (`RELEASE_TAG`), checks out that ref, and publishes only for tags or calls (`PUBLISH`).
- The version is bumped in `dev` (`version.txt` and every crate `Cargo.toml`, listed in `.dev-cycle.json` `devops.version_files`) before the candidate is tagged.

## Alternatives Considered

- **A `qa` or `stage` branch between `dev` and `main`**: one more branch to keep in sync and a second promotion step, while a pre-release already gives the user an installable build of an exact commit.
- **Tag pushed with a personal access token from the merge workflow**: works, but needs another long-lived secret only to trigger a second workflow; calling the release as a reusable workflow needs none.
- **Keep manual tagging**: the release could be forgotten after a merge or published from a commit that never reached `main`.

## Consequences

- Every pull request into `dev` is tested before merge; the final release always matches what was merged into `main`.
- Merging a pull request into `main` without bumping `version.txt` publishes nothing (the tag exists already); the version must be bumped in `dev`.
- Release candidates accumulate as pre-releases on the releases repository; they can be deleted after the final release.
- Verify with `python ~/.ai-code-agent-skills/devops-release/check_ci_policy.py` (0 errors) and, after a merge, that the final tag exists and its release is marked latest.
