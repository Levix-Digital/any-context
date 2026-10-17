# Manual Tests

Curated manual test suite of AnyContext: one section per feature, each test describing how the software behaves today. Only what cannot be automated is tested here (real installation, the terminal interface, real model providers and websites, data on disk). Tests are edited in place when behavior changes and removed when they no longer apply.

How to record a run: in each test of the current run, mark exactly one box of the `Result` line with `x` and, for `Fail` or `Blocked`, describe what happened in `Notes`.

## Installation and Updates

### MT-INST-001: Fresh install puts actx on PATH with the recommended profile

- **Covers**: installer, install scripts
- **Prerequisites**: a machine without AnyContext (or after uninstalling); for the candidate, the pre-release tag to install
- **Steps**:
  1. Windows: `irm https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.ps1 | iex`; Linux: `curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.sh | bash`. To test a candidate, run the downloaded `actx-installer --version=<rc tag>` instead.
  2. Open a new terminal and run `actx --version`.
  3. Run `actx --diagnostics`.
- **Expected**:
  - The installer shows the hardware check with a recommended profile and finishes with a success message.
  - `actx --version` prints the installed version.
  - On a non-Lite profile, `%LOCALAPPDATA%\AnyContext\models\mmarco-reranker` (or `~/.local/share/any-context/models/mmarco-reranker`) contains `model.onnx` and `tokenizer.json`.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-INST-002: Lite profile installs no local models

- **Covers**: installer profiles
- **Prerequisites**: the release's `actx-installer`
- **Steps**:
  1. Run `actx-installer --preflight-only` and note the recommendation.
  2. Run `actx-installer --profile=lite`.
- **Expected**:
  - `--preflight-only` prints the profiles with green, yellow or red status and exits without installing.
  - The Lite install completes without downloading models; `/rerank` in `actx` reports the RRF fallback.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-INST-003: Updating replaces the running and shadowed binaries

- **Covers**: updater
- **Prerequisites**: an older release installed; on Windows, optionally a second `actx.exe` earlier in `PATH` (for example `~/.cargo/bin`)
- **Steps**:
  1. Run `actx --check-update`.
  2. Run `actx --update` (or `actx --update@<tag>`).
  3. Open a new terminal and run `actx --version`.
- **Expected**:
  - The check reports the newer release; the update finishes with a success message and writes to `update.log` in the logs folder.
  - The new terminal runs the new version, also through the shadowing copy.
  - The profile chosen at install time is kept: a Lite install still has no models.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-INST-004: /update inside the interface only gives instructions

- **Covers**: command engine (/update)
- **Prerequisites**: `actx` open
- **Steps**:
  1. Type `/update`.
- **Expected**:
  - The chat explains to exit and run `actx --update` in the terminal; nothing is downloaded and the screen is not corrupted.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-INST-005: Release candidates are not offered to normal users

- **Covers**: release pipeline, updater
- **Prerequisites**: a published `-rc` pre-release newer than the latest final release
- **Steps**:
  1. On a machine with the latest final release, run `actx --check-update`.
- **Expected**:
  - The check compares against the latest final release only; the candidate is not offered.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## First Run and Configuration

### MT-CONF-001: Onboarding wizard on the first run without credentials

- **Covers**: TUI onboarding
- **Prerequisites**: a fresh settings database (rename `settings.db`) and no provider key in the environment
- **Steps**:
  1. Run `actx`.
  2. Complete the three steps: provider and key, Document AI profile, optional first folder.
- **Expected**:
  - The wizard opens on its own, its options show no "recommended" bias, and finishing it returns to the chat with the chosen model active.
  - Running `actx` again does not open the wizard.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-CONF-002: API keys persist on a fresh database

- **Covers**: /keys, settings database
- **Prerequisites**: a fresh settings database and no `OPENAI_API_KEY` in the environment
- **Steps**:
  1. In `actx`, run `/keys openai <your key>`.
  2. Exit, open `actx` again and ask a question with an OpenAI model selected.
- **Expected**:
  - The command answers `API key saved for provider 'openai' (active in runtime & persisted to SQLite)`.
  - After the restart the answer comes from OpenAI (not `Mock response to: ...`).
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-CONF-003: Presets apply their settings and the offline presets require a local model

