---
name: workflow
description: How work flows in this repository and the tool pitfalls already hit. Read before branching, committing, opening a pull request or releasing.
updated: 2026-11-14
---

- 2026-10-24: A fresh clone starts on `main`, which is far behind `dev`; check out `origin/dev` before running the `/dev-cycle` diagnosis, or superseded plans look active.
- 2026-10-25: Spec Kit is used for feature plans only; fix plans never use it.
- 2026-10-25: Retrieval benchmark baselines are refreshed from the `RESULT-JSON` line of the CI benchmark log, not from a local run (local ONNX Runtime versions shift the scores slightly).
- 2026-11-07: Only one session handles the cross-project inbox at a time; two sessions answering the same message produced duplicate pull requests.
- 2026-11-14: Sending a message to a project needs that project's repository reachable with push rights from the session; add it to the session first.
