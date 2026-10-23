# Levix platform shared components

Cross-cutting document: components built once and reused by the Levix products
(AnyContext, DocMapper and future ones).

## Adoption principle

- **Draw the boundaries now:** what is common core, what belongs to each product, and the
  contracts between them.
- **Build within AnyContext's needs**, already as separate crates and service, with no
  dependency on AnyContext.
- **The other products adopt them when their turn comes in the roadmap.** No generic
  platform built ahead of time.

## Components

### 1. Admin Hub (separate product)

Control plane for **several products**: a company that has AnyContext and DocMapper
installs a single Admin Hub (always hosted by the customer, zero-knowledge — see
`identity-and-governance-v2.md`).

- **Common core:** organizations; users (own directory, SSO, SCIM) and groups; devices and
  enrollment; **license and seats per product**; policy distribution and signing; audit;
  ITSM; break-glass; update mirror.
- **Per product:** each product registers its own **policy schema** (AnyContext's: models,
  trust levels, web search, MCP…); the core distributes, signs and audits without knowing
  the details.
- **Contract:** documented API and WebSocket — any product talks to the Admin Hub through
  the protocol, regardless of language.

### 2. Client-side identity

Accounts, `root`/`sudo`, session, key hierarchy, vault, biometrics, enrollment and the
governance engine (enforcement of signed policies) — practically all of
`identity-and-governance-v2.md`, except workspaces, which belong to AnyContext.

**Accounts are never shared across products.** Each product has its own accounts on the
machine; users of different products **never mix**. What is common is the
**administration**: the Admin Hub manages the accounts of every product with the same user
management tools.

### 3. Others

- **Licensing** (DocMapper already has a license module).
- **Verified catalog** of models and updates.
- **LSIE** (form extraction), already decided as a single crate
  (`ingestion-pipeline-v2.md`, step 6).

## Languages

- **Admin Hub:** Rust service (decided in `identity-and-governance-v2.md`, topic 5).
- **Client modules** (identity, policies, LSIE…): **Rust crates, also compiled to WASM**.
  In the browser, storage and cryptography change (WebCrypto, IndexedDB instead of the OS
  vault) through **adapters**; a Tauri desktop product uses the crate directly.

## Repositories

- `levix-admin-hub` — separate product (private).
- `levix-platform` — identity, policy, cryptography, licensing and LSIE crates (private).
- Products depend on these crates by version (git tag).
- Each repository holds its README, its part of the design and its own backlog from the
  start; code is born there when its backlog item is built, never moved there ahead of
  time.

Effect on the backlog: items for these components belong to `levix-admin-hub` and
`levix-platform`, not to AnyContext.