- **Covers**: /preset
- **Prerequisites**: a cloud model active; Ollama running with a pulled model for step 3
- **Steps**:
  1. Run `/preset balanced`, then `/preset frontier`.
  2. Run `/preset offline` with the cloud model still active.
  3. Run `/model ollama/<model>`, then `/preset offline` and `/kit privacy`.
- **Expected**:
  - `balanced` sets `gpt-4o-mini`, hybrid grounding, auto search and web search on; `frontier` sets `claude-3-7-sonnet`, proactive grounding, deep search and web search on; the header updates at once.
  - With a cloud model, `offline` fails, explains that local Gemma 4 is planned, suggests `/model ollama/<model>` and changes nothing.
  - With the Ollama model, both presets succeed, keep the model, and set strict grounding, fast search and web search off.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Workspaces

### MT-WS-001: Workspaces keep their own sources, settings, chat and prompt history

- **Covers**: workspaces, TUI chat buffers
- **Prerequisites**: two workspaces with different folders and models
- **Steps**:
  1. In workspace A, ask a question and set `/mode hybrid`.
  2. Run `/switch B`, ask a question, then press `Up`.
  3. Run `/switch A`.
- **Expected**:
  - In B the header shows B's model and grounding, the chat starts with B's welcome banner, and `Up` recalls only B's prompts.
  - Back in A, A's conversation is shown again with hybrid grounding; answers never cite B's files.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-WS-002: Deleting, renaming and transferring

- **Covers**: /switch --delete, /rename, /transfer
- **Prerequisites**: workspaces A and B, a folder indexed in A
- **Steps**:
  1. `/transfer A B <folder>`, then `/sources` in both.
  2. `/rename B C`.
  3. `/switch --delete C`.
- **Expected**:
  - The folder moves from A to B; the rename keeps C's sources; the deletion removes C and its indexed data, and `/switch` no longer lists it.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Sources and Sync

### MT-SYNC-001: Adding a folder indexes only that folder in the background

- **Covers**: /folder, sync worker, footer telemetry
- **Prerequisites**: a workspace with another large folder already indexed
- **Steps**:
  1. Run `/folder <small folder>`.
  2. Watch the footer and the header while it runs, and type a question meanwhile.
  3. Run `/status` and `/sources`.
- **Expected**:
  - The footer shows a spinner and progress, and the interface stays responsive; no worker output appears on the screen.
  - Only the new folder is scanned (the log `sync_<workspace>.log` lists only its files).
  - At the end the header and footer show `Up to date` and `/status` counts the new chunks.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-SYNC-002: Changed, moved and deleted files are reflected after a sync

- **Covers**: differential sync
- **Prerequisites**: an indexed folder with a few text files
- **Steps**:
  1. Edit one file, rename another, delete a third.
  2. Wait for the header to show `Changes detected (/sync)`, then run `/sync --incremental`.
  3. Ask about content that existed only in the deleted file.
- **Expected**:
  - The change indicator appears within a few seconds; after the sync it returns to `Up to date`.
  - Answers use the edited content, cite the renamed file under its new name, and never cite the deleted file.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-SYNC-003: Cancelling a sync

- **Covers**: /cancel
- **Prerequisites**: a large folder not indexed yet
- **Steps**:
  1. Run `/folder <large folder>`, then `/cancel` while it runs.
- **Expected**:
  - The footer shows `Sync Cancelled`, the worker process ends, and a later `/sync --incremental` resumes from the files not yet indexed.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-SYNC-004: Unreachable and slightly moved folders

- **Covers**: source health, path healing
- **Prerequisites**: an indexed folder you can rename, and one under `Documents` (or a cloud drive mount)
- **Steps**:
  1. Rename the first folder; wait a few seconds.
  2. Register the second folder with an extra or missing `Documents` segment in the path.
- **Expected**:
  - The header and footer show `folder(s) unreachable` for the renamed folder.
  - The second folder is found under its real path and indexed; `/sources` shows the corrected path.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-SYNC-005: Source display names

- **Covers**: /sources rename
- **Prerequisites**: a workspace with a folder and a web source
- **Steps**:
  1. Run `/sources` and `/sources rename 1 "Billing service"`.
  2. Ask a question answered by that source.
