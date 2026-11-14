# Release notes

One file per version, `vX.Y.Z.md`, written in `dev` when the version is ready for testers.

The file is the gate for automatic release candidates (ADR-0162): after a pull request is merged into `dev` and the test suite passes, `.github/workflows/candidate.yml` tags the head of `dev` as `vX.Y.Z-rc.N` and publishes it as a pre-release only when `docs/releases/vX.Y.Z.md` exists for the version in `version.txt`. Until the file is added, merges of work in progress publish nothing and notify no tester.

Content: what changed for the user in this version, in plain English, and anything testers should check first. Keep it short; `tests/MANUAL_TESTS.md` holds the acceptance tests.
