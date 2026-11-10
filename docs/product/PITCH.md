# AnyContext

**Ask your documents without them leaving home.**

AnyContext is an assistant that answers questions from your own documents, citing the source of every answer, without the documents ever leaving your computer or your company.

## The problem

Analysts, lawyers, doctors, engineers and researchers spend a large part of the day looking for things in their own documents:

- **Search returns files, not answers.** You find the right folder, then open each file and read.
- **Chat tools want your documents.** General assistants and cloud notebooks answer only after the documents are uploaded to a third party, which many security policies forbid.
- **Answers you cannot check.** A confident answer with no source, or an invented one, is worse than no answer.
- **On-premise search is heavy.** Enterprise search products that stay inside the company are expensive to deploy and still return lists of files.

## The solution

AnyContext indexes the material you work with on your own machine and answers in plain language, citing the files it used. Nothing leaves the machine except the call to the language model you choose, and with a local model nothing leaves at all. When the answer is not in your documents, it says so instead of guessing.

**Point it at your folders → it indexes them locally → ask → get an answer with its sources**

## What it does today

- **Answers from your files**: source code, Markdown, PDF (including tables and forms), Word, PowerPoint, Excel, CSV, JSON, YAML and many more formats, with the files used cited in every answer.
- **Documentation websites**: add a URL and AnyContext crawls and indexes the site.
- **Fast answers or deep research**: simple questions get one quick search; complex ones are broken into sub-questions and searched in several rounds.
- **Honest grounding**: in strict mode answers come only from your documents, and AnyContext says plainly when the answer is not there.
- **Local re-ranking**: a multilingual model on your machine reorders the search results for more precise answers.
- **Workspaces with memory**: separate projects with their own sources, model, settings and conversation history.
- **Your choice of model**: OpenAI, Anthropic, Google Gemini, DeepSeek or Groq, or a fully local model through Ollama.
- **Terminal interface and CLI** on Windows and Linux.

## Why AnyContext

- **Private by design.** Your documents and index stay on your machine. No telemetry.
- **Answers you can check.** Every answer cites its sources, and "not found" is a valid answer.
- **Works offline.** With a local model, nothing leaves the computer at all.
- **Free for one person.** Complete for personal use or a business run by one person, with no registration.

## Coming soon

- **Desktop app** for Windows, Linux and macOS.
- **Citations to the exact lines** of the passage behind each statement.
- **An MCP server** so your coding and automation agents can query your documentation through AnyContext.
- **More sources**: scanned documents, e-mail, archives, audio and video.
- **For organizations**: shared workspaces, policies, single sign-on and auditing through the Levix Admin Hub, deployed on one machine, in your private cloud or fully offline.

## The vision

AnyContext is the document line of Levix Digital, and the context engine of the **Aurora Dev Suite: Dev Cycle Protocol + Spec Kit + Gedo + AnyContext**.

## Get started

Install with one command on Windows or Linux, add a folder and ask your first question: https://github.com/Levix-Digital/any-context-releases
