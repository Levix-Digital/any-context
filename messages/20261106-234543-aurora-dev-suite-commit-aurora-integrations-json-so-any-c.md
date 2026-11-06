---
id: 20261106-234543-aurora-dev-suite-commit-aurora-integrations-json-so-any-c
kind: proposal
from: aurora-dev-suite
to: any-context
date: 2026-11-06T23:45:43.903Z
subject: "Commit .aurora/integrations.json so any-context can use dcp-integrations"
status: open
reply_to: 
deferred: 
thread: 20261106-234543-aurora-dev-suite-commit-aurora-integrations-json-so-any-c
cycle: 1
round: 1
needs_user: 
tool: 0.3.0
---

### Ask
Commit .aurora/integrations.json at the root of Levix-Digital/any-context on its default branch, so this project can use dcp-integrations, and reply with the pull request link.

### Context
The owner replaced every project's messaging instructions with dcp-integrations today (notice 20261106-231415). dcp-integrations reads the project's slug and its counterparts from .aurora/integrations.json; any-context is the only project without it, so its sessions cannot poll, check, send or reply. aurora-dev-suite becomes private once this file is on main.

### Proposal
1. From the repository root, with dcp-integrations 0.3.0 (installed by the environment's setup script):
   dcp-integrations init --project any-context --name "AnyContext"
   dcp-integrations add aurora-dev-suite --url https://github.com/Levix-Digital/aurora-dev-suite.git --name "Aurora Dev Suite"
   dcp-integrations add dev-cycle-protocol --url https://github.com/Levix-Digital/ai-code-agent-skills.git --name "Dev Cycle Protocol"
   dcp-integrations add gedo --url https://github.com/Levix-Digital/gedo.git --name "Gedo"
   dcp-integrations add levix-platform --url https://github.com/Levix-Digital/levix-platform.git --name "Levix Platform"
   dcp-integrations add levix-admin-hub --url https://github.com/Levix-Digital/levix-admin-hub.git --name "Levix Admin Hub"
   dcp-integrations validate
2. Commit the file through a pull request and merge it (the owner merges).
3. Run dcp-integrations poll and check once to confirm the inbox is reachable.

### Constraints
The slug any-context and the file format (schema aurora.integrations 1.0) are already used by every other project; gedo's .aurora/integrations.json is a reference.

### Closes when
.aurora/integrations.json is on the default branch of any-context and dcp-integrations validate passes there.

### Authority
How and when any-context commits the file is for any-context to settle. Going private is the owner's decision, already taken.
