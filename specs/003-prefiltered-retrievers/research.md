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

To be filled by T009 and T019.