- **Expected**:
  - `/sources` shows descriptive automatic names (project or site names), then the new name, instantly and without re-indexing.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Document Formats

### MT-DOC-001: PDFs with tables and forms are answered exactly

- **Covers**: PDF chunker, quality gate, spatial forms
- **Prerequisites**: a real tax form, invoice or report PDF with a table and labeled fields
- **Steps**:
  1. Index its folder and ask for two specific values (a total and a labeled field).
  2. Run `/inspect <a word from the table>`.
- **Expected**:
  - The answers give the exact values and cite the PDF and page.
  - `/inspect` shows the table rebuilt as Markdown rows, without swapped labels and values.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DOC-002: Office, spreadsheet and structured files

- **Covers**: Office, tabular and structured chunkers
- **Prerequisites**: a folder with a `.docx`, a `.pptx`, an `.xlsx` with a header row, a `.csv` and a `.json`
- **Steps**:
  1. Index the folder and ask one question per file.
- **Expected**:
  - Each answer cites the right file; spreadsheet answers use the header names; slide answers mention the slide content.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DOC-003: Source code is found by symbol

- **Covers**: AST code chunkers
- **Prerequisites**: a repository in a supported language
- **Steps**:
  1. Index it and ask "where is <function name> defined and what does it do?".
- **Expected**:
  - The answer cites the file and the line range of the function.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DOC-004: Scanned images and diagrams

- **Covers**: image chunker, OCR
- **Prerequisites**: a scanned page image and a diagram image; Tesseract installed for the first part
- **Steps**:
  1. Index both and run `/inspect` and `/ocr`.
- **Expected**:
  - With Tesseract, the scan becomes text that can be asked about; the diagram is listed as a visual diagram with its resolution.
  - Without Tesseract, nothing fails; `/ocr` explains the status.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Web Sources

### MT-WEB-001: A documentation site is crawled and answered from

- **Covers**: crawler, Jina Reader
- **Prerequisites**: internet access
- **Steps**:
  1. Run `/web https://docs.rs/tokio/latest/tokio/` and `/sync --incremental`.
  2. Ask a question answered on one of its pages.
- **Expected**:
  - The footer shows web progress; the answer cites the page URL and code blocks are preserved.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-WEB-002: Re-syncing a web source never duplicates chunks

- **Covers**: web sync
- **Prerequisites**: MT-WEB-001 done
- **Steps**:
  1. Run `/status` and note the chunk count.
  2. Run `/sync --incremental` twice, then `/status`.
- **Expected**:
  - The chunk count is the same after each sync, and the second sync is faster because unchanged pages are skipped.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-WEB-003: Live web search

- **Covers**: /web-search
- **Prerequisites**: a cloud model
- **Steps**:
  1. Run `/web-search on` and ask about a recent public event.
  2. Run `/web-search off` and ask again.
- **Expected**:
  - With web search on, the answer cites web results; off, it answers only from the workspace or says the information is not there.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Answers

### MT-ANS-001: Strict grounding answers only from documents and says when nothing is found

- **Covers**: grounding modes, Zero Hallucination Gate
- **Prerequisites**: an indexed workspace in strict mode
- **Steps**:
  1. Ask something the documents answer.
  2. Ask a general question the documents do not cover.
  3. Repeat question 2 with `/mode hybrid`.
- **Expected**:
  - Answer 1 cites the source files at the end.
  - Answer 2 is an English "No information or documents matching the query ..." message listing the workspace's sources and suggesting `/folder`, `/web-search on` or rephrasing, with no invented content.
  - In hybrid mode, workspace facts come first and general knowledge is clearly labeled.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-ANS-002: Quick answers and deep research

- **Covers**: query routing, Deep Search
- **Prerequisites**: an indexed code or documentation workspace
- **Steps**:
  1. With `/search auto`, ask a short factual question, then a broad comparison question.
  2. Open the reasoning panel with `Ctrl+T` during the second answer.
- **Expected**:
  - The first shows the Fast RAG route and streams quickly; the second shows the Deep Search route, its sub-questions and iterations in the panel, and a combined answer with citations.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-ANS-003: Answers follow the question's language

- **Covers**: prompts
- **Prerequisites**: an indexed workspace with English documents
- **Steps**:
  1. Ask the same question in English and in Portuguese, in both fast and deep mode.
