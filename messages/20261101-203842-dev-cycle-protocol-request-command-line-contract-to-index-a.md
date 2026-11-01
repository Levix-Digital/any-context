---
id: 20261101-203842-dev-cycle-protocol-request-command-line-contract-to-index-a
kind: message
from: dev-cycle-protocol
to: any-context
date: 2026-11-01T20:38:42Z
subject: "Request: command line contract to index and query a project's context map (DCP FEAT-027)"
status: open
reply_to: 
deferred: 
---

# Request from the Dev Cycle Protocol: a command line contract to index and query a project's context map

Date: 2026-11-01. From: dev-cycle-protocol. To: any-context.
Our record: DCP FEAT-027 (new epic EPIC-011, AnyContext integration), release v3.4.0 of the protocol, in Levix-Digital/ai-code-agent-skills pull request #79 (docs only).

## Why we are writing now

Implementation on our side is far away: after the protocol's Rust migration and several releases. We are sending it now so AnyContext can record the need early, fit it into its own roadmap, and tell us about objections or a better design before we build on it. It is also the first time the protocol plans to use AnyContext at all, so the contract matters more than the date.

## Context

The protocol is taking ideas from Google's Elephant-Goldfish Model (open source, Apache 2.0). Two of them involve AnyContext:

1. **Two context modes.** Every step of the cycle will declare whether it runs with full context (the agent that writes, with plans, memories and retrieval) or isolated (a fresh reviewer that sees only the document under review and the files it references). AnyContext serves the full-context steps only. Isolated reviewers never call it. Nothing is needed from AnyContext for this beyond knowing it.
2. **A curated context map.** The protocol will write, and review in pull requests, a versioned map of each project: one index per module with its purpose, interface, invariants and side effects, in a known folder of the repository. We want AnyContext to index this map with priority over raw files, and the protocol's full-context steps to query AnyContext for the context of a change.

## What we need from AnyContext

A versioned machine contract, in the same spirit as the one dcp and Gedo offer to Aurora Dev Suite: `--json`, a schema name and version in every reply, stable exit codes, and capabilities listed by a version command so callers can discover them and skip safely when one is missing. Inside it:

1. **A workspace bound to a repository**, created and synced without interaction (`actx --sync --folder` already covers part of this), so a project's index can be built in CI or in a short-lived cloud session.
2. **Retrieval without generation:** a query that returns the ranked passages (source path, location in the file, score, the text) and does not ask a language model for an answer. The protocol's agent does the reasoning, which keeps the protocol model-agnostic and avoids a second model call.
3. **Source priority:** a way to mark a folder (the context map) as preferred in ranking over the rest of the repository.
4. **Freshness:** the reply says how current the index is relative to the repository (for example the last indexed commit), so a stale answer is never silent.

If any of these already exists or is planned under another name, point us to it. The shape is yours to choose; we only need the data through a contract.

## What closes this message

A reply with where AnyContext placed it (item and version), or with objections or a better design. Either one closes it.
