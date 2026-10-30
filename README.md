# AnyContext (`actx`)

AnyContext is a terminal assistant that answers questions from your own folders, documents and documentation sites.

You point it at the material you work with; it indexes everything on your machine and answers with citations to the files it used. Each project lives in its own workspace with its own sources, settings and conversation memory, and the language model can be a cloud provider or a model running locally through Ollama.

[![Release](https://img.shields.io/github/v/release/Levix-Digital/any-context-releases?label=release)](https://github.com/Levix-Digital/any-context-releases/releases/latest)

## Features

- **Ask questions about your files**: source code, Markdown, PDF (including tables and forms), Word, PowerPoint, Excel, CSV, JSON, YAML and many more formats, with answers that cite the files used, and the line range for text and code files.
- **Index documentation websites**: add a URL and AnyContext crawls the site, including single-page documentation portals.
- **Workspaces**: separate projects with their own sources, model, answer settings and memory.
- **Fast answers or deep research**: simple questions get one quick search; complex ones are broken into sub-questions, searched in several rounds and answered with a combined summary.
- **Honest grounding**: in strict mode answers come only from your documents, and AnyContext says plainly when the answer is not there.
- **Local re-ranking**: a multilingual model on your machine judges every search's results, so only the relevant passages reach the answer.
- **Long-term memory**: each workspace remembers earlier conversations.
- **Full-screen terminal interface** with live sync progress, plus one-shot commands for scripts.

## Requirements

- Windows 10/11 (x86_64) or Linux (x86_64).
- One of:
  - an API key for OpenAI, Anthropic, Google Gemini, DeepSeek or Groq; or
  - [Ollama](https://ollama.com) running locally with a model pulled, for fully local answers.
- On Windows, the Microsoft Visual C++ Redistributable for the local re-ranker (without it, answers still work, in a less precise order, and say so).
- Internet access for installation, updates, cloud models and web sources.

## Installation

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.ps1 | iex
```

Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Levix-Digital/any-context/main/install.sh | bash
```

The installer downloads the latest release, checks your hardware, picks a profile (it installs the local re-ranker model on machines with enough memory) and adds `actx` to your `PATH`. Open a new terminal and check:

```bash
actx --version
```

You can also download [`actx-installer`](https://github.com/Levix-Digital/any-context-releases/releases/latest) from the releases page and run it with options:

| Option | Effect |
|---|---|
| `--profile=<tier>` | Choose the profile instead of the recommended one (`lite` installs no local models) |
| `--preflight-only` | Show the hardware check and the recommended profile, then exit |
| `--version=<tag>` | Install a specific release |
| `--dir=<path>` | Install into another folder |

## Quick Start

1. Start the interface:
   ```bash
   actx
   ```
   On the first run without an API key, a setup wizard asks for a model provider and key. You can open it again with `/onboarding`.
2. Add a folder to the current workspace (indexing starts in the background and the footer shows progress):
   ```text
   /folder C:\Projects\my-app
   ```
3. Ask a question:
   ```text
   How does the export command handle large files?
   ```
4. Or ask from any terminal without opening the interface:
   ```bash
   actx "Which settings does the export command read?"
   ```

## Usage

### Workspaces

| Command | What it does |
|---|---|
| `/switch` | List workspaces |
| `/switch <name>` | Switch to a workspace (it is created if it does not exist) |
| `/switch --delete <name>` | Delete a workspace and its indexed data |
| `/rename <old> <new>` | Rename a workspace |
| `/transfer <from> <to> <path-or-url>` | Move a source to another workspace |

### Sources and indexing

| Command | What it does |
|---|---|
| `/folder <path>` | Add a local folder and index it |
| `/folder --remove <path>` | Remove a folder |
| `/web <url>` | Add a website and crawl it on the next sync |
| `/web --remove <url>` | Remove a website |
| `/sources` | List the workspace's sources |
| `/sources rename <number-or-name> <new name>` | Give a source a friendlier name |
| `/sync` | Open the sync menu; `/sync --incremental` indexes what changed in every source and `/sync --force` re-indexes everything |
| `/cancel` | Stop the running sync |
| `/status`, `/info`, `/inspect` | Show index health, statistics and sample chunks (`/inspect <text>` filters them) |

Only new and changed files are processed on each sync. A folder that cannot be read (an unplugged drive, a network share or cloud drive that is not mounted) is reported offline and keeps its index until it is back; AnyContext never guesses another path for it. A workspace with no folder indexes nothing: add one with `/folder <path>`. Online-only cloud files (OneDrive, Dropbox, iCloud) are never downloaded by a sync: they are counted and left in the cloud, and indexed once you mark them "Always keep on this device".

If the embedding provider fails during a sync (an invalid key, a rate limit, a network error), the affected files are not embedded: the footer shows `⚠ N files not embedded (/sync)`, `/status` shows the first error, and keyword search still finds those files. The next sync tries them again. After an authentication error the sync stops calling the provider for the remaining files.

### Answers

| Command | What it does |
|---|---|
| `/mode strict` | Answer only from the workspace's documents (default) |
| `/mode hybrid` | Workspace facts first, then clearly labeled general knowledge |
| `/mode proactive` | Combine documents, web results and general knowledge, each labeled |
| `/search auto` | Choose quick or deep research per question (default) |
| `/search fast`, `/search deep` | Always use quick answers or deep research |
| `/fast <question>`, `/deep <question>` | Use one mode for a single question |
| `/web-search on` / `off` | Allow live web search for this workspace |
| `/history` | Show the workspace's stored conversation |
| `/reset-memory` | Forget the workspace's conversation |

### Models

| Command | What it does |
|---|---|
| `/model` | Choose the language model (cloud models, or `ollama/<model>` for a local one) |
| `/keys <provider> <key>` | Save an API key (`openai`, `anthropic`, `gemini`, `deepseek`, `groq`, `openrouter`, `mistral`) |
| `/preset balanced` / `frontier` | Apply a ready-made combination of model, answer mode and search depth |
| `/preset offline` / `privacy` | With a local Ollama model selected: strict answers, quick search, no web search |
| `/embedding` | Show the embedding model used to index and search, and the models you can choose |
| `/embedding <model> --yes` | Switch the embedding model; the affected workspaces are reindexed on their next sync |
| `/download reranker` | Install or repair the local re-ranker model |
| `/profile` | Show or change the hardware profile |

Local Gemma 4 models are listed as planned and cannot be used yet.

Search embeddings come from a local multilingual model that runs on your machine, chosen by your hardware profile: no key is needed and no document text leaves the computer. It is downloaded with the profile, or on the first sync if it is missing. The language model you pick with `/model` never changes it. `/embedding` shows the model in use and lets you switch to another local model or to a cloud one (`text-embedding-3-small`, `gemini-embedding-2`, or `nomic-embed-text` on Ollama); switching asks for `--yes`, lists the workspaces that will be reindexed and warns when document text would be sent to a provider. If the chosen model cannot run (a missing key, a failed download, a file that does not match its checksum), the sync stops before indexing and says how to fix it; AnyContext never indexes with another model. After an update, workspaces indexed with a cloud model keep that model.

Companies can serve the model files from their own server: set `ACTX_MODELS_MIRROR` (or the `models_mirror` setting) to its address and AnyContext downloads only from `<mirror>/<model>/<file>`, never from the public source. Files copied by hand into the models folder are used as they are, after their checksum is verified.

Questions about AnyContext itself are answered from its built-in documentation. A sync embeds that documentation with your embedding model when it needs it (after an install, an update or a change of embedding model), so it is matched by meaning; until then it is found by keyword.

### Other commands

`/config` (settings overview), `/logs` (recent log lines), `/diagnostics` (system health), `/menu` or `F1` (interactive menu), `/clear` (clear the screen), `/cd` and `/pwd` (working folder), `/version`, `/check-update`, `/update`, `/help`, `/exit`.

### Keyboard

| Keys | Action |
|---|---|
| `Enter` | Send |
| `Shift+Enter`, `Alt+Enter`, `Ctrl+J` | New line |
| `Up` / `Down` | Previous prompts of this workspace |
| `Tab` | Complete a command |
| `PageUp` / `PageDown`, mouse wheel | Scroll the conversation (`End` returns to live output) |
| `Ctrl+T` | Show or hide the reasoning and tool panel |
| `F1` or `Ctrl+M` | Menu |
| `Esc` | Close a menu or cancel a model download |
| `Ctrl+C` | Exit |

### Command line

| Command | What it does |
|---|---|
| `actx "<question>"` | Answer one question and print it (`-w <workspace>` and `-m <model>` to choose) |
| `actx --sync [--folder <path>] [--force]` | Index the current workspace |
| `actx download [reranker] [--check]` | Install local models or list them with a hardware check |
| `actx --diagnostics` | Print versions, paths and hardware |
| `actx --check-update`, `actx --update` | Check for or install the latest release (`actx --update@v1.2.3` for a specific one) |

## Configuration

Most settings are per workspace and change through commands (`/model`, `/mode`, `/search`, `/web-search`). API keys can also come from environment variables, which take priority over saved keys:

| Variable | Use |
|---|---|
| `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`, `GROQ_API_KEY`, `OPENROUTER_API_KEY`, `MISTRAL_API_KEY` | Model provider keys |
| `TAVILY_API_KEY` | Web search key (or `/keys tavily <key>`); without it, web search stays off |
| `JINA_API_KEY` | Higher limits for website extraction |
| `ACTX_MODEL` | Model for this shell, overriding the workspace setting |
| `ACTX_CONTEXT_DB`, `ACTX_SETTINGS_DB`, `ACTX_MODELS_DIR` | Custom locations for the index, settings and models |
| `ACTX_MODELS_MIRROR` | Server that local model files are downloaded from instead of the public source |
| `ACTX_SKIP_MODEL_DOWNLOAD=1` | Install or update without downloading local models |

Where AnyContext keeps its data:

| Data | Windows | Linux |
|---|---|---|
| Program | `%LOCALAPPDATA%\actx\bin` | `~/.local/bin` |
| Settings and memory | `%LOCALAPPDATA%\AnyContext\config` | `~/.local/share/any-context/config` |
| Index | `%LOCALAPPDATA%\AnyContext\data` | `~/.local/share/any-context/data` |
| Local models | `%LOCALAPPDATA%\AnyContext\models` | `~/.local/share/any-context/models` |
| Logs | `%LOCALAPPDATA%\AnyContext\logs` | `~/.local/share/any-context/logs` |

## Updating and Uninstalling

Update from a terminal (not from inside the interface):

```bash
actx --update
```

Re-index a workspace from scratch (switching the embedding model with `/embedding` reindexes on its own):

```bash
actx --sync --force -w <workspace>
```

To uninstall, download and run the uninstaller from the [releases page](https://github.com/Levix-Digital/any-context-releases/releases/latest) (`uninstall.ps1` on Windows, `uninstall.sh` on Linux). It asks whether to keep your workspaces and index.

## Troubleshooting

| Problem | Fix |
|---|---|
| Answers start with `Mock response to:` | No working model is configured: save a key with `/keys <provider> <key>` or select a local model with `/model ollama/<model>`. |
| "No information or documents matching the query" | The workspace has no relevant indexed content yet: check `/sources` and `/status`, wait for the sync to finish, or rephrase. |
| A sync reports `⚠ N source(s) offline, their index is kept` | The folder or a subfolder cannot be read: reconnect the drive, share or cloud mount and run `/sync`. If the folder moved, remove it with `/folder --remove <old path>` and add the new path with `/folder <new path>`. |
| A sync reports `online-only file(s) not indexed` | Those files are only in the cloud. Mark the folder or files "Always keep on this device" in OneDrive, Dropbox or iCloud and run `/sync`. |
| Web search answers `Web search is off: no search API key` | Save a Tavily key with `/keys tavily <key>`; `/web-search on` alone is not enough. |
| A sync reports `has no sources` | The workspace has no folder or site: add one with `/folder <path>` or `/web <url>`. |
| The sync seems stuck | Check the footer or `/status`; read `sync_<workspace>.log` in the logs folder; `/cancel` and `/sync --incremental` again. |
| The footer shows `⚠ N files not embedded (/sync)` | The embedding provider failed during the last sync: check the `Last Sync` line in `/status` (or `sync_<workspace>.log`), fix the key or wait for the rate limit, then run `/sync --incremental`. |
| `/status` shows `Dense Search: BM25 only` | The last question could not be embedded, so it was answered from keyword matches only; the reason follows. Check the embedding model with `/embedding` (and its key with `/keys` for a cloud model). |
| A sync stops with `Embedding model <model> is unavailable` | The chosen embedding model cannot run; the message gives the cause and the fix (download it with `actx download <model>`, copy its files into the models folder, save the provider key, or choose another model with `/embedding`). The index is unchanged. |
| `/status` shows `BM25 only (last query): indexed without an embedding model; sync to embed it` | The workspace was indexed by an older version without an embedding key. Run `/sync`: it is reindexed once with the local embedding model. |
| Answers cite files without line ranges | Files indexed by an earlier version have no section or line data until they change; run `/sync --force` once to re-index them. PDF, Office, spreadsheet and web sources are cited by page, sheet or section, never by line. |
| An answer ends with `Answered without relevance ranking`, or the header shows `⚠ Reranker unavailable` | The local re-ranker, which always ranks the search results, is not installed or could not load; `/status` shows the cause. Run `actx download reranker`, and on Windows install the Visual C++ Redistributable. |
| `actx --update` says the packages are still being built | A release was just published; try again in a few minutes. |
| An old version still runs after updating | Open a new terminal; the updater also replaces copies of `actx` found elsewhere in your `PATH`. |

For anything else, run `actx --diagnostics` and look at the logs folder.

## Privacy

- Your files, index, settings and memory stay on your machine. The text, summaries and keywords stored in the vector index are encrypted with a key bound to this machine, including data indexed by older versions, which is encrypted automatically after upgrading. The keyword search index keeps the words in plaintext.
- Secret values (API keys, tokens, passwords, private keys) are never indexed: they are stored and sent as `[SECRET REDACTED]`, while the rest of the file, including the key names, stays searchable. After upgrading, files that an older version indexed with secrets are reindexed once and the sync tells you how many; if you used a cloud model, rotate those credentials.
- When you use a cloud model, your question and the relevant excerpts of your documents are sent to that provider. With a local Ollama model, nothing leaves your machine.
- Website sources are fetched through the Jina Reader service (`r.jina.ai`) to turn pages into clean text; web search works only with your own Tavily key and, when turned on, sends your query to Tavily. Without a key nothing is searched.
- AnyContext sends no usage telemetry.

## Documentation

- [Technical documentation](TECDOC.md) for maintainers: architecture, subsystems and design decisions.
- [Release notes](https://github.com/Levix-Digital/any-context-releases/releases).

## License

This repository does not include a license file yet. Contact Levix Digital for usage and distribution terms.