- **Expected**:
  - Each answer is written in the language of its question; the interface and messages stay in English.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-ANS-004: Every provider answers and streams

- **Covers**: language model providers
- **Prerequisites**: keys for the providers you use; Ollama for the local case
- **Steps**:
  1. For each of OpenAI, Anthropic, Gemini, DeepSeek, Groq and `ollama/<model>`, select it with `/model` and ask a question that needs the workspace.
- **Expected**:
  - Every answer streams token by token, uses the workspace documents and finishes without HTTP errors.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Local Re-Ranking and Models

### MT-MOD-001: The local re-ranker improves ordering and reports its state

- **Covers**: ONNX re-ranker, /rerank
- **Prerequisites**: the re-ranker installed (non-Lite profile or `actx download reranker`)
- **Steps**:
  1. Run `/rerank` and check the header badge.
  2. Ask a question whose answer is in one of several similar files.
  3. Run `/rerank off`, ask again, then `/rerank auto`.
- **Expected**:
  - `/rerank` shows the model, the CPU device and a measured latency; the badge shows the ONNX model.
  - With re-ranking off the badge shows Off and answers still work.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-MOD-002: Model downloads with progress, cancellation and repair

- **Covers**: /download, actx download
- **Prerequisites**: the re-ranker not installed (delete its folder)
- **Steps**:
  1. In `actx`, run `/download reranker`, then press `Esc` during the download.
  2. Run `actx download reranker` in a terminal and let it finish.
  3. Run `actx download --check`.
- **Expected**:
  - The footer shows MB, percentage, speed and ETA; `Esc` cancels and leaves no `.download.tmp` file.
  - The terminal download shows a progress bar and verifies the files; `--check` lists the re-ranker as installed, and Gemma 4 entries as planned.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-MOD-003: Hardware guardrails for local models

- **Covers**: /model, /profile, hardware profiler
- **Prerequisites**: none
- **Steps**:
  1. Open the model menu (`F1`) and look at the badges.
  2. Run `/profile` and `/profile lite`.
- **Expected**:
  - Model badges read `Compatible`, `Swap Risk` or `Insufficient Hardware - Requires X GB` according to the machine's RAM; Gemma entries are marked planned.
  - `/profile` lists the profiles with their status and switching to Lite takes effect.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Memory

### MT-MEM-001: Long-term memory across sessions

- **Covers**: session memory, /history, /reset-memory
- **Prerequisites**: a cloud or Ollama model
- **Steps**:
  1. Tell the assistant a fact ("our deploy window is Thursday 18:00"), exit, and open `actx` again.
  2. Ask "when is our deploy window?".
  3. Run `/history`, then `/reset-memory`, and ask again.
- **Expected**:
  - The new session starts with a clean screen, yet the assistant remembers the fact.
  - `/history` shows the stored turns; after `/reset-memory` the chat is cleared and the fact is forgotten.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-MEM-002: The assistant knows how AnyContext works

- **Covers**: system knowledge (Global workspace), system_status tool
- **Prerequisites**: a sync running in the background
- **Steps**:
  1. Ask "how do I add a website to this workspace?".
  2. Ask "is everything indexed yet?".
- **Expected**:
  - The first answer explains `/web` from the built-in manual; the second reports the live sync progress and chunk count.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Terminal Interface

### MT-TUI-001: Input, scrolling and the reasoning panel

- **Covers**: TUI input and viewport
- **Prerequisites**: `actx` open
- **Steps**:
  1. Type text with accents and emoji, move the cursor with the arrow keys and edit in the middle.
  2. Write a multi-line prompt with `Shift+Enter` and `Ctrl+J`, and send it.
  3. During a long answer, scroll up with `PageUp` or the mouse wheel, then press `End`.
  4. Press `Ctrl+T` twice.
- **Expected**:
  - No crash; the cursor stays aligned with accented characters; the input box grows up to 8 lines.
  - Scrolling pauses auto-scroll with a "scroll paused" hint; `End` returns to live output.
  - The reasoning panel is collapsed by default and opens and closes with `Ctrl+T`.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-TUI-002: Header, footer, palette and menus

