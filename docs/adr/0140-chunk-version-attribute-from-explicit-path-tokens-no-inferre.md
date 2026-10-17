---
id: 140
title: Chunk version attribute from explicit path tokens, no inferred scope
status: accepted
date: '2026-10-17'
version: v0.45.1
subsystems: [retrieval]
supersedes: []
superseded_by: []
amends: [128]
amended_by: []
---

# ADR-0140: Chunk version attribute from explicit path tokens, no inferred scope

## Context

[ADR-0128](0128-backend-first-deterministic-dual-rag.md) labels every `<chunk>` with `scope` and `version` attributes derived from the path, and the deterministic prompt rule 2 forbade mixing values from chunks whose `scope` or `version` differ. `infer_scope_and_version` tested substrings of the absolute path (the URL, host included, for web pages) against fixed keyword lists, so words that merely contain a keyword produced labels: `latest` became scope `test`, `author` became `auth`, `environment` and `envelope` became `config`, `rapid` became `api`, `feedback` became `database`, a `specs` folder became `test`; `dev2` became version `v2`, and `v10` or `v3.1-rev1` became `v1`. Without a match the version was `current`, also a guess. Two files of one folder (`api-gateway/docs/timeouts.md` and `billing.md`) got different scopes, so rule 2 told the model not to combine them (BUG-008). The `source` and `file` attributes already identify each document. [ADR-0130](0130-real-onnx-runtime-multilingual-cross-encoder.md) showed that the cross-encoder does not separate versions, so a reliable version signal still has a purpose.

## Decision

In `crates/any-context-core-rs/src/retrieval/prompt.rs`:

- The `scope` attribute and `infer_scope_and_version` are removed.
- `infer_version(file_path) -> Option<String>` splits the path into whole tokens on `/`, `\`, `_`, `-` and whitespace, after dropping an alphabetic extension from the last segment. The last token of the form `v<n>[.<n>...]` (case-insensitive) is the version; otherwise a `legacy` or `deprecated` token gives `legacy`; otherwise there is no version. For a URL only its path counts, not the scheme, host, query or fragment.
- `format_xml_chunks` writes `version="..."` only when `infer_version` returns a value.
- Rule 2 of the deterministic prompt becomes a conflict rule: excerpts from different sources or versions may describe different things; combine their values only when the text shows they apply to the same thing, and when they conflict present each value with its source. Rule 1 describes `source` and the optional `version`; rule 5 no longer mentions scope. The `search_db` query anchor asks for the same per-source treatment of conflicts.

## Alternatives Considered

- **Remove both attributes**: not chosen. It loses a clear version signal from `v1/` and `v2/` folders or `_v2` file names.
- **Keep both, matched on whole tokens, with the hard rule**: not chosen. It fixes `latest` or `dev2`, but still labels documents by a fixed domain list and still separates files of one folder such as `billing.md` and `timeouts.md`.
- **Tokens relative to the source root**: not chosen. The formatter receives only the chunk; a version folder above the root labels every file of that root alike and never separates them.
- **No change**: rejected; wrong labels block correct answers.

## Consequences

- Answers can combine excerpts from different files on one subject; differing explicit versions or conflicting values are reported per source instead of being forbidden or merged.
- Versions are only recognized from `v<n>` tokens and `legacy`/`deprecated`; versions stated only in document content, dates or `beta`/`rc` markers are not labeled.
- Prompts are slightly shorter: most chunks carry no `version`.
- Deep Search synthesis does not use `ContextFormatter` yet (BUG-009); once it does, it inherits this behavior.
- Verified by the formatter tests in `prompt.rs` and by manual test MT-ANS-006.
