# Contract: embedding model commands

## TUI slash command `/embedding [id]`

- No argument: prints the model in use (id, local or cloud, dimension, prefixes on or off), whether it comes from the setting or from the profile default, and the catalog models that can be picked, with size and whether they are installed.
- With an id: validates it against the catalog, then asks for confirmation with:
  - "Workspaces indexed with <old> will be reindexed on their next sync: <list>."
  - For a cloud model: "Document text will be sent to <provider> to build the index."
  On confirmation, stores the setting and marks the affected workspaces as reindex pending. Unknown id: error listing the valid ids.

## Headless `actx embedding [id] [--yes]`

Same output as the slash command; with an id, prints the same notices and requires `--yes` to apply (exit code 2 without it). Exit code 1 for an unknown id.

## Unchanged surfaces, changed behavior

- `/model`, `-m/--model`: change only the chat model; no embedding notice anymore.
- `actx --sync`: reports the embedding model used; on `EmbeddingUnavailable` exits non-zero with the message `Embedding model <id> is unavailable: <cause>. <fix>` and writes nothing.
- `/models`: lists embedding models under their own category.

## Benchmark `actx-bench retrieval run --embedding <key>`

Accepts `local/<id>` (no key needed) as well as the cloud keys; the run configuration records the full target key (with dimension and prefix revision).
