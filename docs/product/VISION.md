# AnyContext

AnyContext answers questions from a person's or a company's own documents, folders and documentation sites, with citations, without the documents leaving the machine or the company's perimeter. It is the document line of Levix Digital: "ask your documents without them leaving home".

## Positioning

How AnyContext is described to customers. Use these words, in this order of preference.

- **One sentence**: AnyContext is an assistant that answers questions from your own documents, citing the source of every answer, without the documents ever leaving your computer or your company.
- **Tagline**: Ask your documents without them leaving home.
- **Is it an agent?** No. It uses an agent inside, the Deep mode, which breaks hard questions down and searches in several rounds. But the product is the assistant over your documents, not a general agent that carries out tasks.
- **Is it a platform?** It is not a platform for others to build on. It is a finished product you install. For companies, it plugs into the Levix Admin Hub for licenses, policies and auditing.
- **Technical audience**: a local-first context engine: hybrid search with local reranking, checked citations and an MCP server so the company's agents can query internal documentation without sending it outside.

The one-sentence pitch leaves out the word "AI" on purpose: regulated buyers often work under a "no AI" policy, and what answers their objection is that the documents never leave the perimeter.

## Problem

Analysts, lawyers, doctors, engineers and researchers spend a large part of the day looking for things in internal documents: contracts, records, protocols, specifications, code and its documentation. General chat assistants and cloud notebook tools can answer from documents, but only after the documents are uploaded to a third party, which the security policy of regulated organizations does not allow and which many professionals do not want for their own material. What they do today is search by file name or full text, open each file and read, or copy passages into a chat tool against the policy. Enterprise search products that stay on-premise are expensive, heavy to deploy and do not answer questions; they return lists of files.

## Target Users

- **Individual professional (primary for v1.x)**: an analyst, consultant, lawyer, researcher or developer who works with many local documents and wants grounded answers with citations, using a local model or a cloud model they choose. Gets in the way: privacy, setup cost, answers that invent content.
- **Solo developer or entrepreneur**: one person running their own business, who uses AnyContext as the memory of their projects and as context for their coding agents.
- **Regulated organization (primary from v2.0.0)**: law and accounting firms, clinics and regional health networks, mid-size financial institutions and insurers, municipal and provincial public sector, defense and its suppliers. The buyer is the CISO, CIO or compliance officer; the users are the professionals above. Gets in the way: the policy says "no AI", because every option sends content outside the perimeter.

## Value Proposition

- Answers grounded in the user's own documents, with checked citations to the file and line range, and a plain "not found" when the answer is not there.
- Nothing leaves the machine except the language model call, and with a local model nothing leaves at all. No telemetry.
- Three deployment modes sold by name, because it is the first question the CISO asks: local (one machine), VPC (company index and Model Server) and isolated (fully offline, updates through the Admin Hub mirror).
- One governance place for the company (the Levix Admin Hub): licenses and policies signed and verified offline, auditing of metadata, never of content.
- An MCP server so the company's coding and automation agents can query internal documentation through AnyContext.

## Business Model

Proprietary software (LICENSE), with the source code available to customers with an active agreement for audit only, under confidentiality, and optional escrow. Licenses are annual, signed by Levix and verified offline; never priced by usage, token or document.

- **Free**: complete for one person, for personal use or a business run by that person alone; no registration, no Admin Hub. Unlimited workspaces, local models, deep research, the Web connector (with a fair-use ceiling on web search) and the Audio and video connector. Nothing a person working alone needs is behind payment (connectors-v2, decision 34).
- **Enterprise (organization)**: annual license per seat with a minimum, Admin Hub required: shared workspaces with curators, policies, SSO, auditing, shared index in the VPC, Model Server, corporate connectors, domain packs and the MCP server.
- **Isolated**: annual license per installation plus dedicated support, with the Admin Hub update mirror.
- **Services and pilots**: paid pilots of 30 to 60 days with a measurable goal, deployment, training and domain packs on demand.

The Admin Hub is the billing vehicle: one contract, one invoice per account, and each additional Levix product is one more license on the same Hub. Prices are set after the first pilots.

## Success Metrics

- Questions answered per active user per week (target set after the first month of real usage).
- Share of answers whose citation the user accepts (target set after the first pilots).
- Time from installation to the first useful answer: minutes, not hours (strategy v2).
- Retrieval quality on the project benchmark (IMP-065): no release below the previous one.
- Enterprise: one paid pilot signed in the first phase, then two named customers and a public penetration test report.

## Scope

- **Ingestion**: folders, internal-network shares and documentation sites; incremental, per-chunk diff; many formats (code, Markdown, PDF, Office, data files, archives, e-mail), OCR and form extraction.
- **Retrieval and answers**: hybrid search with re-ranking, relevance bands, token-budget selection, the nothing-found gate, Fast and Deep research modes, checked citations and the source report.
- **Workspaces and memory**: separate projects with their own sources, models, settings and conversation history.
- **Models**: local models by role and hardware profile, cloud providers chosen by the user, the Model Server for companies.
- **Connectors**: Web and Audio and video (Free); corporate connectors (drives, SharePoint, IMAP, and others) as Enterprise features.
- **Surfaces**: the desktop app (Tauri), the CLI and terminal UI, and the MCP server.
- **Identity, security and plans**: personal account and vault, key hierarchy, encrypted index, plan license verified offline, signed downloads.
- **Enterprise governance**: Admin Hub integration, policies, curators, SSO, auditing, the shared index (AnyContext Server) and per-source access control.
- **Domain packs**: vocabularies for legal, medical and tax work.

## Non-Goals

- No Levix-hosted SaaS: AnyContext always runs on the user's machine or in the customer's perimeter.
- No selling where the cloud is allowed and preferred: AnyContext does not compete with cloud notebook tools on their ground.
- No telemetry and no phone-home license checks.
- No usage-based, token-based or per-document pricing.
- No scraping or unofficial APIs: every connector uses the origin's official API (connectors-v2, decision 32).
- No Notion connector; consumer drive connectors are not part of the personal plans.

## Risks and Assumptions

- **Commodity in the cloud**: every IDE and model vendor embeds document search. Mitigation: sell only where the cloud is not allowed, and invest in what the cloud does not offer: Admin Hub governance and domain packs.
- **Answer quality on real documents**: a wrong or uncited answer loses the user. Mitigation: the retrieval benchmark, the reranker as relevance judge, the nothing-found gate and checked citations.
- **Hardware variety**: local models must run on ordinary laptops. Mitigation: hardware profiles and kits, the Lite profile, the Model Server for companies.
- **Isolated mode leaks**: web sources and web search must be off by policy in isolated mode, visibly in the interface.
- **One-person team**: the protocol, the backlog and the technical documentation let another person take over; the first hire is security.

## Open Questions

- Prices, trial length and the web search fair-use ceiling (connectors-v2, "For the roadmap").
- Target numbers for weekly questions per user and for accepted citations, set from the first month of usage data and the first pilots.
