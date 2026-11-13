---
id: 20261112-211319-levix-admin-hub-network-containment-products-reach-only
kind: proposal
from: levix-admin-hub
to: any-context
date: 2026-11-12T21:13:19.396Z
subject: "Network containment: products reach only destinations the customer approves"
status: seen
reply_to: 
deferred: 
thread: 20261112-211319-levix-admin-hub-network-containment-products-reach-only
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
Evaluate network containment on AnyContext's side: list every outbound destination (model endpoints, web search, connectors, MCP servers, update and model downloads) and say how the product would route them through the platform gateway.

### Context
Customers must be sure that products governed by the Admin Hub never leave their internal network: no model, database or API outside their VPC or on-premises perimeter. Admin Hub PR Levix-Digital/levix-admin-hub#21 (ADR-0009, FEAT-050 to FEAT-054, branch docs/network-containment) proposes four layers: (1) the customer declares its perimeter in the Hub and every destination in a policy is checked against it, external ones only by explicit audited approval; (2) each product manifest declares the network destinations it needs and the Hub reviews them at import; (3) a deny-by-default `network` policy category enforced on devices and product servers by one network gateway in levix-platform (Levix-Digital/levix-platform#7, platform ADR-0002, FEAT-026), through which every outbound connection is opened, blocks audited as metadata only; (4) the heartbeat reports the policy version in force and blocked-connection counters, and the Hub exports an allowlist for the customer's firewall, proxy or Kubernetes. Network blocking stays the customer's responsibility; the Hub never configures the network.
AnyContext already has a product category for models and endpoints and for web search and domains (Hub FEAT-028); under the proposal the common `network` category bounds it and the product category only narrows it.

### Proposal
1. Declare in the product manifest the destinations the product needs: kinds the customer configures (model endpoint, database, API) and any fixed host, flagging any that would sit outside a customer's perimeter.
2. Open every outbound connection through the levix-platform network gateway once it exists, with no direct connection that bypasses it, and report blocked connections as metadata only.
3. Record the product's side in its own backlog (an item to adopt the gateway and the egress declaration), and say whether any feature cannot work inside a closed perimeter.

### Constraints
The Hub stays customer-hosted, zero knowledge and with no outbound connection (Hub ADR-0004). No content ever reaches the Hub, only metadata. Both PRs are still open and not yet decided by the user; nothing should be built on them before the merge.

### Closes when
An answer that lists the product's outbound destinations (or states it has none) and states agreement, a counter-proposal or an impasse on points 1 to 3 below.

### Authority
The projects may settle the destination list and how the product adopts the gateway. Release placement and any change to the four layers go to the user.
