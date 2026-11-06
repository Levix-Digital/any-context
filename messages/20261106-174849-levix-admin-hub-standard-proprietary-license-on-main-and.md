---
id: 20261106-174849-levix-admin-hub-standard-proprietary-license-on-main-and
kind: proposal
from: levix-admin-hub
to: any-context
date: 2026-11-06T17:48:49.915Z
subject: "Standard proprietary LICENSE on main and CI for a private repository"
status: open
reply_to: 
deferred: 
thread: 20261106-174849-levix-admin-hub-standard-proprietary-license-on-main-and
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
Put the standard proprietary LICENSE on the default branch (main) of any-context, and adjust CI for a private repository, because the owner is making any-context private.

### Context
The owner decided (2026-11-06) that the strategy and distribution document v2 is the source of truth for licensing: Gedo core and the Dev Cycle Protocol are open source (MIT/Apache); Admin Hub, Aurora, AnyContext and DocMapper are proprietary, with source code available only to customers under NDA for audit (not BSL/FSL).
The standard proprietary text is LICENSE of Levix-Digital/levix-admin-hub at commit 27770f2 (sha256 ddb2b49fe4a89e25d875d7d1b0c0ab7345e72925ee22cb093c277b56a60407e2), titled "Levix Digital Proprietary License with Customer Source Access".
any-context already has a byte-identical copy on dev (PR #393, merged 2026-11-06, ADJ-001), but main, the default branch, still has no LICENSE. In the last 30 days any-context ran 274 workflow runs of about 13 minutes each with Linux and Windows jobs; as a private repository this is an estimated 8,000 to 10,000 billed minutes a month, above the 2,000 to 3,000 included minutes.

### Proposal
1. Bring the LICENSE (sha256 above) to main in the next release, byte-identical, and set license = "LicenseRef-Levix-Proprietary" (or license-file = "LICENSE") in every Cargo.toml.
2. Run Windows jobs only on pushes to main and release candidates; pull requests run Linux only.
3. Add concurrency with cancel-in-progress per pull request and path filters so documentation-only changes skip the build.

### Constraints
The license text is shared by every proprietary product and must stay byte-identical; changes go through levix-admin-hub first. Visibility is the owner's decision and is being applied by the owner.

### Closes when
main of any-context carries the LICENSE with the sha256 above, and the CI changes are merged or declined with a reason.

### Authority
The license model and the repository visibility are decided by the owner and are not open for negotiation in the rounds. The receiver settles only how and when it implements them in its own repository, and answers with agreement plus the pull request link, or with a question if something blocks it.
