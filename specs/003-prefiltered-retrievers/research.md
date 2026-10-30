# Research: Pre-filtered dense and lexical retrievers

## R1 LanceDB applies `only_if` before the vector limit

LanceDB 0.39 pre-filters by default: `vector_search(..).only_if(expr).limit(n)` returns the `n` nearest rows among those that pass `expr` (post-filtering needs an explicit `postfilter()`). The workspace, embedding target and `Global` clauses are already passed this way, so the new filter joins the same expression. To confirm against a real table in T004.

## R2 Escaping filter values

Filter values reach a DataFusion SQL expression. String literals double `'`. A LIKE prefix would need its `%` and `_` escaped, but LanceDB 0.39 refuses any LIKE escape character other than the backslash ("LIKE does not support escape_char other than the backslash"), and the backslash is a Windows path separator. Folder scopes therefore use `file_path = 'x' OR starts_with(file_path, 'x/')`, checked against a real table (T004): the prefix is literal, and `docs` never matches `docs-old`.

## R3 Dates

`last_modified` is stored as UTC RFC 3339 (`DateTime::to_rfc3339` of the file mtime), so lexicographic comparison with a UTC RFC 3339 bound is time order. Rows without a date (some web pages) never match a date filter.

## R4 Candidates per list

Fast already takes 30 per list; Deep takes 40. Both move to 30. Benchmark results with 30 and 50 are recorded below (T009).

## Results

Measured on 2026-10-30 in one development container (4 cores), `actx-bench retrieval run`, release build, reranker on. `dev` is the code before this feature, run on the same machine; latency is the delivered pass and is dominated by the reranker, so it moves by a few hundred milliseconds between runs.

| configuration | code | per list | MRR@10 | Hit@1 | Hit@5 | latency mean / p95 (ms) |
|---|---|---:|---:|---:|---:|---:|
| lexical + reranker | dev | 30 | 0.9429 | 0.9286 | 0.9571 | 2050 / 3456 |
| lexical + reranker | this branch | 30 | 0.9429 | 0.9286 | 0.9571 | 2028 / 3552 |
| lexical + reranker | this branch | 50 | 0.9429 | 0.9286 | 0.9571 | 2139 / 3795 |
| dense bge-m3 + reranker | dev | 30 | 0.9643 | 0.9429 | 1.0000 | 1601 / 3482 |
| dense bge-m3 + reranker | this branch | 30 | 0.9714 | 0.9429 | 1.0000 | 1718 / 3592 |
| dense bge-m3 + reranker | this branch | 50 | 0.9714 | 0.9429 | 1.0000 | 1611 / 3700 |

- No regression against the committed baselines (0.9429 and 0.9714 MRR@10); the baselines are kept as they are.
- 50 candidates per list gain nothing on this corpus, so 30 stays for Fast and Deep.
- The dense `dev` run's 0.9643 against 0.9714 is the INT8 run-to-run variation recorded in `specs/002-local-embedding/research.md`, not this change: the ranking code path without a filter is the same.
- Latency: no measurable change. The lexical search takes milliseconds on this corpus, below the run-to-run noise of the reranker, so SC-003's expected drop cannot be seen here; it matters on large workspaces, where the per-question copy of the BM25 index (removed) and the sequential lexical search grew with the index.
