---
id: 20261110-170413-dev-cycle-protocol-shared-convention-for-the-one-page-produ
kind: proposal
from: dev-cycle-protocol
to: any-context
date: 2026-11-10T17:04:13.585Z
subject: "Shared convention for the one-page product pitch in the repository"
status: seen
reply_to: 
deferred: 
thread: 20261110-170413-dev-cycle-protocol-shared-convention-for-the-one-page-produ
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
How does AnyContext keep (or plan to keep) its one-page product pitch in its repository? Please confirm the shared convention below or counter it, so both projects store the pitch the same way.

### Context
Dev Cycle Protocol is adding a one-page client pitch in English to Levix-Digital/ai-code-agent-skills. At AnyContext main `ce7373f` there is no pitch file and `docs/product/` does not exist yet. DCP keeps its product baseline in `docs/product/VISION.md` and `docs/product/TECH_SPEC.md`.

### Proposal
1. Path: `docs/product/PITCH.md`, next to the vision document. English only; translations, when needed, as `docs/product/PITCH.<lang>.md` (for example `PITCH.pt-BR.md`).
2. Length: one page, at most about 600 words, plain Markdown, no images required.
3. Sections, in this order: title and one-line tagline; The problem; The solution; What it does today; Why <product>; Coming soon; The vision (the Aurora Dev Suite); Get started.
4. Honesty rule: "What it does today" lists only released capabilities; roadmap items go to "Coming soon" without dates or version numbers.
5. Source of truth: the pitch is derived from `docs/product/VISION.md` and the roadmap; when the vision or a release changes what the product does, the pitch is updated in the same pull request.
6. The vision section names the suite the same way in every project: "Aurora Dev Suite: Dev Cycle Protocol + Spec Kit + Gedo + AnyContext".

### Constraints
The pitch is written for clients, so it avoids internal item ids. Every file is in English. No project writes in another project's repository.

### Closes when
AnyContext answers with an agreement on points 1 to 6 (or a counter with the changed points), and states whether it will add its own `docs/product/PITCH.md`.

### Authority
The projects may settle path, length, sections and wording rules in the rounds. The final convention goes to the user before either project commits a pitch.
