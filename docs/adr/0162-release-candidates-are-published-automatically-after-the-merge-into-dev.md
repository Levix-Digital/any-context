---
id: 162
title: Release candidates are published automatically after the merge into dev
status: accepted
date: '2026-11-14'
version: null
subsystems: [ci-release]
supersedes: []
superseded_by: []
amends: [134]
amended_by: []
---

# ADR-0162: Release candidates are published automatically after the merge into dev

## Context

ADR-0134 publishes a release candidate only when someone pushes a `vX.Y.Z-rc.N` tag from `dev` by hand. The owner decided on 2026-11-13 that release candidates are published automatically once the human gate (the review and merge of the pull request) has passed, and asked that every Levix project adopt it (cross-project thread 20261113-215440 with DocMapper, whose implementation follows Aurora's model). In AnyContext the merge into `main` is already the final release gate (`tag-on-merge.yml`), so the candidate belongs to the merge into `dev`, where candidates were always cut.

## Decision

- `.github/workflows/candidate.yml` runs on `workflow_run` of the test suite (`Rust Workspace Test Suite`), completed with success, for pushes to `dev`, and manually. It tags the head of `dev` `vX.Y.Z-rc.N`, with X.Y.Z from `version.txt` and N the next candidate number of that version, and calls `release.yml` for that tag in the same run.
- It cuts no candidate, and says why in the run summary, when `vX.Y.Z` is already released (the version must be bumped first), when the commit already has a candidate of that version, when `dev` has moved on since the tested commit, or when `docs/releases/vX.Y.Z.md` is missing. The notes file is the gate that the version is ready for testers, so merges of work in progress publish nothing.
- Candidates stay pre-releases, never latest, on the releases repository (ADR-0134). The final release keeps its human gate: the owner's merge into `main`.
- A candidate tag pushed by hand still works.

## Alternatives Considered

- **Candidate on `main`, final through a promote workflow (DocMapper's and Aurora's model)**: AnyContext's merge into `main` is already the owner's release gate; adding a promote step would change the final release flow decided in ADR-0134 without need.
- **A candidate on every merge into `dev`**: would publish work in progress and email testers for builds nobody asked to test.

## Consequences

- After the version bump and its notes file are merged into `dev`, each later merged pull request that passes the tests publishes the next candidate of that version without a tag pushed by hand.
- GitHub runs `workflow_run` triggers only from the default branch (`main`), so the automatic candidate starts working once `candidate.yml` reaches `main` with the next release; until then candidates are tagged by hand or with a manual run.
- Documentation-only merges do not run the test suite and cut no candidate; run the workflow manually when a candidate is needed then.
- Verify after a merge into `dev` with the notes file present: the run summary names the tag, and the release is a pre-release on the releases repository.
