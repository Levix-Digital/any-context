---
id: 152
title: Web search uses only allowed search APIs and never scrapes
status: accepted
date: '2026-10-25'
version: v1.0.0
subsystems: [retrieval, agent]
supersedes: []
superseded_by: []
amends: [106]
amended_by: []
---

# ADR-0152: Web search uses only allowed search APIs and never scrapes

## Context

Web search at question time (the `web_search` agent tool) tried Tavily, then Serper, then DuckDuckGo when no key was set. The DuckDuckGo fallback called the Instant Answer API, which is not a search API, and then scraped `html.duckduckgo.com` with a browser user agent (BUG-029). `docs/architecture/connectors-v2.md` (decisions 24 and 32) allows only official APIs, or third-party APIs whose origin's terms allow the use. DuckDuckGo has no official search API. Serper sells Google result pages that it collects, and Google's terms forbid automated queries, so Serper fails the same test. Tavily runs its own index and API under its own terms.

## Decision

- `NativeWebSearchEngine::search` uses only the Tavily API, with the user's key (`TAVILY_API_KEY` or `/keys tavily <key>`). It searches the workspace's web source domains first, then the open web.
- Without a key it returns `NO_SEARCH_PROVIDER` ("Web search is off: no search API key. Save a Tavily key with /keys tavily <key>."), before any request. The `web_search` tool passes that text to the model, which answers from local documents.
- The DuckDuckGo code (Instant Answer, HTML scraping, its parser and URL helpers) and Serper are removed. A Serper key is ignored.
- The HTTP client identifies itself as `AnyContext/<version>`, not as a browser.
- `/web-search on` keeps the setting on and adds the same notice when no key is set, so search works as soon as a key is saved.
- Tests replace key lookup and the HTTP client through `with_key_resolver` and `with_client`.
- This amends the multi-provider web search of ADR-0106. The Levix-provided search API (FEAT-161), the fair-use ceiling (FEAT-170) and query pseudonymization (FEAT-118) come later.

## Alternatives Considered

- **Keep Serper as a second provider**: rejected by Levi on 2026-10-25, because it resells Google result pages against Google's terms.
- **Keep the DuckDuckGo Instant Answer API alone**: rejected. It returns encyclopedia summaries, not web results, so the model would get a weak answer presented as search.
- **Choose the Levix-provided provider now**: left to FEAT-161, which weighs terms and cost (Jina is the reference at about US$ 0.0005 per search).

## Consequences

- Users without a Tavily key lose web search until they save one, and the notice says how.
- No query goes to a service that the user did not configure.
- Regression test: `search_without_a_key_makes_no_request_and_says_so` in `crates/any-context-core-rs/src/retrieval/web_search.rs`.