- **Covers**: TUI rendering, slash palette, F1 menu
- **Prerequisites**: `actx` open in a workspace with sources
- **Steps**:
  1. Look at the header and footer; type `/` and then `/sy`.
  2. Press `F1` and open each submenu, going back with `Esc`.
- **Expected**:
  - The header shows workspace, sync, model, accelerator, grounding, search, re-rank and web search; colors follow the Aurora theme, with yellow and red only for real problems.
  - The palette lists commands and filters them; `Tab` completes.
  - Every submenu opens and closes without leaving artifacts on the screen; all text is in English.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-TUI-003: Errors are always shown

- **Covers**: TUI error surfacing
- **Prerequisites**: a cloud model
- **Steps**:
  1. Disconnect the network (or use an invalid key with `/keys`) and ask a question.
- **Expected**:
  - A system message explains the error and the header shows the error status; the interface does not freeze.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Command Line

### MT-CLI-001: One-shot questions and headless sync

- **Covers**: CLI
- **Prerequisites**: an indexed workspace
- **Steps**:
  1. `actx "<question>" -w <workspace>`.
  2. `echo "<question>" | actx -w <workspace>` (piped input).
  3. `actx --sync -w <workspace>` and `actx --sync --force -w <workspace>`.
  4. `actx --diagnostics`.
- **Expected**:
  - The answers stream to stdout with the routing badge and citations; piped input runs headless.
  - The sync prints its summary; `--force` re-indexes everything.
  - Diagnostics print version, platform, hardware, accelerators and database paths.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

## Privacy and Data

### MT-DATA-001: Indexed text is encrypted on disk

- **Covers**: encryption at rest
- **Prerequisites**: a file containing a unique phrase such as `ZEBRA-CANARY-4711`
- **Steps**:
  1. Index it, then run `/inspect ZEBRA`.
  2. Search the vector table files: `Get-ChildItem "$env:LOCALAPPDATA\AnyContext\data\context_db\lancedb\workspace_chunks.lance" -Recurse -File | Select-String -Pattern "ZEBRA-CANARY-4711" -SimpleMatch -List` (Linux: `grep -rl ZEBRA-CANARY-4711 ~/.local/share/any-context/data/context_db/lancedb/workspace_chunks.lance`).
- **Expected**:
  - `/inspect` shows the decrypted chunk and questions about it are answered.
  - The search in the vector table files finds no plaintext match.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DATA-004: Data indexed by an older version is encrypted after upgrading

- **Covers**: automatic encryption of chunks stored in plaintext
- **Prerequisites**: AnyContext v0.44.0 installed; a folder with a file containing `ZEBRA-CANARY-4711`
- **Steps**:
  1. With v0.44.0, index the folder, then run the table search of MT-DATA-001 step 2 and note that it finds the phrase.
  2. Install the candidate without changing the file, open `actx` (do not sync) and wait a few seconds.
  3. Run the table search again, then run `/inspect ZEBRA` and ask a question about the file.
  4. Run `actx --sync` from a terminal.
- **Expected**:
  - Step 3: the search finds no plaintext match; `/inspect` shows the decrypted chunk and the question is answered.
  - Step 4: the sync reports no chunks to encrypt (no "Encrypted N chunks" line) and does not re-index the unchanged file.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DATA-002: Fully local operation with Ollama

- **Covers**: local providers and embeddings
- **Prerequisites**: Ollama with a chat model and `nomic-embed-text` pulled; network access blocked except to localhost
- **Steps**:
  1. Select `ollama/<model>`, run `/web-search off`, index a folder and ask questions.
- **Expected**:
  - Indexing and answers work with no outgoing connections other than localhost.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:

### MT-DATA-003: Uninstalling

- **Covers**: uninstall scripts
- **Prerequisites**: AnyContext installed
- **Steps**:
  1. Download and run `uninstall.ps1` (Windows) or `uninstall.sh` (Linux) from the latest release, choosing to keep the data; run `actx --version` in a new terminal.
  2. Reinstall, then uninstall choosing to remove the data.
- **Expected**:
  - After the first uninstall `actx` is no longer found but the data folder remains; after the second, the data folder is gone and `PATH` no longer has the bin folder.
- **Result**: [ ] Pass [ ] Fail [ ] Blocked
- **Notes**:
