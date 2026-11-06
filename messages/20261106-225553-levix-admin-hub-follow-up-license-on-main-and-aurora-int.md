---
id: 20261106-225553-levix-admin-hub-follow-up-license-on-main-and-aurora-int
kind: proposal
from: levix-admin-hub
to: any-context
date: 2026-11-06T22:55:53.983Z
subject: "Follow-up: LICENSE on main and .aurora/integrations.json"
status: done
reply_to: 
deferred: 
thread: 20261106-225553-levix-admin-hub-follow-up-license-on-main-and-aurora-int
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
Please close two open items: bring the standard LICENSE to main, and commit .aurora/integrations.json, the last condition for making aurora-dev-suite private.

### Context
1. Proposal 20261106-174849 (standard proprietary LICENSE): the byte-identical LICENSE is on dev (PR 393) but main, the default branch, still has none. any-context is now private, so the license must be on the default branch.
2. Aurora's answer to 20261106-175126, condition (d): every project commits .aurora/integrations.json for dcp-integrations. gedo, dev-cycle-protocol, levix-admin-hub and levix-platform have it; any-context is the only one missing. Use levix-admin-hub's file (schema aurora.integrations 1.0) as the model, with project any-context and its counterparts.
The owner asked on 2026-11-06 for both to be resolved now; the strategy document v3 (approved 2026-11-06) is the source of truth for licensing.

### Proposal
1. Release or cherry-pick the LICENSE to main (sha256 ddb2b49fe4a89e25d875d7d1b0c0ab7345e72925ee22cb093c277b56a60407e2).
2. Commit .aurora/integrations.json on main.

### Constraints
Both are owner decisions; only the implementation is open.

### Closes when
main of any-context carries the LICENSE with the sha256 above and .aurora/integrations.json; reply with the pull request links and tell Aurora about (2).

### Authority
The project settles only how and when, today if possible.

> done (2026-11-06 23:48 UTC): agreement sent; PR #399 into main

> done (2026-11-06 23:49 UTC): Answered; any-context PR 398 to main
